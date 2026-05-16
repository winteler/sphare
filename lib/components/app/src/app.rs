use leptos::ev::TouchEvent;
use leptos::prelude::*;
use const_format::formatcp;
use leptos_fluent::leptos_fluent;
use leptos_meta::{provide_meta_context, HashedStylesheet, Link, Meta, MetaTags, Title};
use leptos_router::{components::{ParentRoute, Route, Router, Routes}, ParamSegment, StaticSegment};
use regex::Regex;

use sphare_core_common::constants::{SITE_NAME};
use sphare_core_common::errors::AppError;
use sphare_core_common::routes::{ABOUT_SHARESPHERE_ROUTE, AUTH_CALLBACK_ROUTE, CONTENT_POLICY_ROUTE, CREATE_POST_SUFFIX, CREATE_SPHERE_SUFFIX, FAQ_ROUTE, NOTIFICATION_ROUTE, POPULAR_ROUTE, POST_ROUTE_PARAM_NAME, POST_ROUTE_PREFIX, PRIVACY_POLICY_ROUTE, PUBLISH_ROUTE, RULES_ROUTE, SATELLITE_ROUTE_PARAM_NAME, SATELLITE_ROUTE_PREFIX, SEARCH_ROUTE, SPHERE_ROUTE_PARAM_NAME, SPHERE_ROUTE_PREFIX, TERMS_AND_CONDITIONS_ROUTE, USER_ROUTE_PARAM_NAME, USER_ROUTE_PREFIX};

use sphare_iface_sphere::sphere::CreateSphere;
use sphare_iface_user::auth::{get_user, EndSession};
use sphare_iface_user::user::{DeleteUser, SetUserSettings};

use sphare_cmp_common::auth_widget::AuthCallback;
use sphare_cmp_common::state::GlobalState;
use sphare_cmp_content::post::{CreatePost, Post};
use sphare_cmp_sphere::satellite::{CreateSatellitePost, SatelliteBanner, SatelliteContent};
use sphare_cmp_sphere::sphere::{CreateSphere, SphereContents};
use sphare_cmp_sphere::sphere_management::{SphereCockpit, SphereCockpitGuard, MANAGE_SPHERE_ROUTE};
use sphare_cmp_ui::navigation_bar::NavigationBar;
use sphare_cmp_ui::policy::{AboutSphare, ContentPolicy, Faq, PrivacyPolicy, Rules, TermsAndConditions};
use sphare_cmp_ui::search::{Search, SphereSearch};
use sphare_cmp_ui::sidebar::LeftSidebar;
use sphare_cmp_utils::errors::ErrorTemplate;

use crate::home::{HomePage, HotPage, LoginGuard, LoginGuardHome, NotificationHome, ProfileHome, SphereHome};

const IS_TEST_SITE_ENV: &str = "IS_TEST_SITE";
const OEMBED_CONNECT_SRC: &str = env!("OEMBED_CONNECT_SRC");
const OEMBED_FRAME_SRC: &str = env!("OEMBED_FRAME_SRC");

#[derive(Clone, Debug)]
pub struct UserAgentHeader {
    pub value: Option<String>,
}

#[component]
pub fn AppMeta() -> impl IntoView {
    let connect_src_csp = match cfg!(debug_assertions) {
        true => formatcp!("connect-src 'self' https: ws://localhost:3001/ ws://127.0.0.1:3001/ {OEMBED_CONNECT_SRC};"),
        false => formatcp!("connect-src 'self' {OEMBED_CONNECT_SRC};"),
    };
    let frame_src_csp = formatcp!("frame-src 'self' {OEMBED_FRAME_SRC};");
    let ios_user_agent_regex = Regex::new("(iPhone|iPad|iPod|iOS).*AppleWebKit").expect("iOS regex should be valid");

    view! {
        <Meta
            http_equiv="Content-Security-Policy"
            content=move || {
                // this will insert the CSP with nonce on the server, be empty on client
                use_nonce().map(|nonce| {
                    let script_src_csp = match use_context::<UserAgentHeader>().map(|header| header.value) {
                        Some(Some(user_agent)) if ios_user_agent_regex.captures(user_agent.as_str()).is_some() => {
                            format!("script-src 'strict-dynamic' 'nonce-{nonce}' 'wasm-unsafe-eval' 'unsafe-eval';")
                        },
                        _ => format!("script-src 'strict-dynamic' 'nonce-{nonce}' 'wasm-unsafe-eval';")
                    };
                    format!(
                        "default-src 'none';
                        {script_src_csp}
                        img-src 'self' https: data:;
                        media-src 'self' https:;
                        {frame_src_csp}
                        style-src 'self' 'nonce-{nonce}';
                        {connect_src_csp}"
                    )
                }).unwrap_or_default()
            }
        />
    }
}

pub fn shell(options: LeptosOptions) -> impl IntoView {
    let is_test_site = std::env::var(IS_TEST_SITE_ENV).is_ok_and(|is_test_site_str| is_test_site_str.to_lowercase() == "true");
    view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8"/>
                <meta name="viewport" content="width=device-width, initial-scale=1"/>
                { match is_test_site {
                    true => Some(view! { <meta name="robots" content="noindex, nofollow"/> }),
                    false => None,
                }}
                <AppMeta/>
                <AutoReload options=options.clone() />
                <HydrationScripts options=options.clone() />
                // id=leptos means cargo-leptos will hot-reload this stylesheet
                <HashedStylesheet id="leptos" options/>
                <MetaTags/>
                <Link rel="icon" href="/favicon.ico" />
            </head>
            <body>
                <App/>
            </body>
        </html>
    }
}

#[component]
pub fn App() -> impl IntoView {
    // Provides context that manages stylesheets, titles, meta tags, etc.
    provide_meta_context();

    // Provide global context for app
    let logout_action = ServerAction::<EndSession>::new();
    let delete_user_action = ServerAction::<DeleteUser>::new();
    let create_sphere_action = ServerAction::<CreateSphere>::new();
    let set_settings_action = ServerAction::<SetUserSettings>::new();
    let user = Resource::new(
        move || {
            (
                logout_action.version().get(),
                delete_user_action.version().get(),
                create_sphere_action.version().get(),
                set_settings_action.version().get(),
            )
        },
        move |_| get_user(),
    );
    let state = GlobalState::new(
        user,
        logout_action,
        delete_user_action,
        create_sphere_action,
        set_settings_action,
    );
    provide_context(state);

    let swipe_start_x = RwSignal::new(None);
    let swipe_start_y = RwSignal::new(None);
    let swipe_id = RwSignal::new(None);

    let on_touch_start = move |ev: TouchEvent| {
        if let Some(touch) = ev.touches().item(0) {
            swipe_start_x.set(Some(touch.client_x()));
            swipe_start_y.set(Some(touch.client_y()));
            swipe_id.set(Some(touch.identifier()));
        }
    };
    let on_touch_end = move |ev: TouchEvent| {
        log::debug!("Touch end. {:?}, {:?}, {:?}", swipe_start_x.get_untracked(), swipe_start_y.get_untracked(), swipe_id.get_untracked());
        if let Some(touch) = ev.changed_touches().item(0) {
            log::debug!("Touch x: {}, touch y: {}, touch id: {}", touch.client_x(), touch.client_x(), touch.identifier());
            if swipe_id.get_untracked().is_some_and(|swipe_id| swipe_id == touch.identifier()) {
                let threshold = 50;
                let delta_x = touch.client_x() - swipe_start_x.get_untracked().unwrap_or(touch.client_x());
                let delta_y = touch.client_y() - swipe_start_y.get_untracked().unwrap_or(touch.client_y());
                match (delta_x, delta_y) {
                    (delta_x, delta_y) if delta_x < -threshold && delta_y.abs() < threshold => {
                        log::debug!("Swipe left: delta_x = {delta_x}, delta_y = {delta_y}");
                        handle_left_swipe(state.show_left_sidebar, state.show_right_sidebar);
                    },
                    (delta_x, delta_y) if delta_x > threshold && delta_y.abs() < threshold => {
                        log::debug!("Swipe right: delta_x = {delta_x}, delta_y = {delta_y}");
                        handle_right_swipe(state.show_left_sidebar, state.show_right_sidebar);
                    },
                    _ => log::debug!("No swipe: delta_x = {delta_x}, delta_y = {delta_y}"),
                }
            }
        }
        swipe_start_x.set(None);
        swipe_start_y.set(None);
        swipe_id.set(None);
    };

    view! {
        <I18nProvider>
            <Title text=SITE_NAME/>
            <Router>
                <main
                    class="h-screen w-screen overflow-hidden text-white relative"
                    on:touchstart=on_touch_start
                    on:touchend=on_touch_end
                >
                    <div class="h-full flex flex-col max-lg:items-center">
                        <NavigationBar/>
                        <div class="grow flex w-full overflow-hidden min-h-0">
                            <LeftSidebar/>
                            <Routes fallback=|| {
                                let mut outside_errors = Errors::default();
                                outside_errors.insert_with_default_key(AppError::NotFound);
                                view! {
                                    <ErrorTemplate outside_errors/>
                                }
                            }>
                                <Route path=StaticSegment("") view=HomePage/>
                                <Route path=StaticSegment(POPULAR_ROUTE) view=HotPage/>
                                <ParentRoute path=(StaticSegment(SPHERE_ROUTE_PREFIX), ParamSegment(SPHERE_ROUTE_PARAM_NAME)) view=SphereHome>
                                    <ParentRoute path=(StaticSegment(SATELLITE_ROUTE_PREFIX), ParamSegment(SATELLITE_ROUTE_PARAM_NAME)) view=SatelliteBanner>
                                        <Route path=(StaticSegment(POST_ROUTE_PREFIX), ParamSegment(POST_ROUTE_PARAM_NAME)) view=Post/>
                                        <ParentRoute path=StaticSegment(PUBLISH_ROUTE) view=LoginGuard>
                                            <Route path=StaticSegment(CREATE_POST_SUFFIX) view=CreateSatellitePost/>
                                        </ParentRoute>
                                        <Route path=StaticSegment("") view=SatelliteContent/>
                                    </ParentRoute>
                                    <Route path=(StaticSegment(POST_ROUTE_PREFIX), ParamSegment(POST_ROUTE_PARAM_NAME)) view=Post/>
                                    <ParentRoute path=StaticSegment(MANAGE_SPHERE_ROUTE) view=SphereCockpitGuard>
                                        <Route path=StaticSegment("") view=SphereCockpit/>
                                    </ParentRoute>
                                    <Route path=StaticSegment(SEARCH_ROUTE) view=SphereSearch/>
                                    <Route path=StaticSegment("") view=SphereContents/>
                                </ParentRoute>
                                <Route path=(StaticSegment(USER_ROUTE_PREFIX), ParamSegment(USER_ROUTE_PARAM_NAME)) view=ProfileHome/>
                                <Route path=StaticSegment(AUTH_CALLBACK_ROUTE) view=AuthCallback/>
                                <ParentRoute path=StaticSegment(PUBLISH_ROUTE) view=LoginGuardHome>
                                    <Route path=StaticSegment(CREATE_SPHERE_SUFFIX) view=CreateSphere/>
                                    <Route path=StaticSegment(CREATE_POST_SUFFIX) view=CreatePost/>
                                </ParentRoute>
                                <Route path=StaticSegment(NOTIFICATION_ROUTE) view=NotificationHome/>
                                <Route path=StaticSegment(SEARCH_ROUTE) view=Search/>
                                <Route path=StaticSegment(ABOUT_SHARESPHERE_ROUTE) view=AboutSphare/>
                                <Route path=StaticSegment(TERMS_AND_CONDITIONS_ROUTE) view=TermsAndConditions/>
                                <Route path=StaticSegment(PRIVACY_POLICY_ROUTE) view=PrivacyPolicy/>
                                <Route path=StaticSegment(CONTENT_POLICY_ROUTE) view=ContentPolicy/>
                                <Route path=StaticSegment(RULES_ROUTE) view=Rules/>
                                <Route path=StaticSegment(FAQ_ROUTE) view=Faq/>
                            </Routes>
                        </div>
                    </div>
                </main>
            </Router>
        </I18nProvider>
    }
}

#[component]
pub fn I18nProvider(children: Children) -> impl IntoView {
    leptos_fluent! {
        children: children(),
        locales: "../../../locales",
        languages: "../../../locales/languages.json",
        default_language: "en",
        #[cfg(not(feature = "ssr"))]
        check_translations: "../../**/*.rs",
        sync_html_tag_lang: true,
        sync_html_tag_dir: true,
        cookie_name: "lang",
        cookie_attrs: "SameSite=Strict; Secure; path=/; max-age=600",
        initial_language_from_cookie: true,
        initial_language_from_cookie_to_local_storage: true,
        set_language_to_cookie: true,
        url_param: "lang",
        initial_language_from_url_param: true,
        initial_language_from_url_param_to_local_storage: true,
        initial_language_from_url_param_to_cookie: true,
        set_language_to_url_param: true,
        local_storage_key: "language",
        initial_language_from_local_storage: true,
        initial_language_from_local_storage_to_cookie: true,
        set_language_to_local_storage: true,
        initial_language_from_navigator: true,
        initial_language_from_navigator_to_local_storage: true,
        initial_language_from_accept_language_header: true,
    }
}
fn handle_right_swipe(
    show_left_sidebar: RwSignal<bool>,
    show_right_sidebar: RwSignal<bool>,
) {
    if show_right_sidebar.get_untracked() {
        show_right_sidebar.set(false);
    } else if !show_left_sidebar.get_untracked() {
        show_left_sidebar.set(true);
    }
}

fn handle_left_swipe(
    show_left_sidebar: RwSignal<bool>,
    show_right_sidebar: RwSignal<bool>,
) {
    if show_left_sidebar.get_untracked() {
        show_left_sidebar.set(false);
    } else if !show_right_sidebar.get_untracked() {
        show_right_sidebar.set(true);
    }
}
