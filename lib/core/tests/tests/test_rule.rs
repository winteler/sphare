use leptos::prelude::*;
use leptos_fluent::tr;
use sphare_core_common::editor::ssr::get_html_and_markdown_strings;
use sphare_core_common::errors::AppError;
use sphare_core_sphere::rule::ssr::{add_rule, remove_rule, update_rule};
use sphare_core_sphere::rule::ssr::{get_rule_vec, load_rule_by_id};
use sphare_core_sphere::rule::{get_rule_description, get_rule_title, BaseRule};
use sphare_core_sphere::sphere::ssr::create_sphere;
use sphare_core_user::role::AdminRole;
use sphare_core_user::user::User;

use crate::common::{create_user, get_db_pool, get_i18n};
use crate::data_factory::{add_base_rule, remove_base_rule, update_base_rule};

mod common;
mod data_factory;

#[tokio::test]
async fn test_load_rule_by_id() -> Result<(), AppError> {
    let db_pool = get_db_pool().await;
    let user = create_user("test", &db_pool).await;
    let mut admin = create_user("admin", &db_pool).await;
    admin.admin_role = AdminRole::Admin;
    let sphere_1 = create_sphere("1", "a", false, &user, &db_pool).await?;
    let user = User::get(user.user_id, &db_pool).await.expect("User should be loaded after sphere creation");

    let expected_common_rule = add_base_rule(
        0,
        BaseRule::BeRespectful.into(),
        "0",
        None,
        &admin,
        &db_pool
    ).await.expect("Rule should be created.");

    let expected_sphere_rule = add_rule(
        &sphere_1.sphere_name,
        1,
        "sphere_1_rule_1",
        "test",
        true,
        &user,
        &db_pool
    ).await.expect("Rule should be created.");

    let common_rule = load_rule_by_id(expected_common_rule.rule_id, &db_pool).await?;
    let sphere_rule = load_rule_by_id(expected_sphere_rule.rule_id, &db_pool).await?;

    assert_eq!(common_rule, expected_common_rule);
    assert_eq!(sphere_rule, expected_sphere_rule);

    Ok(())
}

#[tokio::test]
async fn test_get_rule_vec() -> Result<(), AppError> {
    let db_pool = get_db_pool().await;
    let user = create_user("test", &db_pool).await;
    let mut admin = create_user("admin", &db_pool).await;
    admin.admin_role = AdminRole::Admin;
    let sphere_1 = create_sphere("1", "a", false, &user, &db_pool).await?;
    let sphere_2 = create_sphere("2", "b", false, &user, &db_pool).await?;
    let user = User::get(user.user_id, &db_pool).await.expect("User should be loaded after sphere creation");

    let common_rule_1 = add_base_rule(0, BaseRule::BeRespectful.into(), "0", None, &admin, &db_pool).await.expect("Rule should be created.");
    let common_rule_2 = add_base_rule(3, BaseRule::NoIllegalContent.into(), "0", Some("md"), &admin, &db_pool).await.expect("Rule should be created.");
    let sphere_1_rule_1 = add_rule(
        &sphere_1.sphere_name, 1, "sphere_1_rule_1", "test", false, &user, &db_pool
    ).await.expect("Rule should be created.");
    let sphere_1_rule_2 = add_rule(
        &sphere_1.sphere_name, 2, "sphere_1_rule_2", "test_md1_1", true, &user, &db_pool
    ).await.expect("Rule should be created.");
    let sphere_2_rule_1 = add_rule(
        &sphere_2.sphere_name, 1, "sphere_2_rule_1", "test", false, &user, &db_pool
    ).await.expect("Rule should be created.");

    let sphere_1_rule_vec = get_rule_vec(Some(&sphere_1.sphere_name), &db_pool).await.expect("Sphere rules should be loaded");
    assert_eq!(sphere_1_rule_vec.len(), 4);
    assert_eq!(sphere_1_rule_vec.first(), Some(&common_rule_1));
    assert_eq!(sphere_1_rule_vec.get(1), Some(&common_rule_2));
    assert_eq!(sphere_1_rule_vec.get(2), Some(&sphere_1_rule_1));
    assert_eq!(sphere_1_rule_vec.get(3), Some(&sphere_1_rule_2));

    let sphere_2_rule_vec = get_rule_vec(Some(&sphere_2.sphere_name), &db_pool).await.expect("Sphere rules should be loaded");
    assert_eq!(sphere_2_rule_vec.len(), 3);
    assert_eq!(sphere_2_rule_vec.first(), Some(&common_rule_1));
    assert_eq!(sphere_2_rule_vec.get(1), Some(&common_rule_2));
    assert_eq!(sphere_2_rule_vec.get(2), Some(&sphere_2_rule_1));

    let common_rule_vec = get_rule_vec(None, &db_pool).await.expect("Common rules should be loaded");
    assert_eq!(common_rule_vec.len(), 2);
    assert_eq!(common_rule_vec.first(), Some(&common_rule_1));
    assert_eq!(common_rule_vec.get(1), Some(&common_rule_2));

    Ok(())
}

#[tokio::test]
async fn test_add_rule() -> Result<(), AppError> {
    let db_pool = get_db_pool().await;
    let user = create_user("user", &db_pool).await;
    let lead = create_user("lead", &db_pool).await;
    let mut admin = create_user("admin", &db_pool).await;
    admin.admin_role = AdminRole::Admin;
    let sphere = create_sphere("sphere", "a", false, &lead, &db_pool).await?;
    let lead = User::get(lead.user_id, &db_pool).await.expect("User should be loaded after sphere creation");

    let title = BaseRule::BeRespectful.into();
    let description = "description";
    let (md_description, _) = get_html_and_markdown_strings(description, true).expect("Should get md description");

    let common_rule_1 = add_base_rule(0, title, description, None, &admin, &db_pool).await.expect("Rule should be created.");

    assert_eq!(add_rule(&sphere.sphere_name, 1, title, description, false, &user, &db_pool).await, Err(AppError::InsufficientPrivileges));
    let rule_1 = add_rule(&sphere.sphere_name, 1, title, description, false, &lead, &db_pool).await.expect("Rule should be created.");
    // creating rule_2 should increment rule_1's priority
    let rule_2 = add_rule(&sphere.sphere_name, 1, title, description, true, &admin, &db_pool).await.expect("Rule should be created.");

    assert_eq!(rule_1.sphere_id, Some(sphere.sphere_id));
    assert_eq!(rule_1.priority, 1);
    assert_eq!(rule_1.title, title);
    assert_eq!(rule_1.description, description);
    assert_eq!(rule_1.markdown_description, None);
    assert_eq!(rule_1.user_id, lead.user_id);

    assert_eq!(rule_2.sphere_id, Some(sphere.sphere_id));
    assert_eq!(rule_2.priority, 1);
    assert_eq!(rule_2.title, title);
    assert_eq!(rule_2.description, md_description);
    assert_eq!(rule_2.markdown_description.as_deref(), Some(description));
    assert_eq!(rule_2.user_id, admin.user_id);

    let common_rule_2 = add_base_rule(0, title, description, Some("common_md"), &admin, &db_pool).await.expect("Rule should be created.");
    let sphere_rule_vec = get_rule_vec(Some(&sphere.sphere_name), &db_pool).await.expect("Sphere rules should be loaded");
    assert_eq!(sphere_rule_vec.len(), 4);
    assert_eq!(sphere_rule_vec.first(), Some(&common_rule_2));
    assert_eq!(sphere_rule_vec.get(1).unwrap().rule_id, common_rule_1.rule_id);
    assert_eq!(sphere_rule_vec.get(2), Some(&rule_2));
    assert_eq!(sphere_rule_vec.get(3).unwrap().rule_id, rule_1.rule_id);

    Ok(())
}

#[tokio::test]
async fn test_update_rule() -> Result<(), AppError> {
    let db_pool = get_db_pool().await;
    let user = create_user("user", &db_pool).await;
    let lead = create_user("lead", &db_pool).await;
    let mut admin = create_user("admin", &db_pool).await;
    admin.admin_role = AdminRole::Admin;
    let sphere = create_sphere("sphere", "a", false, &lead, &db_pool).await?;
    let lead = User::get(lead.user_id, &db_pool).await.expect("User should be loaded after sphere creation");

    let title: &str = BaseRule::BeRespectful.into();
    let description = "description";
    let updated_title = BaseRule::PlatformIntegrity.into();
    let updated_desc = "updated";
    let (updated_md_desc, _) = get_html_and_markdown_strings(updated_desc, true).expect("Should get updated md description");

    let common_rule_1 = add_base_rule(0, title, description, None, &admin, &db_pool).await.expect("Rule should be created.");
    let common_rule_2 = add_base_rule(1, BaseRule::RespectRules.into(), "0", None, &admin, &db_pool).await.expect("Rule should be created.");
    let common_rule_3 = add_base_rule(2, BaseRule::RespectRules.into(), "0", Some("md"), &admin, &db_pool).await.expect("Rule should be created.");
    let rule_1 = add_rule(&sphere.sphere_name, 0, title, description, false, &lead, &db_pool).await.expect("Rule should be created.");
    let rule_2 = add_rule(&sphere.sphere_name, 1, title, description, false, &admin, &db_pool).await.expect("Rule should be created.");
    let rule_3 = add_rule(&sphere.sphere_name, 2, title, description, true, &admin, &db_pool).await.expect("Rule should be created.");

    let common_rule_1_updated = update_base_rule(0, 1, updated_title, updated_desc, None, &admin, &db_pool).await.expect("Rule should be updated.");
    assert_eq!(common_rule_1_updated.rule_key, common_rule_1.rule_key);
    assert_eq!(common_rule_1_updated.priority, 1);
    assert_eq!(common_rule_1_updated.sphere_id, None);
    assert_eq!(common_rule_1_updated.title, updated_title);
    assert_eq!(common_rule_1_updated.description, updated_desc);

    assert_eq!(update_rule(&sphere.sphere_name, 1, 0, updated_title, updated_desc, false, &user, &db_pool).await, Err(AppError::InsufficientPrivileges));
    let rule_2_updated = update_rule(
        &sphere.sphere_name, 1, 0, updated_title, updated_desc, true, &lead, &db_pool
    ).await.expect("Rule should be updated.");
    assert_eq!(rule_2_updated.rule_key, rule_2.rule_key);
    assert_eq!(rule_2_updated.priority, 0);
    assert_eq!(rule_2_updated.sphere_id, Some(sphere.sphere_id));
    assert_eq!(rule_2_updated.title, updated_title);
    assert_eq!(rule_2_updated.description, updated_md_desc);
    assert_eq!(rule_2_updated.markdown_description.as_deref(), Some(updated_desc));
    let rule_3_updated = update_rule(
        &sphere.sphere_name, 2, 1, updated_title, updated_desc, false, &admin, &db_pool
    ).await.expect("Rule should be updated.");
    assert_eq!(rule_3_updated.rule_key, rule_3.rule_key);
    assert_eq!(rule_3_updated.priority, 1);
    assert_eq!(rule_3_updated.sphere_id, Some(sphere.sphere_id));
    assert_eq!(rule_3_updated.title, updated_title);
    assert_eq!(rule_3_updated.description, updated_desc);
    assert_eq!(rule_3_updated.markdown_description, None);

    let sphere_rule_vec = get_rule_vec(Some(&sphere.sphere_name), &db_pool).await.expect("Sphere rules should be loaded");
    assert_eq!(sphere_rule_vec.len(), 6);
    assert_eq!(sphere_rule_vec.first().unwrap().rule_id, common_rule_2.rule_id);
    assert_eq!(sphere_rule_vec.get(1), Some(&common_rule_1_updated));
    assert_eq!(sphere_rule_vec.get(2), Some(&common_rule_3));
    assert_eq!(sphere_rule_vec.get(3), Some(&rule_2_updated));
    assert_eq!(sphere_rule_vec.get(4), Some(&rule_3_updated));
    assert_eq!(sphere_rule_vec.get(5).unwrap().rule_id, rule_1.rule_id);

    Ok(())
}

#[tokio::test]
async fn test_remove_rule() -> Result<(), AppError> {
    let db_pool = get_db_pool().await;
    let user = create_user("user", &db_pool).await;
    let lead = create_user("lead", &db_pool).await;
    let mut admin = create_user("admin", &db_pool).await;
    admin.admin_role = AdminRole::Admin;
    let sphere = create_sphere("sphere", "a", false, &lead, &db_pool).await?;
    let lead = User::get(lead.user_id, &db_pool).await.expect("User should be loaded after sphere creation");

    let title: &str = BaseRule::BeRespectful.into();
    let description = "description";

    let _common_rule_1 = add_base_rule(0, title, description, None, &admin, &db_pool).await.expect("Rule should be created.");
    let common_rule_2 = add_base_rule(1, BaseRule::RespectRules.into(), "0", Some("md"), &admin, &db_pool).await.expect("Rule should be created.");
    let _rule_1 = add_rule(&sphere.sphere_name, 0, title, description, false, &lead, &db_pool).await.expect("Rule should be created.");
    let rule_2 = add_rule(&sphere.sphere_name, 1, title, description, true, &admin, &db_pool).await.expect("Rule should be created.");

    assert_eq!(remove_base_rule(0, &admin, &db_pool).await, Ok(()));

    assert_eq!(remove_rule(&sphere.sphere_name, 0, &user, &db_pool).await, Err(AppError::InsufficientPrivileges));
    assert_eq!(remove_rule(&sphere.sphere_name, 0, &lead, &db_pool).await, Ok(()));

    let sphere_rule_vec = get_rule_vec(Some(&sphere.sphere_name), &db_pool).await.expect("Sphere rules should be loaded");
    assert_eq!(sphere_rule_vec.len(), 2);
    assert_eq!(sphere_rule_vec.first().unwrap().rule_id, common_rule_2.rule_id);
    assert_eq!(sphere_rule_vec.first().unwrap().priority, 0);
    assert_eq!(sphere_rule_vec.get(1).unwrap().rule_id, rule_2.rule_id);
    assert_eq!(sphere_rule_vec.get(1).unwrap().priority, 0);

    assert_eq!(remove_rule(&sphere.sphere_name, 0, &admin, &db_pool).await, Ok(()));

    let sphere_rule_vec = get_rule_vec(Some(&sphere.sphere_name), &db_pool).await.expect("Sphere rules should be loaded");
    assert_eq!(sphere_rule_vec.len(), 1);
    assert_eq!(sphere_rule_vec.first().unwrap().rule_id, common_rule_2.rule_id);

    Ok(())
}

#[test]
fn test_get_rule_title() {
    let owner = Owner::new();
    owner.set();

    provide_context(get_i18n());

    assert_eq!(get_rule_title(BaseRule::BeRespectful.into(), false).get_untracked(), tr!("rule-respectful-title"));
    assert_eq!(get_rule_title(BaseRule::RespectRules.into(), false).get_untracked(), tr!("rule-respect-rules-title"));
    assert_eq!(get_rule_title(BaseRule::NoIllegalContent.into(), false).get_untracked(), tr!("rule-no-illegal-content-title"));
    assert_eq!(get_rule_title(BaseRule::PlatformIntegrity.into(), false).get_untracked(), tr!("rule-platform-integrity-title"));
    assert_eq!(get_rule_title("test-non-base-rule", false).get_untracked(), tr!("rule-respectful-title"));
    assert_eq!(get_rule_title("test-non-base-rule", true).get_untracked(), "test-non-base-rule");
}

#[test]
fn test_get_rule_description() {
    let owner = Owner::new();
    owner.set();

    provide_context(get_i18n());

    assert_eq!(get_rule_description(BaseRule::BeRespectful.into(), "", false).get_untracked(), tr!("rule-respectful-description"));
    assert_eq!(get_rule_description(BaseRule::RespectRules.into(), "",false).get_untracked(), tr!("rule-respect-rules-description"));
    assert_eq!(get_rule_description(BaseRule::NoIllegalContent.into(), "", false).get_untracked(), tr!("rule-no-illegal-content-description"));
    assert_eq!(get_rule_description(BaseRule::PlatformIntegrity.into(), "", false).get_untracked(), tr!("rule-platform-integrity-description"));
    assert_eq!(get_rule_description("test-non-base-rule", "", false).get_untracked(), tr!("rule-respectful-description"));
    assert_eq!(get_rule_description("", "test-non-base-rule", true).get_untracked(), "test-non-base-rule");
}