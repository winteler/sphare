use leptos::prelude::*;
use leptos::server_fn::codec::{MultipartData, MultipartFormData};
use sphare_core_common::errors::AppError;

#[cfg(feature = "ssr")]
use {
    sphare_core_common::db_utils::ssr::get_db_pool,
    sphare_core_sphere::sphere_management::ssr::{
        SphereImageType, OBJECT_CONTAINER_URL_ENV
    },
    sphare_core_sphere::sphere_management::*,
    sphare_core_user::auth::ssr::{check_user, reload_user},
};

use sphare_core_user::user::UserBan;

#[server]
pub async fn get_sphere_ban_vec(
    sphere_name: String,
    username_prefix: String,
) -> Result<Vec<UserBan>, AppError> {
    let db_pool = get_db_pool()?;
    ssr::get_sphere_ban_vec(&sphere_name, &username_prefix, &db_pool).await
}

#[server]
pub async fn remove_user_ban(
    ban_id: i64
) -> Result<(), AppError> {
    let user = check_user().await?;
    let db_pool = get_db_pool()?;
    let deleted_user_ban = ssr::remove_user_ban(ban_id, &user, &db_pool).await?;
    reload_user(deleted_user_ban.user_id)?;
    Ok(())
}

#[server(input = MultipartFormData)]
pub async fn set_sphere_icon(
    data: MultipartData,
) -> Result<(), AppError> {
    let user = check_user().await?;
    let db_pool = get_db_pool()?;

    let image_type = SphereImageType::ICON;
    let object_container_url = std::env::var(OBJECT_CONTAINER_URL_ENV)?;
    let bucket_name = image_type.get_bucket_name()?;
    let object_store = ssr::get_object_store(image_type)?;
    ssr::set_sphere_image(
        image_type,
        data,
        &object_store,
        &object_container_url,
        &bucket_name,
        &user,
        &db_pool,
    ).await?;

    Ok(())
}

#[server(input = MultipartFormData)]
pub async fn set_sphere_banner(
    data: MultipartData,
) -> Result<(), AppError> {
    let user = check_user().await?;
    let db_pool = get_db_pool()?;

    let image_type = SphereImageType::BANNER;
    let object_container_url = std::env::var(OBJECT_CONTAINER_URL_ENV)?;
    let bucket_name = image_type.get_bucket_name()?;
    let object_store = ssr::get_object_store(image_type)?;
    ssr::set_sphere_image(
        image_type,
        data,
        &object_store,
        &object_container_url,
        &bucket_name,
        &user,
        &db_pool,
    ).await?;

    Ok(())
}