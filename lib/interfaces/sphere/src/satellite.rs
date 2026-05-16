use leptos::prelude::*;

#[cfg(feature = "ssr")]
use {
    sphare_core_common::db_utils::ssr::get_db_pool,
    sphare_core_sphere::satellite::*,
    sphare_core_user::auth::ssr::check_user,
};

use sphare_core_common::errors::AppError;
use sphare_core_sphere::satellite::Satellite;

#[server]
pub async fn get_satellite_by_id(
    satellite_id: i64,
) -> Result<Satellite, AppError> {
    let db_pool = get_db_pool()?;
    let satellite = ssr::get_satellite_by_id(satellite_id, &db_pool).await?;
    Ok(satellite)
}

#[server]
pub async fn get_satellite_vec_by_sphere_name(
    sphere_name: String,
    include_inactive: bool,
) -> Result<Vec<Satellite>, AppError> {
    let db_pool = get_db_pool()?;
    ssr::get_satellite_vec_by_sphere_name(&sphere_name, include_inactive, &db_pool).await
}

#[server]
pub async fn create_satellite(
    sphere_name: String,
    satellite_name: String,
    body: String,
    is_markdown: bool,
    is_nsfw: bool,
    is_spoiler: bool,
) -> Result<Satellite, AppError> {
    let db_pool = get_db_pool()?;
    let user = check_user().await?;

    ssr::create_satellite(
        &sphere_name,
        &satellite_name,
        &body,
        is_markdown,
        is_nsfw,
        is_spoiler,
        &user,
        &db_pool
    ).await
}

#[server]
pub async fn update_satellite(
    satellite_id: i64,
    satellite_name: String,
    body: String,
    is_markdown: bool,
    is_nsfw: bool,
    is_spoiler: bool,
) -> Result<Satellite, AppError> {
    let db_pool = get_db_pool()?;
    let user = check_user().await?;

    ssr::update_satellite(
        satellite_id,
        &satellite_name,
        &body,
        is_markdown,
        is_nsfw,
        is_spoiler,
        &user,
        &db_pool
    ).await
}

#[server]
pub async fn activate_satellite(
    satellite_id: i64,
) -> Result<Satellite, AppError> {
    let db_pool = get_db_pool()?;
    let user = check_user().await?;
    ssr::activate_satellite(
        satellite_id,
        &user,
        &db_pool
    ).await
}

#[server]
pub async fn deactivate_satellite(
    satellite_id: i64,
) -> Result<Satellite, AppError> {
    let db_pool = get_db_pool()?;
    let user = check_user().await?;
    ssr::deactivate_satellite(
        satellite_id,
        &user,
        &db_pool
    ).await
}