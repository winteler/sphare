use leptos::prelude::*;

#[cfg(feature = "ssr")]
use {
    sphare_core_common::db_utils::get_db_pool,
    sphare_core_user::auth::ssr::{check_user, delete_user_in_oidc_provider, get_user, reload_user},
};

use sphare_core_common::errors::AppError;
use sphare_core_user::user::UserHeader;

#[server]
pub async fn get_matching_user_header_vec(
    username_prefix: String,
    show_nsfw: Option<bool>,
    load_count: usize,
) -> Result<Vec<UserHeader>, AppError> {
    let db_pool = get_db_pool()?;
    // TODO check if show_nsfw can be simplified
    let show_nsfw = show_nsfw.unwrap_or_default() || get_user().await.unwrap_or(None).map(|user| user.show_nsfw).unwrap_or_default();
    let user_header_vec = sphare_core_user::user::ssr::get_matching_user_header_vec(&username_prefix, show_nsfw, load_count as i64, &db_pool).await?;
    Ok(user_header_vec)
}

#[server]
pub async fn delete_user() -> Result<(), AppError> {
    let db_pool = get_db_pool()?;
    let user = check_user().await?;

    sphare_core_user::user::ssr::delete_user(&user, &db_pool).await?;
    if let Err(e) = delete_user_in_oidc_provider(&user).await {
        log::error!("Failed to delete user ({}, {}): {e}", user.user_id, user.oidc_id);
    }

    leptos_axum::redirect("/");
    Ok(())
}

#[server]
pub async fn set_user_settings(
    is_nsfw: bool,
    show_nsfw: bool,
    days_hide_spoilers: u32,
) -> Result<(), AppError> {
    let db_pool = get_db_pool()?;
    let user = check_user().await?;

    sphare_core_user::user::ssr::set_user_settings(is_nsfw, show_nsfw, days_hide_spoilers, &user, &db_pool).await?;
    reload_user(user.person_id)?;
    Ok(())
}

#[cfg(feature = "ssr")]
pub mod ssr {
    use activitypub_federation::config::Data;
    use activitypub_federation::traits::Object;
    use axum::extract::Path;
    use leptos::serde_json;
    use sphare_core_common::activity_pub::ApHelper;
    use sphare_core_common::errors::AppError;
    use sphare_core_common::to_app_error;
    use sphare_core_user::person::{get_person_by_username, Person};

    pub async fn http_get_user(
        Path(name): Path<String>,
        data: Data<ApHelper>,
    ) -> Result<String, AppError> {
        let person = get_person_by_username(&name, data.app_data().get_db_pool()).await?;
        let person: Person = person.try_into()?;
        let person_json = serde_json::to_string(&person.into_json(&data).await?).map_err(to_app_error!("Couldn't convert person to json"))?;
        Ok(person_json)
    }
}