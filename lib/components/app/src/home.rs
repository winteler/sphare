use leptos::html::Div;
use leptos::prelude::*;
use leptos_fluent::move_tr;
use leptos_router::components::Outlet;
use leptos_use::{signal_throttled_with_options, ThrottleOptions};

use sphare_core_common::constants::{LOGO_ICON_PATH, POPULAR_ICON_PATH, POST_BATCH_SIZE, SCROLL_LOAD_THROTTLE_DELAY};
use sphare_core_common::unpack::{handle_additional_load, reset_additional_load};
use sphare_core_content::post::PostWithSphereInfo;

use sphare_iface_content::post::{get_homepage_post_vec, get_sorted_post_vec};

use sphare_cmp_base::post::PostListWithInitLoad;
use sphare_cmp_base::ranking::PostSortWidget;
use sphare_cmp_common::auth_widget::LoginWindow;
use sphare_cmp_common::notification::NotificationList;
use sphare_cmp_common::state::GlobalState;
use sphare_cmp_content::profile::UserProfile;
use sphare_cmp_sphere::sphere::SphereBanner;
use sphare_cmp_ui::sidebar::{HomeSidebar, SphereSidebar};
use sphare_cmp_utils::node_utils::has_reached_scroll_load_threshold;
use sphare_cmp_utils::unpack::SuspenseUnpack;
use sphare_cmp_utils::widget::{BannerContent, RefreshButton};

/// Login guard with home sidebar
#[component]
pub fn LoginGuardHome() -> impl IntoView {
    view! {
        <LoginGuard/>
        <HomeSidebar/>
    }
}

/// Component to guard pages requiring a login, and enable the user to log in with a redirect
#[component]
pub fn LoginGuard() -> impl IntoView {
    let state = expect_context::<GlobalState>();
    view! {
        <SuspenseUnpack resource=state.user let:user>
        {
            match user {
                Some(_) => view! { <Outlet/> }.into_any(),
                None => view! { <LoginWindow/> }.into_any(),
            }
        }
        </SuspenseUnpack>
    }
}

/// Renders the home page of Sphare.
#[component]
pub fn HomePage() -> impl IntoView {
    let state = expect_context::<GlobalState>();
    let refresh_count = RwSignal::new(0);
    let additional_load_count = RwSignal::new(0);
    let is_loading = RwSignal::new(false);
    let additional_post_vec = RwSignal::new(Vec::<PostWithSphereInfo>::new());
    let load_error = RwSignal::new(None);
    let div_ref = NodeRef::<Div>::new();

    let additional_load_count_throttled: Signal<i32> = signal_throttled_with_options(
        additional_load_count,
        SCROLL_LOAD_THROTTLE_DELAY,
        ThrottleOptions::default().leading(true).trailing(false)
    );

    let post_vec_resource = Resource::new(
        move || (state.post_sort_type.get(), refresh_count.get()),
        move |(sort_type, _)|  async move {
            #[cfg(feature = "hydrate")]
            is_loading.set(true);
            reset_additional_load(additional_post_vec, additional_load_count, Some(div_ref));
            let result = get_homepage_post_vec(sort_type, 0).await;
            #[cfg(feature = "hydrate")]
            is_loading.set(false);
            result
        }
    );

    let _additional_post_resource = LocalResource::new(
        move || async move {
            if additional_load_count_throttled.get() > 0 {
                is_loading.set(true);
                let num_post = (POST_BATCH_SIZE as usize) + additional_post_vec.read_untracked().len();
                let additional_load = get_homepage_post_vec(state.post_sort_type.get_untracked(), num_post).await;
                handle_additional_load(additional_load, additional_post_vec, load_error);
                is_loading.set(false);
            }
        }
    );

    view! {
        <div
            class="flex flex-col flex-1 w-full overflow-x-hidden overflow-y-auto px-2 xl:px-4"
            on:scroll=move |_| if has_reached_scroll_load_threshold(div_ref) && !is_loading.get_untracked() {
                additional_load_count.update(|value| *value += 1);
            }
            node_ref=div_ref
        >
            <BannerWithWidgets title=move_tr!("home") icon_url=Some(String::from(LOGO_ICON_PATH)) banner_url=None refresh_count/>
            <PostListWithInitLoad
                post_vec_resource
                additional_post_vec
                is_loading
                load_error
                add_y_overflow_auto=false
            />
        </div>
        <HomeSidebar/>
    }
}

/// Renders a page with the most popular content of Sphare.
#[component]
pub fn HotPage() -> impl IntoView {
    let state = expect_context::<GlobalState>();
    let refresh_count = RwSignal::new(0);
    let additional_load_count = RwSignal::new(0);
    let is_loading = RwSignal::new(false);
    let additional_post_vec = RwSignal::new(Vec::<PostWithSphereInfo>::new());
    let load_error = RwSignal::new(None);
    let div_ref = NodeRef::<Div>::new();

    let additional_load_count_throttled: Signal<i32> = signal_throttled_with_options(
        additional_load_count,
        SCROLL_LOAD_THROTTLE_DELAY,
        ThrottleOptions::default().leading(true).trailing(false)
    );

    let post_vec_resource = Resource::new(
        move || (state.post_sort_type.get(), refresh_count.get()),
        move |(sort_type, _)| async move {
            #[cfg(feature = "hydrate")]
            is_loading.set(true);
            reset_additional_load(additional_post_vec, additional_load_count, Some(div_ref));
            let result = get_sorted_post_vec(sort_type, 0).await;
            #[cfg(feature = "hydrate")]
            is_loading.set(false);
            result
        }
    );

    let _additional_post_resource = LocalResource::new(
        move || async move {
            if additional_load_count_throttled.get() > 0 {
                is_loading.set(true);
                let num_post = (POST_BATCH_SIZE as usize) + additional_post_vec.read_untracked().len();
                let additional_load = get_sorted_post_vec(state.post_sort_type.get_untracked(), num_post).await;
                handle_additional_load(additional_load, additional_post_vec, load_error);
                is_loading.set(false);
            }
        }
    );

    view! {
        <div
            class="flex flex-col flex-1 w-full overflow-x-hidden overflow-y-auto px-2 xl:px-4"
            on:scroll=move |_| if has_reached_scroll_load_threshold(div_ref) && !is_loading.get_untracked() {
                additional_load_count.update(|value| *value += 1);
            }
            node_ref=div_ref
        >
            <BannerWithWidgets title=move_tr!("popular") icon_url=Some(String::from(POPULAR_ICON_PATH)) banner_url=None refresh_count/>
            <PostListWithInitLoad
                post_vec_resource
                additional_post_vec
                is_loading=is_loading
                load_error=load_error
                add_y_overflow_auto=false
            />
        </div>
        <HomeSidebar/>
    }
}

/// Component to display the content of a banner
#[component]
fn BannerWithWidgets(
    #[prop(into)]
    title: Signal<String>,
    icon_url: Option<String>,
    banner_url: Option<String>,
    refresh_count: RwSignal<usize>,
) -> impl IntoView {
    let state = expect_context::<GlobalState>();
    view! {
        <div class="mt-2 relative flex-none rounded-sm w-full h-16 2xl:h-24 4xl:h-32 flex items-center justify-center max-w-full overflow-hidden">
            <BannerContent title icon_url banner_url sphere_icon_class="h-8 w-8 2xl:h-12 2xl:w-12 rounded-none"/>
        </div>
        <div class="sticky top-0 bg-base-100 py-2 flex justify-between items-center">
            <PostSortWidget sort_signal=state.post_sort_type is_tooltip_bottom=true/>
            <RefreshButton refresh_count is_tooltip_bottom=true/>
        </div>
    }
}

/// Main page for notifications
#[component]
pub fn NotificationHome() -> impl IntoView {
    let state = expect_context::<GlobalState>();
    view! {
        <SuspenseUnpack resource=state.user let:user>
        {
            match user {
                Some(_) => view! { <NotificationList/> }.into_any(),
                None => view! { <LoginWindow/> }.into_any(),
            }
        }
        </SuspenseUnpack>
        <HomeSidebar/>
    }
}

/// Displays a user's profile
#[component]
pub fn SphereHome() -> impl IntoView {
    view! {
        <SphereBanner/>
        <SphereSidebar/>
    }
}

/// Displays a user's profile
#[component]
pub fn ProfileHome() -> impl IntoView {
    view! {
        <UserProfile/>
        <HomeSidebar/>
    }
}