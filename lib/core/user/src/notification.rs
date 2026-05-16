use std::collections::{BTreeMap, HashSet};

use chrono::{DateTime, Utc};
use leptos::prelude::*;
use leptos_fluent::{move_tr, tr};
use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumString, IntoStaticStr};

use sphare_core_common::common::SphereHeader;
use sphare_core_common::errors::AppError;
use sphare_core_common::routes::{get_comment_path, get_post_path};

pub const NOTIF_STATE_STORAGE: &str = "notification_state";
pub const NOTIF_TAG: &str = "sphare-notif";
pub const NOTIF_RETENTION_DAYS: i64 = 31;
pub const NOTIF_RELOAD_INTERVAL_MS: u64 = 900000;

#[repr(i16)]
#[derive(Clone, Copy, Debug, Default, Display, EnumString, Eq, IntoStaticStr, Hash, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
#[cfg_attr(feature = "ssr", derive(sqlx::Type))]
pub enum NotificationType {
    #[default]
    PostReply = 0,
    CommentReply = 1,
    Moderation = 2,
}

#[cfg_attr(feature = "ssr", derive(sqlx::FromRow))]
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Notification {
    pub notification_id: i64,
    pub sphere_id: i64,
    #[cfg_attr(feature = "ssr", sqlx(flatten))]
    pub sphere_header: SphereHeader,
    pub satellite_id: Option<i64>,
    pub post_id: i64,
    pub comment_id: Option<i64>,
    pub user_id: i64,
    pub trigger_user_id: i64,
    pub trigger_username: String,
    pub notification_type: NotificationType,
    pub is_read: bool,
    pub create_timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct NotifHandler {
    emitted_notif_id_set: HashSet<i64>,
    timestamp_2_notif_id: BTreeMap<chrono::DateTime<chrono::Utc>, i64>,
}

impl NotifHandler {
    fn identify_new_notifications(
        &mut self,
        notif_vec: Vec<Notification>,
        unread_notif_id_set: RwSignal<usize>,
    ) -> Vec<Notification> {
        let unread_notif_vec: Vec<Notification> = notif_vec
            .into_iter()
            .filter(|notif| !notif.is_read)
            .collect();
        *unread_notif_id_set.write() = unread_notif_vec.len();

        let mut new_notif_vec = Vec::new();
        for notif in unread_notif_vec.into_iter() {
            if self.emitted_notif_id_set.insert(notif.notification_id) {
                self.timestamp_2_notif_id.insert(notif.create_timestamp, notif.notification_id);
                new_notif_vec.push(notif);
            }
        }
        new_notif_vec
    }

    fn clear_stale_notifications(&mut self, threshold_datetime: DateTime<Utc>) {
        let notif_to_keep = self.timestamp_2_notif_id.split_off(&threshold_datetime);

        for (_, value) in &self.timestamp_2_notif_id {
            self.emitted_notif_id_set.remove(value);
        }
        self.timestamp_2_notif_id = notif_to_keep;
    }

    fn send_notifications_to_browser(
        &self,
        new_notif_vec: Vec<Notification>,
        unread_notif_count: RwSignal<usize>,
        build_and_send_notif_fn: impl Fn(String) + Clone + Send + Sync,
    ) {
        if let Some(notif) = new_notif_vec.first() {
            let new_notif_count = new_notif_vec.len();
            let unread_notif_count = unread_notif_count.get_untracked();
            let body = match (new_notif_count, unread_notif_count) {
                (1, 1) => get_web_notif_text(notif),
                (1, _) => get_web_notif_text(notif) + tr!("web-notif-unread-addon", {"unread_notif_count" => unread_notif_count}).as_str(),
                (new_notif_count, unread_notif_count) if new_notif_count == unread_notif_count => {
                    tr!("multi-web-notif", {"new_notif_count" => new_notif_count})
                },
                (new_notif_count, unread_notif_count) => tr!(
                    "multi-web-notif-with-unread", {"new_notif_count" => new_notif_count, "unread_notif_count" => unread_notif_count}
                ),
            };
            build_and_send_notif_fn(body);
        }
    }

    pub fn handle_notifications(
        &mut self,
        notif_vec: Vec<Notification>,
        unread_notif_count: RwSignal<usize>,
        build_and_send_notif_fn: impl Fn(String) + Clone + Send + Sync,
    ) {
        let notif_timestamp_threshold = Utc::now() - chrono::Duration::days(NOTIF_RETENTION_DAYS);

        let new_notif_vec = self.identify_new_notifications(notif_vec, unread_notif_count);
        self.clear_stale_notifications(notif_timestamp_threshold);
        self.send_notifications_to_browser(new_notif_vec, unread_notif_count, build_and_send_notif_fn);
    }
}

#[cfg(feature = "ssr")]
pub mod ssr {
    use sqlx::PgPool;

    use sphare_core_common::errors::AppError;

    use crate::notification::{Notification, NotificationType, NOTIF_RETENTION_DAYS};

    pub async fn create_notification(
        post_id: i64,
        notif_comment_id: Option<i64>,
        link_comment_id: Option<i64>,
        trigger_user_id: i64,
        notification_type: NotificationType,
        db_pool: &PgPool,
    ) -> Result<Option<Notification>, AppError> {
        let notification = sqlx::query_as::<_, Notification>(
            "WITH trigger_user AS (
                SELECT username FROM users WHERE user_id = $4
            ), post_info AS (
                SELECT sphere_id, satellite_id, creator_id FROM posts WHERE post_id = $1
            ), notified_user AS (
                SELECT
                    CASE
                        WHEN $2 IS NULL THEN
                            (SELECT creator_id FROM post_info)
                        ELSE
                            (SELECT creator_id FROM comments WHERE comment_id = $2)
                    END AS creator_id
            ), new_notification AS (
                INSERT INTO notifications (sphere_id, satellite_id, post_id, comment_id, user_id, trigger_user_id, notification_type)
                SELECT
                    p.sphere_id,
                    p.satellite_id,
                    $1, $3,
                    nu.creator_id,
                    $4, $5
                FROM post_info p, trigger_user tu, notified_user nu
                WHERE $4 != nu.creator_id
                RETURNING *
            )
            SELECT n.*, u.username AS trigger_username, s.sphere_name, s.icon_url, s.is_nsfw
            FROM new_notification n, trigger_user u, spheres s
            WHERE s.sphere_id = n.sphere_id",
        )
            .bind(post_id)
            .bind(notif_comment_id)
            .bind(link_comment_id)
            .bind(trigger_user_id)
            .bind(notification_type as i16)
            .fetch_optional(db_pool)
            .await?;

        Ok(notification)
    }

    pub async fn get_notifications(
        user_id: i64,
        db_pool: &PgPool,
    ) -> Result<Vec<Notification>, AppError> {
        let notification_vec = sqlx::query_as::<_, Notification>(
            "SELECT n.*, u.username AS trigger_username, s.sphere_name, s.icon_url, s.is_nsfw
            FROM notifications n
            JOIN USERS u ON u.user_id = n.trigger_user_id
            JOIN spheres s ON s.sphere_id = n.sphere_id
            WHERE n.user_id = $1
            ORDER BY n.create_timestamp DESC",
        )
            .bind(user_id)
            .fetch_all(db_pool)
            .await?;

        Ok(notification_vec)
    }

    pub async fn set_notification_read(
        notification_id: i64,
        user_id: i64,
        db_pool: &PgPool,
    ) -> Result<(), AppError> {
        sqlx::query!(
            "UPDATE notifications SET is_read = TRUE
            WHERE notification_id = $1 and user_id = $2",
            notification_id,
            user_id,
        )
            .execute(db_pool)
            .await?;

        Ok(())
    }

    pub async fn set_all_notifications_read(
        user_id: i64,
        db_pool: &PgPool,
    ) -> Result<(), AppError> {
        sqlx::query!(
            "UPDATE notifications SET is_read = TRUE
            WHERE user_id = $1",
            user_id,
        )
            .execute(db_pool)
            .await?;

        Ok(())
    }

    pub async fn delete_stale_notifications(
        db_pool: &PgPool,
    ) -> Result<(), AppError> {
        sqlx::query!(
            "DELETE FROM notifications
            WHERE create_timestamp < NOW() - (INTERVAL '1 day' * $1)",
            NOTIF_RETENTION_DAYS as f64,
        )
            .execute(db_pool)
            .await?;

        Ok(())
    }
}

pub fn on_read_notif(
    is_notif_read: ArcRwSignal<bool>,
    unread_notif_count: RwSignal<usize>,
    read_notif_action: Action<(), Result<(), AppError>>,
) {
    let mut unread_notif_count = unread_notif_count.write();
    if !is_notif_read.get() && *unread_notif_count > 0 {
        *unread_notif_count -= 1;
    }
    is_notif_read.set(true);
    read_notif_action.dispatch(());
}

pub fn get_notification_path(notification: &Notification) -> String {
    match notification.comment_id {
        Some(comment_id) => get_comment_path(
            &notification.sphere_header.sphere_name,
            notification.satellite_id,
            notification.post_id,
            comment_id,
        ),
        None => get_post_path(
            &notification.sphere_header.sphere_name,
            notification.satellite_id,
            notification.post_id,
        ),
    }
}

pub fn get_notification_text(notification: &Notification) -> Signal<String> {
    match (notification.notification_type, notification.comment_id) {
        (NotificationType::PostReply, _) => move_tr!("notification-post-reply"),
        (NotificationType::CommentReply, _) => move_tr!("notification-comment-reply"),
        (NotificationType::Moderation, Some(_)) => move_tr!("notification-moderate-comment"),
        (NotificationType::Moderation, None) => move_tr!("notification-moderate-post"),
    }
}

pub fn get_web_notif_text(notification: &Notification) -> String {
    let username = notification.trigger_username.clone();
    let sphere_name = notification.sphere_header.sphere_name.clone();
    match (notification.notification_type, notification.comment_id) {
        (NotificationType::PostReply, _) => tr!(
            "web-notif-post-reply", {"username" => username, "sphere_name" => sphere_name}
        ),
        (NotificationType::CommentReply, _) => tr!(
            "web-notif-comment-reply", {"username" => username, "sphere_name" => sphere_name}
        ),
        (NotificationType::Moderation, Some(_)) => tr!(
            "web-notif-moderate-comment", {"username" => username, "sphere_name" => sphere_name}
        ),
        (NotificationType::Moderation, None) => tr!(
            "web-notif-moderate-post", {"username" => username, "sphere_name" => sphere_name}
        ),
    }
}

#[cfg(test)]
mod tests {
    use std::sync::LazyLock;

    use fluent_templates::{static_loader, StaticLoader};
    use leptos::prelude::*;
    use leptos_fluent::{tr, I18n, Language};

    use sphare_core_common::common::SphereHeader;
    use sphare_core_common::routes::{get_comment_path, get_post_path};

    use crate::notification::{get_notification_path, get_notification_text, get_web_notif_text, NotifHandler, Notification, NotificationType, NOTIF_RETENTION_DAYS};


    const EN_LANG: Language = Language {
        id: "en",
        name: "English",
        dir: &leptos_fluent::WritingDirection::Ltr,
        flag: None,
        script: None,
    };
    const FR_LANG: Language = Language {
        id: "fr",
        name: "Français",
        dir: &leptos_fluent::WritingDirection::Ltr,
        flag: None,
        script: None,
    };
    const LANGUAGES: &'static [&Language] = &[
        &EN_LANG,
        &FR_LANG,
    ];

    fn get_i18n() -> I18n {
        static_loader! {
            static TRANSLATIONS = {
                locales: "../../../locales",
                fallback_language: "en",
            };
        }
        let compound: Vec<&LazyLock<StaticLoader>> = vec![&TRANSLATIONS];
        I18n::new(
            RwSignal::new(&LANGUAGES[0]),
            LANGUAGES,
            Signal::derive(move || compound.clone())
        )
    }

    #[test]
    fn test_notif_handler_identify_new_notifications() {
        let owner = Owner::new();
        owner.set();
        let mut notif_handler = NotifHandler {
            emitted_notif_id_set: [2].into(),
            ..Default::default()
        };

        let timestamp_1 = chrono::Utc::now();
        let timestamp_2 = timestamp_1 - chrono::Duration::days(1);

        let unread_notif_count = RwSignal::new(0);
        let new_notif = Notification {
            notification_id: 3,
            create_timestamp: timestamp_2,
            ..Default::default()
        };

        let notif_vec = vec![
            Notification {
                notification_id: 1,
                is_read: true,
                ..Default::default()
            },
            Notification {
                notification_id: 2,
                create_timestamp: timestamp_1,
                ..Default::default()
            },
            new_notif.clone(),
        ];

        let new_notif_vec = notif_handler.identify_new_notifications(notif_vec, unread_notif_count);
        assert_eq!(new_notif_vec.len(), 1);
        assert_eq!(*new_notif_vec.first().unwrap(), new_notif);

        assert_eq!(unread_notif_count.get_untracked(), 2);

        assert_eq!(notif_handler.emitted_notif_id_set, [2, 3].into());
        assert_eq!(notif_handler.timestamp_2_notif_id, [(timestamp_2, 3)].into());
    }

    #[test]
    fn test_notif_handler_clear_stale_notifications() {
        let current_timestamp = chrono::Utc::now();
        let threshold_timestamp = current_timestamp - chrono::Duration::days(NOTIF_RETENTION_DAYS);
        let stale_timestamp = current_timestamp - chrono::Duration::days(NOTIF_RETENTION_DAYS + 1);

        let mut notif_handler = NotifHandler {
            emitted_notif_id_set: [1, 2, 3].into(),
            timestamp_2_notif_id: [(stale_timestamp, 1), (threshold_timestamp, 2), (current_timestamp, 3)].into(),
        };

        notif_handler.clear_stale_notifications(threshold_timestamp);
        assert_eq!(notif_handler.emitted_notif_id_set, [2, 3].into());
        assert_eq!(notif_handler.timestamp_2_notif_id, [(threshold_timestamp, 2), (current_timestamp, 3)].into());
    }

    #[test]
    fn test_notif_handler_send_notifications_to_browser() {
        let owner = Owner::new();
        owner.set();

        provide_context(get_i18n());

        let notif_handler = NotifHandler::default();

        let notif_1 = Notification {
            notification_id: 1,
            ..Default::default()
        };
        let notif_2 = Notification {
            notification_id: 2,
            ..Default::default()
        };

        let mut notif_vec = vec![
            notif_1.clone(),
        ];
        let unread_notif_count = RwSignal::new(1);

        let expected_body = get_web_notif_text(&notif_1);
        let mock_show_fn = move |body: String| assert_eq!(body, expected_body);
        notif_handler.send_notifications_to_browser(notif_vec.clone(), unread_notif_count, mock_show_fn);

        unread_notif_count.set(2);
        let expected_body =
            get_web_notif_text(&notif_1) +
                tr!(
                    "web-notif-unread-addon",
                    {"unread_notif_count" => unread_notif_count.get_untracked()}
                ).as_str();
        let mock_show_fn = move |body: String| assert_eq!(body, expected_body);
        notif_handler.send_notifications_to_browser(notif_vec.clone(), unread_notif_count, mock_show_fn);

        notif_vec.push(notif_2);
        let expected_body = tr!("multi-web-notif", {"new_notif_count" => notif_vec.len()});
        let mock_show_fn = move |body: String| assert_eq!(body, expected_body);
        notif_handler.send_notifications_to_browser(notif_vec.clone(), unread_notif_count, mock_show_fn);

        unread_notif_count.set(3);
        let expected_body = tr!(
            "multi-web-notif-with-unread",
            {"new_notif_count" => notif_vec.len(), "unread_notif_count" => unread_notif_count.get()}
        );
        let mock_show_fn = move |body: String| assert_eq!(body, expected_body);
        notif_handler.send_notifications_to_browser(notif_vec, unread_notif_count, mock_show_fn);
    }

    #[test]
    fn test_notif_handler_handle_notifications() {
        let owner = Owner::new();
        owner.set();

        provide_context(get_i18n());

        let current_timestamp = chrono::Utc::now();
        let threshold_timestamp = current_timestamp - chrono::Duration::days(NOTIF_RETENTION_DAYS);
        let stale_timestamp = current_timestamp - chrono::Duration::days(NOTIF_RETENTION_DAYS + 1);

        let mut notif_handler = NotifHandler {
            emitted_notif_id_set: [1, 2, 3, 4].into(),
            timestamp_2_notif_id: [
                (stale_timestamp, 1),
                (threshold_timestamp, 2),
                (threshold_timestamp, 3),
                (current_timestamp, 4),
            ].into(),
        };

        let notif_1 = Notification {
            notification_id: 2,
            is_read: true,
            create_timestamp: threshold_timestamp,
            ..Default::default()
        };
        let notif_2 = Notification {
            notification_id: 3,
            is_read: true,
            create_timestamp: threshold_timestamp,
            ..Default::default()
        };
        let notif_3 = Notification {
            notification_id: 4,
            create_timestamp: current_timestamp,
            ..Default::default()
        };
        let notif_4 = Notification {
            notification_id: 5,
            create_timestamp: current_timestamp,
            ..Default::default()
        };
        let notif_5 = Notification {
            notification_id: 6,
            create_timestamp: current_timestamp,
            ..Default::default()
        };

        let notif_vec = vec![
            notif_1,
            notif_2,
            notif_3,
            notif_4,
            notif_5,
        ];
        let unread_notif_count = RwSignal::new(0);

        let expected_body = tr!(
            "multi-web-notif-with-unread",
            {"new_notif_count" => 2, "unread_notif_count" => 3}
        );
        let mock_show_fn = move |body: String| assert_eq!(body, expected_body);
        notif_handler.handle_notifications(notif_vec, unread_notif_count, mock_show_fn);
        assert_eq!(unread_notif_count.get_untracked(), 3);
    }

    #[test]
    fn test_get_notification_path() {
        let post_notif = Notification {
            post_id: 1,
            comment_id: None,
            sphere_header: SphereHeader::new(String::from("a"), None, false),
            satellite_id: Some(1),
            ..Default::default()
        };
        assert_eq!(
            get_notification_path(&post_notif),
            get_post_path(
                &post_notif.sphere_header.sphere_name,
                post_notif.satellite_id,
                post_notif.post_id
            )
        );

        let comment_notif = Notification {
            post_id: 2,
            comment_id: Some(1),
            sphere_header: SphereHeader::new(String::from("b"), None, false),
            ..Default::default()
        };
        assert_eq!(
            get_notification_path(&comment_notif),
            get_comment_path(
                &comment_notif.sphere_header.sphere_name,
                comment_notif.satellite_id,
                comment_notif.post_id,
                comment_notif.comment_id.expect("Should have comment_id")
            )
        )
    }

    #[test]
    fn test_get_notification_text() {
        let owner = Owner::new();
        owner.set();

        provide_context(get_i18n());

        let notif_post_reply = Notification {
            notification_type: NotificationType::PostReply,
            ..Default::default()
        };
        let notif_text = get_notification_text(&notif_post_reply);
        assert_eq!(
            *notif_text.read(),
            tr!("notification-post-reply"),
        );

        let notif_comment_reply = Notification {
            notification_type: NotificationType::CommentReply,
            ..Default::default()
        };
        let notif_text = get_notification_text(&notif_comment_reply);
        assert_eq!(
            *notif_text.read(),
            tr!("notification-comment-reply"),
        );

        let notif_post_moderation = Notification {
            notification_type: NotificationType::Moderation,
            comment_id: None,
            ..Default::default()
        };
        let notif_text = get_notification_text(&notif_post_moderation);
        assert_eq!(
            *notif_text.read(),
            tr!("notification-moderate-post"),
        );

        let notif_comment_moderation = Notification {
            notification_type: NotificationType::Moderation,
            comment_id: Some(1),
            ..Default::default()
        };
        let notif_text = get_notification_text(&notif_comment_moderation);
        assert_eq!(
            *notif_text.read(),
            tr!("notification-moderate-comment"),
        );
    }

    #[test]
    fn test_get_web_notif_text() {
        let owner = Owner::new();
        owner.set();

        provide_context(get_i18n());

        let notif_post_reply = Notification {
            notification_type: NotificationType::PostReply,
            trigger_username: String::from("a"),
            sphere_header: SphereHeader::new(String::from("i"), None, false),
            ..Default::default()
        };
        let notif_text = get_web_notif_text(&notif_post_reply);
        assert_eq!(
            notif_text,
            tr!(
                "web-notif-post-reply",
                {
                    "username" => notif_post_reply.trigger_username,
                    "sphere_name" => notif_post_reply.sphere_header.sphere_name
                }
            ),
        );

        let notif_comment_reply = Notification {
            notification_type: NotificationType::CommentReply,
            trigger_username: String::from("b"),
            sphere_header: SphereHeader::new(String::from("j"), None, false),
            ..Default::default()
        };
        let notif_text = get_web_notif_text(&notif_comment_reply);
        assert_eq!(
            notif_text,
            tr!(
                "web-notif-comment-reply",
                {
                    "username" => notif_comment_reply.trigger_username,
                    "sphere_name" => notif_comment_reply.sphere_header.sphere_name
                }
            ),
        );

        let notif_post_moderation = Notification {
            notification_type: NotificationType::Moderation,
            comment_id: None,
            trigger_username: String::from("c"),
            sphere_header: SphereHeader::new(String::from("k"), None, false),
            ..Default::default()
        };
        let notif_text = get_web_notif_text(&notif_post_moderation);
        assert_eq!(
            notif_text,
            tr!(
                "web-notif-moderate-post",
                {
                    "username" => notif_post_moderation.trigger_username,
                    "sphere_name" => notif_post_moderation.sphere_header.sphere_name
                }
            ),
        );

        let notif_comment_moderation = Notification {
            notification_type: NotificationType::Moderation,
            comment_id: Some(1),
            trigger_username: String::from("d"),
            sphere_header: SphereHeader::new(String::from("l"), None, false),
            ..Default::default()
        };
        let notif_text = get_web_notif_text(&notif_comment_moderation);
        assert_eq!(
            notif_text,
            tr!(
                "web-notif-moderate-comment",
                {
                    "username" => notif_comment_moderation.trigger_username,
                    "sphere_name" => notif_comment_moderation.sphere_header.sphere_name
                }
            ),
        );
    }
}