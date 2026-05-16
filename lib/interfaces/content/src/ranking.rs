use leptos::prelude::*;

#[cfg(feature = "ssr")]
use {
    sphare_core_common::db_utils::ssr::get_db_pool,
    sphare_core_content::ranking::*,
    sphare_core_user::auth::ssr::check_user,
};

use sphare_core_common::errors::AppError;
use sphare_core_content::ranking::{Vote, VoteValue};

#[server]
pub async fn vote_on_content(
    vote_value: VoteValue,
    post_id: i64,
    comment_id: Option<i64>,
    vote_id: Option<i64>,
) -> Result<Option<Vote>, AppError> {
    let user = check_user().await?;
    let db_pool = get_db_pool()?;

    ssr::vote_on_content(
        vote_value,
        post_id,
        comment_id,
        vote_id,
        &user,
        &db_pool,
    ).await
}