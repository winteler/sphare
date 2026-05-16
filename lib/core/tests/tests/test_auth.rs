use std::ops::Add;

use chrono::Days;

use sphare_core_common::errors::AppError;
use sphare_core_content::moderation::ssr::ban_user_from_sphere;
use sphare_core_sphere::sphere;
use sphare_core_user::role::ssr::set_user_sphere_role;
use sphare_core_user::role::{AdminRole, PermissionLevel};
use sphare_core_user::user::ssr::{create_or_update_user, SqlUser};
use sphare_core_user::user::User;

use crate::common::{create_user, get_db_pool};
use crate::data_factory::{add_base_rule, create_sphere_with_post};

mod common;
mod data_factory;

#[tokio::test]
async fn test_sql_user_get_by_username() -> Result<(), AppError> {
    let db_pool = get_db_pool().await;
    let oidc_id = "id";
    let username = "username";
    let email = "user@user.com";
    let user = create_or_update_user(oidc_id, username, email, &db_pool).await.expect("Sql user should be created");
    let sql_user = SqlUser::get_by_username(&user.username, &db_pool).await?;

    assert_eq!(sql_user.user_id, user.user_id);
    assert_eq!(sql_user.oidc_id, oidc_id);
    assert_eq!(sql_user.username, username);
    assert_eq!(sql_user.email, email);
    assert_eq!(sql_user.admin_role, AdminRole::None);

    Ok(())
}

#[tokio::test]
async fn test_user_get() -> Result<(), AppError> {
    let db_pool = get_db_pool().await;
    let mut creator_user = create_user("creator", &db_pool).await;
    let test_user = create_user("user", &db_pool).await;
    let mut admin = create_user("admin", &db_pool).await;
    admin.admin_role = AdminRole::Admin;

    // Create utils rule to enable bans
    let rule = add_base_rule(0, "BeRespectful", "test", None, &admin, &db_pool).await.expect("Rule should be added.");

    let (sphere_a, _post_a) = create_sphere_with_post("a", &mut creator_user, &db_pool).await;
    let (sphere_b, _post_b) = create_sphere_with_post("b", &mut creator_user, &db_pool).await;
    let (sphere_c, post_c) = create_sphere_with_post("c", &mut creator_user, &db_pool).await;
    let (sphere_d, post_d) = create_sphere_with_post("d", &mut creator_user, &db_pool).await;
    let (sphere_e, post_e) = create_sphere_with_post("e", &mut creator_user, &db_pool).await;

    set_user_sphere_role(&test_user.username, &sphere_a.sphere_name, PermissionLevel::Moderate, &creator_user, &db_pool).await?;
    set_user_sphere_role(&test_user.username, &sphere_b.sphere_name, PermissionLevel::Manage, &creator_user, &db_pool).await?;

    assert_eq!(
        ban_user_from_sphere(test_user.user_id, sphere_c.sphere_id, post_c.post_id, None, rule.rule_id, Some(0), &creator_user, &db_pool).await.expect("User ban should be created for sphere c."),
        None
    );
    let sphere_ban_d = ban_user_from_sphere(test_user.user_id, sphere_d.sphere_id, post_d.post_id, None, rule.rule_id, Some(1), &creator_user, &db_pool)
        .await?
        .expect("User should have ban for sphere d.");
    ban_user_from_sphere(test_user.user_id, sphere_e.sphere_id, post_e.post_id, None, rule.rule_id, None, &creator_user, &db_pool).await
        .expect("User ban should be created for sphere e.")
        .expect("User should have ban for sphere e.");

    let result_user = User::get(test_user.user_id, &db_pool).await.expect("result_user should be available in DB.");

    assert_eq!(result_user.user_id, test_user.user_id);
    assert_eq!(result_user.oidc_id, test_user.oidc_id);
    assert_eq!(result_user.username, test_user.username);
    assert_eq!(result_user.email, test_user.email);
    assert_eq!(result_user.admin_role, test_user.admin_role);
    assert_eq!(result_user.show_nsfw, test_user.show_nsfw);
    assert_eq!(result_user.days_hide_spoiler, test_user.days_hide_spoiler);
    
    assert_eq!(result_user.check_sphere_permissions_by_name(&sphere_a.sphere_name, PermissionLevel::Moderate), Ok(()));
    assert_eq!(result_user.check_sphere_permissions_by_name(&sphere_b.sphere_name, PermissionLevel::Moderate), Ok(()));
    assert_eq!(result_user.check_sphere_permissions_by_name(&sphere_c.sphere_name, PermissionLevel::Moderate), Err(AppError::InsufficientPrivileges));
    assert_eq!(result_user.check_sphere_permissions_by_name(&sphere_d.sphere_name, PermissionLevel::Moderate), Err(AppError::InsufficientPrivileges));
    assert_eq!(result_user.check_sphere_permissions_by_name(&sphere_e.sphere_name, PermissionLevel::Moderate), Err(AppError::InsufficientPrivileges));

    assert_eq!(result_user.check_can_publish_on_sphere(&sphere_a.sphere_name), Ok(()));
    assert_eq!(result_user.check_can_publish_on_sphere(&sphere_b.sphere_name), Ok(()));
    assert_eq!(result_user.check_can_publish_on_sphere(&sphere_c.sphere_name), Ok(()));
    assert_eq!(result_user.check_can_publish_on_sphere(&sphere_d.sphere_name), Err(AppError::SphereBanUntil(sphere_ban_d.create_timestamp.add(Days::new(1)))));
    assert_eq!(result_user.check_can_publish_on_sphere(&sphere_e.sphere_name), Err(AppError::PermanentSphereBan));

    // TODO test global ban when ssr function is implemented

    Ok(())
}

#[tokio::test]
async fn test_user_check_can_set_user_sphere_role() -> Result<(), AppError> {
    let db_pool = get_db_pool().await;
    let lead_user = create_user("lead", &db_pool).await;
    let manage_mod = create_user("elect", &db_pool).await;
    let simple_mod = create_user("mod", &db_pool).await;
    let mut global_moderator = create_user("gmod", &db_pool).await;
    let mut admin = create_user("admin", &db_pool).await;
    let std_user  = create_user("std", &db_pool).await;
    let test_user = create_user("test", &db_pool).await;

    let sphere_name = "sphere";
    let sphere = sphere::ssr::create_sphere(sphere_name, "sphere", false, &lead_user, &db_pool).await?;
    let lead_user = User::get(lead_user.user_id, &db_pool)
        .await
        .expect("Should be able to reload lead_user.");

    // set user roles
    set_user_sphere_role(&manage_mod.username, &sphere.sphere_name, PermissionLevel::Manage, &lead_user, &db_pool)
        .await.expect("Moderate role should be assignable by lead_user.");
    set_user_sphere_role(&simple_mod.username, &sphere.sphere_name, PermissionLevel::Ban, &lead_user, &db_pool)
        .await.expect("Moderate role should be assignable by lead_user.");
    let manage_mod = User::get(manage_mod.user_id, &db_pool).await.expect("Should be able to get elect mod.");
    let simple_mod = User::get(simple_mod.user_id, &db_pool).await.expect("Should be able to get simple mod.");
    admin.admin_role = AdminRole::Admin;
    global_moderator.admin_role = AdminRole::Moderator;

    // normal user, simple moderator, global moderator cannot set any role
    assert_eq!(
        std_user.check_can_set_user_sphere_role(PermissionLevel::Moderate, test_user.user_id, sphere_name, &db_pool).await,
        Err(AppError::InsufficientPrivileges)
    );
    assert_eq!(
        simple_mod.check_can_set_user_sphere_role(PermissionLevel::Moderate, test_user.user_id, sphere_name, &db_pool).await,
        Err(AppError::InsufficientPrivileges)
    );
    assert_eq!(
        global_moderator.check_can_set_user_sphere_role(PermissionLevel::Moderate, test_user.user_id, sphere_name, &db_pool).await,
        Err(AppError::InsufficientPrivileges)
    );

    // manage mods can set user role for normal users, moderators with a lower level but not manage mods and leaders
    assert_eq!(
        manage_mod.check_can_set_user_sphere_role(PermissionLevel::Ban, test_user.user_id, sphere_name, &db_pool).await,
        Ok(())
    );
    assert_eq!(
        manage_mod.check_can_set_user_sphere_role(PermissionLevel::None, simple_mod.user_id, sphere_name, &db_pool).await,
        Ok(())
    );
    assert_eq!(
        manage_mod.check_can_set_user_sphere_role(PermissionLevel::Ban, simple_mod.user_id, sphere_name, &db_pool).await,
        Ok(())
    );
    assert_eq!(
        manage_mod.check_can_set_user_sphere_role(PermissionLevel::Manage, simple_mod.user_id, sphere_name, &db_pool).await,
        Err(AppError::InsufficientPrivileges)
    );
    assert_eq!(
        manage_mod.check_can_set_user_sphere_role(PermissionLevel::Lead, simple_mod.user_id, sphere_name, &db_pool).await,
        Err(AppError::InsufficientPrivileges)
    );
    assert_eq!(
        manage_mod.check_can_set_user_sphere_role(PermissionLevel::Moderate, manage_mod.user_id, sphere_name, &db_pool).await,
        Err(AppError::InsufficientPrivileges)
    );
    assert_eq!(
        manage_mod.check_can_set_user_sphere_role(PermissionLevel::Moderate, lead_user.user_id, sphere_name, &db_pool).await,
        Err(AppError::InsufficientPrivileges)
    );
    assert_eq!(
        manage_mod.check_can_set_user_sphere_role(PermissionLevel::Manage, lead_user.user_id, sphere_name, &db_pool).await,
        Err(AppError::InsufficientPrivileges)
    );

    // lead users and admin can set user role for everyone (lead user cannot set for himself, as this function is not used for leader changes)
    assert_eq!(
        lead_user.check_can_set_user_sphere_role(PermissionLevel::Ban, test_user.user_id, sphere_name, &db_pool).await,
        Ok(())
    );
    assert_eq!(
        lead_user.check_can_set_user_sphere_role(PermissionLevel::Manage, test_user.user_id, sphere_name, &db_pool).await,
        Ok(())
    );
    assert_eq!(
        lead_user.check_can_set_user_sphere_role(PermissionLevel::Manage, simple_mod.user_id, sphere_name, &db_pool).await,
        Ok(())
    );
    assert_eq!(
        lead_user.check_can_set_user_sphere_role(PermissionLevel::None, manage_mod.user_id, sphere_name, &db_pool).await,
        Ok(())
    );
    assert_eq!(
        lead_user.check_can_set_user_sphere_role(PermissionLevel::Manage, lead_user.user_id, sphere_name, &db_pool).await,
        Err(AppError::InsufficientPrivileges)
    );
    assert_eq!(
        admin.check_can_set_user_sphere_role(PermissionLevel::Lead, test_user.user_id, sphere_name, &db_pool).await,
        Ok(())
    );
    assert_eq!(
        admin.check_can_set_user_sphere_role(PermissionLevel::Manage, simple_mod.user_id, sphere_name, &db_pool).await,
        Ok(())
    );
    assert_eq!(
        admin.check_can_set_user_sphere_role(PermissionLevel::Moderate, manage_mod.user_id, sphere_name, &db_pool).await,
        Ok(())
    );
    // An admin cannot reduce a leader's permission, but can instead set another user as leader
    assert_eq!(
        admin.check_can_set_user_sphere_role(PermissionLevel::None, lead_user.user_id, sphere_name, &db_pool).await,
        Err(AppError::InsufficientPrivileges)
    );

    Ok(())
}

#[tokio::test]
async fn test_create_user() -> Result<(), AppError> {
    let db_pool = get_db_pool().await;
    let user_1_value = "1";
    let sql_user_1 = create_or_update_user(user_1_value, user_1_value, user_1_value, &db_pool).await.expect("Sql user 1 should be created");
    assert_eq!(sql_user_1.oidc_id, user_1_value);
    assert_eq!(sql_user_1.username, user_1_value);
    assert_eq!(sql_user_1.email, user_1_value);
    assert_eq!(sql_user_1.admin_role, AdminRole::None);
    assert_eq!(sql_user_1.delete_timestamp, None);

    // test cannot create user with duplicate username or email
    let user_2_value = "2";
    assert!(create_or_update_user(user_2_value, user_1_value, user_2_value, &db_pool).await.is_err());
    assert!(create_or_update_user(user_2_value, user_2_value, user_1_value, &db_pool).await.is_err());

    let sql_user_2 = create_or_update_user(user_2_value, user_2_value, user_2_value, &db_pool).await.expect("Sql user 2 should be created");
    assert_eq!(sql_user_2.oidc_id, user_2_value);
    assert_eq!(sql_user_2.username, user_2_value);
    assert_eq!(sql_user_2.email, user_2_value);
    assert_eq!(sql_user_2.admin_role, AdminRole::None);
    assert_eq!(sql_user_2.delete_timestamp, None);

    let user_1 = User::get(sql_user_1.user_id, &db_pool).await.expect("Should be able to get user 1");
    assert_eq!(user_1.user_id, sql_user_1.user_id);
    assert_eq!(user_1.oidc_id, sql_user_1.oidc_id);
    assert_eq!(user_1.username, sql_user_1.username);
    assert_eq!(user_1.email, sql_user_1.email);
    assert_eq!(user_1.admin_role, sql_user_1.admin_role);
    assert_eq!(user_1.delete_timestamp, sql_user_1.delete_timestamp);

    let user_1_updated_value = "3";
    let sql_user_1_updated = create_or_update_user(
        user_1_value,
        user_1_updated_value,
        user_1_updated_value,
        &db_pool
    ).await.expect("user 1 should be updated");
    assert_eq!(sql_user_1_updated.oidc_id, user_1_value);
    assert_eq!(sql_user_1_updated.username, user_1_updated_value);
    assert_eq!(sql_user_1_updated.email, user_1_updated_value);
    assert_eq!(sql_user_1_updated.admin_role, AdminRole::None);
    assert_eq!(sql_user_1_updated.delete_timestamp, None);

    Ok(())
}