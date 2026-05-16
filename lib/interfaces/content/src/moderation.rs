use leptos::prelude::*;
use sphare_core_common::errors::AppError;

#[cfg(feature = "ssr")]
use {
    sphare_core_common::db_utils::ssr::get_db_pool,
    sphare_core_content::moderation::*,
    sphare_core_user::auth::ssr::{check_user, reload_user},
};

use sphare_core_content::comment::Comment;
use sphare_core_content::moderation::ModerationInfo;
use sphare_core_content::post::Post;

#[server]
pub async fn get_moderation_info(
    post_id: i64,
    comment_id: Option<i64>,
) -> Result<ModerationInfo, AppError> {
    let db_pool = get_db_pool()?;
    ssr::get_moderation_info(post_id, comment_id, &db_pool).await
}

/// Function to moderate a post and optionally ban its author
///
/// The ban is performed for the sphere of the given post and the duration is given by `ban_num_days`.
/// If `ban_num_days == None`, the duration of the ban is permanent.
#[server]
pub async fn moderate_post(
    post_id: i64,
    rule_id: i64,
    moderator_message: String,
    ban_duration_days: Option<usize>,
) -> Result<Post, AppError> {
    let user = check_user().await?;
    let db_pool = get_db_pool()?;

    let (post, _, _) = ssr::moderate_post_and_ban_user(post_id, rule_id, &moderator_message, ban_duration_days, &user, &db_pool).await?;

    reload_user(post.creator_id)?;

    Ok(post)
}

/// Function to moderate a comment and optionally ban its author
///
/// The ban is performed for the sphere of the given comment and the duration is given by `ban_num_days`.
/// If `ban_num_days == None`, the duration of the ban is permanent.
#[server]
pub async fn moderate_comment(
    comment_id: i64,
    rule_id: i64,
    moderator_message: String,
    ban_duration_days: Option<usize>,
) -> Result<Comment, AppError> {
    let user = check_user().await?;
    let db_pool = get_db_pool()?;

    let (comment, _, _) = ssr::moderate_comment_and_ban_user(
        comment_id,
        rule_id,
        &moderator_message,
        ban_duration_days,
        &user,
        &db_pool
    ).await?;

    reload_user(comment.creator_id)?;

    Ok(comment)
}