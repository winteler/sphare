use sphare_core_content::comment::ssr::create_comment;
use sphare_core_user::notification::ssr::{create_notification, delete_stale_notifications, get_notifications, set_all_notifications_read, set_notification_read};
use sphare_core_user::notification::{NotificationType, NOTIF_RETENTION_DAYS};

use crate::common::*;
use crate::data_factory::*;
use crate::utils::{get_notification, update_notification_timestamp};

mod common;
mod data_factory;
mod utils;

#[tokio::test]
async fn test_create_notification() {
    let db_pool = get_db_pool().await;
    let mut user_1 = create_test_user(&db_pool).await;
    let user_2 = create_user("trigger", &db_pool).await;

    let (sphere, post) = create_sphere_with_post("sphere", &mut user_1, &db_pool).await;

    let comment = create_comment(
        post.post_id,
        None,
        "a",
        None,
        false,
        &user_2,
        &db_pool
    ).await.expect("Should create comment");
    let nested_comment = create_comment(
        post.post_id,
        Some(comment.comment_id),
        "b",
        None,
        false,
        &user_1,
        &db_pool
    ).await.expect("Should create nested comment");

    let post_comment_notif = create_notification(
        comment.post_id,
        comment.parent_id,
        Some(comment.comment_id),
        comment.creator_id,
        NotificationType::PostReply,
        &db_pool
    )
        .await
        .expect("Should create post comment notification")
        .expect("Should have notification");

    assert_eq!(post_comment_notif.sphere_id, sphere.sphere_id);
    assert_eq!(post_comment_notif.sphere_header, (&sphere).into());
    assert_eq!(post_comment_notif.post_id, post.post_id);
    assert_eq!(post_comment_notif.comment_id, Some(comment.comment_id));
    assert_eq!(post_comment_notif.user_id, user_1.user_id);
    assert_eq!(post_comment_notif.trigger_user_id, user_2.user_id);
    assert_eq!(post_comment_notif.trigger_username, user_2.username);
    assert_eq!(post_comment_notif.notification_type, NotificationType::PostReply);
    assert_eq!(post_comment_notif.is_read, false);

    let nested_comment_notif = create_notification(
        nested_comment.post_id,
        nested_comment.parent_id,
        Some(nested_comment.comment_id),
        nested_comment.creator_id,
        NotificationType::CommentReply,
        &db_pool
    )
        .await
        .expect("Should create nested comment notification")
        .expect("Should have notification");

    assert_eq!(nested_comment_notif.sphere_id, sphere.sphere_id);
    assert_eq!(nested_comment_notif.sphere_header, (&sphere).into());
    assert_eq!(nested_comment_notif.post_id, nested_comment.post_id);
    assert_eq!(nested_comment_notif.comment_id, Some(nested_comment.comment_id));
    assert_eq!(nested_comment_notif.user_id, user_2.user_id);
    assert_eq!(nested_comment_notif.trigger_user_id, user_1.user_id);
    assert_eq!(nested_comment_notif.trigger_username, user_1.username);
    assert_eq!(nested_comment_notif.notification_type, NotificationType::CommentReply);
    assert_eq!(nested_comment_notif.is_read, false);

    let moderate_comment_notif = create_notification(
        comment.post_id,
        Some(comment.comment_id),
        Some(comment.comment_id),
        user_1.user_id,
        NotificationType::Moderation,
        &db_pool
    )
        .await
        .expect("Should create post comment notification")
        .expect("Should have notification");

    assert_eq!(moderate_comment_notif.sphere_id, sphere.sphere_id);
    assert_eq!(moderate_comment_notif.sphere_header, (&sphere).into());
    assert_eq!(moderate_comment_notif.post_id, comment.post_id);
    assert_eq!(moderate_comment_notif.comment_id, Some(comment.comment_id));
    assert_eq!(moderate_comment_notif.user_id, user_2.user_id);
    assert_eq!(moderate_comment_notif.trigger_user_id, user_1.user_id);
    assert_eq!(moderate_comment_notif.trigger_username, user_1.username);
    assert_eq!(moderate_comment_notif.notification_type, NotificationType::Moderation);
    assert_eq!(moderate_comment_notif.is_read, false);

    // Returns None when replying to/moderating self and no notification created
    let self_comment = create_comment(
        post.post_id,
        None,
        "self",
        None,
        false,
        &user_1,
        &db_pool
    ).await.expect("Should create self comment");

    let self_comment_notif = create_notification(
        self_comment.post_id,
        self_comment.parent_id,
        Some(self_comment.comment_id),
        self_comment.creator_id,
        NotificationType::CommentReply,
        &db_pool
    )
        .await
        .expect("Should not create notification and return Ok(None)");

    assert_eq!(self_comment_notif, None);

    let moderate_self_post_notif = create_notification(
        post.post_id,
        None,
        None,
        post.creator_id,
        NotificationType::Moderation,
        &db_pool
    )
        .await
        .expect("Should not create post moderate notification and return Ok(None)");

    assert_eq!(moderate_self_post_notif, None);

    let moderate_self_comment_notif = create_notification(
        nested_comment.post_id,
        Some(nested_comment.comment_id),
        Some(nested_comment.comment_id),
        nested_comment.creator_id,
        NotificationType::Moderation,
        &db_pool
    )
        .await
        .expect("Should not create comment moderate notification and return Ok(None)");

    assert_eq!(moderate_self_comment_notif, None);

    // Check still only 1 notification for user 1, 2 for user 2
    let user_1_notif_vec = get_notifications(user_1.user_id, &db_pool).await.expect("Should get user 1 notification vec");
    let user_2_notif_vec = get_notifications(user_2.user_id, &db_pool).await.expect("Should get user 2 notification vec");
    assert_eq!(user_1_notif_vec.len(), 1);
    assert_eq!(user_2_notif_vec.len(), 2);
}

#[tokio::test]
async fn test_get_notifications() {
    let db_pool = get_db_pool().await;
    let mut user = create_test_user(&db_pool).await;
    let trigger_user = create_user("trigger", &db_pool).await;

    let (_, post, comment) = create_sphere_with_post_and_comment("sphere", &mut user, &db_pool).await;

    let post_comment_notif = create_notification(
        post.post_id,
        None,
        None,
        trigger_user.user_id,
        NotificationType::Moderation,
        &db_pool
    )
        .await
        .expect("Should create post comment notification")
        .expect("Should have notification");

    let comment_comment_notif = create_notification(
        comment.post_id,
        comment.parent_id,
        Some(comment.comment_id),
        trigger_user.user_id,
        NotificationType::CommentReply,
        &db_pool
    )
        .await
        .expect("Should create comment comment notification")
        .expect("Should have notification");

    let expected_notif_vec = vec![comment_comment_notif, post_comment_notif];

    let notif_vec = get_notifications(user.user_id, &db_pool).await.expect("Should get notification vec");
    assert_eq!(notif_vec, expected_notif_vec);
}

#[tokio::test]
async fn test_set_notification_read() {
    let db_pool = get_db_pool().await;
    let mut user = create_test_user(&db_pool).await;
    let trigger_user = create_user("trigger", &db_pool).await;

    let (_, post, comment) = create_sphere_with_post_and_comment("sphere", &mut user, &db_pool).await;

    let mut notification = create_notification(
        post.post_id,
        comment.parent_id,
        Some(comment.comment_id),
        trigger_user.user_id,
        NotificationType::PostReply,
        &db_pool
    ).await.expect("Should create post comment notification").expect("Should have notification");

    set_notification_read(notification.notification_id, user.user_id, &db_pool).await.expect("Should read notification");

    notification.is_read = true;

    let read_notif = get_notification(notification.notification_id, &db_pool).await.expect("Should get notification");
    assert_eq!(read_notif, notification);
}

#[tokio::test]
async fn test_set_all_notifications_read() {
    let db_pool = get_db_pool().await;
    let mut user = create_test_user(&db_pool).await;
    let trigger_user = create_user("trigger", &db_pool).await;

    let (_, post, comment) = create_sphere_with_post_and_comment("sphere", &mut user, &db_pool).await;

    let mut post_comment_notif = create_notification(
        post.post_id,
        None,
        None,
        trigger_user.user_id,
        NotificationType::Moderation,
        &db_pool
    )
        .await
        .expect("Should create post comment notification")
        .expect("Should have notification");

    let mut comment_comment_notif = create_notification(
        comment.post_id,
        comment.parent_id,
        Some(comment.comment_id),
        trigger_user.user_id,
        NotificationType::PostReply,
        &db_pool
    )
        .await
        .expect("Should create comment comment notification")
        .expect("Should have notification");

    set_all_notifications_read(user.user_id, &db_pool).await.expect("Should read all notification");

    post_comment_notif.is_read = true;
    comment_comment_notif.is_read = true;

    let expected_notif_vec = vec![comment_comment_notif, post_comment_notif];

    let notif_vec = get_notifications(user.user_id, &db_pool).await.expect("Should get notification vec");
    assert_eq!(notif_vec, expected_notif_vec);
}

#[tokio::test]
async fn test_delete_stale_notifications() {
    let db_pool = get_db_pool().await;
    let mut user = create_test_user(&db_pool).await;
    let trigger_user = create_user("trigger", &db_pool).await;

    let (_, post, comment) = create_sphere_with_post_and_comment("sphere", &mut user, &db_pool).await;

    let notif_1 = create_notification(
        post.post_id,
        None,
        None,
        trigger_user.user_id,
        NotificationType::Moderation,
        &db_pool
    )
        .await
        .expect("Should create post comment notification")
        .expect("Should have notification");

    let notif_2 = create_notification(
        comment.post_id,
        comment.parent_id,
        Some(comment.comment_id),
        trigger_user.user_id,
        NotificationType::PostReply,
        &db_pool
    )
        .await
        .expect("Should create comment comment notification")
        .expect("Should have notification");

    update_notification_timestamp(notif_2.notification_id, (NOTIF_RETENTION_DAYS + 1) as f64, &db_pool).await.expect("Should update notification timestamp");

    delete_stale_notifications(&db_pool).await.expect("Should delete stale notifications");

    let notif_vec = get_notifications(user.user_id, &db_pool).await.expect("Should get notification vec");
    assert_eq!(notif_vec.contains(&notif_1), true);
    assert_eq!(notif_vec.contains(&notif_2), false);
}