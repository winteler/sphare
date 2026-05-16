use object_store::memory::InMemory;
use object_store::ObjectStoreExt;
use sphare_core_common::constants::{IMAGE_FILE_PARAM, SPHERE_NAME_PARAM};
use sphare_core_common::errors::AppError;
use sphare_core_content::comment::ssr::create_comment_with_notif;
use sphare_core_content::moderation::ssr::{ban_user_from_sphere};
use sphare_core_content::post::{PostDataInputs, PostLocation};
use sphare_core_content::post::ssr::create_post_and_vote;
use sphare_core_sphere::rule::ssr::add_rule;
use sphare_core_sphere::sphere::ssr::{create_sphere, get_sphere_by_name};
use sphare_core_sphere::sphere_management::ssr::{delete_sphere_image, get_sphere_ban_vec, remove_user_ban, set_sphere_banner_url, set_sphere_icon_url, set_sphere_image, store_sphere_image, SphereImageType, MAX_ICON_SIZE};
use sphare_core_sphere::sphere_management::ssr::{BANNER_FILE_INFER_ERROR_STR, INCORRECT_BANNER_FILE_TYPE_STR, MISSING_BANNER_FILE_STR, MISSING_SPHERE_STR};
use sphare_core_user::role::ssr::{is_user_sphere_moderator, set_user_admin_role};
use sphare_core_user::role::AdminRole;
use sphare_core_user::user::User;

use crate::common::*;
use crate::data_factory::{create_sphere_with_post};
use crate::utils::*;

mod common;
mod data_factory;
mod utils;

#[tokio::test]
async fn test_get_sphere_ban_vec() -> Result<(), AppError> {
    let db_pool = get_db_pool().await;
    let mut lead = create_user("test", &db_pool).await;
    let banned_user_1 = create_user("1", &db_pool).await;
    let banned_user_2 = create_user("2", &db_pool).await;

    let (sphere, post) = create_sphere_with_post("sphere", &mut lead, &db_pool).await;

    let rule = add_rule(&sphere.sphere_name, 0, "test", "test", false, &lead, &db_pool).await.expect("Rule should be added.");

    let ban_user_1 = ban_user_from_sphere(
        banned_user_1.user_id,
        sphere.sphere_id,
        post.post_id,
        None,
        rule.rule_id,
        Some(1),
        &lead,
        &db_pool
    ).await.expect("User 1 should be banned").expect("User 1 ban should be Some.");
    let banned_user_vec = get_sphere_ban_vec(&sphere.sphere_name, "", &db_pool).await.expect("Should load sphere bans");
    assert_eq!(banned_user_vec.len(), 1);
    assert!(banned_user_vec.contains(&ban_user_1));

    let ban_user_2 = ban_user_from_sphere(
        banned_user_2.user_id,
        sphere.sphere_id,
        post.post_id,
        None,
        rule.rule_id,
        Some(7),
        &lead,
        &db_pool
    ).await.expect("User 2 should be banned").expect("User 2 ban should be Some.");
    let banned_user_vec = get_sphere_ban_vec(&sphere.sphere_name, "", &db_pool).await.expect("Should load sphere bans");
    assert_eq!(banned_user_vec.len(), 2);
    assert!(banned_user_vec.contains(&ban_user_1));
    assert!(banned_user_vec.contains(&ban_user_2));

    let banned_user_vec = get_sphere_ban_vec(&sphere.sphere_name, "1", &db_pool).await.expect("Should load sphere bans");
    assert_eq!(banned_user_vec.len(), 1);
    assert!(banned_user_vec.contains(&ban_user_1));

    let banned_user_vec = get_sphere_ban_vec(&sphere.sphere_name, "x", &db_pool).await.expect("Should load sphere bans");
    assert_eq!(banned_user_vec.len(), 0);

    Ok(())
}

#[tokio::test]
async fn test_remove_user_ban() -> Result<(), AppError> {
    let db_pool = get_db_pool().await;
    let mut lead = create_user("test", &db_pool).await;
    let mut global_mod = create_user("global", &db_pool).await;
    global_mod.admin_role = AdminRole::Moderator;
    let banned_user_1 = create_user("1", &db_pool).await;

    let (sphere, post) = create_sphere_with_post("sphere", &mut lead, &db_pool).await;
    
    let rule = add_rule(&sphere.sphere_name, 0, "test", "test", false, &lead, &db_pool).await.expect("Rule should be added.");

    let ban_user_1 = ban_user_from_sphere(
        banned_user_1.user_id,
        sphere.sphere_id,
        post.post_id,
        None,
        rule.rule_id,
        Some(1),
        &lead,
        &db_pool
    ).await.expect("User 1 should be banned").expect("User 1 ban should be Some.");
    let banned_user_vec = get_sphere_ban_vec(&sphere.sphere_name, "", &db_pool).await.expect("Should load sphere bans");
    assert_eq!(banned_user_vec.len(), 1);
    assert!(banned_user_vec.contains(&ban_user_1));

    assert_eq!(remove_user_ban(ban_user_1.ban_id, &banned_user_1, &db_pool).await, Err(AppError::InsufficientPrivileges));
    assert_eq!(remove_user_ban(ban_user_1.ban_id, &lead, &db_pool).await, Ok(ban_user_1));

    let banned_user_vec = get_sphere_ban_vec(&sphere.sphere_name, "", &db_pool).await.expect("Should load sphere bans");
    assert!(banned_user_vec.is_empty());

    // Check user can again create posts and comments
    let (post, _, _) = create_post_and_vote(
        PostLocation {
            sphere: sphere.sphere_name.clone(),
            satellite_id: None,
        },
        PostDataInputs {
            title: "a".to_string(),
            body: "b".to_string(),
            is_markdown: false,
            embed_type: Default::default(),
            link: None,
            post_tags: Default::default(),
        },
        &banned_user_1,
        &db_pool,
    ).await.expect("Should create post and vote");

    create_comment_with_notif(post.post_id, None, "c", false, false, &banned_user_1, &db_pool).await.expect("Should create comment");

    let ban_user_1 = ban_user_from_sphere(
        banned_user_1.user_id,
        sphere.sphere_id,
        post.post_id,
        None,
        rule.rule_id,
        Some(1),
        &lead,
        &db_pool
    ).await.expect("User 1 should be banned").expect("User 1 ban should be Some.");
    let banned_user_vec = get_sphere_ban_vec(&sphere.sphere_name, "", &db_pool).await.expect("Should load sphere bans");
    assert_eq!(banned_user_vec.len(), 1);
    assert!(banned_user_vec.contains(&ban_user_1));

    assert_eq!(remove_user_ban(ban_user_1.ban_id, &global_mod, &db_pool).await, Ok(ban_user_1.clone()));

    let banned_user_vec = get_sphere_ban_vec(&sphere.sphere_name, "", &db_pool).await.expect("Should load sphere bans");
    assert!(banned_user_vec.is_empty());

    let removed_ban = get_user_ban_by_id(ban_user_1.ban_id, &db_pool).await?;
    assert_eq!(removed_ban.ban_id, ban_user_1.ban_id);
    assert_eq!(removed_ban.user_id, ban_user_1.user_id);
    assert_eq!(removed_ban.username, ban_user_1.username);
    assert_eq!(removed_ban.sphere_id, ban_user_1.sphere_id);
    assert_eq!(removed_ban.sphere_name, ban_user_1.sphere_name);
    assert_eq!(removed_ban.post_id, ban_user_1.post_id);
    assert_eq!(removed_ban.comment_id, ban_user_1.comment_id);
    assert_eq!(removed_ban.infringed_rule_id, ban_user_1.infringed_rule_id);
    assert_eq!(removed_ban.moderator_id, ban_user_1.moderator_id);
    assert_eq!(removed_ban.until_timestamp, ban_user_1.until_timestamp);
    assert!(removed_ban.delete_timestamp.is_some_and(|delete_timestamp| delete_timestamp > removed_ban.create_timestamp));

    // TODO add test to remove global ban when possible to create it

    Ok(())
}

#[tokio::test]
async fn test_is_user_sphere_moderator() -> Result<(), AppError> {
    let db_pool = get_db_pool().await;
    let user = create_user("test", &db_pool).await;
    let mut global_moderator = create_user("mod", &db_pool).await;
    let mut admin = create_user("admin", &db_pool).await;
    // set user role in the DB, needed to test that global Moderators/Admin cannot be banned
    global_moderator.admin_role = AdminRole::Moderator;
    admin.admin_role = AdminRole::Admin;
    set_user_admin_role(global_moderator.user_id, AdminRole::Moderator, &admin, &db_pool).await?;
    set_user_admin_role(admin.user_id, AdminRole::Admin, &admin, &db_pool).await?;
    let ordinary_user = create_user("user", &db_pool).await;

    let sphere = create_sphere("sphere", "a", false, &user, &db_pool).await?;

    assert_eq!(is_user_sphere_moderator(user.user_id, sphere.sphere_id, &db_pool).await, Ok(true));
    assert_eq!(is_user_sphere_moderator(global_moderator.user_id, sphere.sphere_id, &db_pool).await, Ok(true));
    assert_eq!(is_user_sphere_moderator(admin.user_id, sphere.sphere_id, &db_pool).await, Ok(true));
    assert_eq!(is_user_sphere_moderator(ordinary_user.user_id, sphere.sphere_id, &db_pool).await, Ok(false));
    assert!(is_user_sphere_moderator(ordinary_user.user_id + 1, sphere.sphere_id, &db_pool).await.is_err());

    Ok(())
}

#[tokio::test]
async fn test_set_sphere_image() {
    let db_pool = get_db_pool().await;
    let user = create_test_user(&db_pool).await;
    let sphere = create_sphere("sphere", "a", false, &user, &db_pool).await.expect("Should create sphere");
    let user = User::get(user.user_id, &db_pool).await.expect("Should get user after sphere creation");
    let object_store = InMemory::new();
    let container_url = "https://objectstorage.com";
    let icon_bucket_name = "icon_bucket";
    let banner_bucket_name = "banner_bucket";

    let icon_url = set_sphere_image(
        SphereImageType::ICON,
        get_multipart_image_with_string(IMAGE_FILE_PARAM, SPHERE_NAME_PARAM, &sphere.sphere_name).await,
        &object_store,
        container_url,
        icon_bucket_name,
        &user,
        &db_pool,
    ).await.expect("Should set sphere icon");

    let banner_url = set_sphere_image(
        SphereImageType::BANNER,
        get_multipart_image_with_string(IMAGE_FILE_PARAM, SPHERE_NAME_PARAM, &sphere.sphere_name).await,
        &object_store,
        container_url,
        banner_bucket_name,
        &user,
        &db_pool,
    ).await.expect("Should set sphere banner");

    let updated_sphere = get_sphere_by_name(&sphere.sphere_name, &db_pool).await.expect("Should get sphere");

    assert_eq!(updated_sphere.icon_url, icon_url);
    assert_eq!(updated_sphere.banner_url, banner_url);

    let icon_url = updated_sphere.icon_url.expect("Sphere icon should be set");
    let banner_url = updated_sphere.banner_url.expect("Sphere icon should be set");

    let icon_filename = icon_url.split('/').next_back().expect("Should get icon filename");
    let banner_filename = banner_url.split('/').next_back().expect("Should get icon filename");

    assert_eq!(icon_url, format!("{container_url}/{icon_bucket_name}/{icon_filename}"));
    assert_eq!(banner_url, format!("{container_url}/{banner_bucket_name}/{banner_filename}"));

    assert!(object_store.get(&object_store::path::Path::from(icon_filename)).await.is_ok());
    assert!(object_store.get(&object_store::path::Path::from(banner_filename)).await.is_ok());

    let updated_icon_url = set_sphere_image(
        SphereImageType::ICON,
        get_multipart_image_with_string(IMAGE_FILE_PARAM, SPHERE_NAME_PARAM, &sphere.sphere_name).await,
        &object_store,
        container_url,
        icon_bucket_name,
        &user,
        &db_pool,
    ).await.expect("Should update sphere icon");

    assert!(object_store.get(&object_store::path::Path::from(icon_filename)).await.is_err());

    let updated_sphere = get_sphere_by_name(&sphere.sphere_name, &db_pool).await.expect("Should get sphere");
    assert_eq!(updated_sphere.icon_url, updated_icon_url);

    let icon_url = updated_icon_url.expect("Updated sphere icon should be set");
    let icon_filename = icon_url.split('/').next_back().expect("Should get updated icon filename");

    assert_eq!(icon_url, format!("{container_url}/{icon_bucket_name}/{icon_filename}"));

    assert!(object_store.get(&object_store::path::Path::from(icon_filename)).await.is_ok());
    assert!(object_store.get(&object_store::path::Path::from(banner_filename)).await.is_ok());
}

#[tokio::test]
async fn test_delete_sphere_image() {
    let db_pool = get_db_pool().await;
    let user = create_test_user(&db_pool).await;
    let base_user = create_user("base", &db_pool).await;
    let sphere = create_sphere("sphere", "a", false, &user, &db_pool).await.expect("Should create sphere");
    let object_store = InMemory::new();
    // reload user to have updated permissions
    let user = User::get(user.user_id, &db_pool).await.expect("Should reload user.");

    let (sphere_name, image_file_name) = store_sphere_image(
        get_multipart_image_with_string(IMAGE_FILE_PARAM, SPHERE_NAME_PARAM, &sphere.sphere_name).await,
        MAX_ICON_SIZE,
        &object_store,
        &user,
    ).await.expect("Should store image");

    set_sphere_icon_url(
        &sphere_name.clone(),
        image_file_name.clone().map(|file_name| format!("https://test.com/{file_name}")).as_deref(),
        &user,
        &db_pool
    ).await.expect("Should set sphere icon url");

    let image_file_name = image_file_name.expect("Should have file name.");
    assert!(object_store.get(&object_store::path::Path::from(image_file_name.clone())).await.is_ok());

    delete_sphere_image(
        &sphere_name,
        SphereImageType::ICON,
        &object_store,
        &base_user,
        &db_pool
    ).await.expect_err("Base user should not have permission to store sphere image");

    delete_sphere_image(
        &sphere_name,
        SphereImageType::ICON,
        &object_store,
        &user,
        &db_pool
    ).await.expect("Should delete Sphere icon");

    assert!(object_store.get(&object_store::path::Path::from(image_file_name)).await.is_err());
}

#[tokio::test]
async fn test_store_sphere_image() {
    let db_pool = get_db_pool().await;
    let user = create_test_user(&db_pool).await;
    let base_user = create_user("base", &db_pool).await;
    let sphere = create_sphere("sphere", "a", false, &user, &db_pool).await.expect("Should create sphere");
    let object_store = InMemory::new();
    // reload user to have updated permissions
    let user = User::get(user.user_id, &db_pool).await.expect("Should reload user.");

    // Test need manage permissions to store image

    store_sphere_image(
        get_multipart_image_with_string(IMAGE_FILE_PARAM, SPHERE_NAME_PARAM, &sphere.sphere_name).await,
        MAX_ICON_SIZE,
        &object_store,
        &base_user,
    ).await.expect_err("Base user should not have permission to store sphere image");

    let (sphere_name, image_file_name) = store_sphere_image(
        get_multipart_image_with_string(IMAGE_FILE_PARAM, SPHERE_NAME_PARAM, &sphere.sphere_name).await,
        MAX_ICON_SIZE,
        &object_store,
        &user,
    ).await.expect("Should store image");
    assert_eq!(sphere_name, sphere.sphere_name);
    assert!(image_file_name.clone().is_some_and(|file_name| file_name.starts_with(&sphere_name) && file_name.ends_with(".webp")));
    assert!(object_store.get(&object_store::path::Path::from(image_file_name.unwrap())).await.is_ok());
    assert_eq!(
        store_sphere_image(
            get_multipart_image(IMAGE_FILE_PARAM).await,
            MAX_ICON_SIZE,
            &object_store,
            &user,
        ).await,
        Err(AppError::new(MISSING_SPHERE_STR))
    );
    assert_eq!(
        store_sphere_image(
            get_multipart_string(SPHERE_NAME_PARAM, &sphere.sphere_name).await,
            MAX_ICON_SIZE,
            &object_store,
            &user,
        ).await,
        Err(AppError::new(MISSING_BANNER_FILE_STR))
    );
    assert_eq!(
        store_sphere_image(
            get_multipart_pdf_with_string(IMAGE_FILE_PARAM, SPHERE_NAME_PARAM, &sphere.sphere_name).await,
            MAX_ICON_SIZE,
            &object_store,
            &user,
        ).await,
        Err(AppError::new(INCORRECT_BANNER_FILE_TYPE_STR))
    );
    assert_eq!(
        store_sphere_image(
            get_invalid_multipart_image_with_string(IMAGE_FILE_PARAM, SPHERE_NAME_PARAM, &sphere.sphere_name).await,
            MAX_ICON_SIZE,
            &object_store,
            &user,
        ).await,
        Err(AppError::new(BANNER_FILE_INFER_ERROR_STR))
    );
}

#[tokio::test]
async fn test_set_sphere_icon_url() -> Result<(), AppError> {
    let db_pool = get_db_pool().await;
    let user = create_test_user(&db_pool).await;
    let base_user = create_user("base", &db_pool).await;
    let sphere = create_sphere("sphere", "a", false, &user, &db_pool).await?;
    // reload user to have updated permissions
    let user = User::get(user.user_id, &db_pool).await.expect("Should reload user.");
    let icon_url = "a";
    assert_eq!(sphere.icon_url, None);

    set_sphere_icon_url(&sphere.sphere_name, Some(icon_url), &user, &db_pool).await?;
    let sphere = get_sphere_by_name(&sphere.sphere_name, &db_pool).await?;
    assert_eq!(sphere.icon_url, Some(String::from(icon_url)));

    set_sphere_icon_url(&sphere.sphere_name, Some(icon_url), &base_user, &db_pool).await.expect_err("Base user should not have permission to set sphere icon url");

    Ok(())
}

#[tokio::test]
async fn test_set_sphere_banner_url() -> Result<(), AppError> {
    let db_pool = get_db_pool().await;
    let user = create_test_user(&db_pool).await;
    let base_user = create_user("base", &db_pool).await;
    let sphere = create_sphere("sphere", "a", false, &user, &db_pool).await?;
    // reload user to have updated permissions
    let user = User::get(user.user_id, &db_pool).await.expect("Should reload user.");
    let banner_url = "a";
    assert_eq!(sphere.banner_url, None);

    set_sphere_banner_url(&sphere.sphere_name, Some(banner_url), &user, &db_pool).await?;
    let sphere = get_sphere_by_name(&sphere.sphere_name, &db_pool).await?;
    assert_eq!(sphere.banner_url, Some(String::from(banner_url)));

    set_sphere_banner_url(&sphere.sphere_name, Some(banner_url), &base_user, &db_pool).await.expect_err("Base user should not have permission to set sphere banner url");

    Ok(())
}