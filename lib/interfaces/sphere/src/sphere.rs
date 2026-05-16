use leptos::prelude::*;

#[cfg(feature = "ssr")]
use {
    sphare_core_common::db_utils::ssr::get_db_pool,
    sphare_core_sphere::sphere::*,
    sphare_core_user::auth::ssr::{check_user, get_user, reload_user},
};

use sphare_core_common::common::SphereHeader;
use sphare_core_common::errors::AppError;
use sphare_core_sphere::sphere::{Sphere, SphereWithUserInfo};

#[server]
pub async fn is_sphere_available(sphere_name: String) -> Result<bool, AppError> {
    let db_pool = get_db_pool()?;
    ssr::is_sphere_available(&sphere_name, &db_pool).await
}

#[server]
pub async fn get_sphere_by_name(sphere_name: String) -> Result<Sphere, AppError> {
    let db_pool = get_db_pool()?;
    ssr::get_sphere_by_name(&sphere_name, &db_pool).await
}

#[server]
pub async fn get_subscribed_sphere_headers() -> Result<Vec<SphereHeader>, AppError> {
    match get_user().await {
        Ok(Some(user)) => {
            let db_pool = get_db_pool()?;
            let sphere_header_vec = ssr::get_subscribed_sphere_headers(user.user_id, &db_pool).await?;
            Ok(sphere_header_vec)
        }
        _ => Ok(Vec::new()),
    }
}

#[server]
pub async fn get_popular_sphere_headers() -> Result<Vec<SphereHeader>, AppError> {
    let db_pool = get_db_pool()?;
    ssr::get_popular_sphere_headers(20, &db_pool).await
}

#[server]
pub async fn get_sphere_with_user_info(
    sphere_name: String,
) -> Result<SphereWithUserInfo, AppError> {
    let db_pool = get_db_pool()?;
    let user_id = match get_user().await {
        Ok(Some(user)) => Some(user.user_id),
        _ => None,
    };

    ssr::get_sphere_with_user_info(sphere_name.as_str(), user_id, &db_pool).await
}

#[server]
pub async fn create_sphere(
    sphere_name: String,
    description: String,
    is_nsfw: bool,
) -> Result<(), AppError> {
    let user = check_user().await?;
    let db_pool = get_db_pool()?;

    let (_, new_sphere_path) = ssr::create_sphere_and_subscribe(
        &sphere_name,
        &description,
        is_nsfw,
        &user,
        &db_pool
    ).await?;

    reload_user(user.user_id)?;

    // Redirect to the new sphere
    leptos_axum::redirect(&new_sphere_path);
    Ok(())
}

#[server]
pub async fn update_sphere_description(
    sphere_name: String,
    description: String,
) -> Result<(), AppError> {
    let user = check_user().await?;
    let db_pool = get_db_pool()?;

    ssr::update_sphere_description(&sphere_name, &description, &user, &db_pool).await?;
    Ok(())

}

#[server]
pub async fn subscribe(sphere_id: i64) -> Result<(), AppError> {
    let user = check_user().await?;
    let db_pool = get_db_pool()?;

    ssr::subscribe(sphere_id, user.user_id, &db_pool).await
}

#[server]
pub async fn unsubscribe(sphere_id: i64) -> Result<(), AppError> {
    let user = check_user().await?;
    let db_pool = get_db_pool()?;

    ssr::unsubscribe(sphere_id, user.user_id, &db_pool).await
}
