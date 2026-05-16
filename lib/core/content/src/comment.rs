use serde::{Deserialize, Serialize};

use sphare_core_common::common::SphereHeader;

use crate::post::Post;
use crate::ranking::Vote;

#[cfg_attr(feature = "ssr", derive(sqlx::FromRow))]
#[derive(Clone, Debug, Default, PartialEq, Eq, Ord, PartialOrd, Serialize, Deserialize)]
pub struct Comment {
    pub comment_id: i64,
    pub body: String,
    pub markdown_body: Option<String>,
    pub is_edited: bool,
    pub moderator_message: Option<String>,
    pub infringed_rule_id: Option<i64>,
    #[cfg_attr(feature = "ssr", sqlx(default))]
    pub infringed_rule_title: Option<String>,
    #[cfg_attr(feature = "ssr", sqlx(default))]
    pub is_sphere_rule: bool,
    pub parent_id: Option<i64>,
    pub post_id: i64,
    pub creator_id: i64,
    pub creator_name: String,
    pub is_creator_moderator: bool,
    pub moderator_id: Option<i64>,
    #[cfg_attr(feature = "ssr", sqlx(default))]
    pub moderator_name: Option<String>,
    pub is_pinned: bool,
    pub score: i32,
    pub score_minus: i32,
    pub create_timestamp: chrono::DateTime<chrono::Utc>,
    pub edit_timestamp: Option<chrono::DateTime<chrono::Utc>>,
    pub delete_timestamp: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Clone, Debug, PartialEq, Eq, Ord, PartialOrd, Serialize, Deserialize)]
pub struct CommentWithChildren {
    pub comment: Comment,
    pub vote: Option<Vote>,
    pub child_comments: Vec<CommentWithChildren>,
}

#[cfg_attr(feature = "ssr", derive(sqlx::FromRow))]
#[derive(Clone, Debug, PartialEq, Eq, Ord, PartialOrd, Serialize, Deserialize)]
pub struct CommentWithContext {
    #[cfg_attr(feature = "ssr", sqlx(flatten))]
    pub comment: Comment,
    #[cfg_attr(feature = "ssr", sqlx(flatten))]
    pub sphere_header: SphereHeader,
    pub satellite_id: Option<i64>,
    pub post_title: String,
}

impl Comment {
    pub fn is_active(&self) -> bool {
        self.delete_timestamp.is_none() && self.moderator_id.is_none()
    }
}

impl CommentWithContext {
    pub fn from_comment(
        comment: Comment,
        sphere_header: SphereHeader,
        post: &Post,
    ) -> CommentWithContext {
        CommentWithContext {
            comment,
            sphere_header,
            satellite_id: post.satellite_id,
            post_title: post.title.clone(),
        }
    }
}

#[cfg(feature = "ssr")]
pub mod ssr {
    use sqlx::PgPool;
    use sphare_core_common::checks::check_string_length;
    use sphare_core_common::constants::{COMMENT_BATCH_SIZE, MAX_CONTENT_LENGTH};
    use sphare_core_common::editor::ssr::get_html_and_markdown_strings;
    use sphare_core_common::errors::AppError;
    use sphare_core_sphere::sphere::ssr::get_post_sphere;
    use sphare_core_sphere::sphere::Sphere;
    use sphare_core_user::notification::NotificationType;
    use sphare_core_user::notification::ssr::create_notification;
    use sphare_core_user::role::PermissionLevel;
    use sphare_core_user::user::User;

    use crate::post::ssr::increment_post_comment_count;
    use crate::ranking::{SortType, VoteValue};
    use crate::ranking::ssr::vote_on_content;
    use super::*;

    #[derive(Clone, Debug, PartialEq, Eq, sqlx::FromRow, Ord, PartialOrd, Serialize, Deserialize)]
    pub struct CommentWithVote {
        #[sqlx(flatten)]
        pub comment: Comment,
        pub vote_id: Option<i64>,
        pub vote_post_id: Option<i64>,
        pub vote_comment_id: Option<Option<i64>>,
        pub vote_user_id: Option<i64>,
        pub value: Option<i16>,
        pub vote_timestamp: Option<chrono::DateTime<chrono::Utc>>,
    }

    impl CommentWithVote {
        pub fn into_comment_with_children(self) -> CommentWithChildren {
            let comment_vote = if self.vote_id.is_some() {
                Some(Vote {
                    vote_id: self.vote_id.unwrap(),
                    user_id: self.vote_user_id.unwrap(),
                    comment_id: self.vote_comment_id.unwrap(),
                    post_id: self.vote_post_id.unwrap(),
                    value: VoteValue::from(self.value.unwrap()),
                    timestamp: self.vote_timestamp.unwrap(),
                })
            } else {
                None
            };

            CommentWithChildren {
                comment: self.comment,
                vote: comment_vote,
                child_comments: Vec::<CommentWithChildren>::new(),
            }
        }
    }

    pub async fn get_comment_by_id(
        comment_id: i64,
        db_pool: &PgPool,
    ) -> Result<Comment, AppError> {
        let comment = sqlx::query_as::<_, Comment>(
            "SELECT
                c.*,
                COALESCE(u.username, '') as creator_name,
                m.username as moderator_name,
                r.title as infringed_rule_title,
                r.sphere_id IS NOT NULL AS is_sphere_rule
            FROM comments c
            LEFT JOIN users u ON u.user_id = c.creator_id AND c.delete_timestamp IS NULL
            LEFT JOIN users m ON m.user_id = c.moderator_id AND c.delete_timestamp IS NULL
            LEFT JOIN rules r ON r.rule_id = c.infringed_rule_id AND c.delete_timestamp IS NULL
            WHERE comment_id = $1"
        )
            .bind(comment_id)
            .fetch_one(db_pool)
            .await?;

        Ok(comment)
    }

    pub async fn get_comment_sphere(
        comment_id: i64,
        db_pool: &PgPool,
    ) -> Result<Sphere, AppError> {
        let sphere = sqlx::query_as::<_, Sphere>(
            "SELECT s.*
            FROM spheres s
            JOIN posts p on p.sphere_id = s.sphere_id
            JOIN comments c on c.post_id = p.post_id
            WHERE c.comment_id = $1"
        )
            .bind(comment_id)
            .fetch_one(db_pool)
            .await?;

        Ok(sphere)
    }

    fn process_comment_tree(
        comment_with_vote_vec: Vec<CommentWithVote>,
        allow_partial_tree: bool,
    ) -> Vec<CommentWithChildren> {
        let mut comment_tree = Vec::new();
        let mut stack = Vec::<(i64, Vec<CommentWithChildren>)>::new();
        for comment_with_vote in comment_with_vote_vec {
            let mut current = comment_with_vote.into_comment_with_children();

            if let Some((top_parent_id, child_comments)) = stack.last_mut() {
                if *top_parent_id == current.comment.comment_id {
                    // child comments at the top of the stack belong to the current comment, add them
                    current.child_comments.append(child_comments);
                    stack.pop();
                }
            }

            // if the current element has a parent, add it to the stack. Otherwise, add it to the comment tree as a root element.
            if let Some(parent_id) = current.comment.parent_id {
                if let Some((top_parent_id, top_child_comments)) = stack.last_mut() {
                    if parent_id == *top_parent_id {
                        // same parent id as the top of the stack, add it
                        top_child_comments.push(current);
                    } else {
                        // different parent id as the top of the stack, add it as a new element on the stack
                        stack.push((parent_id, Vec::from([current])));
                    }
                } else {
                    // no element on the stack, add the current comment as a new element
                    stack.push((parent_id, Vec::from([current])));
                }
            } else {
                comment_tree.push(current);
            }
        }

        // Handle comment trees that do not start from a root comment
        if allow_partial_tree && comment_tree.is_empty() && stack.len() == 1 {
            let (_, partial_comment_tree) = stack.into_iter().next().unwrap();
            comment_tree = partial_comment_tree;
        }

        comment_tree
    }

    pub async fn get_post_comment_tree(
        post_id: i64,
        sort_type: SortType,
        max_depth: Option<usize>,
        user_id: Option<i64>,
        limit: i64,
        offset: i64,
        db_pool: &PgPool,
    ) -> Result<Vec<CommentWithChildren>, AppError> {
        if post_id < 1 {
            return Err(AppError::new("Invalid post id."));
        }

        let sort_column = sort_type.to_order_by_code();

        let comment_with_vote_vec = sqlx::query_as::<_, CommentWithVote>(
            format!(
                "WITH RECURSIVE comment_tree AS (
                    (
                        SELECT
                            c.*,
                            1 AS depth,
                            ARRAY[(c.is_pinned, c.{sort_column}, c.comment_id)] AS path
                        FROM comments c
                        WHERE
                            c.post_id = $2 AND
                            c.parent_id IS NULL
                        ORDER BY c.is_pinned DESC, c.{sort_column} DESC
                        LIMIT $4
                        OFFSET $5
                    )
                    UNION ALL (
                        SELECT
                            n.*,
                            r.depth + 1,
                            r.path || (n.is_pinned, n.{sort_column}, n.comment_id)
                        FROM comment_tree r
                        JOIN comments n ON n.parent_id = r.comment_id
                        WHERE ($3 IS NULL OR r.depth <= $3)
                    )
                )
                SELECT
                    c.*,
                    COALESCE(u.username, '') as creator_name,
                    m.username as moderator_name,
                    r.title as infringed_rule_title,
                    r.sphere_id IS NOT NULL AS is_sphere_rule,
                    v.vote_id,
                    v.user_id as vote_user_id,
                    v.post_id as vote_post_id,
                    v.comment_id as vote_comment_id,
                    v.value,
                    v.timestamp as vote_timestamp
                FROM comment_tree c
                LEFT JOIN users u ON u.user_id = c.creator_id AND c.delete_timestamp IS NULL
                LEFT JOIN users m ON m.user_id = c.moderator_id AND c.delete_timestamp IS NULL
                LEFT JOIN rules r ON r.rule_id = c.infringed_rule_id AND c.delete_timestamp IS NULL
                LEFT JOIN votes v ON v.comment_id = c.comment_id AND v.user_id = $1
                ORDER BY c.path DESC"
            )
                .as_str(),
        )
            .bind(user_id)
            .bind(post_id)
            .bind(max_depth.map(|max_depth| (max_depth+ 1) as i64))
            .bind(limit)
            .bind(offset)
            .fetch_all(db_pool)
            .await?;

        let comment_tree = process_comment_tree(comment_with_vote_vec, false);

        Ok(comment_tree)
    }

    /// Retrieves the comment tree of `comment_id`'s parent, itself and its children
    pub async fn get_comment_tree_by_id(
        comment_id: i64,
        sort_type: SortType,
        max_depth: Option<usize>,
        user_id: Option<i64>,
        db_pool: &PgPool,
    ) -> Result<CommentWithChildren, AppError> {
        if comment_id < 1 {
            return Err(AppError::new("Invalid comment id."));
        }

        let sort_column = sort_type.to_order_by_code();

        let comment_with_vote_vec = sqlx::query_as::<_, CommentWithVote>(
            format!(
                "WITH RECURSIVE comment_tree AS (
                    (
                        SELECT
                            c.*,
                            1 AS depth,
                            ARRAY[(c.is_pinned, c.{sort_column}, c.comment_id)] AS path
                        FROM comments c
                        WHERE
                            c.comment_id = $2
                        ORDER BY c.is_pinned DESC, c.{sort_column} DESC
                        LIMIT $4
                    )
                    UNION ALL (
                        SELECT
                            n.*,
                            r.depth + 1,
                            r.path || (n.is_pinned, n.{sort_column}, n.comment_id)
                        FROM comment_tree r
                        JOIN comments n ON n.parent_id = r.comment_id
                        WHERE ($3 IS NULL OR r.depth <= $3)
                    )
                ),
                selected_comments AS (
                    SELECT * FROM (
                        SELECT * FROM comment_tree
                    ) AS selected_tree
                    UNION ALL (
                        SELECT
                            c1.*,
                            0 as depth,
                            ARRAY[(c1.is_pinned, c1.{sort_column}, c1.comment_id)] AS path
                        FROM comments c1
                        WHERE c1.comment_id = (
                            SELECT c2.parent_id
                            FROM comments c2
                            WHERE comment_id = $2
                        )
                    )
                )
                SELECT
                    c.*,
                    COALESCE(u.username, '') as creator_name,
                    m.username as moderator_name,
                    r.title as infringed_rule_title,
                    r.sphere_id IS NOT NULL AS is_sphere_rule,
                    v.vote_id,
                    v.user_id as vote_user_id,
                    v.post_id as vote_post_id,
                    v.comment_id as vote_comment_id,
                    v.value,
                    v.timestamp as vote_timestamp
                FROM selected_comments c
                LEFT JOIN users u ON u.user_id = c.creator_id AND c.delete_timestamp IS NULL
                LEFT JOIN users m ON m.user_id = c.moderator_id AND c.delete_timestamp IS NULL
                LEFT JOIN rules r ON r.rule_id = c.infringed_rule_id AND c.delete_timestamp IS NULL
                LEFT JOIN votes v ON v.comment_id = c.comment_id AND v.user_id = $1
                ORDER BY depth DESC, c.path DESC"
            ).as_str(),
        )
            .bind(user_id)
            .bind(comment_id)
            .bind(max_depth.map(|max_depth| (max_depth+ 1) as i64))
            .bind(COMMENT_BATCH_SIZE)
            .fetch_all(db_pool)
            .await?;

        let comment_tree = process_comment_tree(comment_with_vote_vec, true);

        if comment_tree.len() > 1 {
            return Err(AppError::new(format!("Comment tree for comment {comment_id} should have a single root element.")));
        }

        comment_tree.into_iter().next().ok_or(AppError::new(format!("No comment tree found for comment {comment_id}")))
    }

    pub async fn search_comments(
        search_query: &str,
        sphere_name: Option<&str>,
        limit: i64,
        offset: i64,
        db_pool: &PgPool,
    ) -> Result<Vec<CommentWithContext>, AppError> {
        let comment_vec = sqlx::query_as::<_, CommentWithContext>(
            "SELECT
                c.*,
                u.username as creator_name,
                p.sphere_id,
                p.satellite_id,
                p.title as post_title,
                s.sphere_name,
                s.icon_url,
                s.is_nsfw,
                ts_rank(c.comment_document,
                plainto_tsquery('simple', $1)) AS rank
                FROM comments c
                JOIN users u ON u.user_id = c.creator_id
                JOIN posts p ON p.post_id = c.post_id
                JOIN spheres s ON s.sphere_id = p.sphere_id
                WHERE
                    c.comment_document @@ plainto_tsquery('simple', $1) AND
                    c.moderator_id IS NULL AND
                    c.delete_timestamp IS NULL AND
                    ($2 IS NULL OR s.sphere_name = $2)
                ORDER BY rank DESC, c.score DESC
                LIMIT $3
                OFFSET $4"
        )
            .bind(search_query)
            .bind(sphere_name)
            .bind(limit)
            .bind(offset)
            .fetch_all(db_pool)
            .await?;

        Ok(comment_vec)
    }

    pub async fn create_comment_with_notif(
        post_id: i64,
        parent_comment_id: Option<i64>,
        comment: &str,
        is_markdown: bool,
        is_pinned: bool,
        user: &User,
        db_pool: &PgPool,
    ) -> Result<CommentWithChildren, AppError> {
        log::trace!("Create comment for post {post_id}");
        check_string_length(comment, "Comment", MAX_CONTENT_LENGTH as usize, false)?;
        let (comment, markdown_comment) = get_html_and_markdown_strings(comment, is_markdown)?;

        let mut comment = create_comment(
            post_id,
            parent_comment_id,
            comment.as_str(),
            markdown_comment,
            is_pinned,
            user,
            db_pool,
        )
            .await?;

        let vote = vote_on_content(
            VoteValue::Up,
            comment.post_id,
            Some(comment.comment_id),
            None,
            user,
            db_pool,
        ).await?;

        comment.score = 1;

        let notif_type = match parent_comment_id {
            Some(_) => NotificationType::CommentReply,
            None => NotificationType::PostReply,
        };
        create_notification(post_id, comment.parent_id, Some(comment.comment_id), user.user_id, notif_type, &db_pool).await?;

        Ok(CommentWithChildren {
            comment,
            vote,
            child_comments: Vec::<CommentWithChildren>::default(),
        })
    }

    pub async fn create_comment(
        post_id: i64,
        parent_comment_id: Option<i64>,
        comment: &str,
        markdown_comment: Option<&str>,
        is_pinned: bool,
        user: &User,
        db_pool: &PgPool,
    ) -> Result<Comment, AppError> {
        let sphere = get_post_sphere(post_id, &db_pool).await?;
        user.check_can_publish_on_sphere(&sphere.sphere_name)?;
        if comment.is_empty() {
            return Err(AppError::new("Cannot create empty comment."));
        }
        if is_pinned {
            user.check_sphere_permissions_by_name(&sphere.sphere_name, PermissionLevel::Moderate)?;
        }
        let comment = sqlx::query_as::<_, Comment>(
            r#"
            WITH new_comment AS (
                INSERT INTO comments (
                    body, markdown_body, parent_id, post_id, is_pinned, creator_id, is_creator_moderator
                )
                VALUES ($1, $2, $3, $4, $5, $6, $7)
                RETURNING *
            )
            SELECT *, $8 as creator_name FROM new_comment
            "#,
        )
            .bind(comment)
            .bind(markdown_comment)
            .bind(parent_comment_id)
            .bind(post_id)
            .bind(is_pinned)
            .bind(user.user_id)
            .bind(user.check_sphere_permissions_by_name(&sphere.sphere_name, PermissionLevel::Moderate).is_ok())
            .bind(user.username.clone())
            .fetch_one(db_pool)
            .await?;

        increment_post_comment_count(post_id, &db_pool).await?;

        Ok(comment)
    }

    pub async fn edit_comment(
        comment_id: i64,
        comment: &str,
        is_markdown: bool,
        is_pinned: bool,
        user: &User,
        db_pool: &PgPool,
    ) -> Result<Comment, AppError> {
        log::trace!("Edit comment {comment_id}");
        check_string_length(comment, "Comment", MAX_CONTENT_LENGTH as usize, false)?;

        let (comment, markdown_comment) = get_html_and_markdown_strings(comment, is_markdown)?;

        let comment = update_comment(
            comment_id,
            comment.as_str(),
            markdown_comment,
            is_pinned,
            user,
            db_pool,
        ).await?;

        Ok(comment)
    }

    pub async fn update_comment(
        comment_id: i64,
        comment_body: &str,
        comment_markdown_body: Option<&str>,
        is_pinned: bool,
        user: &User,
        db_pool: &PgPool,
    ) -> Result<Comment, AppError> {
        if is_pinned {
            let sphere = get_comment_sphere(comment_id, &db_pool).await?;
            user.check_sphere_permissions_by_name(&sphere.sphere_name, PermissionLevel::Moderate)?;
        }
        let comment = sqlx::query_as::<_, Comment>(
            "WITH updated_comment AS (
                UPDATE comments SET
                    body = $1,
                    markdown_body = $2,
                    is_pinned = $3,
                    edit_timestamp = NOW()
                WHERE
                    comment_id = $4 AND
                    creator_id = $5 AND
                    moderator_id IS NULL AND
                    delete_timestamp IS NULL
                RETURNING *
            )
            SELECT *, $6 as creator_name FROM updated_comment",
        )
            .bind(comment_body)
            .bind(comment_markdown_body)
            .bind(is_pinned)
            .bind(comment_id)
            .bind(user.user_id)
            .bind(user.username.clone())
            .fetch_one(db_pool)
            .await?;

        Ok(comment)
    }

    pub async fn delete_comment(
        comment_id: i64,
        user: &User,
        db_pool: &PgPool,
    ) -> Result<Comment, AppError> {
        let deleted_comment = sqlx::query_as::<_, Comment>(
            "WITH deleted_comment AS (
                UPDATE comments SET
                    body = '',
                    markdown_body = NULL,
                    is_pinned = false,
                    edit_timestamp = NOW(),
                    delete_timestamp = NOW()
                WHERE
                    comment_id = $1 AND
                    creator_id = $2 AND
                    moderator_id IS NULL
                RETURNING *
            )
            SELECT *, '' as creator_name FROM deleted_comment",
        )
            .bind(comment_id)
            .bind(user.user_id)
            .fetch_one(db_pool)
            .await?;

        Ok(deleted_comment)
    }

    #[cfg(test)]
    mod tests {
        use crate::comment::ssr::CommentWithVote;
        use crate::comment::Comment;
        use crate::ranking::VoteValue;
        use sphare_core_user::user::User;

        #[test]
        fn test_comment_join_vote_into_comment_with_children() {
            let user = User::default();
            let mut comment = Comment::default();
            comment.creator_id = user.user_id;

            let comment_without_vote = CommentWithVote {
                comment: comment.clone(),
                vote_id: None,
                vote_post_id: None,
                vote_comment_id: None,
                vote_user_id: None,
                value: None,
                vote_timestamp: None,
            };
            let comment_without_vote = comment_without_vote.into_comment_with_children();
            assert_eq!(comment_without_vote.comment, comment);
            assert_eq!(comment_without_vote.vote, None);
            assert_eq!(comment_without_vote.child_comments.is_empty(), true);

            let comment_with_vote = CommentWithVote {
                comment: comment.clone(),
                vote_id: Some(0),
                vote_post_id: Some(comment.post_id),
                vote_comment_id: Some(Some(comment.comment_id)),
                vote_user_id: Some(user.user_id),
                value: Some(1),
                vote_timestamp: Some(comment.create_timestamp),
            };
            let comment_with_vote = comment_with_vote.into_comment_with_children();
            let user_vote = comment_with_vote.vote.expect("CommentWithChildren should contain vote.");
            assert_eq!(comment_with_vote.comment, comment);
            assert_eq!(user_vote.user_id, user.user_id);
            assert_eq!(user_vote.comment_id, Some(comment.comment_id));
            assert_eq!(user_vote.value, VoteValue::Up);
            assert_eq!(user_vote.comment_id, Some(comment.comment_id));
            assert_eq!(comment_with_vote.child_comments.is_empty(), true);
        }
    }
}