use leptos::prelude::*;

#[cfg(feature = "ssr")]
use {
    sphare_core_common::constants::POST_BATCH_SIZE,
    sphare_core_common::db_utils::ssr::get_db_pool,
    sphare_core_content::post::*,
    sphare_core_user::auth::{ssr::check_user, ssr::get_user},
};

use sphare_core_common::errors::AppError;
use sphare_core_content::filter::SphereCategoryFilter;
use sphare_core_content::post::{Post, PostDataInputs, PostInheritedAttributes, PostLocation, PostWithInfo, PostWithSphereInfo};
use sphare_core_content::ranking::SortType;

#[server]
pub async fn get_post_with_info_by_id(post_id: i64) -> Result<PostWithInfo, AppError> {
    let db_pool = get_db_pool()?;
    let user = get_user().await?;
    Ok(ssr::get_post_with_info_by_id(post_id, user.as_ref(), &db_pool).await?)
}

#[server]
pub async fn get_post_inherited_attributes(post_id: i64) -> Result<PostInheritedAttributes, AppError> {
    let db_pool = get_db_pool()?;
    Ok(ssr::get_post_inherited_attributes(post_id, &db_pool).await?)
}

#[server]
pub async fn get_sorted_post_vec(
    sort_type: SortType,
    num_already_loaded: usize,
) -> Result<Vec<PostWithSphereInfo>, AppError> {
    let user = get_user().await.unwrap_or(None);
    let db_pool = get_db_pool()?;

    ssr::get_sorted_post_vec(
        sort_type,
        POST_BATCH_SIZE,
        num_already_loaded as i64,
        user.as_ref(),
        &db_pool,
    ).await
}

#[server]
pub async fn get_homepage_post_vec(
    sort_type: SortType,
    num_already_loaded: usize,
) -> Result<Vec<PostWithSphereInfo>, AppError> {
    let user = get_user().await?;
    let db_pool = get_db_pool()?;

    ssr::get_homepage_post_vec(sort_type, num_already_loaded, user.as_ref(), &db_pool).await
}

#[server]
pub async fn get_post_vec_by_sphere_name(
    sphere_name: String,
    sphere_category_set: SphereCategoryFilter,
    sort_type: SortType,
    num_already_loaded: usize,
) -> Result<Vec<Post>, AppError> {
    let user = get_user().await.unwrap_or(None);
    let db_pool = get_db_pool()?;
    ssr::get_post_vec_by_sphere_name(
        sphere_name.as_str(),
        sphere_category_set,
        sort_type,
        POST_BATCH_SIZE,
        num_already_loaded as i64,
        user.as_ref(),
        &db_pool,
    ).await
}

#[server]
pub async fn get_post_vec_by_satellite_id(
    satellite_id: i64,
    sphere_category_id: Option<i64>,
    sort_type: SortType,
    num_already_loaded: usize,
) -> Result<Vec<Post>, AppError> {
    let user = get_user().await.unwrap_or(None);
    let db_pool = get_db_pool()?;
    ssr::get_post_vec_by_satellite_id(
        satellite_id,
        sphere_category_id,
        sort_type,
        POST_BATCH_SIZE,
        num_already_loaded as i64,
        user.as_ref(),
        &db_pool,
    ).await
}

#[server]
pub async fn create_post(
    post_location: PostLocation,
    post_inputs: PostDataInputs
) -> Result<(), AppError> {
    let user = check_user().await?;
    let db_pool = get_db_pool()?;

    let (_, _, new_post_path) = ssr::create_post_and_vote(post_location, post_inputs, &user, &db_pool).await?;

    leptos_axum::redirect(new_post_path.as_str());
    Ok(())
}

#[server]
pub async fn edit_post(
    post_id: i64,
    post_inputs: PostDataInputs,
) -> Result<Post, AppError> {
    let user = check_user().await?;
    let db_pool = get_db_pool()?;

    ssr::edit_post(post_id, post_inputs, &user, &db_pool).await
}

#[server]
pub async fn delete_post(
    post_id: i64,
) -> Result<(), AppError> {
    let user = check_user().await?;
    let db_pool = get_db_pool()?;

    ssr::delete_post(post_id, &user, &db_pool).await?;

    Ok(())
}