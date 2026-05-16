use std::collections::HashMap;

use codee::string::JsonSerdeCodec;
use leptos::prelude::*;
use leptos_fluent::{move_tr, tr};
use leptos_use::{breakpoints_tailwind, storage::use_local_storage, use_breakpoints, use_interval_fn, use_timeout_fn, BreakpointsTailwind};
use leptos_use::{use_permission, use_web_notification_with_options, ShowOptions, UseWebNotificationOptions, UseWebNotificationReturn};

use sphare_core_common::constants::{LOGO_ICON_PATH, SITE_NAME};
use sphare_core_common::errors::AppError;
use sphare_core_common::routes::NOTIFICATION_ROUTE;
use sphare_core_user::notification::{get_notification_path, get_notification_text, on_read_notif, NotifHandler, Notification, NotificationType, NOTIF_RELOAD_INTERVAL_MS, NOTIF_STATE_STORAGE, NOTIF_TAG};

use sphare_iface_user::notification::{set_all_notifications_read, set_notification_read};

use sphare_cmp_utils::icons::{LoadingIcon, NotificationIcon, ReadAllIcon, ReadIcon, RefreshIcon, UnreadIcon};
use sphare_cmp_utils::unpack::SuspenseUnpack;
use sphare_cmp_utils::widget::{TimeSinceWidget};

use crate::auth_widget::AuthorWidget;
use crate::sphere::SphereHeaderLink;
use crate::state::GlobalState;

/// When logged in, displays a bell button with the number of unread notifications, redirects to the notification page on click.
#[component]
pub fn NotificationButton() -> impl IntoView {
    let state = expect_context::<GlobalState>();
    let (_, set_notif_handler, _) = use_local_storage::<NotifHandler, JsonSerdeCodec>(NOTIF_STATE_STORAGE);
    let is_wide_screen = use_breakpoints(breakpoints_tailwind()).ge(BreakpointsTailwind::Lg);

    let UseWebNotificationReturn {
        show,
        ..
    } = use_web_notification_with_options(
        UseWebNotificationOptions::default()
            .renotify(true)
            .tag(NOTIF_TAG)
            .icon(LOGO_ICON_PATH)
            .title(SITE_NAME)
    );

    let send_notif_fn = StoredValue::new(move |body: String| {
        show(ShowOptions::default().body(body));
    });

    view! {
        <Transition fallback=move || view! {  <LoadingIcon/> }>
            {
                move || Suspend::new(async move {
                    match state.user.await {
                        Ok(Some(_)) => {
                            use_interval_fn(
                                move || state.notif_resource.refetch(),
                                NOTIF_RELOAD_INTERVAL_MS,
                            );
                            match state.notif_resource.await {
                                Ok(notif_vec) => {
                                    set_notif_handler.write().handle_notifications(notif_vec, state.unread_notif_count, send_notif_fn.get_value());
                                    view! {
                                        <a class="button-rounded-ghost relative flex" href=NOTIFICATION_ROUTE>
                                            <NotificationIcon/>
                                            <Show when=move || { state.unread_notif_count.get() > 0 }>
                                                <div class="notif_counter">
                                                    { move || match (state.unread_notif_count.get(), is_wide_screen.get()) {
                                                        (x, true) if x > 99 => String::from("99+"),
                                                        (x, false) if x > 9 => String::from("9+"),
                                                        (x, _) => x.to_string(),
                                                    }}
                                                </div>
                                            </Show>
                                        </a>
                                    }.into_any()
                                },
                                Err(e) => {
                                    log::error!("Failed to fetch notifications: {}", e);
                                    view! {
                                        <a class="button-rounded-ghost" href=NOTIFICATION_ROUTE>
                                            <NotificationIcon/>
                                        </a>
                                    }.into_any()
                                }
                            }
                        },
                        _ => ().into_any(),
                    }
                })
            }
        </Transition>
    }
}

/// List of notifications
#[component]
pub fn NotificationList() -> impl IntoView {
    let state = expect_context::<GlobalState>();

    view! {
        <div class="w-full xl:w-3/5 4xl:w-2/5 p-2 xl:px-4 mx-auto flex flex-col gap-2">
            <h2 class="py-4 text-4xl text-center">{move_tr!("notifications")}</h2>
            <div class="flex justify-end px-4">
                <RefreshNotificationsButton resource=state.notif_resource/>
                <ReadAllNotificationsButton is_notif_read_map=state.is_notif_read_map/>
            </div>
            <ul class="flex flex-col flex-1 w-full overflow-x-hidden overflow-y-auto divide-y divide-base-content/20">
            <SuspenseUnpack resource=state.notif_resource let:notif_vec>
            {
                let mut is_notif_read_map = state.is_notif_read_map.write_value();
                notif_vec.iter().map(|notification| {
                    let is_notif_read = is_notif_read_map
                        .entry(notification.notification_id)
                        .or_insert(ArcRwSignal::new(notification.is_read));
                    view! {
                        <li><NotificationItem notification=notification.clone() is_notif_read=is_notif_read.clone()/></li>
                    }
                }).collect_view()
            }
            </SuspenseUnpack>
            </ul>
        </div>
    }
}

/// Button to set all notifications as read
#[component]
fn RefreshNotificationsButton(
    resource: Resource<Result<Vec<Notification>, AppError>>
) -> impl IntoView {
    let button_class = "button-rounded-ghost w-fit tooltip tooltip-bottom";

    let notif_permission = use_permission("notifications");
    let UseWebNotificationReturn {
        is_supported,
        ..
    } = use_web_notification_with_options(
        UseWebNotificationOptions::default()
            .renotify(true)
            .tag(NOTIF_TAG)
            .icon(LOGO_ICON_PATH)
            .title(SITE_NAME)
    );
    let show_notif_toast = RwSignal::new(false);
    let notif_toast_message = move || match (is_supported.get(), notif_permission.get()) {
        (false, _) => tr!("notif-not-supported"),
        (true, leptos_use::PermissionState::Granted) => tr!("notif-permission-granted"),
        (true, leptos_use::PermissionState::Unknown) => tr!("notif-permission-unknown"),
        (true, leptos_use::PermissionState::Prompt) => tr!("notif-permission-prompt"),
        (true, leptos_use::PermissionState::Denied) => tr!("notif-permission-denied"),
    };
    let toast_class = move || match (is_supported.get(), notif_permission.get()) {
        (false, _) | (_, leptos_use::PermissionState::Denied) => "alert alert-error",
        (true, leptos_use::PermissionState::Prompt) | (true, leptos_use::PermissionState::Unknown) => "alert alert-warning",
        (true, leptos_use::PermissionState::Granted) => "alert alert-success",
    };

    let toast_fade_timeout_fn = use_timeout_fn(
        move |_| show_notif_toast.set(false),
        10000.0
    );

    view! {
        <button
            class=button_class
            data-tip=move_tr!("refresh")
            on:click=move |_| {
                match (is_supported.get_untracked(), notif_permission.get_untracked()) {
                    (true, leptos_use::PermissionState::Granted) => show_notif_toast.set(false),
                    (_, permission_state) if permission_state != leptos_use::PermissionState::Denied => {
                        log::info!("Notifications permission missing: {}, requesting...", permission_state);
                        leptos::task::spawn_local(async move {
                            if let Ok(notification_permission) = web_sys::Notification::request_permission() {
                                let _ = notification_permission.await;
                            }
                        });
                        show_notif_toast.set(true);
                        (toast_fade_timeout_fn.start)(());
                    },
                    _ => log::warn!("Notifications permission denied."),
                };
                resource.refetch();
            }
        >
            <RefreshIcon/>
        </button>
        <Show when=show_notif_toast>
            <div class="toast toast-center">
                <div class=toast_class>
                    <span>{notif_toast_message}</span>
                </div>
            </div>
        </Show>
    }
}

/// Button to set all notifications as read
#[component]
fn ReadAllNotificationsButton(
    is_notif_read_map: StoredValue<HashMap<i64, ArcRwSignal<bool>>>
) -> impl IntoView {
    let state = expect_context::<GlobalState>();
    let read_all_action = Action::new(move |_: &()| async move {
        set_all_notifications_read().await
    });
    view! {
        <button
            class="button-rounded-ghost w-fit tooltip"
            data-tip=move_tr!("read-all-notifs")
            on:click=move |_| {
                state.unread_notif_count.set(0);
                for (_, is_notif_read) in &*is_notif_read_map.write_value() {
                    is_notif_read.set(true);
                }
                read_all_action.dispatch(());
            }
        >
            <ReadAllIcon/>
        </button>
    }
}

/// Button to set all notifications as read
#[component]
fn ReadNotificationButton(
    is_notif_read: ArcRwSignal<bool>,
    unread_notif_count: RwSignal<usize>,
    read_notif_action: Action<(), Result<(), AppError>>,
) -> impl IntoView {
    let is_notif_read = is_notif_read.clone();
    view! {
        <button
            class="button-rounded-ghost w-fit tooltip tooltip-left"
            data-tip=move_tr!("read-notif")
            on:click=move |ev| {
                ev.prevent_default();
                ev.stop_immediate_propagation();
                on_read_notif(is_notif_read.clone(), unread_notif_count, read_notif_action);
            }
        >
            <UnreadIcon/>
        </button>
    }
}

/// Single notification
#[component]
pub fn NotificationItem(
    notification: Notification,
    is_notif_read: ArcRwSignal<bool>,
) -> impl IntoView {
    let state = expect_context::<GlobalState>();
    let notif_id = notification.notification_id;
    let is_notif_read = StoredValue::new(is_notif_read);
    let is_moderation = notification.notification_type == NotificationType::Moderation;
    let message = get_notification_text(&notification);
    let link = get_notification_path(&notification);

    let read_notif_action = Action::new(move |_: &()| async move {
        set_notification_read(notif_id).await
    });

    Effect::new(move || if let Some(Err(e)) = &*read_notif_action.value().read() {
        log::error!("Failed to set notification as read: {}", e);
    });

    view! {
        <a
            href=link
            class="w-full p-2 px-4 my-1 flex justify-between items-center rounded-sm hover:bg-base-200"
            class:text-gray-400=is_notif_read.get_value()
            on:click=move |_| on_read_notif(is_notif_read.get_value(), state.unread_notif_count, read_notif_action)
        >
            <div class="flex flex-col gap-1">
                <div class="leading-7">
                    <div class="inline-block align-middle">
                        <AuthorWidget
                            author_id=notification.trigger_user_id
                            author=notification.trigger_username
                            is_moderator=is_moderation
                            is_grayed_out=is_notif_read.get_value()
                        />
                    </div>
                    <span
                        class="align-middle px-1"
                        class:text-gray-400=is_notif_read.get_value()
                    >
                        {message}
                    </span>
                </div>
                <div class="flex gap-1 items-center">
                    <SphereHeaderLink sphere_header=notification.sphere_header/>
                    <TimeSinceWidget timestamp=notification.create_timestamp is_grayed_out=is_notif_read.get_value()/>
                </div>
            </div>
            <Show
                when=move || is_notif_read.get_value().get()
                fallback=move || view! {
                    <ReadNotificationButton is_notif_read=is_notif_read.get_value() unread_notif_count=state.unread_notif_count read_notif_action/>
                }
            >
                <div class="p-1 lg:p-2"><ReadIcon/></div>
            </Show>
        </a>
    }
}