use sphare_core_common::errors::AppError;
use sphare_core_sphere::sphere;
use sphare_core_user::role::ssr::{get_sphere_role_vec, get_user_sphere_role, set_user_admin_role, set_user_sphere_role};
use sphare_core_user::role::{AdminRole, PermissionLevel};
use sphare_core_user::user::User;

use crate::common::{create_user, get_db_pool};
use crate::utils::*;

mod common;
mod utils;

#[tokio::test]
async fn test_get_user_sphere_role() -> Result<(), AppError> {
    let db_pool = get_db_pool().await;
    let user_a = create_user("a", &db_pool).await;
    let user_b = create_user("b", &db_pool).await;
    let user_c = create_user("c", &db_pool).await;

    let sphere_1 = sphere::ssr::create_sphere("1", "sphere", false, &user_a, &db_pool).await?;
    let sphere_2 = sphere::ssr::create_sphere("2", "sphere", false, &user_a, &db_pool).await?;
    let sphere_3 = sphere::ssr::create_sphere("3", "sphere", false, &user_b, &db_pool).await?;
    let user_a = User::get(user_a.user_id, &db_pool).await.expect("Should be able to reload user.");
    let user_b = User::get(user_b.user_id, &db_pool).await.expect("Should be able to reload user.");

    set_user_sphere_role(&user_b.username, &sphere_1.sphere_name, PermissionLevel::Manage, &user_a, &db_pool).await.expect("User should be able to grant Manage permissions.");
    set_user_sphere_role(&user_c.username, &sphere_1.sphere_name, PermissionLevel::Moderate, &user_a, &db_pool).await.expect("User should be able to grant Moderate permissions.");
    set_user_sphere_role(&user_b.username, &sphere_2.sphere_name, PermissionLevel::Ban, &user_a, &db_pool).await.expect("User should be able to grant Ban permissions.");
    set_user_sphere_role(&user_c.username, &sphere_2.sphere_name, PermissionLevel::Moderate, &user_a, &db_pool).await.expect("User should be able to grant Moderate permissions.");
    set_user_sphere_role(&user_a.username, &sphere_3.sphere_name, PermissionLevel::None, &user_b, &db_pool).await.expect("User should be able to grant Moderate permissions.");

    let user_a_sphere_1_role = get_user_sphere_role(user_a.user_id, &sphere_1.sphere_name, &db_pool).await.expect("get_user_sphere_role should return user role.");
    assert_eq!(user_a_sphere_1_role.user_id, user_a.user_id);
    assert_eq!(user_a_sphere_1_role.sphere_id, sphere_1.sphere_id);
    assert_eq!(user_a_sphere_1_role.sphere_name, sphere_1.sphere_name);
    assert_eq!(user_a_sphere_1_role.grantor_id, user_a.user_id);
    assert_eq!(user_a_sphere_1_role.permission_level, PermissionLevel::Lead);

    let user_b_sphere_1_role = get_user_sphere_role(user_b.user_id, &sphere_1.sphere_name, &db_pool).await.expect("get_user_sphere_role should return user role.");
    assert_eq!(user_b_sphere_1_role.user_id, user_b.user_id);
    assert_eq!(user_b_sphere_1_role.sphere_id, sphere_1.sphere_id);
    assert_eq!(user_b_sphere_1_role.sphere_name, sphere_1.sphere_name);
    assert_eq!(user_b_sphere_1_role.grantor_id, user_a.user_id);
    assert_eq!(user_b_sphere_1_role.permission_level, PermissionLevel::Manage);

    let user_c_sphere_1_role = get_user_sphere_role(user_c.user_id, &sphere_1.sphere_name, &db_pool).await.expect("get_user_sphere_role should return user role.");
    assert_eq!(user_c_sphere_1_role.user_id, user_c.user_id);
    assert_eq!(user_c_sphere_1_role.sphere_id, sphere_1.sphere_id);
    assert_eq!(user_c_sphere_1_role.sphere_name, sphere_1.sphere_name);
    assert_eq!(user_c_sphere_1_role.grantor_id, user_a.user_id);
    assert_eq!(user_c_sphere_1_role.permission_level, PermissionLevel::Moderate);

    let user_a_sphere_2_role = get_user_sphere_role(user_a.user_id, &sphere_2.sphere_name, &db_pool).await.expect("get_user_sphere_role should return user role.");
    assert_eq!(user_a_sphere_2_role.user_id, user_a.user_id);
    assert_eq!(user_a_sphere_2_role.sphere_id, sphere_2.sphere_id);
    assert_eq!(user_a_sphere_2_role.sphere_name, sphere_2.sphere_name);
    assert_eq!(user_a_sphere_2_role.grantor_id, user_a.user_id);
    assert_eq!(user_a_sphere_2_role.permission_level, PermissionLevel::Lead);

    let user_b_sphere_2_role = get_user_sphere_role(user_b.user_id, &sphere_2.sphere_name, &db_pool).await.expect("get_user_sphere_role should return user role.");
    assert_eq!(user_b_sphere_2_role.user_id, user_b.user_id);
    assert_eq!(user_b_sphere_2_role.sphere_id, sphere_2.sphere_id);
    assert_eq!(user_b_sphere_2_role.sphere_name, sphere_2.sphere_name);
    assert_eq!(user_b_sphere_2_role.grantor_id, user_a.user_id);
    assert_eq!(user_b_sphere_2_role.permission_level, PermissionLevel::Ban);

    let user_c_sphere_2_role = get_user_sphere_role(user_c.user_id, &sphere_2.sphere_name, &db_pool).await.expect("get_user_sphere_role should return user role.");
    assert_eq!(user_c_sphere_2_role.user_id, user_c.user_id);
    assert_eq!(user_c_sphere_2_role.sphere_id, sphere_2.sphere_id);
    assert_eq!(user_c_sphere_2_role.sphere_name, sphere_2.sphere_name);
    assert_eq!(user_c_sphere_2_role.grantor_id, user_a.user_id);
    assert_eq!(user_c_sphere_2_role.permission_level, PermissionLevel::Moderate);

    let user_a_sphere_3_role = get_user_sphere_role(user_a.user_id, &sphere_3.sphere_name, &db_pool).await.expect("get_user_sphere_role should return user role.");
    assert_eq!(user_a_sphere_3_role.user_id, user_a.user_id);
    assert_eq!(user_a_sphere_3_role.sphere_id, sphere_3.sphere_id);
    assert_eq!(user_a_sphere_3_role.sphere_name, sphere_3.sphere_name);
    assert_eq!(user_a_sphere_3_role.grantor_id, user_b.user_id);
    assert_eq!(user_a_sphere_3_role.permission_level, PermissionLevel::None);

    let user_b_sphere_3_role = get_user_sphere_role(user_b.user_id, &sphere_3.sphere_name, &db_pool).await.expect("get_user_sphere_role should return user role.");
    assert_eq!(user_b_sphere_3_role.user_id, user_b.user_id);
    assert_eq!(user_b_sphere_3_role.sphere_id, sphere_3.sphere_id);
    assert_eq!(user_b_sphere_3_role.sphere_name, sphere_3.sphere_name);
    assert_eq!(user_b_sphere_3_role.grantor_id, user_b.user_id);
    assert_eq!(user_b_sphere_3_role.permission_level, PermissionLevel::Lead);

    assert_eq!(get_user_sphere_role(user_c.user_id, &sphere_3.sphere_name, &db_pool).await, Err(AppError::NotFound));

    Ok(())
}

#[tokio::test]
async fn test_get_sphere_role_vec() -> Result<(), AppError> {
    let db_pool = get_db_pool().await;
    let user_a = create_user("a", &db_pool).await;
    let user_b = create_user("b", &db_pool).await;
    let user_c = create_user("c", &db_pool).await;

    let sphere = sphere::ssr::create_sphere("1", "sphere", false, &user_a, &db_pool).await?;
    let user_a = User::get(user_a.user_id, &db_pool).await.expect("Should be able to reload user.");

    let user_a_sphere_role = get_user_sphere_role(user_a.user_id, &sphere.sphere_name, &db_pool).await.expect("User a should have lead role.");
    let (user_b_sphere_role, _) = set_user_sphere_role(
        &user_b.username,
        &sphere.sphere_name,
        PermissionLevel::Manage,
        &user_a,
        &db_pool
    ).await.expect("User should be able to grant Manage permissions.");
    let (user_c_sphere_role, _) = set_user_sphere_role(
        &user_c.username,
        &sphere.sphere_name,
        PermissionLevel::None,
        &user_a,
        &db_pool
    ).await.expect("User should be able to grant None permissions.");

    let sphere_role_vec = get_sphere_role_vec(&sphere.sphere_name, &db_pool).await.expect("Should load sphere role vec");

    assert_eq!(sphere_role_vec.len(), 2);
    assert!(sphere_role_vec.contains(&user_a_sphere_role));
    assert!(sphere_role_vec.contains(&user_b_sphere_role));
    assert!(!sphere_role_vec.contains(&user_c_sphere_role));

    Ok(())
}

#[tokio::test]
async fn test_set_user_sphere_role() {
    let db_pool = get_db_pool().await;
    let lead_user = create_user("lead", &db_pool).await;
    let ordinary_user = create_user("a", &db_pool).await;
    let moderator = create_user("mod", &db_pool).await;

    let sphere_name = "sphere";
    let sphere = sphere::ssr::create_sphere(
        sphere_name,
        "sphere",
        false,
        &lead_user,
        &db_pool
    ).await.expect("Should create sphere");
    let lead_user = User::get(lead_user.user_id, &db_pool)
        .await
        .expect("Should be able to reload lead_user.");

    let lead_role = get_user_sphere_role(lead_user.user_id, &sphere.sphere_name, &db_pool).await.expect("Should get lead role");

    // test elect moderator
    let (moderate_role, prev_leader_id) = set_user_sphere_role(
        &moderator.username,
        sphere_name,
        PermissionLevel::Moderate,
        &lead_user,
        &db_pool,
    ).await.expect("Moderate role should be assignable by lead_user.");

    assert_eq!(moderate_role.user_id, moderator.user_id);
    assert_eq!(moderate_role.sphere_id, sphere.sphere_id);
    assert_eq!(moderate_role.sphere_name, sphere.sphere_name);
    assert_eq!(moderate_role.grantor_id, lead_user.user_id);
    assert_eq!(moderate_role.permission_level, PermissionLevel::Moderate);
    assert_eq!(moderate_role.delete_timestamp, None);
    assert_eq!(prev_leader_id, None);
    let moderator = User::get(moderator.user_id, &db_pool)
        .await
        .expect("Should be able to reload moderator.");
    assert_eq!(
        moderator.permission_by_sphere_name_map.get(sphere_name),
        Some(&PermissionLevel::Moderate)
    );

    // test need elect permissions to add moderators
    assert_eq!(
        set_user_sphere_role(
            &ordinary_user.username,
            sphere_name,
            PermissionLevel::Moderate,
            &moderator,
            &db_pool
        ).await,
        Err(AppError::InsufficientPrivileges)
    );

    // test change permission level to Manage
    let (manage_role, prev_leader_id) = set_user_sphere_role(
        &moderator.username,
        sphere_name,
        PermissionLevel::Manage,
        &lead_user,
        &db_pool,
    ).await.expect("lead_user should be able to update role to Manage.");

    assert_eq!(manage_role.user_id, moderator.user_id);
    assert_eq!(manage_role.sphere_id, sphere.sphere_id);
    assert_eq!(manage_role.sphere_name, sphere.sphere_name);
    assert_eq!(manage_role.grantor_id, lead_user.user_id);
    assert_eq!(manage_role.permission_level, PermissionLevel::Manage);
    assert_eq!(manage_role.delete_timestamp, None);
    assert_eq!(prev_leader_id, None);
    let moderator = User::get(moderator.user_id, &db_pool)
        .await
        .expect("Should be able to reload moderator after role update.");
    assert_eq!(
        moderator.permission_by_sphere_name_map.get(sphere_name),
        Some(&PermissionLevel::Manage)
    );

    // Check that moderator role has been updated and has a deleted timestamp
    let deleted_moderator_role = get_user_role_by_id(moderate_role.role_id, &db_pool).await.expect("Should load deleted mod role");

    assert_eq!(deleted_moderator_role.role_id, moderate_role.role_id);
    assert_eq!(deleted_moderator_role.user_id, moderate_role.user_id);
    assert_eq!(deleted_moderator_role.username, moderate_role.username);
    assert_eq!(deleted_moderator_role.sphere_id, moderate_role.sphere_id);
    assert_eq!(deleted_moderator_role.sphere_name, moderate_role.sphere_name);
    assert_eq!(deleted_moderator_role.grantor_id, moderate_role.grantor_id);
    assert_eq!(deleted_moderator_role.permission_level, PermissionLevel::Moderate);
    assert!(deleted_moderator_role.delete_timestamp.is_some_and(|delete_timestamp| delete_timestamp > moderate_role.create_timestamp));

    // test can now elect other moderators
    let (moderate_role_2, prev_leader_id) = set_user_sphere_role(
        &ordinary_user.username,
        sphere_name,
        PermissionLevel::Moderate,
        &moderator,
        &db_pool,
    ).await.expect("Should set moderator role");

    assert_eq!(moderate_role_2.user_id, ordinary_user.user_id);
    assert_eq!(moderate_role_2.sphere_id, sphere.sphere_id);
    assert_eq!(moderate_role_2.sphere_name, sphere.sphere_name);
    assert_eq!(moderate_role_2.grantor_id, moderator.user_id);
    assert_eq!(moderate_role_2.permission_level, PermissionLevel::Moderate);
    assert_eq!(prev_leader_id, None);
    let ordinary_user = User::get(ordinary_user.user_id, &db_pool)
        .await
        .expect("Should be able to reload ordinary_user.");
    assert_eq!(
        ordinary_user.permission_by_sphere_name_map.get(sphere_name),
        Some(&PermissionLevel::Moderate)
    );

    // test moderator cannot set leader or downgrade higher up moderator
    assert!(
        set_user_sphere_role(
            &ordinary_user.username,
            sphere_name,
            PermissionLevel::Lead,
            &moderator,
            &db_pool
        ).await.is_err()
    );
    assert_eq!(
        set_user_sphere_role(
            &lead_user.username,
            sphere_name,
            PermissionLevel::Moderate,
            &moderator,
            &db_pool
        ).await,
        Err(AppError::InsufficientPrivileges)
    );

    // test leader can choose another leader
    let (new_lead_role, prev_leader_id) = set_user_sphere_role(
        &ordinary_user.username,
        sphere_name,
        PermissionLevel::Lead,
        &lead_user,
        &db_pool,
    ).await.expect("lead_user should be able to elect new leader.");

    assert_eq!(new_lead_role.user_id, ordinary_user.user_id);
    assert_eq!(new_lead_role.sphere_id, sphere.sphere_id);
    assert_eq!(new_lead_role.sphere_name, sphere.sphere_name);
    assert_eq!(new_lead_role.grantor_id, lead_user.user_id);
    assert_eq!(new_lead_role.permission_level, PermissionLevel::Lead);
    assert_eq!(prev_leader_id, Some(lead_user.user_id));
    let ordinary_user = User::get(ordinary_user.user_id, &db_pool)
        .await
        .expect("Should be able to reload ordinary_user after lead update.");
    assert_eq!(
        ordinary_user.permission_by_sphere_name_map.get(sphere_name),
        Some(&PermissionLevel::Lead)
    );
    let prev_lead_user = User::get(lead_user.user_id, &db_pool)
        .await
        .expect("Should be able to reload lead_use after lead update.");
    assert_eq!(
        prev_lead_user.permission_by_sphere_name_map.get(sphere_name),
        Some(&PermissionLevel::Manage)
    );

    // Check that roles that have been updated now have a deleted timestamp
    let deleted_mod_role_2 = get_user_role_by_id(moderate_role_2.role_id, &db_pool).await.expect("Should load deleted mod role");

    assert_eq!(deleted_mod_role_2.role_id, moderate_role_2.role_id);
    assert_eq!(deleted_mod_role_2.user_id, moderate_role_2.user_id);
    assert_eq!(deleted_mod_role_2.username, moderate_role_2.username);
    assert_eq!(deleted_mod_role_2.sphere_id, moderate_role_2.sphere_id);
    assert_eq!(deleted_mod_role_2.sphere_name, moderate_role_2.sphere_name);
    assert_eq!(deleted_mod_role_2.grantor_id, moderate_role_2.grantor_id);
    assert_eq!(deleted_mod_role_2.permission_level, PermissionLevel::Moderate);
    assert!(deleted_mod_role_2.delete_timestamp.is_some_and(|delete_timestamp| delete_timestamp > moderate_role_2.create_timestamp));

    // Check that roles that have been updated now have a deleted timestamp
    let deleted_lead_role = get_user_role_by_id(lead_role.role_id, &db_pool).await.expect("Should load deleted lead role");

    assert_eq!(deleted_lead_role.role_id, lead_role.role_id);
    assert_eq!(deleted_lead_role.user_id, lead_role.user_id);
    assert_eq!(deleted_lead_role.username, lead_role.username);
    assert_eq!(deleted_lead_role.sphere_id, lead_role.sphere_id);
    assert_eq!(deleted_lead_role.sphere_name, lead_role.sphere_name);
    assert_eq!(deleted_lead_role.grantor_id, lead_role.grantor_id);
    assert_eq!(deleted_lead_role.permission_level, PermissionLevel::Lead);
    assert!(deleted_lead_role.delete_timestamp.is_some_and(|delete_timestamp| delete_timestamp > lead_role.create_timestamp));
}

#[tokio::test]
async fn test_set_user_admin_role() -> Result<(), AppError> {

    let db_pool = get_db_pool().await;
    let ordinary_user = create_user("user", &db_pool).await;
    let moderator = create_user("mod", &db_pool).await;
    let mut admin = create_user("admin", &db_pool).await;

    // ordinary user cannot set admin role
    assert_eq!(set_user_admin_role(ordinary_user.user_id, AdminRole::Admin, &ordinary_user, &db_pool).await, Err(AppError::InsufficientPrivileges));
    assert_eq!(set_user_admin_role(ordinary_user.user_id, AdminRole::Moderator, &ordinary_user, &db_pool).await, Err(AppError::InsufficientPrivileges));
    assert_eq!(set_user_admin_role(ordinary_user.user_id, AdminRole::None, &ordinary_user, &db_pool).await, Err(AppError::InsufficientPrivileges));

    // admin can set admin roles
    admin.admin_role = AdminRole::Admin;
    let sql_admin = set_user_admin_role(admin.user_id, AdminRole::Admin, &admin, &db_pool).await.expect("Admin should be able to grant admin role.");
    assert_eq!(sql_admin.user_id, admin.user_id);
    assert_eq!(sql_admin.admin_role, AdminRole::Admin);
    let admin = User::get(admin.user_id, &db_pool).await.expect("Should be able to reload admin.");
    assert_eq!(admin.admin_role, AdminRole::Admin);

    let sql_moderator = set_user_admin_role(moderator.user_id, AdminRole::Moderator, &admin, &db_pool).await.expect("Admin should be able to grant moderator role.");
    assert_eq!(sql_moderator.user_id, moderator.user_id);
    assert_eq!(sql_moderator.admin_role, AdminRole::Moderator);
    let moderator = User::get(moderator.user_id, &db_pool).await.expect("Should be able to reload moderator.");
    assert_eq!(moderator.admin_role, AdminRole::Moderator);

    // moderator cannot set admin roles
    assert_eq!(set_user_admin_role(ordinary_user.user_id, AdminRole::Admin, &moderator, &db_pool).await, Err(AppError::InsufficientPrivileges));
    assert_eq!(set_user_admin_role(ordinary_user.user_id, AdminRole::Moderator, &moderator, &db_pool).await, Err(AppError::InsufficientPrivileges));
    assert_eq!(set_user_admin_role(ordinary_user.user_id, AdminRole::None, &moderator, &db_pool).await, Err(AppError::InsufficientPrivileges));

    Ok(())
}
