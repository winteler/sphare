use leptos::prelude::*;

#[cfg(feature = "ssr")]
use {
    sphare_core_common::db_utils::ssr::get_db_pool,
    sphare_core_user::auth::ssr::{check_user, reload_user},
    sphare_core_user::role::*,
};

use sphare_core_common::errors::AppError;
use sphare_core_user::role::{PermissionLevel, UserSphereRole};

#[server]
pub async fn get_sphere_role_vec(sphere_name: String) -> Result<Vec<UserSphereRole>, AppError> {
    let db_pool = get_db_pool()?;
    ssr::get_sphere_role_vec(&sphere_name, &db_pool).await
}

#[server]
pub async fn set_user_sphere_role(
    username: String,
    sphere_name: String,
    permission_level: PermissionLevel,
) -> Result<UserSphereRole, AppError> {
    let user = check_user().await?;
    let db_pool = get_db_pool()?;

    let (sphere_role, prev_sphere_leader_id) = ssr::set_user_sphere_role(
        &username,
        &sphere_name,
        permission_level,
        &user,
        &db_pool,
    ).await?;

    reload_user(sphere_role.user_id)?;

    if let Some(prev_leader_id) = prev_sphere_leader_id {
        // In case the sphere leader changed, also reload previous leader
        reload_user(prev_leader_id)?;
    };

    Ok(sphere_role)
}