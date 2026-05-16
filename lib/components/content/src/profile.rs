use leptos::html;
use leptos::prelude::*;
use leptos_fluent::move_tr;
use leptos_router::hooks::use_params_map;
use leptos_use::{signal_throttled_with_options, ThrottleOptions};
use serde::{Deserialize, Serialize};
use strum::IntoEnumIterator;
use strum_macros::{Display, EnumIter, EnumString, IntoStaticStr};

use sphare_core_common::constants::{POST_BATCH_SIZE, SCROLL_LOAD_THROTTLE_DELAY};
use sphare_core_common::routes::get_username_memo;
use sphare_core_common::unpack::{handle_additional_load, handle_initial_load, reset_additional_load};
use sphare_core_content::ranking::{CommentSortType, PostSortType, SortType};

use sphare_iface_content::profile::{get_user_comment_vec, get_user_post_vec};
use sphare_iface_user::auth::NavigateToUserAccount;

use sphare_cmp_base::comment::CommentMiniatureList;
use sphare_cmp_base::post::PostListWithInitLoad;
use sphare_cmp_base::ranking::{CommentSortWidget, PostSortWidget};
use sphare_cmp_common::state::GlobalState;
use sphare_cmp_utils::form::LabeledFormCheckbox;
use sphare_cmp_utils::icons::{LoadingIcon, UserIcon, UserSettingsIcon};
use sphare_cmp_utils::unpack::ActionError;
use sphare_cmp_utils::view::ToView;
use sphare_cmp_utils::widget::{EnumQueryTabs, ModalDialog, ModalFormButtons};

pub const PROFILE_TAB_QUERY_PARAM: &str = "tab";

#[derive(Clone, Copy, Debug, Default, Display, EnumIter, EnumString, Eq, IntoStaticStr, Hash, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub enum ProfileTabs {
    #[default]
    Posts,
    Comments,
}

#[derive(Clone, Copy, Debug, Default, Display, EnumIter, EnumString, Eq, IntoStaticStr, Hash, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub enum SelfProfileTabs {
    #[default]
    Posts,
    Comments,
    Settings,
}

impl Into<Signal<String>> for ProfileTabs {
    fn into(self) -> Signal<String> {
        match self {
            ProfileTabs::Posts => move_tr!("posts"),
            ProfileTabs::Comments => move_tr!("comments"),
        }
    }
}

impl ToView for ProfileTabs {
    fn to_view(self) -> impl IntoView + 'static {
        match self {
            ProfileTabs::Posts => view! { <UserPosts/> }.into_any(),
            ProfileTabs::Comments => view! { <UserComments/> }.into_any(),
        }
    }
}

impl Into<Signal<String>> for SelfProfileTabs {
    fn into(self) -> Signal<String> {
        match self {
            SelfProfileTabs::Posts => move_tr!("posts"),
            SelfProfileTabs::Comments => move_tr!("comments"),
            SelfProfileTabs::Settings => move_tr!("settings"),
        }
    }
}

impl ToView for SelfProfileTabs {
    fn to_view(self) -> impl IntoView + 'static {
        match self {
            SelfProfileTabs::Posts => view! { <UserPosts/> }.into_any(),
            SelfProfileTabs::Comments => view! { <UserComments/> }.into_any(),
            SelfProfileTabs::Settings => view! { <UserSettings/> }.into_any(),
        }
    }
}

/// Displays a user's profile
#[component]
pub fn UserProfile() -> impl IntoView {
    let state = expect_context::<GlobalState>();
    let params = use_params_map();
    let query_username = get_username_memo(params);
    view! {
        <div class="w-full flex justify-center">
            <div class="w-full 2xl:w-4/5 4xl:w-2/3 flex flex-col max-lg:items-center">
                <div class="p-2 pt-4 flex items-center gap-1 text-2xl font-bold">
                    <UserIcon/>
                    {move || query_username.get()}
                </div>
                <Transition fallback=move || view! {  <LoadingIcon/> }>
                {
                    move || Suspend::new(async move {
                        match state.user.await {
                            Ok(Some(user)) if user.username == query_username.get() => view! {
                                <EnumQueryTabs
                                    query_param=PROFILE_TAB_QUERY_PARAM
                                    query_enum_iter=SelfProfileTabs::iter()
                                />
                            }.into_any(),
                            _ => view! {
                                <EnumQueryTabs
                                    query_param=PROFILE_TAB_QUERY_PARAM
                                    query_enum_iter=ProfileTabs::iter()
                                />
                            }.into_any(),
                        }
                    })
                }
                </Transition>
            </div>
        </div>
    }
}

/// Displays a user's posts
#[component]
pub fn UserPosts() -> impl IntoView {
    let params = use_params_map();
    let username = get_username_memo(params);
    let sort_signal = RwSignal::new(SortType::Post(PostSortType::Hot));
    let additional_post_vec = RwSignal::new(Vec::new());
    let additional_load_count = RwSignal::new(0);
    let is_loading = RwSignal::new(false);
    let load_error = RwSignal::new(None);
    let list_ref = NodeRef::<html::Ul>::new();

    let post_vec_resource = Resource::new(
        move || (username.get(), sort_signal.get()),
        move |(username, sort_type)| async move {
            #[cfg(feature = "hydrate")]
            is_loading.set(true);
            reset_additional_load(additional_post_vec, additional_load_count, Some(list_ref));
            let result = get_user_post_vec(username, sort_type, 0).await;
            #[cfg(feature = "hydrate")]
            is_loading.set(false);
            result
        }
    );

    let additional_load_count_throttled: Signal<i32> = signal_throttled_with_options(
        additional_load_count,
        SCROLL_LOAD_THROTTLE_DELAY,
        ThrottleOptions::default().leading(true).trailing(false)
    );

    let _additional_post_resource = LocalResource::new(
        move || async move {
            if additional_load_count_throttled.get() > 0 {
                is_loading.set(true);
                let num_post = (POST_BATCH_SIZE as usize) + additional_post_vec.read_untracked().len();
                let additional_load = get_user_post_vec(username.get_untracked(), sort_signal.get_untracked(), num_post).await;
                handle_additional_load(additional_load, additional_post_vec, load_error);
                is_loading.set(false);
            }
        }
    );

    view! {
        <PostSortWidget sort_signal/>
        <PostListWithInitLoad
            post_vec_resource
            additional_post_vec
            is_loading
            load_error
            additional_load_count
            list_ref
        />
    }
}

/// Displays a user's comments
#[component]
pub fn UserComments() -> impl IntoView {
    let params = use_params_map();
    let username = get_username_memo(params);
    let sort_signal = RwSignal::new(SortType::Comment(CommentSortType::Recent));
    let comment_vec = RwSignal::new(Vec::new());
    let additional_load_count = RwSignal::new(0);
    let is_loading = RwSignal::new(false);
    let load_error = RwSignal::new(None);
    let list_ref = NodeRef::<html::Ul>::new();

    let _initial_comment_resource = LocalResource::new(
        move || async move {
            is_loading.set(true);
            let initial_load = get_user_comment_vec(username.get(), sort_signal.get(), 0).await;
            handle_initial_load(initial_load, comment_vec, load_error, Some(list_ref));
            is_loading.set(false);
        }
    );

    let additional_load_count_throttled: Signal<i32> = signal_throttled_with_options(
        additional_load_count,
        SCROLL_LOAD_THROTTLE_DELAY,
        ThrottleOptions::default().leading(true).trailing(false)
    );

    let _additional_comment_resource = LocalResource::new(
        move || async move {
            if additional_load_count_throttled.get() > 0 {
                is_loading.set(true);
                let additional_load = get_user_comment_vec(
                    username.get_untracked(),
                    sort_signal.get_untracked(),
                    comment_vec.read_untracked().len(),
                ).await;
                handle_additional_load(additional_load, comment_vec, load_error);
                is_loading.set(false);
            }
        }
    );

    view! {
        <CommentSortWidget sort_signal/>
        <CommentMiniatureList
            comment_vec
            is_loading
            load_error
            additional_load_count
            list_ref
        />
    }
}

/// Displays a user's settings
#[component]
pub fn UserSettings() -> impl IntoView {
    let state = expect_context::<GlobalState>();

    view! {
        <div class="self-center flex flex-col gap-3 w-4/5 lg:w-full xl:w-4/5 4xl:3/5">
            <Suspense fallback=move || view! {  <LoadingIcon/> }>
            {
                move || Suspend::new(async move {
                    let (is_nsfw, show_nsfw, days_hide_spoiler) = match state.user.await {
                        Ok(Some(user)) => (user.is_nsfw, user.show_nsfw, user.days_hide_spoiler.unwrap_or_default()),
                        _ => (false, false, 0),
                    };
                    view! {
                        <ActionForm action=state.set_settings_action attr:class="flex flex-col gap-3">
                            <LabeledFormCheckbox name="is_nsfw" label=move_tr!("nsfw-profile") value=is_nsfw/>
                            <LabeledFormCheckbox name="show_nsfw" label=move_tr!("show-nsfw") value=show_nsfw/>
                            <div class="flex justify-between items-center">
                                {move_tr!("hide-spoiler-duration")}
                                <input
                                    type="number"
                                    min="0"
                                    max="999"
                                    name="days_hide_spoilers"
                                    class="input input-primary no-spinner text-right w-16"
                                    autocomplete="off"
                                    value=days_hide_spoiler
                                />
                            </div>
                            <button type="submit" class="button-secondary">
                                {move_tr!("save")}
                            </button>
                        </ActionForm>
                        <ActionError action=state.set_settings_action.into()/>
                    }
                })
            }
            </Suspense>
            <div class="flex justify-between items-center">
                <UserAccountButton/>
                <DeleteUserButton/>
            </div>
        </div>
    }
}

/// Button to delete one's account
#[component]
pub fn DeleteUserButton() -> impl IntoView {
    let state = expect_context::<GlobalState>();
    let show_dialog = RwSignal::new(false);
    view! {
        <button
            class="button-error"
            on:click=move |_| show_dialog.update(|value| *value = !*value)
        >
            {move_tr!("delete-account")}
        </button>
        <ModalDialog
            class="w-full max-w-lg"
            show_dialog
        >
            <div class="bg-base-100 shadow-xl p-3 rounded-xs flex flex-col gap-3">
                <div class="text-center font-bold text-2xl">{move_tr!("delete-account")}</div>
                <div class="text-center font-bold text-xl">{move_tr!("delete-warning")}</div>
                <ActionForm action=state.delete_user_action>
                    <ModalFormButtons
                        disable_publish=false
                        show_form=show_dialog
                    />
                </ActionForm>
                <ActionError action=state.delete_user_action.into()/>
            </div>
        </ModalDialog>
    }
}

/// Button to navigate to the user's account on the OIDC provider
#[component]
pub fn UserAccountButton() -> impl IntoView {
    let navigate_to_account_action = ServerAction::<NavigateToUserAccount>::new();
    view! {
        <ActionForm action=navigate_to_account_action attr:class="flex justify-center items-center">
            <button type="submit" class="button-primary flex items-center gap-2">
                <UserSettingsIcon/>
                {move_tr!("account")}
            </button>
        </ActionForm>
    }
}