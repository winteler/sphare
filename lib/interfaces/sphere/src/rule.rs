use leptos::prelude::*;

#[cfg(feature = "ssr")]
use {
    sphare_core_common::db_utils::ssr::get_db_pool,
    sphare_core_sphere::rule::*,
    sphare_core_user::auth::ssr::check_user,
};

use sphare_core_common::common::Rule;
use sphare_core_common::errors::AppError;

#[server]
pub async fn get_rule_by_id(
    rule_id: i64
) -> Result<Rule, AppError> {
    let db_pool = get_db_pool()?;
    ssr::load_rule_by_id(rule_id, &db_pool).await
}

#[server]
pub async fn get_rule_vec(
    sphere_name: Option<String>
) -> Result<Vec<Rule>, AppError> {
    let db_pool = get_db_pool()?;
    ssr::get_rule_vec(sphere_name.as_deref(), &db_pool).await
}

#[server]
pub async fn add_rule(
    sphere_name: String,
    priority: i16,
    title: String,
    description: String,
    is_markdown: bool,
) -> Result<Rule, AppError> {
    let db_pool = get_db_pool()?;
    let user = check_user().await?;

    ssr::add_rule(
        sphere_name.as_ref(),
        priority,
        &title,
        &description,
        is_markdown,
        &user,
        &db_pool
    ).await
}

#[server]
pub async fn update_rule(
    sphere_name: String,
    current_priority: i16,
    priority: i16,
    title: String,
    description: String,
    is_markdown: bool,
) -> Result<Rule, AppError> {
    let db_pool = get_db_pool()?;
    let user = check_user().await?;

    ssr::update_rule(
        sphere_name.as_ref(),
        current_priority,
        priority,
        &title,
        &description,
        is_markdown,
        &user,
        &db_pool
    ).await
}

#[server]
pub async fn remove_rule(
    sphere_name: String,
    priority: i16,
) -> Result<(), AppError> {
    let db_pool = get_db_pool()?;
    let user = check_user().await?;
    ssr::remove_rule(sphere_name.as_ref(), priority, &user, &db_pool).await
}