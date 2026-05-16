use std::ops::Add;
use chrono::Days;

use sphare_core_common::errors::AppError;
use sphare_core_content::comment::ssr::{create_comment, get_comment_by_id};
use sphare_core_content::embed::Link;
use sphare_core_content::moderation::{Content};
use sphare_core_content::moderation::ssr::{ban_user_from_sphere, get_moderation_info, moderate_comment, moderate_comment_and_ban_user, moderate_post, moderate_post_and_ban_user};
use sphare_core_content::post::PostTags;
use sphare_core_content::post::ssr::{create_post, get_post_by_id};
use sphare_core_sphere::rule::BaseRule;
use sphare_core_sphere::rule::ssr::add_rule;
use sphare_core_user::notification::NotificationType;
use sphare_core_user::role::AdminRole;
use sphare_core_user::role::ssr::set_user_admin_role;
use sphare_core_user::user::User;
use crate::common::{create_test_user, create_user, get_db_pool};
use crate::data_factory::{add_base_rule, create_sphere_with_post, create_sphere_with_post_and_comment};
use crate::utils::get_notification;

mod common;
mod data_factory;
mod utils;

#[tokio::test]
async fn test_get_moderation_info() {
    let db_pool = get_db_pool().await;
    let mut user = create_test_user(&db_pool).await;
    let _basic_user = create_user("test", &db_pool).await;

    let (sphere, post, comment) = create_sphere_with_post_and_comment("a", &mut user, &db_pool).await;
    let rule = add_rule(&sphere.sphere_name, 0, "test", "test", false, &user, &db_pool).await.expect("Rule should be added.");

    let moderation_message = "test_moderate";
    moderate_post(post.post_id, rule.rule_id, moderation_message, &user, &db_pool).await.expect("Should moderate post");
    moderate_comment(comment.comment_id, rule.rule_id, moderation_message, &user, &db_pool).await.expect("Should moderate comment");

    let post_moderation_info = get_moderation_info(post.post_id, None, &db_pool).await.expect("Should get post moderation info");
    let comment_moderation_info = get_moderation_info(comment.post_id, Some(comment.comment_id), &db_pool).await.expect("Should get comment moderation info");

    let moderated_post = get_post_by_id(post.post_id, &db_pool).await.expect("Should get post by id");
    let moderated_comment = get_comment_by_id(comment.comment_id, &db_pool).await.expect("Should get comment by id");

    assert_eq!(post_moderation_info.rule, rule);
    assert_eq!(post_moderation_info.content, Content::Post(moderated_post));

    assert_eq!(comment_moderation_info.rule, rule);
    assert_eq!(comment_moderation_info.content, Content::Comment(moderated_comment));
}

#[tokio::test]
async fn test_moderate_post_and_ban_user() {
    let db_pool = get_db_pool().await;
    let mut user = create_user("test", &db_pool).await;
    let base_user = create_user("user", &db_pool).await;
    let current_timestamp = chrono::Utc::now();

    let (sphere, mod_post) = create_sphere_with_post("sphere", &mut user, &db_pool).await;
    let post_1 = create_post(&sphere.sphere_name, None, "1", "1", None, Link::default(), PostTags::default(), &base_user, &db_pool).await.expect("Should create post 1");
    let post_2 = create_post(&sphere.sphere_name, None, "2", "2", None, Link::default(), PostTags::default(), &base_user, &db_pool).await.expect("Should create post 2");
    let rule = add_rule(&sphere.sphere_name, 0, "test", "test", false, &user, &db_pool).await.expect("Rule should be added.");

    assert!(moderate_post_and_ban_user(post_1.post_id, rule.rule_id, "unauthorized", Some(1), &base_user, &db_pool).await.is_err());
    let (moderated_post_1, user_ban, notif) = moderate_post_and_ban_user(
        post_1.post_id,
        rule.rule_id,
        "test post 1",
        Some(0),
        &user,
        &db_pool
    ).await.expect("Moderate post without ban");

    let reloaded_post_1 = get_post_by_id(post_1.post_id, &db_pool).await.expect("Should get post by id");

    assert_eq!(moderated_post_1, reloaded_post_1);
    assert_eq!(moderated_post_1.infringed_rule_id, Some(rule.rule_id));
    assert_eq!(moderated_post_1.infringed_rule_title, Some(rule.title.clone()));
    assert_eq!(moderated_post_1.moderator_message.as_deref(), Some("test post 1"));
    assert!(moderated_post_1.edit_timestamp.is_some());

    // Check notification
    let notif = notif.expect("Notification should be generated");
    let loaded_notif = get_notification(notif.notification_id, &db_pool).await.expect("Should load notification");
    assert_eq!(notif, loaded_notif);
    assert_eq!(notif.user_id, base_user.user_id);
    assert_eq!(notif.post_id, post_1.post_id);
    assert_eq!(notif.comment_id, None);
    assert_eq!(notif.trigger_user_id, user.user_id);
    assert_eq!(notif.trigger_username, user.username);
    assert_eq!(notif.is_read, false);
    assert_eq!(notif.notification_type, NotificationType::Moderation);

    // Check user is not banned
    assert_eq!(user_ban, None);
    let base_user = User::get(base_user.user_id, &db_pool).await.expect("Should get user");
    base_user.check_can_publish_on_sphere(&sphere.sphere_name).expect("User should not be banned");

    let (moderated_post_2, user_ban, notif_2) = moderate_post_and_ban_user(
        post_2.post_id,
        rule.rule_id,
        "test post 2",
        Some(1),
        &user,
        &db_pool
    ).await.expect("Moderate post 2 with ban");

    let reloaded_post_2 = get_post_by_id(post_2.post_id, &db_pool).await.expect("Should get post 2 by id");

    assert_eq!(moderated_post_2, reloaded_post_2);
    assert_eq!(moderated_post_2.infringed_rule_id, Some(rule.rule_id));
    assert_eq!(moderated_post_2.infringed_rule_title, Some(rule.title.clone()));
    assert_eq!(moderated_post_2.moderator_message.as_deref(), Some("test post 2"));
    assert!(moderated_post_2.edit_timestamp.is_some());

    // Check notification
    let notif_2 = notif_2.expect("Notification should be generated");
    let loaded_notif_2 = get_notification(notif_2.notification_id, &db_pool).await.expect("Should load notification");
    assert_eq!(notif_2, loaded_notif_2);
    assert_eq!(notif_2.user_id, base_user.user_id);
    assert_eq!(notif_2.post_id, post_2.post_id);
    assert_eq!(notif_2.comment_id, None);
    assert_eq!(notif_2.trigger_user_id, user.user_id);
    assert_eq!(notif_2.trigger_username, user.username);
    assert_eq!(notif_2.is_read, false);
    assert_eq!(notif_2.notification_type, NotificationType::Moderation);

    // Check user is banned
    let user_ban = user_ban.expect("User should be banned");
    assert_eq!(user_ban.user_id, base_user.user_id);
    assert_eq!(user_ban.username, base_user.username);
    assert_eq!(user_ban.post_id, post_2.post_id);
    assert_eq!(user_ban.comment_id, None);
    assert_eq!(user_ban.infringed_rule_id, rule.rule_id);
    assert_eq!(user_ban.moderator_id, user.user_id);
    assert!(user_ban.until_timestamp.is_some_and(|until| until > current_timestamp + chrono::Duration::days(1)));

    let base_user = User::get(base_user.user_id, &db_pool).await.expect("Should get base user");
    base_user.check_can_publish_on_sphere(&sphere.sphere_name).expect_err("User should be banned");

    // Self-moderation should generate error
    moderate_post_and_ban_user(mod_post.post_id, rule.rule_id, "test self-moderation", Some(1), &user, &db_pool).await.expect_err("Self-moderation should generate error");
    // Post is still moderated
    let reloaded_mod_post = get_post_by_id(mod_post.post_id, &db_pool).await.expect("Should get mod post by id");
    assert_eq!(reloaded_mod_post.moderator_id, Some(user.user_id));
    assert_eq!(moderated_post_2.infringed_rule_id, Some(rule.rule_id));
    assert_eq!(moderated_post_2.infringed_rule_title, Some(rule.title));
    assert_eq!(reloaded_mod_post.moderator_message.as_deref(), Some("test self-moderation"));
    assert!(moderated_post_2.edit_timestamp.is_some());
    // Check user is not banned
    let user = User::get(user.user_id, &db_pool).await.expect("Should get user");
    user.check_can_publish_on_sphere(&sphere.sphere_name).expect("User should not be banned");
}

#[tokio::test]
async fn test_moderate_post() -> Result<(), AppError> {
    let db_pool = get_db_pool().await;
    let mut user = create_user("test", &db_pool).await;
    let mut global_moderator = create_user("mod", &db_pool).await;
    global_moderator.admin_role = AdminRole::Moderator;
    let unauthorized_user = create_user("user", &db_pool).await;

    let (sphere, post) = create_sphere_with_post("sphere", &mut user, &db_pool).await;
    let rule = add_rule(&sphere.sphere_name, 0, "test", "test", false, &user, &db_pool).await.expect("Rule should be added.");

    assert!(moderate_post(post.post_id, rule.rule_id, "unauthorized", &unauthorized_user, &db_pool).await.is_err());

    let moderated_post = moderate_post(post.post_id, rule.rule_id, "test", &user, &db_pool).await?;
    assert_eq!(moderated_post.moderator_id, Some(user.user_id));
    assert_eq!(moderated_post.moderator_name, Some(user.username));
    assert_eq!(moderated_post.moderator_message, Some(String::from("test")));
    assert_eq!(moderated_post.infringed_rule_id, Some(rule.rule_id));
    assert_eq!(moderated_post.infringed_rule_title, Some(rule.title.clone()));

    let remoderated_post = moderate_post(post.post_id, rule.rule_id, "global", &global_moderator, &db_pool).await?;
    assert_eq!(remoderated_post.moderator_id, Some(global_moderator.user_id));
    assert_eq!(remoderated_post.moderator_name, Some(global_moderator.username));
    assert_eq!(remoderated_post.moderator_message, Some(String::from("global")));
    assert_eq!(moderated_post.infringed_rule_id, Some(rule.rule_id));
    assert_eq!(moderated_post.infringed_rule_title, Some(rule.title));

    Ok(())
}

#[tokio::test]
async fn test_moderate_comment_and_ban_user() {
    let db_pool = get_db_pool().await;
    let mut user = create_user("test", &db_pool).await;
    let base_user = create_user("user", &db_pool).await;
    let current_timestamp = chrono::Utc::now();

    let (sphere, post, mod_comment) = create_sphere_with_post_and_comment("sphere", &mut user, &db_pool).await;
    let comment_1 = create_comment(post.post_id, None, "1", None, false, &base_user, &db_pool).await.expect("Should create comment 1");
    let comment_2 = create_comment(post.post_id, None, "2", None, false, &base_user, &db_pool).await.expect("Should create comment 2");
    let rule = add_rule(&sphere.sphere_name, 0, "test", "test", false, &user, &db_pool).await.expect("Rule should be added.");

    assert!(moderate_comment_and_ban_user(comment_1.comment_id, rule.rule_id, "unauthorized", Some(1), &base_user, &db_pool).await.is_err());
    let (moderated_comment_1, user_ban, notif) = moderate_comment_and_ban_user(
        comment_1.comment_id,
        rule.rule_id,
        "test comment 1",
        Some(0),
        &user,
        &db_pool
    ).await.expect("Moderate comment without ban");

    let reloaded_comment_1 = get_comment_by_id(comment_1.comment_id, &db_pool).await.expect("Should get comment by id");

    assert_eq!(moderated_comment_1, reloaded_comment_1);
    assert_eq!(moderated_comment_1.infringed_rule_id, Some(rule.rule_id));
    assert_eq!(moderated_comment_1.infringed_rule_title, Some(rule.title.clone()));
    assert_eq!(moderated_comment_1.moderator_message.as_deref(), Some("test comment 1"));
    assert!(moderated_comment_1.edit_timestamp.is_some());

    // Check notification
    let notif = notif.expect("Notification should be generated");
    let loaded_notif = get_notification(notif.notification_id, &db_pool).await.expect("Should load notification");
    assert_eq!(notif, loaded_notif);
    assert_eq!(notif.user_id, base_user.user_id);
    assert_eq!(notif.post_id, comment_1.post_id);
    assert_eq!(notif.comment_id, Some(comment_1.comment_id));
    assert_eq!(notif.trigger_user_id, user.user_id);
    assert_eq!(notif.trigger_username, user.username);
    assert_eq!(notif.is_read, false);
    assert_eq!(notif.notification_type, NotificationType::Moderation);

    // Check user is not banned
    assert_eq!(user_ban, None);
    let base_user = User::get(base_user.user_id, &db_pool).await.expect("Should get user");
    base_user.check_can_publish_on_sphere(&sphere.sphere_name).expect("User should not be banned");

    let (moderated_comment_2, user_ban, notif_2) = moderate_comment_and_ban_user(
        comment_2.comment_id,
        rule.rule_id,
        "test comment 2",
        Some(1),
        &user,
        &db_pool
    ).await.expect("Moderate comment 2 with ban");

    let reloaded_comment_2 = get_comment_by_id(comment_2.comment_id, &db_pool).await.expect("Should get comment 2 by id");

    assert_eq!(moderated_comment_2, reloaded_comment_2);
    assert_eq!(moderated_comment_2.infringed_rule_id, Some(rule.rule_id));
    assert_eq!(moderated_comment_2.infringed_rule_title, Some(rule.title.clone()));
    assert_eq!(moderated_comment_2.moderator_message.as_deref(), Some("test comment 2"));
    assert!(moderated_comment_2.edit_timestamp.is_some());

    // Check notification
    let notif_2 = notif_2.expect("Notification should be generated");
    let loaded_notif_2 = get_notification(notif_2.notification_id, &db_pool).await.expect("Should load notification");
    assert_eq!(notif_2, loaded_notif_2);
    assert_eq!(notif_2.user_id, base_user.user_id);
    assert_eq!(notif_2.post_id, comment_2.post_id);
    assert_eq!(notif_2.comment_id, Some(comment_2.comment_id));
    assert_eq!(notif_2.trigger_user_id, user.user_id);
    assert_eq!(notif_2.trigger_username, user.username);
    assert_eq!(notif_2.is_read, false);
    assert_eq!(notif_2.notification_type, NotificationType::Moderation);

    // Check user is banned
    let user_ban = user_ban.expect("User should be banned");
    assert_eq!(user_ban.user_id, base_user.user_id);
    assert_eq!(user_ban.username, base_user.username);
    assert_eq!(user_ban.post_id, comment_2.post_id);
    assert_eq!(user_ban.comment_id, Some(comment_2.comment_id));
    assert_eq!(user_ban.infringed_rule_id, rule.rule_id);
    assert_eq!(user_ban.moderator_id, user.user_id);
    assert!(user_ban.until_timestamp.is_some_and(|until| until > current_timestamp + chrono::Duration::days(1)));

    let base_user = User::get(base_user.user_id, &db_pool).await.expect("Should get base user");
    base_user.check_can_publish_on_sphere(&sphere.sphere_name).expect_err("User should be banned");

    // Self-moderation should generate error
    moderate_comment_and_ban_user(mod_comment.comment_id, rule.rule_id, "test self-moderation", Some(1), &user, &db_pool).await.expect_err("Self-moderation should generate error");
    // Post is still moderated
    let reloaded_mod_comment = get_comment_by_id(mod_comment.comment_id, &db_pool).await.expect("Should get mod comment by id");
    assert_eq!(reloaded_mod_comment.moderator_id, Some(user.user_id));
    assert_eq!(moderated_comment_2.infringed_rule_id, Some(rule.rule_id));
    assert_eq!(moderated_comment_2.infringed_rule_title, Some(rule.title));
    assert_eq!(reloaded_mod_comment.moderator_message.as_deref(), Some("test self-moderation"));
    assert!(moderated_comment_2.edit_timestamp.is_some());
    // Check user is not banned
    let user = User::get(user.user_id, &db_pool).await.expect("Should get user");
    user.check_can_publish_on_sphere(&sphere.sphere_name).expect("User should not be banned");
}

#[tokio::test]
async fn test_moderate_comment() -> Result<(), AppError> {
    let db_pool = get_db_pool().await;
    let mut user = create_user("test", &db_pool).await;
    let mut global_moderator = create_user("mod", &db_pool).await;
    global_moderator.admin_role = AdminRole::Moderator;
    let unauthorized_user = create_user("user", &db_pool).await;

    let (sphere, _post, comment) = create_sphere_with_post_and_comment("sphere", &mut user, &db_pool).await;
    let rule = add_rule(&sphere.sphere_name, 0, "test", "test", false, &user, &db_pool).await.expect("Rule should be added.");

    assert!(moderate_comment(comment.comment_id, rule.rule_id, "unauthorized", &unauthorized_user, &db_pool).await.is_err());

    let moderated_comment = moderate_comment(comment.comment_id, rule.rule_id, "test", &user, &db_pool).await?;
    assert_eq!(moderated_comment.moderator_id, Some(user.user_id));
    assert_eq!(moderated_comment.moderator_name, Some(user.username));
    assert_eq!(moderated_comment.moderator_message, Some(String::from("test")));
    assert_eq!(moderated_comment.infringed_rule_id, Some(rule.rule_id));
    assert_eq!(moderated_comment.infringed_rule_title, Some(rule.title.clone()));

    let remoderated_comment = moderate_comment(comment.comment_id, rule.rule_id, "global", &global_moderator, &db_pool).await?;
    assert_eq!(remoderated_comment.moderator_id, Some(global_moderator.user_id));
    assert_eq!(remoderated_comment.moderator_name, Some(global_moderator.username));
    assert_eq!(remoderated_comment.moderator_message, Some(String::from("global")));
    assert_eq!(remoderated_comment.infringed_rule_id, Some(rule.rule_id));
    assert_eq!(remoderated_comment.infringed_rule_title, Some(rule.title));

    Ok(())
}

#[tokio::test]
async fn test_ban_user_from_sphere() -> Result<(), AppError> {
    let db_pool = get_db_pool().await;
    let mut user = create_user("test", &db_pool).await;
    let mut global_moderator = create_user("mod", &db_pool).await;
    let mut admin = create_user("admin", &db_pool).await;
    // set user role in the DB, needed to test that global Moderators/Admin cannot be banned
    global_moderator.admin_role = AdminRole::Moderator;
    admin.admin_role = AdminRole::Admin;
    set_user_admin_role(global_moderator.user_id, AdminRole::Moderator, &admin, &db_pool).await?;
    set_user_admin_role(admin.user_id, AdminRole::Admin, &admin, &db_pool).await?;
    let unauthorized_user = create_user("user", &db_pool).await;
    let banned_user = create_user("banned", &db_pool).await;

    let (sphere, post) = create_sphere_with_post("sphere", &mut user, &db_pool).await;
    let rule = add_base_rule(0, BaseRule::BeRespectful.into(), "test", None, &admin, &db_pool).await.expect("Rule should be added.");

    // unauthorized used cannot ban
    assert!(ban_user_from_sphere(banned_user.user_id, sphere.sphere_id, post.post_id, None, rule.rule_id, None, &unauthorized_user, &db_pool).await.is_err());
    // ban with 0 days has no effect
    assert_eq!(ban_user_from_sphere(unauthorized_user.user_id, sphere.sphere_id, post.post_id, None, rule.rule_id, Some(0), &user, &db_pool).await?, None);
    let post = create_post(
        &sphere.sphere_name, None,"a", "b", None, Link::default(),PostTags::default(), &unauthorized_user, &db_pool
    ).await?;

    // cannot ban moderators
    assert!(ban_user_from_sphere(user.user_id, sphere.sphere_id, post.post_id, None, rule.rule_id, Some(1), &global_moderator, &db_pool).await.is_err());
    assert!(ban_user_from_sphere(global_moderator.user_id, sphere.sphere_id, post.post_id, None, rule.rule_id, Some(1), &user, &db_pool).await.is_err());
    assert!(ban_user_from_sphere(admin.user_id, sphere.sphere_id, post.post_id, None, rule.rule_id, Some(1), &user, &db_pool).await.is_err());
    assert!(ban_user_from_sphere(user.user_id, sphere.sphere_id, post.post_id, None, rule.rule_id, Some(1), &admin, &db_pool).await.is_err());

    // sphere moderator can ban ordinary users
    let user_ban = ban_user_from_sphere(
        unauthorized_user.user_id, sphere.sphere_id, post.post_id, None, rule.rule_id, Some(1), &user, &db_pool
    ).await?.expect("User ban from sphere should be possible.");
    assert_eq!(user_ban.user_id, unauthorized_user.user_id);
    assert_eq!(user_ban.sphere_id, Some(sphere.sphere_id));
    assert_eq!(user_ban.sphere_name, Some(sphere.sphere_name.clone()));
    assert_eq!(user_ban.moderator_id, user.user_id);
    assert_eq!(user_ban.until_timestamp, Some(user_ban.create_timestamp.add(Days::new(1))));

    // banned user cannot create new content
    let unauthorized_user = User::get(unauthorized_user.user_id, &db_pool).await.expect("Should be able to reload user.");
    assert!(
        matches!(
            create_post(
                &sphere.sphere_name, None,"c", "d", None, Link::default(), PostTags::default(), &unauthorized_user, &db_pool
            ).await,
            Err(AppError::SphereBanUntil(_)),
        )
    );
    assert!(
        matches!(
            create_comment(post.post_id, None, "a", None, false, &unauthorized_user, &db_pool).await,
            Err(AppError::SphereBanUntil(_)),
        )
    );

    // global moderator can ban ordinary users
    let user_ban = ban_user_from_sphere(banned_user.user_id, sphere.sphere_id, post.post_id, None, rule.rule_id, Some(2), &global_moderator, &db_pool).await?.expect("User ban from sphere should be possible.");
    assert_eq!(user_ban.user_id, banned_user.user_id);
    assert_eq!(user_ban.sphere_id, Some(sphere.sphere_id));
    assert_eq!(user_ban.sphere_name, Some(sphere.sphere_name.clone()));
    assert_eq!(user_ban.moderator_id, global_moderator.user_id);
    assert_eq!(user_ban.until_timestamp, Some(user_ban.create_timestamp.add(Days::new(2))));

    // global moderator can ban ordinary users
    let user_ban = ban_user_from_sphere(banned_user.user_id, sphere.sphere_id, post.post_id, None, rule.rule_id, None, &admin, &db_pool).await?.expect("User ban from sphere should be possible.");
    assert_eq!(user_ban.user_id, banned_user.user_id);
    assert_eq!(user_ban.sphere_id, Some(sphere.sphere_id));
    assert_eq!(user_ban.sphere_name, Some(sphere.sphere_name.clone()));
    assert_eq!(user_ban.moderator_id, admin.user_id);
    assert_eq!(user_ban.until_timestamp, None);

    // banned user cannot create new content
    let banned_user = User::get(banned_user.user_id, &db_pool).await.expect("Should be possible to reload banned user.");
    assert_eq!(
        create_post(&sphere.sphere_name, None,"c", "d", None, Link::default(), PostTags::default(), &banned_user, &db_pool).await,
        Err(AppError::PermanentSphereBan),
    );
    assert_eq!(
        create_comment(post.post_id, None, "a", None, false, &banned_user, &db_pool).await,
        Err(AppError::PermanentSphereBan),
    );

    Ok(())
}