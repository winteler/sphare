use leptos::prelude::*;

#[cfg(feature = "ssr")]
use {
    sphare_core_common::constants::{COMMENT_BATCH_SIZE, POST_BATCH_SIZE, SPHERE_HEADER_FETCH_LIMIT},
    sphare_core_common::db_utils::ssr::get_db_pool,
    sphare_core_content::search::*,
    sphare_core_user::auth::ssr::{get_user},
};

use sphare_core_common::common::SphereHeader;
use sphare_core_common::errors::AppError;
use sphare_core_content::comment::CommentWithContext;
use sphare_core_content::post::PostWithSphereInfo;

#[server]
pub async fn get_matching_sphere_header_vec(
    sphere_prefix: String,
) -> Result<Vec<SphereHeader>, AppError> {
    let db_pool = get_db_pool()?;
    ssr::get_matching_sphere_header_vec(
        &sphere_prefix,
        SPHERE_HEADER_FETCH_LIMIT as i64,
        &db_pool
    ).await
}

#[server]
pub async fn search_spheres(
    search_query: String,
    load_count: usize,
    num_already_loaded: usize,
) -> Result<Vec<SphereHeader>, AppError> {
    let db_pool = get_db_pool()?;
    let show_nsfw = get_user().await.unwrap_or(None).map(|user| user.show_nsfw).unwrap_or_default();
    ssr::search_spheres(&search_query, show_nsfw, load_count as i64, num_already_loaded as i64, &db_pool).await
}

#[server]
pub async fn search_posts(
    search_query: String,
    sphere_name: Option<String>,
    show_spoilers: bool,
    num_already_loaded: usize,
) -> Result<Vec<PostWithSphereInfo>, AppError> {
    let db_pool = get_db_pool()?;
    let show_nsfw = get_user().await.unwrap_or(None).map(|user| user.show_nsfw).unwrap_or_default();
    ssr::search_posts(
        &search_query,
        sphere_name.as_deref(),
        show_spoilers,
        show_nsfw,
        POST_BATCH_SIZE,
        num_already_loaded as i64,
        &db_pool
    ).await
}

#[server]
pub async fn search_comments(
    search_query: String,
    sphere_name: Option<String>,
    num_already_loaded: usize,
) -> Result<Vec<CommentWithContext>, AppError> {
    let db_pool = get_db_pool()?;
    ssr::search_comments(
        &search_query,
        sphere_name.as_deref(),
        COMMENT_BATCH_SIZE,
        num_already_loaded as i64,
        &db_pool
    ).await
}