use leptos::prelude::*;

#[cfg(feature = "ssr")]
use {
    sphare_core_common::db_utils::ssr::get_db_pool,
    sphare_core_sphere::sphere_category::*,
    sphare_core_user::auth::ssr::check_user,
};

use sphare_core_common::colors::Color;
use sphare_core_common::errors::AppError;
use sphare_core_sphere::sphere_category::SphereCategory;

#[server]
pub async fn get_sphere_category_vec(
    sphere_name: String,
) -> Result<Vec<SphereCategory>, AppError> {
    let db_pool = get_db_pool()?;
    ssr::get_sphere_category_vec(&sphere_name, &db_pool).await
}

#[server]
pub async fn set_sphere_category(
    sphere_name: String,
    category_name: String,
    category_color: Color,
    description: String,
    is_active: bool,
) -> Result<SphereCategory, AppError> {
    let db_pool = get_db_pool()?;
    let user = check_user().await?;
    ssr::set_sphere_category(&sphere_name, &category_name, category_color, &description, is_active, &user, &db_pool).await
}

#[server]
pub async fn delete_sphere_category(
    sphere_name: String,
    category_name: String,
) -> Result<(), AppError> {
    let db_pool = get_db_pool()?;
    let user = check_user().await?;
    ssr::delete_sphere_category(&sphere_name, &category_name, &user, &db_pool).await
}