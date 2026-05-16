use leptos::prelude::*;

#[cfg(feature = "ssr")]
use {
    sphare_core_common::db_utils::ssr::get_db_pool,
    sphare_core_user::auth::ssr::check_user,
};

use sphare_core_common::errors::AppError;
use sphare_core_user::notification::*;

#[server]
pub async fn get_notifications() -> Result<Vec<Notification>, AppError> {
    let user = check_user().await?;
    let db_pool = get_db_pool()?;

    ssr::get_notifications(user.user_id, &db_pool).await
}

#[server]
pub async fn set_notification_read(
    notification_id: i64,
) -> Result<(), AppError> {
    let user = check_user().await?;
    let db_pool = get_db_pool()?;

    ssr::set_notification_read(notification_id, user.user_id, &db_pool).await
}

#[server]
pub async fn set_all_notifications_read() -> Result<(), AppError> {
    let user = check_user().await?;
    let db_pool = get_db_pool()?;

    ssr::set_all_notifications_read(user.user_id, &db_pool).await
}