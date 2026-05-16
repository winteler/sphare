use leptos::html::{Div, Select};
use leptos::prelude::*;
use leptos_fluent::{move_tr, I18n};
#[cfg(feature = "hydrate")]
use leptos_use::on_click_outside;

use sphare_core_common::common::{Rule, SphereHeader};
use sphare_core_common::errors::AppError;
use sphare_core_common::routes::{ABOUT_SHARESPHERE_ROUTE, CONTENT_POLICY_ROUTE, FAQ_ROUTE, GITHUB_REPO_URL, POPULAR_ROUTE, PRIVACY_POLICY_ROUTE, RULES_ROUTE, TERMS_AND_CONDITIONS_ROUTE};
use sphare_core_common::traits::ToLocalizedStr;
use sphare_core_content::search::SearchState;

use sphare_iface_sphere::sphere::{get_popular_sphere_headers, get_subscribed_sphere_headers};

use sphare_cmp_base::filter::{AllCategoriesToggle, OnlyCategoriesToggle};
use sphare_cmp_base::rule::{BaseRuleList, RuleList};
use sphare_cmp_base::search::SearchSpheres;
use sphare_cmp_base::sphere::SphereLinkList;
use sphare_cmp_base::sphere_category::SphereCategoryCollapseWithFilter;
use sphare_cmp_common::state::{GlobalState, SphereState};
use sphare_cmp_utils::icons::{GithubIcon, HomeIcon, PopularIcon};
use sphare_cmp_utils::unpack::TransitionUnpack;
use sphare_cmp_utils::widget::{Badge, TitleCollapse};

/// Component to display a collapsable list of sphere links
#[component]
pub fn SphereLinkListCollapse(
    #[prop(into)]
    title: Signal<String>,
    sphere_header_vec: Vec<SphereHeader>,
    #[prop(default = true)]
    is_open: bool,
) -> impl IntoView {
    if sphere_header_vec.is_empty() {
        return ().into_any()
    }
    view! {
        <TitleCollapse title=title is_open>
            <SphereLinkList sphere_header_vec=sphere_header_vec.clone()/>
        </TitleCollapse>
    }.into_any()
}

/// Component showing links to homepage and popular posts
#[component]
pub fn BaseLinks() -> impl IntoView {
    let state = expect_context::<GlobalState>();
    view! {
        <div class="flex flex-col gap-1">
            <a
                href="/"
                on:click=move |_| state.show_left_sidebar.set(false)
                class="px-2 py-1 rounded-sm hover:bg-base-content/20"
            >
                <Badge text=move_tr!("home")>
                    <HomeIcon/>
                </Badge>
            </a>
            <a
                href=POPULAR_ROUTE
                on:click=move |_| state.show_left_sidebar.set(false)
                class="px-2 py-1 rounded-sm hover:bg-base-content/20"
            >
                <Badge text=move_tr!("popular")>
                    <PopularIcon class="filter-icon-size"/>
                </Badge>
            </a>
        </div>
    }
}

#[component]
fn LanguageSelector() -> impl IntoView {
    // `expect_context::<leptos_fluent::I18n>()` to get the i18n context
    // `i18n.languages` exposes a static array with the available languages
    // `i18n.language.get()` to get the active language
    // `i18n.language.set(lang)` to set the active language

    let i18n = expect_context::<I18n>();
    let select_ref = NodeRef::<Select>::new();

    view! {
        <select
            class="select_input"
            on:change=move |_| {
                if let Some(select_ref) = select_ref.get_untracked() {
                    let lang_str = select_ref.value();
                    if let Some(lang) = i18n.languages.iter().find(|lang| lang.id.to_string() == lang_str) {
                        i18n.language.set(lang)
                    }
                };
            }
            node_ref=select_ref
        >
        {
            i18n.languages.iter().map(|lang| {
                view! {
                    <option
                        value=lang
                        selected=move || &i18n.language.get() == lang
                    >
                        {lang.name}
                    </option>
                }
            }).collect_view()
        }
        </select>
    }
}

/// Left sidebar component
#[component]
pub fn LeftSidebar() -> impl IntoView {
    let state = expect_context::<GlobalState>();
    let search_state = SearchState::default();
    let subscribed_sphere_vec_resource = Resource::new(
        move || {
            (
                state.logout_action.version().get(),
                state.create_sphere_action.version().get(),
                state.sphere_reload_signal.get(),
                state.subscribe_action.version().get(),
                state.unsubscribe_action.version().get(),
            )
        },
        |_| get_subscribed_sphere_headers(),
    );
    let popular_sphere_vec_resource = Resource::new(
        move || state.sphere_reload_signal.get(),
        |_| get_popular_sphere_headers(),
    );

    let sidebar_class = move || match state.show_left_sidebar.get() {
        true => "left_sidebar_base_class max-lg:translate-x-0 transition-transform duration-300 ease-in-out",
        false => "left_sidebar_base_class max-lg:-translate-x-100 transition-transform duration-300 ease-in-out",
    };
    let sidebar_ref = NodeRef::<Div>::new();
    #[cfg(feature = "hydrate")]
    {
        // only enable with "hydrate" to avoid server side "Dropped SendWrapper" error
        let _ = on_click_outside(sidebar_ref, move |_| state.show_left_sidebar.set(false));
    }

    view! {
        <div class=sidebar_class node_ref=sidebar_ref>
            <div class="flex flex-col gap-1">
                <BaseLinks/>
                <TransitionUnpack resource=subscribed_sphere_vec_resource let:sphere_header_vec>
                    <SphereLinkListCollapse
                        title=move_tr!("subscribed-spheres")
                        sphere_header_vec=sphere_header_vec.clone()
                    />
                </TransitionUnpack>
                <TransitionUnpack resource=popular_sphere_vec_resource let:popular_sphere_header_vec>
                    <SphereLinkListCollapse
                        title=move_tr!("popular-spheres")
                        sphere_header_vec=popular_sphere_header_vec.clone()
                        is_open=false
                    />
                </TransitionUnpack>
                <SearchSpheres search_state class="w-full gap-1" autofocus=false is_dropdown_style=true/>
            </div>
            <LanguageSelector/>
        </div>
        <Show when=state.show_left_sidebar>
            <div class="absolute top-0 right-0 h-full w-full bg-base-200/50"/>
        </Show>
    }
}

/// Home right sidebar component
#[component]
pub fn HomeSidebar() -> impl IntoView {
    let state = expect_context::<GlobalState>();
    let sidebar_class = move || match state.show_right_sidebar.get() {
        true => "right_sidebar_base_class max-lg:translate-x-0 transition-transform duration-300 ease-in-out",
        false => "right_sidebar_base_class max-lg:translate-x-100 transition-transform duration-300 ease-in-out",
    };
    let sidebar_ref = NodeRef::<Div>::new();
    #[cfg(feature = "hydrate")]
    {
        // only enable with "hydrate" to avoid server side "Dropped SendWrapper" error
        let _ = on_click_outside(sidebar_ref, move |_| state.show_right_sidebar.set(false));
    }

    view! {
        <div class=sidebar_class node_ref=sidebar_ref>
            <h1 class="text-xl font-semibold text-center">{move_tr!("welcome-to-sphare")}</h1>
            <div class="flex flex-col gap-2">
                <p>{move_tr!("sphare-right-sidebar-1")}</p>
                <p>{move_tr!("sphare-right-sidebar-2")}</p>
            </div>
            <ul class="list-disc list-inside">
                <li><a href=ABOUT_SHARESPHERE_ROUTE class="link text-primary">{move_tr!("about-sphare")}</a></li>
                <li><a href=TERMS_AND_CONDITIONS_ROUTE class="link text-primary">{move_tr!("terms-and-conditions")}</a></li>
                <li><a href=PRIVACY_POLICY_ROUTE class="link text-primary">{move_tr!("privacy-policy")}</a></li>
                <li><a href=CONTENT_POLICY_ROUTE class="link text-primary">{move_tr!("content-policy")}</a></li>
                <li><a href=RULES_ROUTE class="link text-primary">{move_tr!("rules")}</a></li>
                <li><a href=FAQ_ROUTE class="link text-primary">{move_tr!("faq")}</a></li>
                <li class="inline-flex items-center">
                    <a href=GITHUB_REPO_URL class="h-full inline-flex items-center gap-2">
                        <GithubIcon/>
                        <div class="link text-primary">{move_tr!("github-repo")}</div>
                    </a>
                </li>
            </ul>
            <BaseRuleList/>
        </div>
        <Show when=state.show_right_sidebar>
            <div class="absolute top-0 left-0 h-full w-full bg-base-200/50"/>
        </Show>
    }
}

/// Sphere right sidebar component
#[component]
pub fn SphereSidebar() -> impl IntoView {
    let state = expect_context::<GlobalState>();
    let sphere_state = expect_context::<SphereState>();

    let sidebar_class = move || match state.show_right_sidebar.get() {
        true => "right_sidebar_base_class max-lg:translate-x-0 transition-transform duration-300 ease-in-out",
        false => "right_sidebar_base_class max-lg:translate-x-100 transition-transform duration-300 ease-in-out",
    };
    let sidebar_ref = NodeRef::<Div>::new();
    #[cfg(feature = "hydrate")]
    {
        // only enable with "hydrate" to avoid server side "Dropped SendWrapper" error
        let _ = on_click_outside(sidebar_ref, move |_| state.show_right_sidebar.set(false));
    }
    view! {
        <div class=sidebar_class node_ref=sidebar_ref>
            <div class="flex flex-col gap-2">
                <div class="text-xl font-semibold text-center text-wrap wrap-anywhere">{sphere_state.sphere_name}</div>
                <TransitionUnpack resource=sphere_state.sphere_with_user_info_resource let:sphere_with_user_info>
                    <div class="pl-4 whitespace-pre-wrap">{sphere_with_user_info.sphere.description.clone()}</div>
                </TransitionUnpack>
            </div>
            <div class="border-b border-primary/80"/>
            <SphereCategoryList/>
            <div class="border-b border-primary/80"/>
            <SphereRuleList rule_resource=sphere_state.sphere_rules_resource/>
            <div class="border-b border-primary/80"/>
            <ModeratorList/>
        </div>
        <Show when=state.show_right_sidebar>
            <div class="absolute top-0 left-0 h-full w-full bg-base-200/50"/>
        </Show>
    }
}

/// List of rules given in the input resource
#[component]
fn SphereRuleList(
    rule_resource: Resource<Result<Vec<Rule>, AppError>>
) -> impl IntoView {
    view! {
        <TitleCollapse title=move_tr!("rules")>
            <TransitionUnpack resource=rule_resource let:rule_vec>
                <RuleList rule_vec=rule_vec.clone()/>
            </TransitionUnpack>
        </TitleCollapse>
    }
}

/// List of categories for a sphere
#[component]
pub fn SphereCategoryList() -> impl IntoView {
    let sphere_state = expect_context::<SphereState>();
    view! {
        <TitleCollapse title=move_tr!("categories")>
            <div class="flex flex-col pl-2 pt-1">
                <TransitionUnpack resource=sphere_state.sphere_categories_resource let:sphere_category_vec>
                {
                    sphere_category_vec.iter().map(|sphere_category| view! {
                        <SphereCategoryCollapseWithFilter sphere_category=sphere_category.clone()/>
                    }).collect_view()
                }
                </TransitionUnpack>
                <div class="w-full border-b border-0.5 border-base-content/20 mb-2"/>
                <div class="flex flex-col gap-1">
                    <AllCategoriesToggle/>
                    <OnlyCategoriesToggle/>
                </div>
            </div>
        </TitleCollapse>
    }
}

/// List of moderators for a sphere
#[component]
pub fn ModeratorList() -> impl IntoView {
    let sphere_state = expect_context::<SphereState>();
    view! {
         <TitleCollapse title=move_tr!("moderators")>
            <div class="flex flex-col gap-1">
                <div class="flex border-b border-base-content/20 pl-4">
                    <div class="w-1/2 py-2 text-left font-semibold">Username</div>
                    <div class="w-1/2 py-2 text-left font-semibold">Role</div>
                </div>
                <TransitionUnpack resource=sphere_state.sphere_roles_resource let:sphere_role_vec>
                {
                    sphere_role_vec.iter().map(|role| {
                        view! {
                            <div class="flex py-1 pl-4">
                                <div class="w-1/2 select-none">{role.username.clone()}</div>
                                <div class="w-1/2 select-none">{role.permission_level.to_localized_str()}</div>
                            </div>
                        }
                    }).collect_view()
                }
                </TransitionUnpack>
            </div>
        </TitleCollapse>
    }
}
