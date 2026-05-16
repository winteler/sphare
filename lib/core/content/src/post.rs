use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use validator::Validate;

use sphare_core_common::checks::{check_post_title, check_sphere_name};
use sphare_core_common::common::SphereCategoryHeader;
use sphare_core_common::constants::{MAX_CONTENT_LENGTH, MAX_LINK_LENGTH};

use crate::embed::{EmbedType, Link};
use crate::ranking::Vote;

#[cfg_attr(feature = "ssr", derive(sqlx::FromRow))]
#[derive(Clone, Debug, Default, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct Post {
    pub post_id: i64,
    pub title: String,
    pub body: String,
    pub markdown_body: Option<String>,
    #[cfg_attr(feature = "ssr", sqlx(flatten))]
    pub link: Link,
    pub is_nsfw: bool,
    pub is_spoiler: bool,
    pub category_id: Option<i64>,
    pub is_edited: bool,
    pub sphere_id: i64,
    pub satellite_id: Option<i64>,
    pub creator_id: i64,
    pub creator_name: String,
    pub is_creator_moderator: bool,
    pub moderator_message: Option<String>,
    pub infringed_rule_id: Option<i64>,
    #[cfg_attr(feature = "ssr", sqlx(default))]
    pub infringed_rule_title: Option<String>,
    #[cfg_attr(feature = "ssr", sqlx(default))]
    pub is_sphere_rule: bool,
    pub moderator_id: Option<i64>,
    #[cfg_attr(feature = "ssr", sqlx(default))]
    pub moderator_name: Option<String>,
    pub num_comments: i32,
    pub is_pinned: bool,
    pub score: i32,
    pub score_minus: i32,
    pub recommended_score: f32,
    pub trending_score: f32,
    pub create_timestamp: chrono::DateTime<chrono::Utc>,
    pub edit_timestamp: Option<chrono::DateTime<chrono::Utc>>,
    pub scoring_timestamp: chrono::DateTime<chrono::Utc>,
    pub delete_timestamp: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Clone, Debug, Default, PartialEq, PartialOrd, Validate, Serialize, Deserialize)]
pub struct PostLocation {
    #[validate(custom(function = "check_sphere_name"))]
    pub sphere: String,
    #[validate(range(min = 1))]
    pub satellite_id: Option<i64>,
}

#[derive(Clone, Debug, Default, PartialEq, PartialOrd, Validate, Serialize, Deserialize)]
pub struct PostDataInputs {
    #[validate(custom(function = "check_post_title"))]
    pub title: String,
    #[validate(length(max = MAX_CONTENT_LENGTH))]
    pub body: String,
    pub is_markdown: bool,
    pub embed_type: EmbedType,
    #[validate(length(min = 1, max = MAX_LINK_LENGTH))]
    pub link: Option<String>,
    #[validate(nested)]
    pub post_tags: PostTags
}

#[derive(Clone, Debug, Default, PartialEq, PartialOrd, Validate, Serialize, Deserialize)]
pub struct PostTags {
    pub is_spoiler: bool,
    pub is_nsfw: bool,
    #[serde(default)]
    pub is_pinned: bool,
    #[validate(range(min = 1))]
    pub category_id: Option<i64>,
}

#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct PostWithInfo {
    pub post: Post,
    pub sphere_category: Option<SphereCategoryHeader>,
    pub vote: Option<Vote>,
}

#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct PostWithSphereInfo {
    pub post: Post,
    pub sphere_name: String,
    pub sphere_category: Option<SphereCategoryHeader>,
    pub sphere_icon_url: Option<String>,
}

#[cfg_attr(feature = "ssr", derive(sqlx::FromRow))]
#[derive(Clone, Debug, Default, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct PostInheritedAttributes {
    pub is_spoiler: bool,
    pub is_nsfw: bool,
}

impl Post {
    pub fn is_active(&self) -> bool {
        self.delete_timestamp.is_none() && self.moderator_id.is_none()
    }
}

impl PostTags {
    pub fn new(
        is_spoiler: bool,
        is_nsfw: bool,
        is_pinned: bool,
        category_id: Option<i64>,
    ) -> Self {
        Self {
            is_spoiler,
            is_nsfw,
            is_pinned,
            category_id,
        }
    }
}

impl PostWithSphereInfo {
    pub fn from_post(
        post: Post,
        sphere_name: String,
        sphere_category: Option<SphereCategoryHeader>,
        sphere_icon_url: Option<String>,
    ) -> Self {
        PostWithSphereInfo {
            post,
            sphere_name,
            sphere_category,
            sphere_icon_url,
        }
    }
}

#[cfg(feature = "ssr")]
pub mod ssr {
    use serde::{Deserialize, Serialize};
    use sqlx::PgPool;
    use validator::Validate;
    use sphare_core_common::checks::check_sphere_name;
    use sphare_core_common::colors::Color;
    use sphare_core_common::common::SphereCategoryHeader;
    use sphare_core_common::constants::POST_BATCH_SIZE;
    use sphare_core_common::editor::clear_newlines;
    use sphare_core_common::editor::ssr::get_html_and_markdown_strings;
    use sphare_core_common::errors::AppError;
    use sphare_core_common::routes::get_post_path;
    use sphare_core_user::role::PermissionLevel;
    use sphare_core_user::user::User;

    use crate::embed::{verify_link_and_get_embed, EmbedType, Link};
    use crate::filter::SphereCategoryFilter;
    use crate::post::{Post, PostDataInputs, PostInheritedAttributes, PostLocation, PostTags, PostWithInfo, PostWithSphereInfo};
    use crate::ranking::{SortType, Vote, VoteValue};
    use crate::ranking::ssr::vote_on_content;

    #[cfg_attr(feature = "ssr", derive(sqlx::FromRow))]
    #[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize)]
    pub struct PostJoinSphereInfo {
        #[cfg_attr(feature = "ssr", sqlx(flatten))]
        pub post: Post,
        pub sphere_name: String,
        pub category_name: Option<String>,
        pub category_color: Option<Color>,
        pub sphere_icon_url: Option<String>,
    }

    #[derive(Clone, Debug, PartialEq, sqlx::FromRow, PartialOrd, Serialize, Deserialize)]
    pub struct PostJoinInfo {
        #[sqlx(flatten)]
        pub post: Post,
        pub category_name: Option<String>,
        pub category_color: Option<Color>,
        pub vote_id: Option<i64>,
        pub vote_post_id: Option<i64>,
        pub vote_comment_id: Option<Option<i64>>,
        pub vote_user_id: Option<i64>,
        pub value: Option<i16>,
        pub vote_timestamp: Option<chrono::DateTime<chrono::Utc>>,
    }

    impl PostJoinSphereInfo {
        pub fn into_post_with_sphere_info(self) -> PostWithSphereInfo {
            let sphere_category = match (self.category_name, self.category_color) {
                (Some(category_name), Some(category_color)) => Some(SphereCategoryHeader {
                    category_name,
                    category_color,
                }),
                _ => None,
            };
            PostWithSphereInfo {
                post: self.post,
                sphere_name: self.sphere_name,
                sphere_category,
                sphere_icon_url: self.sphere_icon_url,
            }
        }
    }

    impl PostJoinInfo {
        pub fn into_post_with_info(self) -> PostWithInfo {
            let sphere_category = match (self.category_name, self.category_color) {
                (Some(category_name), Some(category_color)) => Some(SphereCategoryHeader {
                    category_name,
                    category_color,
                }),
                _ => None,
            };
            let post_vote = match (self.vote_id, self.vote_user_id, self.value, self.vote_timestamp) {
                (Some(vote_id), Some(vote_user_id), Some(value), Some(vote_timestamp)) => Some(Vote {
                    vote_id,
                    post_id: self.post.post_id,
                    comment_id: None,
                    user_id: vote_user_id,
                    value: VoteValue::from(value),
                    timestamp: vote_timestamp,
                }),
                _ => None,
            };

            PostWithInfo {
                post: self.post,
                sphere_category,
                vote: post_vote,
            }
        }
    }

    pub async fn get_post_by_id(
        post_id: i64,
        db_pool: &PgPool,
    ) -> Result<Post, AppError> {
        let post = sqlx::query_as::<_, Post>(
            "SELECT
                p.*,
                COALESCE(u.username, '') as creator_name,
                m.username as moderator_name,
                r.title as infringed_rule_title,
                r.sphere_id IS NOT NULL AS is_sphere_rule
            FROM posts p
            JOIN users u ON u.user_id = p.creator_id AND p.delete_timestamp IS NULL
            LEFT JOIN users m ON m.user_id = p.moderator_id AND p.delete_timestamp IS NULL
            LEFT JOIN rules r ON r.rule_id = p.infringed_rule_id AND p.delete_timestamp IS NULL
            WHERE post_id = $1",
        )
            .bind(post_id)
            .fetch_one(db_pool)
            .await?;

        Ok(post)
    }

    pub async fn get_post_with_info_by_id(
        post_id: i64,
        user: Option<&User>,
        db_pool: &PgPool,
    ) -> Result<PostWithInfo, AppError> {
        let user_id = user.map(|user| user.user_id);

        let post_join_vote = sqlx::query_as::<_, PostJoinInfo>(
            "SELECT p.*,
                COALESCE(u.username, '') as creator_name,
                m.username as moderator_name,
                r.title as infringed_rule_title,
                r.sphere_id IS NOT NULL AS is_sphere_rule,
                c.category_name,
                c.category_color,
                v.vote_id,
                v.user_id as vote_user_id,
                v.post_id as vote_post_id,
                v.comment_id as vote_comment_id,
                v.value,
                v.timestamp as vote_timestamp
            FROM posts p
            LEFT JOIN users u ON u.user_id = p.creator_id AND p.delete_timestamp IS NULL
            LEFT JOIN users m ON m.user_id = p.moderator_id AND p.delete_timestamp IS NULL
            LEFT JOIN rules r ON r.rule_id = p.infringed_rule_id AND p.delete_timestamp IS NULL
            LEFT JOIN sphere_categories c on c.category_id = p.category_id
            LEFT JOIN votes v
            ON v.post_id = p.post_id AND
               v.comment_id IS NULL AND
               v.user_id = $1
            WHERE p.post_id = $2",
        )
            .bind(user_id)
            .bind(post_id)
            .fetch_one(db_pool)
            .await?;

        Ok(post_join_vote.into_post_with_info())
    }

    pub async fn get_post_inherited_attributes(
        post_id: i64,
        db_pool: &PgPool,
    ) -> Result<PostInheritedAttributes, AppError> {
        let inherited_attributes = sqlx::query_as::<_, PostInheritedAttributes>(
            "SELECT
                COALESCE(sa.is_spoiler, FALSE) AS is_spoiler,
                COALESCE(sa.is_nsfw, s.is_nsfw) AS is_nsfw
            FROM posts p
            JOIN spheres s on s.sphere_id = p.sphere_id
            LEFT JOIN satellites sa on sa.satellite_id = p.satellite_id
            WHERE p.post_id = $1",
        )
            .bind(post_id)
            .fetch_one(db_pool)
            .await?;

        Ok(inherited_attributes)
    }

    pub async fn get_post_sphere_name(
        post_id: i64,
        db_pool: &PgPool,
    ) -> Result<String, AppError> {
        let record = sqlx::query!(
            "SELECT s.sphere_name
            FROM spheres s
            JOIN posts p on p.sphere_id = s.sphere_id
            WHERE p.post_id = $1",
            post_id
        )
            .fetch_one(db_pool)
            .await?;

        Ok(record.sphere_name)
    }

    pub async fn get_post_vec_by_sphere_name(
        sphere_name: &str,
        sphere_category_filter: SphereCategoryFilter,
        sort_type: SortType,
        limit: i64,
        offset: i64,
        user: Option<&User>,
        db_pool: &PgPool,
    ) -> Result<Vec<Post>, AppError> {
        check_sphere_name(sphere_name)?;
        let posts_filters = user.map(|user| user.get_posts_filter()).unwrap_or_default();
        let post_vec = sqlx::query_as::<_, Post>(
            format!(
                "WITH base_posts AS NOT MATERIALIZED (
                    SELECT p.*, u.username as creator_name
                    FROM posts p
                    JOIN users u ON u.user_id = p.creator_id
                    JOIN spheres s on s.sphere_id = p.sphere_id
                    WHERE
                        s.sphere_name = $1 AND
                        (
                            $2 OR (
                                $3 AND p.category_id IS NULL
                            ) OR (
                                p.category_id IS NOT NULL AND p.category_id = ANY($4)
                            )
                        ) AND
                        p.moderator_id IS NULL AND
                        p.delete_timestamp IS NULL AND
                        p.satellite_id IS NULL AND
                        (
                            $5 IS NULL OR NOT p.is_spoiler OR p.create_timestamp < NOW() - (INTERVAL '1 day' * $5)
                        ) AND
                        (
                            $6 OR NOT p.is_nsfw
                        )
                )
                (
                  SELECT * FROM base_posts WHERE is_pinned = TRUE ORDER BY {order_by} DESC
                )
                UNION ALL
                (
                  SELECT * FROM base_posts WHERE is_pinned = FALSE ORDER BY {order_by} DESC
                )
                LIMIT $7
                OFFSET $8",
                order_by=sort_type.to_order_by_code(),
            ).as_str(),
        )
            .bind(sphere_name)
            .bind(sphere_category_filter == SphereCategoryFilter::All)
            .bind(match &sphere_category_filter {
                SphereCategoryFilter::All => false,
                SphereCategoryFilter::CategorySet(category_filter_set) => !category_filter_set.only_category
            })
            .bind(match sphere_category_filter {
                SphereCategoryFilter::All => None,
                SphereCategoryFilter::CategorySet(category_filter_set) => Some(Vec::from_iter(category_filter_set.filters))
            })
            .bind(posts_filters.days_hide_spoiler)
            .bind(posts_filters.show_nsfw)
            .bind(limit)
            .bind(offset)
            .fetch_all(db_pool)
            .await?;

        Ok(post_vec)
    }

    pub async fn get_post_vec_by_satellite_id(
        satellite_id: i64,
        sphere_category_id: Option<i64>,
        sort_type: SortType,
        limit: i64,
        offset: i64,
        user: Option<&User>,
        db_pool: &PgPool,
    ) -> Result<Vec<Post>, AppError> {
        let posts_filters = user.map(|user| user.get_posts_filter()).unwrap_or_default();
        let post_vec = sqlx::query_as::<_, Post>(
            format!(
                "WITH base_posts AS NOT MATERIALIZED (
                    SELECT p.*, u.username as creator_name
                    FROM posts p
                    JOIN users u ON u.user_id = p.creator_id
                    JOIN satellites s ON s.satellite_id = p.satellite_id
                    WHERE
                        s.satellite_id = $1 AND
                        p.category_id IS NOT DISTINCT FROM COALESCE($2, p.category_id) AND
                        p.moderator_id IS NULL AND
                        p.delete_timestamp IS NULL AND
                        (
                            $3 IS NULL OR NOT p.is_spoiler OR p.create_timestamp < NOW() - (INTERVAL '1 day' * $3)
                        ) AND
                        (
                            $4 OR NOT p.is_nsfw
                        )
                )
                (
                  SELECT * FROM base_posts WHERE is_pinned = TRUE ORDER BY {order_by} DESC
                )
                UNION ALL
                (
                  SELECT * FROM base_posts WHERE is_pinned = FALSE ORDER BY {order_by} DESC
                )
                LIMIT $5
                OFFSET $6",
                order_by=sort_type.to_order_by_code(),
            ).as_str(),
        )
            .bind(satellite_id)
            .bind(sphere_category_id)
            .bind(posts_filters.days_hide_spoiler)
            .bind(posts_filters.show_nsfw)
            .bind(limit)
            .bind(offset)
            .fetch_all(db_pool)
            .await?;

        Ok(post_vec)
    }

    pub async fn get_homepage_post_vec(
        sort_type: SortType,
        num_already_loaded: usize,
        user: Option<&User>,
        db_pool: &PgPool,
    ) -> Result<Vec<PostWithSphereInfo>, AppError> {
        let post_vec = match user {
            Some(user) => get_subscribed_post_vec(
                sort_type,
                POST_BATCH_SIZE,
                num_already_loaded as i64,
                user,
                db_pool,
            ).await?,
            None => get_sorted_post_vec(
                sort_type,
                POST_BATCH_SIZE,
                num_already_loaded as i64,
                None,
                db_pool,
            ).await?,
        };

        Ok(post_vec)
    }

    pub async fn get_sorted_post_vec(
        sort_type: SortType,
        limit: i64,
        offset: i64,
        user: Option<&User>,
        db_pool: &PgPool,
    ) -> Result<Vec<PostWithSphereInfo>, AppError> {
        let (days_hide_spoiler, show_nsfw) = match user {
            Some(user) => (user.days_hide_spoiler, user.show_nsfw),
            None => (None, false),
        };
        let post_vec = sqlx::query_as::<_, PostJoinSphereInfo>(
            format!(
                "SELECT
                    p.*,
                    u.username as creator_name,
                    c.category_name,
                    c.category_color,
                    s.icon_url as sphere_icon_url,
                    s.sphere_name
                FROM posts p
                JOIN users u ON u.user_id = p.creator_id
                JOIN spheres s on s.sphere_id = p.sphere_id
                LEFT JOIN sphere_categories c on c.category_id = p.category_id
                WHERE
                    p.moderator_id IS NULL AND
                    p.delete_timestamp IS NULL AND
                    p.satellite_id IS NULL AND
                    (
                        $1 IS NULL OR NOT p.is_spoiler OR p.create_timestamp < NOW() - (INTERVAL '1 day' * $1)
                    ) AND
                    (
                        $2 OR NOT p.is_nsfw
                    )
                ORDER BY {} DESC
                LIMIT $3
                OFFSET $4",
                sort_type.to_order_by_code(),
            ).as_str()
        )
            .bind(days_hide_spoiler)
            .bind(show_nsfw)
            .bind(limit)
            .bind(offset)
            .fetch_all(db_pool)
            .await?;

        let post_vec = post_vec.into_iter().map(PostJoinSphereInfo::into_post_with_sphere_info).collect();

        Ok(post_vec)
    }

    pub async fn get_subscribed_post_vec(
        sort_type: SortType,
        limit: i64,
        offset: i64,
        user: &User,
        db_pool: &PgPool,
    ) -> Result<Vec<PostWithSphereInfo>, AppError> {
        let posts_filters = user.get_posts_filter();
        let order_by = sort_type.to_order_by_code();
        let mut post_vec = sqlx::query_as::<_, PostJoinSphereInfo>(
            format!(
                "SELECT
                    p.*,
                    u.username AS creator_name,
                    c.category_name,
                    c.category_color,
                    s.icon_url AS sphere_icon_url,
                    s.sphere_name
                FROM posts p
                JOIN users u ON u.user_id = p.creator_id
                JOIN spheres s on s.sphere_id = p.sphere_id
                JOIN sphere_subscriptions su ON su.sphere_id = s.sphere_id AND su.user_id = $1
                LEFT JOIN sphere_categories c on c.category_id = p.category_id
                WHERE
                    p.moderator_id IS NULL AND
                    p.delete_timestamp IS NULL AND
                    p.satellite_id IS NULL AND
                    (
                        $2 IS NULL OR NOT p.is_spoiler OR p.create_timestamp < NOW() - (INTERVAL '1 day' * $2)
                    ) AND
                    (
                        $3 OR NOT p.is_nsfw
                    )
                ORDER BY {order_by} DESC
                LIMIT $4
                OFFSET $5"
            ).as_str(),
        )
            .bind(user.user_id)
            .bind(posts_filters.days_hide_spoiler)
            .bind(posts_filters.show_nsfw)
            .bind(limit)
            .bind(offset)
            .fetch_all(db_pool)
            .await?;

        let loaded_post_count = post_vec.len();
        // If no posts are returned, fetch posts from not subscribed spheres
        if loaded_post_count < limit as usize {
            let mut additional_posts = sqlx::query_as::<_, PostJoinSphereInfo>(
                format!(
                    "WITH subscribed_post_count AS (
                        SELECT COUNT(*) AS total
                        FROM posts p
                        JOIN spheres s on s.sphere_id = p.sphere_id
                        JOIN sphere_subscriptions su ON su.sphere_id = s.sphere_id AND su.user_id = $1
                        WHERE
                            p.moderator_id IS NULL AND
                            p.delete_timestamp IS NULL AND
                            p.satellite_id IS NULL AND
                            (
                                $2 IS NULL OR NOT p.is_spoiler OR p.create_timestamp < NOW() - (INTERVAL '1 day' * $2)
                            ) AND
                            (
                                $3 OR NOT p.is_nsfw
                            )
                    )
                    SELECT
                        p.*,
                        u.username as creator_name,
                        c.category_name,
                        c.category_color,
                        s.icon_url as sphere_icon_url,
                        s.sphere_name
                    FROM posts p
                    JOIN users u ON u.user_id = p.creator_id
                    JOIN spheres s on s.sphere_id = p.sphere_id
                    LEFT JOIN sphere_categories c on c.category_id = p.category_id
                    WHERE
                        p.moderator_id IS NULL AND
                        p.delete_timestamp IS NULL AND
                        p.satellite_id IS NULL AND
                        (
                            $2 IS NULL OR NOT p.is_spoiler OR p.create_timestamp < NOW() - (INTERVAL '1 day' * $2)
                        ) AND
                        (
                            $3 OR NOT p.is_nsfw
                        ) AND
                        s.sphere_id NOT IN (
                            SELECT sphere_id FROM sphere_subscriptions su where su.user_id = $1
                        )
                    ORDER BY {order_by} DESC
                    LIMIT $4
                    OFFSET GREATEST(0, $5 - (SELECT total FROM subscribed_post_count))"
                    ).as_str(),
                )
                    .bind(user.user_id)
                    .bind(posts_filters.days_hide_spoiler)
                    .bind(posts_filters.show_nsfw)
                    .bind(limit - loaded_post_count as i64)
                    .bind(offset)
                    .fetch_all(db_pool)
                    .await?;
            post_vec.append(&mut additional_posts);
        }

        let post_vec = post_vec.into_iter().map(PostJoinSphereInfo::into_post_with_sphere_info).collect();

        Ok(post_vec)
    }

    pub async fn create_post_and_vote(
        post_location: PostLocation,
        post_inputs: PostDataInputs,
        user: &User,
        db_pool: &PgPool,
    ) -> Result<(Post, Option<Vote>, String), AppError> {
        post_location.validate()?;
        post_inputs.validate()?;

        let (body, markdown_body) = get_html_and_markdown_strings(&post_inputs.body, post_inputs.is_markdown)?;

        let link = process_embed_link(post_inputs.embed_type, post_inputs.link).await;

        let post = create_post(
            post_location.sphere.as_str(),
            post_location.satellite_id,
            clear_newlines(post_inputs.title, true).as_str(),
            body.as_str(),
            markdown_body,
            link,
            post_inputs.post_tags,
            user,
            db_pool,
        ).await?;

        let vote = vote_on_content(VoteValue::Up, post.post_id, None, None, user, db_pool).await?;

        log::trace!("Created post with id: {}", post.post_id);
        let new_post_path = get_post_path(&post_location.sphere, post_location.satellite_id, post.post_id);

        Ok((post, vote, new_post_path))
    }

    pub async fn create_post(
        sphere_name: &str,
        satellite_id: Option<i64>,
        post_title: &str,
        post_body: &str,
        post_markdown_body: Option<&str>,
        link: Link,
        post_tags: PostTags,
        user: &User,
        db_pool: &PgPool,
    ) -> Result<Post, AppError> {
        user.check_can_publish_on_sphere(sphere_name)?;
        if sphere_name.is_empty() || post_title.is_empty() {
            return Err(AppError::new(
                "Cannot create post without a valid sphere and title.",
            ));
        }
        if post_tags.is_pinned {
            user.check_sphere_permissions_by_name(sphere_name, PermissionLevel::Moderate)?;
        }

        let post = sqlx::query_as::<_, Post>(
            "WITH new_post AS (
                    INSERT INTO posts (
                        title, body, markdown_body, link_type, link_url, link_embed, link_thumbnail_url, is_nsfw, is_spoiler, category_id,
                        sphere_id, satellite_id, is_pinned, creator_id, is_creator_moderator
                    )
                    VALUES (
                        $1, $2, $3, $4, $5, $6, $7,
                        (
                            CASE
                                WHEN $8 THEN TRUE
                                ELSE (
                                    (SELECT is_nsfw FROM spheres s WHERE s.sphere_name = $11) OR
                                    COALESCE(
                                        (SELECT is_nsfw FROM satellites sa WHERE sa.satellite_id = $12),
                                        FALSE
                                    )
                                )
                            END
                        ),
                        (
                            CASE
                                WHEN $9 THEN TRUE
                                ELSE COALESCE(
                                    (SELECT is_spoiler FROM satellites sa WHERE sa.satellite_id = $12),
                                    FALSE
                                )
                            END
                        ),
                        $10,
                        (SELECT sphere_id FROM spheres s WHERE s.sphere_name = $11),
                        $12, $13, $14, $15
                ) RETURNING *
            )
            SELECT *, $16 as creator_name FROM new_post",
        )
            .bind(post_title)
            .bind(post_body)
            .bind(post_markdown_body)
            .bind(link.link_type as i16)
            .bind(link.link_url)
            .bind(link.link_embed)
            .bind(link.link_thumbnail_url)
            .bind(post_tags.is_nsfw)
            .bind(post_tags.is_spoiler)
            .bind(post_tags.category_id)
            .bind(sphere_name)
            .bind(satellite_id)
            .bind(post_tags.is_pinned)
            .bind(user.user_id)
            .bind(user.check_sphere_permissions_by_name(sphere_name, PermissionLevel::Moderate).is_ok())
            .bind(user.username.clone())
            .fetch_one(db_pool)
            .await?;

        Ok(post)
    }

    pub async fn edit_post(
        post_id: i64,
        post_inputs: PostDataInputs,
        user: &User,
        db_pool: &PgPool,
    ) -> Result<Post, AppError> {
        post_inputs.validate()?;
        log::trace!("Edit post {post_id}, title = {}", post_inputs.title);

        let (body, markdown_body) = get_html_and_markdown_strings(
            &post_inputs.body,
            post_inputs.is_markdown,
        )?;

        let link = process_embed_link(post_inputs.embed_type, post_inputs.link).await;

        let post = update_post(
            post_id,
            post_inputs.title.as_str(),
            body.as_str(),
            markdown_body.as_deref(),
            link,
            post_inputs.post_tags,
            &user,
            &db_pool,
        ).await?;

        log::trace!("Updated post with id: {}", post.post_id);
        Ok(post)
    }

    pub async fn update_post(
        post_id: i64,
        post_title: &str,
        post_body: &str,
        post_markdown_body: Option<&str>,
        link: Link,
        post_tags: PostTags,
        user: &User,
        db_pool: &PgPool,
    ) -> Result<Post, AppError> {
        if post_title.is_empty() {
            return Err(AppError::new(
                "Cannot update post without a valid title.",
            ));
        }
        if post_tags.is_pinned {
            let sphere_name = get_post_sphere_name(post_id, db_pool).await?;
            user.check_sphere_permissions_by_name(&sphere_name, PermissionLevel::Moderate)?;
        }

        let post = sqlx::query_as::<_, Post>(
            "WITH updated_post AS (
                UPDATE posts SET
                    title = $1,
                    body = $2,
                    markdown_body = $3,
                    link_type = $4,
                    link_url = $5,
                    link_embed = $6,
                    link_thumbnail_url = $7,
                    is_nsfw = (
                        CASE
                            WHEN $8 THEN TRUE
                            ELSE (
                                SELECT s.is_nsfw OR COALESCE(sa.is_nsfw, FALSE) FROM posts p
                                JOIN spheres s ON s.sphere_id = p.sphere_id
                                LEFT JOIN satellites sa ON sa.satellite_id = p.satellite_id
                                WHERE p.post_id = $12
                            )
                        END
                    ),
                    is_spoiler = (
                        CASE
                            WHEN $9 THEN TRUE
                            ELSE (
                                SELECT COALESCE(sa.is_spoiler, FALSE) FROM posts p
                                LEFT JOIN satellites sa ON sa.satellite_id = p.satellite_id
                                WHERE post_id = $12
                            )
                        END
                    ),
                    is_pinned = $10,
                    category_id = $11,
                    edit_timestamp = NOW()
                WHERE
                    post_id = $12 AND
                    creator_id = $13 AND
                    moderator_id IS NULL AND
                    delete_timestamp IS NULL
                RETURNING *
            )
            SELECT *, $14 as creator_name
            FROM updated_post",
        )
            .bind(post_title)
            .bind(post_body)
            .bind(post_markdown_body)
            .bind(link.link_type as i16)
            .bind(link.link_url)
            .bind(link.link_embed)
            .bind(link.link_thumbnail_url)
            .bind(post_tags.is_nsfw)
            .bind(post_tags.is_spoiler)
            .bind(post_tags.is_pinned)
            .bind(post_tags.category_id)
            .bind(post_id)
            .bind(user.user_id)
            .bind(user.username.clone())
            .fetch_one(db_pool)
            .await?;

        Ok(post)
    }

    pub async fn delete_post(
        post_id: i64,
        user: &User,
        db_pool: &PgPool,
    ) -> Result<Post, AppError> {
        let deleted_post = sqlx::query_as::<_, Post>(
            "WITH deleted_post AS (
                UPDATE posts SET
                    title = '',
                    body = '',
                    markdown_body = NULL,
                    link_type = -1,
                    link_url = NULL,
                    link_embed = NULL,
                    link_thumbnail_url = NULL,
                    is_nsfw = false,
                    is_spoiler = false,
                    is_pinned = false,
                    category_id = NULL,
                    edit_timestamp = NOW(),
                    delete_timestamp = NOW()
                WHERE
                    post_id = $1 AND
                    creator_id = $2 AND
                    moderator_id IS NULL
                RETURNING *
            )
            SELECT *, '' AS creator_name
            FROM deleted_post"
        )
            .bind(post_id)
            .bind(user.user_id)
            .fetch_one(db_pool)
            .await?;

        Ok(deleted_post)
    }

    pub async fn increment_post_comment_count(
        post_id: i64,
        db_pool: &PgPool,
    ) -> Result<(), AppError> {
        sqlx::query!(
            "UPDATE posts
            SET num_comments = num_comments + 1
            WHERE post_id = $1",
            post_id,
        )
            .execute(db_pool)
            .await?;

        Ok(())
    }

    pub async fn update_post_scores(db_pool: &PgPool) -> Result<(), AppError> {
        sqlx::query!(
            "UPDATE posts
            SET scoring_timestamp = NOW()
            WHERE create_timestamp > (NOW() - INTERVAL '2 days')",
        )
            .execute(db_pool)
            .await?;

        Ok(())
    }

    pub async fn process_embed_link(embed_type: EmbedType, link: Option<String>) -> Link {
        let (link, _) = match (embed_type, link) {
            (embed_type, Some(link)) if embed_type != EmbedType::None => verify_link_and_get_embed(embed_type, &link).await,
            _ => (Link::default(), None),
        };
        link
    }

    #[cfg(test)]
    mod tests {
        use sphare_core_common::colors::Color;
        use sphare_core_user::user::User;

        use crate::embed::{EmbedType, Link, LinkType};
        use crate::post::ssr::{process_embed_link, PostJoinInfo};
        use crate::post::Post;
        use crate::ranking::VoteValue;

        #[test]
        fn test_post_join_vote_into_post_with_info() {
            let user = User::default();
            let mut user_post = Post::default();
            user_post.creator_id = user.user_id;

            let user_post_without_vote = PostJoinInfo {
                post: user_post.clone(),
                category_name: None,
                category_color: None,
                vote_id: None,
                vote_post_id: None,
                vote_comment_id: None,
                vote_user_id: None,
                value: None,
                vote_timestamp: None,
            };
            let user_post_with_info = user_post_without_vote.into_post_with_info();
            assert_eq!(user_post_with_info.post, user_post);
            assert_eq!(user_post_with_info.sphere_category, None);
            assert_eq!(user_post_with_info.vote, None);

            let user_post_with_vote = PostJoinInfo {
                post: user_post.clone(),
                category_name: Some(String::from("a")),
                category_color: None,
                vote_id: Some(0),
                vote_post_id: Some(user_post.post_id),
                vote_comment_id: None,
                vote_user_id: Some(user.user_id),
                value: Some(1),
                vote_timestamp: Some(user_post.create_timestamp),
            };
            let user_post_with_info = user_post_with_vote.into_post_with_info();
            let user_vote = user_post_with_info.vote.expect("PostWithInfo should contain vote.");
            assert_eq!(user_post_with_info.post, user_post);
            assert_eq!(user_post_with_info.sphere_category, None);
            assert_eq!(user_vote.user_id, user.user_id);
            assert_eq!(user_vote.post_id, user_post.post_id);
            assert_eq!(user_vote.value, VoteValue::Up);
            assert_eq!(user_vote.comment_id, None);

            let mut other_post = Post::default();
            other_post.creator_id = user.user_id + 1;

            let other_post_with_vote = PostJoinInfo {
                post: other_post.clone(),
                category_name: Some(String::from("a")),
                category_color: Some(Color::Green),
                vote_id: Some(0),
                vote_post_id: Some(other_post.post_id),
                vote_comment_id: None,
                vote_user_id: Some(user.user_id),
                value: Some(-1),
                vote_timestamp: Some(other_post.create_timestamp),
            };
            let other_post_with_info = other_post_with_vote.into_post_with_info();
            let user_vote = other_post_with_info.vote.expect("PostWithInfo should contain vote.");
            let sphere_category = other_post_with_info.sphere_category.expect("PostWithInfo should contain category.");
            assert_eq!(other_post_with_info.post, other_post);
            assert_eq!(sphere_category.category_name, String::from("a"));
            assert_eq!(sphere_category.category_color, Color::Green);
            assert_eq!(user_vote.user_id, user.user_id);
            assert_eq!(user_vote.post_id, other_post.post_id);
            assert_eq!(user_vote.value, VoteValue::Down);
            assert_eq!(user_vote.comment_id, None);
        }

        #[tokio::test]
        async fn test_process_embed_link() {
            let default_link = process_embed_link(EmbedType::None, None).await;
            assert_eq!(default_link, Link::default());

            let link_url = String::from("https://test.com/");
            let simple_link = process_embed_link(EmbedType::Link, Some(link_url.clone())).await;
            assert_eq!(simple_link, Link::new(LinkType::Link, Some(link_url), None, None));
        }
    }
}

pub fn add_sphere_info_to_post_vec(
    post_vec: Vec<Post>,
    sphere_name: String,
    sphere_category_map: &HashMap<i64, SphereCategoryHeader>,
    sphere_icon_url: Option<String>,
) -> Vec<PostWithSphereInfo> {
    post_vec.into_iter().map(|post| {
        let category_id = match post.category_id {
            Some(category_id) => sphere_category_map.get(&category_id).cloned(),
            None => None,
        };
        PostWithSphereInfo::from_post(post, sphere_name.clone(), category_id, sphere_icon_url.clone())
    }).collect()
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use sphare_core_common::colors::Color;
    use sphare_core_common::common::SphereCategoryHeader;

    use crate::embed::Link;
    use crate::post::{add_sphere_info_to_post_vec, Post, PostWithSphereInfo};

    fn create_post_with_category(title: &str, category_id: Option<i64>) -> Post {
        Post {
            post_id: 0,
            title: title.to_string(),
            body: String::default(),
            markdown_body: None,
            link: Link::default(),
            is_nsfw: false,
            is_spoiler: false,
            category_id,
            is_edited: false,
            sphere_id: 0,
            satellite_id: None,
            creator_id: 0,
            creator_name: String::default(),
            is_creator_moderator: false,
            moderator_message: None,
            infringed_rule_id: None,
            infringed_rule_title: None,
            is_sphere_rule: false,
            moderator_id: None,
            moderator_name: None,
            num_comments: 0,
            is_pinned: false,
            score: 0,
            score_minus: 0,
            recommended_score: 0.0,
            trending_score: 0.0,
            create_timestamp: Default::default(),
            edit_timestamp: None,
            scoring_timestamp: Default::default(),
            delete_timestamp: None,
        }
    }

    #[test]
    fn test_from_post() {
        let category_header_a = SphereCategoryHeader {
            category_name: String::from("a"),
            category_color: Color::Blue,
        };
        let category_header_b = SphereCategoryHeader {
            category_name: String::from("b"),
            category_color: Color::Red,
        };

        let post_1 = create_post_with_category("i", Some(1));
        let post_2 = create_post_with_category("j", Some(2));
        let post_3 = create_post_with_category("k", Some(3));
        let post_4 = create_post_with_category("l", None);

        let post_with_sphere_info_1 = PostWithSphereInfo::from_post(post_1.clone(), "a".to_string(), Some(category_header_a.clone()), None);
        let post_with_sphere_info_2 = PostWithSphereInfo::from_post(post_2.clone(), "b".to_string(), Some(category_header_b.clone()), None);
        let post_with_sphere_info_3 = PostWithSphereInfo::from_post(post_3.clone(), "c".to_string(), None, None);
        let post_with_sphere_info_4 = PostWithSphereInfo::from_post(post_4.clone(), "d".to_string(), None, None);

        assert_eq!(post_with_sphere_info_1.post, post_1);
        assert_eq!(post_with_sphere_info_1.sphere_category, Some(category_header_a));
        assert_eq!(post_with_sphere_info_1.sphere_icon_url, None);

        assert_eq!(post_with_sphere_info_2.post, post_2);
        assert_eq!(post_with_sphere_info_2.sphere_category, Some(category_header_b));
        assert_eq!(post_with_sphere_info_2.sphere_icon_url, None);

        assert_eq!(post_with_sphere_info_3.post, post_3);
        assert_eq!(post_with_sphere_info_3.sphere_category, None);
        assert_eq!(post_with_sphere_info_3.sphere_icon_url, None);

        assert_eq!(post_with_sphere_info_4.post, post_4);
        assert_eq!(post_with_sphere_info_4.sphere_category, None);
        assert_eq!(post_with_sphere_info_4.sphere_icon_url, None);
    }
    #[test]
    fn test_add_sphere_info_to_post_vec() {
        let sphere_icon_url = String::from("https://www.image.com/sphere_icon.jpg");
        let sphere_category_1 = SphereCategoryHeader {
            category_name: "red".to_string(),
            category_color: Color::Red,
        };
        let sphere_category_2 = SphereCategoryHeader {
            category_name: "blue".to_string(),
            category_color: Color::Blue,
        };
        let sphere_category_map = HashMap::from([
            (
                1,
                sphere_category_1.clone(),
            ),
            (
                2,
                sphere_category_2.clone(),
            ),
        ]);
        let post_vec = vec![
            create_post_with_category("Red", Some(1)),
            create_post_with_category("Blue", Some(2)),
            create_post_with_category("Other", None),
        ];

        let post_with_sphere_info_vec = add_sphere_info_to_post_vec(
            post_vec.clone(),
            "a".to_string(),
            &sphere_category_map,
            Some(sphere_icon_url.clone())
        );
        assert_eq!(post_with_sphere_info_vec[0].post, post_vec[0]);
        assert_eq!(post_with_sphere_info_vec[1].post, post_vec[1]);
        assert_eq!(post_with_sphere_info_vec[2].post, post_vec[2]);

        assert_eq!(post_with_sphere_info_vec[0].sphere_category, Some(sphere_category_1));
        assert_eq!(post_with_sphere_info_vec[1].sphere_category, Some(sphere_category_2));
        assert_eq!(post_with_sphere_info_vec[2].sphere_category, None);

        assert_eq!(post_with_sphere_info_vec[0].sphere_icon_url, Some(sphere_icon_url.clone()));
        assert_eq!(post_with_sphere_info_vec[1].sphere_icon_url, Some(sphere_icon_url.clone()));
        assert_eq!(post_with_sphere_info_vec[2].sphere_icon_url, Some(sphere_icon_url));
    }
}