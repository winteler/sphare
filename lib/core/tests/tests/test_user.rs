use sphare_core_common::errors::AppError;
use sphare_core_content::moderation::ssr::ban_user_from_sphere;
use sphare_core_content::profile::ssr::{get_user_comment_vec, get_user_post_vec};
use sphare_core_content::ranking::{CommentSortType, PostSortType, SortType};
use sphare_core_sphere::rule::ssr::add_rule;
use sphare_core_user::role::ssr::get_user_sphere_role;
use sphare_core_user::role::AdminRole;
use sphare_core_user::user::ssr::{create_or_update_user, delete_user, set_user_settings};
use sphare_core_user::user::User;

use crate::common::{create_test_user, create_user, get_db_pool};
use crate::data_factory::{create_simple_post, create_sphere_with_post_and_comment};

mod common;
mod data_factory;

#[tokio::test]
async fn test_create_or_update_user() -> Result<(), AppError> {
    let db_pool = get_db_pool().await;

    let oidc_id = "a";
    let username = "b";
    let email = "c";

    let user = create_or_update_user(oidc_id, username, email, &db_pool).await.expect("Should create user");

    assert_eq!(user.oidc_id, oidc_id);
    assert_eq!(user.username, username);
    assert_eq!(user.email, email);
    assert_eq!(user.admin_role, AdminRole::None);
    assert_eq!(user.show_nsfw, false);
    assert_eq!(user.days_hide_spoiler, None);
    assert_eq!(user.delete_timestamp, None);

    let loaded_user = User::get(user.user_id, &db_pool).await.expect("Should get user");
    assert_eq!(loaded_user.user_id, user.user_id);
    assert_eq!(loaded_user.oidc_id, user.oidc_id);
    assert_eq!(loaded_user.username, user.username);
    assert_eq!(loaded_user.email, user.email);
    assert_eq!(loaded_user.admin_role, user.admin_role);
    assert_eq!(loaded_user.show_nsfw, user.show_nsfw);
    assert_eq!(loaded_user.days_hide_spoiler, user.days_hide_spoiler);
    assert_eq!(loaded_user.delete_timestamp, None);

    Ok(())
}

#[tokio::test]
async fn test_delete_user() {
    let db_pool = get_db_pool().await;
    let mut user = create_test_user(&db_pool).await;
    let banned_user = create_user("banned", &db_pool).await;

    let (sphere, _, _) = create_sphere_with_post_and_comment("sphere", &mut user, &db_pool).await;
    let rule = add_rule(&sphere.sphere_name, 0, "Don't", "pet the cat", false, &user, &db_pool).await.expect("Should add rule");
    let post_to_moderate = create_simple_post(&sphere.sphere_name, None, "ban me", "if you can", None, &user, &db_pool).await;
    ban_user_from_sphere(banned_user.user_id, sphere.sphere_id, post_to_moderate.post.post_id, None, rule.rule_id, Some(1), &user, &db_pool).await.expect("Should ban user");

    delete_user(&banned_user, &db_pool).await.expect("Should delete user");

    let deleted_ban_user = User::get(banned_user.user_id, &db_pool).await.expect("Should get user");
    assert_eq!(deleted_ban_user.user_id, banned_user.user_id);
    assert!(deleted_ban_user.ban_status_by_sphere_map.is_empty());
    let deleted_ban_user_post_vec = get_user_post_vec(&banned_user.username, SortType::Post(PostSortType::Hot), 1, 0, &db_pool).await.expect("Should get user posts");
    assert!(deleted_ban_user_post_vec.is_empty());

    delete_user(&user, &db_pool).await.expect("Should delete user");

    let deleted_user_post_vec = get_user_post_vec(&user.username, SortType::Post(PostSortType::Hot), 1, 0, &db_pool).await.expect("Should get user posts");
    let deleted_user_comment_vec = get_user_comment_vec(&user.username, SortType::Comment(CommentSortType::Recent), 1, 0, &db_pool).await.expect("Should get user comments");
    assert!(deleted_user_post_vec.is_empty());
    assert!(deleted_user_comment_vec.is_empty());
    assert_eq!(get_user_sphere_role(user.user_id, &sphere.sphere_name, &db_pool).await, Err(AppError::NotFound));

    let deleted_user = User::get(user.user_id, &db_pool).await.expect("Should get user");
    assert_eq!(deleted_user.user_id, user.user_id);
    assert!(deleted_user.ban_status_by_sphere_map.is_empty());
    assert!(deleted_user.permission_by_sphere_name_map.is_empty());
}

#[tokio::test]
async fn test_set_user_settings() {
    let db_pool = get_db_pool().await;
    let user = create_test_user(&db_pool).await;
    
    set_user_settings(true, true, 0, &user, &db_pool).await.expect("Should set user settings");
    let user = User::get(user.user_id, &db_pool).await.expect("Should get user");
    assert_eq!(user.is_nsfw, true);
    assert_eq!(user.show_nsfw, true);
    assert_eq!(user.days_hide_spoiler, None);

    set_user_settings(true, false, 1, &user, &db_pool).await.expect("Should set user settings");
    let user = User::get(user.user_id, &db_pool).await.expect("Should get user");
    assert_eq!(user.is_nsfw, true);
    assert_eq!(user.show_nsfw, false);
    assert_eq!(user.days_hide_spoiler, Some(1));

    set_user_settings(false, true, 10, &user, &db_pool).await.expect("Should set user settings");
    let user = User::get(user.user_id, &db_pool).await.expect("Should get user");
    assert_eq!(user.is_nsfw, false);
    assert_eq!(user.show_nsfw, true);
    assert_eq!(user.days_hide_spoiler, Some(10));

    set_user_settings(false, false, 0, &user, &db_pool).await.expect("Should set user preferences");
    let user = User::get(user.user_id, &db_pool).await.expect("Should get user");
    assert_eq!(user.is_nsfw, false);
    assert_eq!(user.show_nsfw, false);
    assert_eq!(user.days_hide_spoiler, None);
}
