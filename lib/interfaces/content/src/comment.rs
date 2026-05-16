use leptos::prelude::*;

#[cfg(feature = "ssr")]
use {
    sphare_core_common::constants::COMMENT_BATCH_SIZE,
    sphare_core_common::db_utils::ssr::get_db_pool,
    sphare_core_content::comment::*,
    sphare_core_user::auth::ssr::{check_user, get_user},
};

use sphare_core_common::errors::AppError;
use sphare_core_content::comment::{Comment, CommentWithChildren};
use sphare_core_content::ranking::SortType;

#[server]
pub async fn get_post_comment_tree(
    post_id: i64,
    sort_type: SortType,
    max_depth: Option<usize>,
    num_already_loaded: usize,
) -> Result<Vec<CommentWithChildren>, AppError> {
    let user_id = match get_user().await {
        Ok(Some(user)) => Some(user.user_id),
        _ => None,
    };
    let db_pool = get_db_pool()?;
    ssr::get_post_comment_tree(
        post_id,
        sort_type,
        max_depth,
        user_id,
        COMMENT_BATCH_SIZE,
        num_already_loaded as i64,
        &db_pool,
    ).await
}

#[server]
pub async fn get_comment_tree_by_id(
    comment_id: i64,
    sort_type: SortType,
    max_depth: Option<usize>,
) -> Result<CommentWithChildren, AppError> {
    let user_id = match get_user().await {
        Ok(Some(user)) => Some(user.user_id),
        _ => None,
    };
    let db_pool = get_db_pool()?;
    ssr::get_comment_tree_by_id(
        comment_id,
        sort_type,
        max_depth,
        user_id,
        &db_pool,
    ).await
}

#[server]
pub async fn create_comment(
    post_id: i64,
    parent_comment_id: Option<i64>,
    comment: String,
    is_markdown: bool,
    is_pinned: Option<bool>,
) -> Result<CommentWithChildren, AppError> {
    let user = check_user().await?;
    let db_pool = get_db_pool()?;

    ssr::create_comment_with_notif(post_id, parent_comment_id, &comment, is_markdown, is_pinned.unwrap_or(false), &user, &db_pool).await
}

#[server]
pub async fn edit_comment(
    comment_id: i64,
    comment: String,
    is_markdown: bool,
    is_pinned: Option<bool>,
) -> Result<Comment, AppError> {
    let user = check_user().await?;
    let db_pool = get_db_pool()?;

    ssr::edit_comment(comment_id, &comment, is_markdown, is_pinned.unwrap_or(false), &user, &db_pool).await
}

#[server]
pub async fn delete_comment(
    comment_id: i64,
) -> Result<(), AppError> {
    log::trace!("Delete comment {comment_id}");
    let user = check_user().await?;
    let db_pool = get_db_pool()?;

    ssr::delete_comment(
        comment_id,
        &user,
        &db_pool,
    ).await?;

    Ok(())
}