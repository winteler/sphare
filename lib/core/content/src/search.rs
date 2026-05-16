use leptos::prelude::*;
use leptos_use::signal_debounced;

use sphare_core_common::checks::check_string_length;
use sphare_core_common::constants::MAX_SEARCH_QUERY_LENGTH;
use sphare_core_common::errors::AppError;

#[derive(Clone, Debug)]
pub struct SearchState {
    pub search_input: RwSignal<String>,
    pub search_input_debounced: Signal<String>,
    pub show_spoiler: RwSignal<bool>,
}

impl Default for SearchState {
    fn default() -> Self {
        let search_input = RwSignal::new(String::new());
        SearchState {
            search_input,
            search_input_debounced: signal_debounced(search_input, 500.0),
            show_spoiler: RwSignal::new(false),
        }
    }
}

#[cfg(feature = "ssr")]
pub mod ssr {
    use sqlx::PgPool;
    use std::cmp::min;
    use sphare_core_common::checks::{check_sphere_name, check_sphere_name_with_options, check_string_length};
    use sphare_core_common::common::SphereHeader;
    use sphare_core_common::constants::{MAX_SEARCH_QUERY_LENGTH, SPHERE_FETCH_LIMIT};
    use sphare_core_common::errors::AppError;
    use crate::comment::CommentWithContext;
    use crate::post::ssr::PostJoinSphereInfo;
    use crate::post::PostWithSphereInfo;


    pub async fn get_matching_sphere_header_vec(
        sphere_prefix: &str,
        limit: i64,
        db_pool: &PgPool,
    ) -> Result<Vec<SphereHeader>, AppError> {
        check_sphere_name_with_options(&sphere_prefix, false)?;
        let sphere_header_vec = sqlx::query_as!(
            SphereHeader,
            "SELECT sphere_name, icon_url, is_nsfw
            FROM spheres
            WHERE normalized_sphere_name LIKE normalize_sphere_name($1)
            ORDER BY sphere_name LIMIT $2",
            format!("{sphere_prefix}%"),
            limit,
        )
            .fetch_all(db_pool)
            .await?;

        Ok(sphere_header_vec)
    }

    pub async fn search_spheres(
        search_query: &str,
        show_nsfw: bool,
        limit: i64,
        offset: i64,
        db_pool: &PgPool,
    ) -> Result<Vec<SphereHeader>, AppError> {
        check_string_length(search_query, "Sphere search", MAX_SEARCH_QUERY_LENGTH, false)?;
        let sphere_vec = sqlx::query_as::<_, SphereHeader>(
            "WITH search AS (
                    SELECT *, 0.5 as rank
                    FROM spheres
                    WHERE
                        normalized_sphere_name LIKE format_for_search($1 || '%') AND
                        ($2 OR NOT is_nsfw) AND
                        NOT is_banned
                    UNION ALL
                    SELECT ws.*
                    FROM (
                        SELECT *, word_similarity(normalized_sphere_name, format_for_search($1)) as rank
                        FROM spheres
                        WHERE $2 OR NOT is_nsfw AND
                        NOT is_banned
                    ) ws
                    WHERE rank > 0.3
                    UNION ALL
                    SELECT ts.*
                    FROM (
                        SELECT *, ts_rank(sphere_document, plainto_tsquery('simple', $1)) as rank
                        FROM spheres
                        WHERE
                            sphere_document @@ plainto_tsquery('simple', $1) AND
                            ($2 OR NOT is_nsfw) AND
                            NOT is_banned
                    ) ts
                    WHERE rank > 0.01
                )
                SELECT ts.sphere_name, ts.icon_url, ts.is_nsfw
                FROM (
                    SELECT * FROM (
                        SELECT DISTINCT ON (sphere_name) * FROM search
                        ORDER BY sphere_name, rank DESC, num_members DESC
                    ) ts_distinct
                    ORDER BY rank DESC, num_members DESC
                ) ts
                LIMIT $3
                OFFSET $4"
        )
            .bind(search_query)
            .bind(show_nsfw)
            .bind(min(limit, SPHERE_FETCH_LIMIT as i64))
            .bind(offset)
            .fetch_all(db_pool)
            .await?;

        Ok(sphere_vec)
    }

    pub async fn search_posts(
        search_query: &str,
        sphere_name: Option<&str>,
        show_spoilers: bool,
        show_nsfw: bool,
        limit: i64,
        offset: i64,
        db_pool: &PgPool,
    ) -> Result<Vec<PostWithSphereInfo>, AppError> {
        if let Some(sphere_name) = &sphere_name {
            check_sphere_name(sphere_name)?;
        }
        check_string_length(search_query, "Search query", MAX_SEARCH_QUERY_LENGTH, false)?;
        let post_vec = sqlx::query_as::<_, PostJoinSphereInfo>(
            "SELECT
                p.*,
                u.username as creator_name,
                c.category_name,
                c.category_color,
                s.icon_url as sphere_icon_url,
                s.sphere_name,
                ts_rank(p.post_document,
                plainto_tsquery('simple', $1)) AS rank
            FROM posts p
            JOIN users u ON u.user_id = p.creator_id
            JOIN spheres s ON s.sphere_id = p.sphere_id
            LEFT JOIN sphere_categories c ON c.category_id = p.category_id
            WHERE
                p.post_document @@ plainto_tsquery('simple', $1) AND
                ($2 IS NULL OR s.sphere_name = $2) AND
                ($3 OR NOT p.is_spoiler) AND
                ($4 OR NOT p.is_nsfw) AND
                p.moderator_id IS NULL AND
                p.delete_timestamp IS NULL
            ORDER BY rank DESC, p.score DESC
            LIMIT $5
            OFFSET $6"
        )
            .bind(search_query)
            .bind(sphere_name)
            .bind(show_spoilers)
            .bind(show_nsfw)
            .bind(limit)
            .bind(offset)
            .fetch_all(db_pool)
            .await?;

        let post_vec = post_vec.into_iter().map(PostJoinSphereInfo::into_post_with_sphere_info).collect();

        Ok(post_vec)
    }

    pub async fn search_comments(
        search_query: &str,
        sphere_name: Option<&str>,
        limit: i64,
        offset: i64,
        db_pool: &PgPool,
    ) -> Result<Vec<CommentWithContext>, AppError> {
        if let Some(sphere_name) = &sphere_name {
            check_sphere_name(sphere_name)?;
        }
        check_string_length(search_query, "Search query", MAX_SEARCH_QUERY_LENGTH, false)?;
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
}

/// # Returns whether a search for content is valid
pub fn is_content_search_valid(search_query: Signal<String>) -> Signal<Option<AppError>> {
    Signal::derive(move || {
        check_string_length(
            &*search_query.read(),
            "Sphere search",
            MAX_SEARCH_QUERY_LENGTH,
            true
        ).err()
    })
}

#[cfg(test)]
mod tests {
    use leptos::prelude::*;

    use sphare_core_common::constants::MAX_SEARCH_QUERY_LENGTH;
    use sphare_core_common::errors::AppError;

    use crate::search::is_content_search_valid;

    #[test]
    fn test_is_content_search_valid() {
        let owner = Owner::new();
        owner.set();

        let search = RwSignal::new(String::new());
        let is_search_valid = is_content_search_valid(search.into());
        assert_eq!(is_search_valid.get_untracked(), None);

        search.set(String::from(&"a".repeat(MAX_SEARCH_QUERY_LENGTH)));
        assert_eq!(is_search_valid.get_untracked(), None);

        search.set(String::from(&"a".repeat(MAX_SEARCH_QUERY_LENGTH + 1)));
        assert_eq!(
            is_search_valid.get_untracked(),
            Some(AppError::new(format!("Sphere search exceeds the maximum length: {MAX_SEARCH_QUERY_LENGTH}.")))
        );
    }
}