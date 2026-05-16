use serde::{Deserialize, Serialize};

use sphare_core_common::common::Rule;

use crate::comment::Comment;
use crate::post::Post;

#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum Content {
    Post(Post),
    Comment(Comment),
}

#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct ModerationInfo {
    pub rule: Rule,
    pub content: Content,
}

#[cfg(feature = "ssr")]
pub mod ssr {
    use sqlx::PgPool;
    use sphare_core_common::checks::check_string_length;
    use sphare_core_common::constants::MAX_MOD_MESSAGE_LENGTH;
    use sphare_core_common::errors::AppError;
    use sphare_core_sphere::rule::ssr::load_rule_by_id;
    use sphare_core_user::notification::{Notification, NotificationType};
    use sphare_core_user::notification::ssr::create_notification;
    use sphare_core_user::role::{AdminRole, PermissionLevel};
    use sphare_core_user::role::ssr::is_user_sphere_moderator;
    use sphare_core_user::user::{User, UserBan};

    use crate::comment::Comment;
    use crate::comment::ssr::{get_comment_by_id, get_comment_sphere};
    use crate::moderation::{Content, ModerationInfo};
    use crate::post::Post;
    use crate::post::ssr::get_post_by_id;

    pub async fn get_moderation_info(
        post_id: i64,
        comment_id: Option<i64>,
        db_pool: &PgPool,
    ) -> Result<ModerationInfo, AppError> {
        let (rule_id, content) = match comment_id {
            Some(comment_id) => {
                let comment = get_comment_by_id(comment_id, db_pool).await?;
                (comment.infringed_rule_id, Content::Comment(comment))
            },
            None => {
                let post = get_post_by_id(post_id, db_pool).await?;
                (post.infringed_rule_id, Content::Post(post))
            },
        };
        let rule = match rule_id {
            Some(rule_id) => load_rule_by_id(rule_id, db_pool).await,
            None => Err(AppError::InternalServerError(String::from("Content is not moderated, cannot find moderation info.")))
        }?;

        Ok(ModerationInfo {
            rule,
            content,
        })
    }

    pub async fn moderate_post_and_ban_user(
        post_id: i64,
        rule_id: i64,
        moderator_message: &str,
        ban_duration_days: Option<usize>,
        user: &User,
        db_pool: &PgPool,
    ) -> Result<(Post, Option<UserBan>, Option<Notification>), AppError> {
        log::debug!("Moderate post {post_id}, ban duration = {ban_duration_days:?}");
        check_string_length(moderator_message, "Moderator message", MAX_MOD_MESSAGE_LENGTH, true)?;

        let post = moderate_post(
            post_id,
            rule_id,
            moderator_message,
            user,
            db_pool
        ).await?;

        let user_ban = ban_user_from_sphere(
            post.creator_id,
            post.sphere_id,
            post.post_id,
            None,
            rule_id,
            ban_duration_days,
            user,
            db_pool,
        ).await?;

        let notif = match create_notification(
            post.post_id,
            None,
            None,
            user.user_id,
            NotificationType::Moderation,
            db_pool,
        ).await {
            Ok(notif) => notif,
            Err(e) => {
                log::error!("Failed to notify user for the moderation of post {}, error: {e}", post.post_id);
                None
            },
        };

        Ok((post, user_ban, notif))
    }

    pub async fn moderate_post(
        post_id: i64,
        rule_id: i64,
        moderator_message: &str,
        user: &User,
        db_pool: &PgPool,
    ) -> Result<Post, AppError> {
        let post = if user.check_admin_role(AdminRole::Moderator).is_ok() {
            sqlx::query_as::<_, Post>(
                "WITH moderated_post AS (
                    UPDATE posts SET
                        moderator_message = $1,
                        infringed_rule_id = $2,
                        edit_timestamp = NOW(),
                        moderator_id = $3
                    WHERE
                        post_id = $4
                    RETURNING *
                )
                SELECT
                    p.*,
                    u.username as creator_name,
                    $5 as moderator_name,
                    r.title as infringed_rule_title,
                    r.sphere_id IS NOT NULL AS is_sphere_rule
                FROM moderated_post p
                JOIN users u ON u.user_id = p.creator_id
                JOIN rules r ON r.rule_id = p.infringed_rule_id",
            )
                .bind(moderator_message)
                .bind(rule_id)
                .bind(user.user_id)
                .bind(post_id)
                .bind(user.username.clone())
                .fetch_one(db_pool)
                .await?
        } else {
            sqlx::query_as::<_, Post>(
                "WITH moderated_post AS (
                    UPDATE posts p SET
                        moderator_message = $1,
                        infringed_rule_id = $2,
                        edit_timestamp = NOW(),
                        moderator_id = $3
                    WHERE
                        p.post_id = $4 AND
                        EXISTS (
                            SELECT * FROM user_sphere_roles r
                            WHERE
                                r.sphere_id = p.sphere_id AND
                                r.user_id = $3 AND
                                r.permission_level != 'None'
                        )
                    RETURNING *
                )
                SELECT
                    p.*,
                    u.username as creator_name,
                    $5 as moderator_name,
                    r.title as infringed_rule_title,
                    r.sphere_id IS NOT NULL AS is_sphere_rule
                FROM moderated_post p
                JOIN users u ON u.user_id = p.creator_id
                JOIN rules r ON r.rule_id = p.infringed_rule_id",
            )
                .bind(moderator_message)
                .bind(rule_id)
                .bind(user.user_id)
                .bind(post_id)
                .bind(user.username.clone())
                .fetch_one(db_pool)
                .await?
        };

        Ok(post)
    }

    pub async fn moderate_comment_and_ban_user(
        comment_id: i64,
        rule_id: i64,
        moderator_message: &str,
        ban_duration_days: Option<usize>,
        user: &User,
        db_pool: &PgPool,
    ) -> Result<(Comment, Option<UserBan>, Option<Notification>), AppError> {
        log::trace!("Moderate comment {comment_id}");
        check_string_length(moderator_message, "Moderation message", MAX_MOD_MESSAGE_LENGTH, false)?;

        let comment = moderate_comment(
            comment_id,
            rule_id,
            moderator_message,
            user,
            db_pool
        ).await?;

        let sphere = get_comment_sphere(comment_id, &db_pool).await?;

        let user_ban = ban_user_from_sphere(
            comment.creator_id,
            sphere.sphere_id,
            comment.post_id,
            Some(comment.comment_id),
            rule_id,
            ban_duration_days,
            user,
            db_pool
        ).await?;

        let notif = match create_notification(
            comment.post_id,
            Some(comment.comment_id),
            Some(comment.comment_id),
            user.user_id,
            NotificationType::Moderation,
            db_pool
        ).await {
            Ok(notif) => notif,
            Err(e) => {
                log::error!("Failed to notify user for the moderation of comment {}, error: {e}", comment.comment_id);
                None
            },
        };

        Ok((comment, user_ban, notif))
    }

    pub async fn moderate_comment(
        comment_id: i64,
        rule_id: i64,
        moderator_message: &str,
        user: &User,
        db_pool: &PgPool,
    ) -> Result<Comment, AppError> {
        let comment = if user.check_admin_role(AdminRole::Moderator).is_ok() {
            sqlx::query_as::<_, Comment>(
                "WITH moderated_comment AS (
                        UPDATE comments SET
                        moderator_message = $1,
                        infringed_rule_id = $2,
                        edit_timestamp = NOW(),
                        moderator_id = $3
                    WHERE
                        comment_id = $4
                    RETURNING *
                )
                SELECT
                    c.*,
                    u.username as creator_name,
                    $5 as moderator_name,
                    r.title as infringed_rule_title,
                    r.sphere_id IS NOT NULL AS is_sphere_rule
                FROM moderated_comment c
                JOIN users u ON u.user_id = c.creator_id
                JOIN rules r ON r.rule_id = c.infringed_rule_id",
            )
                .bind(moderator_message)
                .bind(rule_id)
                .bind(user.user_id)
                .bind(comment_id)
                .bind(user.username.clone())
                .fetch_one(db_pool)
                .await?
        } else {
            // check if the user has at least the moderate permission for this sphere
            sqlx::query_as::<_, Comment>(
                "WITH moderated_comment AS (
                    UPDATE comments c SET
                        moderator_message = $1,
                        infringed_rule_id = $2,
                        edit_timestamp = NOW(),
                        moderator_id = $3
                    WHERE
                        c.comment_id = $4 AND
                        EXISTS (
                            SELECT * FROM user_sphere_roles r
                            JOIN posts p ON p.sphere_id = r.sphere_id
                            WHERE
                                p.post_id = c.post_id AND
                                r.user_id = $3  AND
                                r.permission_level != 'None'
                        )
                    RETURNING *
                )
                SELECT
                    c.*,
                    u.username as creator_name,
                    $5 as moderator_name,
                    r.title as infringed_rule_title,
                    r.sphere_id IS NOT NULL AS is_sphere_rule
                FROM moderated_comment c
                JOIN users u ON u.user_id = c.creator_id
                JOIN rules r ON r.rule_id = c.infringed_rule_id",
            )
                .bind(moderator_message)
                .bind(rule_id)
                .bind(user.user_id)
                .bind(comment_id)
                .bind(user.username.clone())
                .fetch_one(db_pool)
                .await?
        };

        Ok(comment)
    }

    pub async fn ban_user_from_sphere(
        user_id: i64,
        sphere_id: i64,
        post_id: i64,
        comment_id: Option<i64>,
        rule_id: i64,
        ban_duration_days: Option<usize>,
        user: &User,
        db_pool: &PgPool,
    ) -> Result<Option<UserBan>, AppError> {
        if user.check_sphere_permissions_by_id(sphere_id, PermissionLevel::Moderate).is_ok() &&
            user.user_id != user_id &&
            !is_user_sphere_moderator(user_id, sphere_id, db_pool).await?
        {
            let user_ban = match ban_duration_days {
                Some(0) => None,
                ban_duration => {
                    Some(
                        sqlx::query_as!(
                            UserBan,
                            "WITH ban AS (
                                INSERT INTO user_bans (user_id, sphere_id, post_id, comment_id, infringed_rule_id, moderator_id, until_timestamp)
                                 VALUES (
                                    $1, $2, $3, $4, $5, $6, NOW() + $7 * interval '1 day'
                                ) RETURNING *
                            )
                            SELECT b.*, u.username, s.sphere_name FROM ban b
                            JOIN users u ON u.user_id = b.user_id
                            JOIN spheres s ON s.sphere_id = b.sphere_id",
                            user_id,
                            sphere_id,
                            post_id,
                            comment_id,
                            rule_id,
                            user.user_id,
                            ban_duration.map(|duration| duration as f64),
                        )
                            .fetch_one(db_pool)
                            .await?
                    )
                }
            };
            Ok(user_ban)
        } else {
            Err(AppError::InternalServerError(format!("Error while trying to ban user {user_id}. Insufficient permissions or user is a moderator of the sphere.")))
        }
    }
}