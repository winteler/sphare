use leptos::either::{Either, EitherOf4};
use leptos::html;
use leptos::prelude::*;
use leptos_fluent::move_tr;
use leptos_router::components::Form;
use leptos_use::{signal_throttled_with_options, ThrottleOptions};
use serde::{Deserialize, Serialize};
use strum::IntoEnumIterator;
use strum_macros::{Display, EnumIter, EnumString, IntoStaticStr};

use sphare_core_common::checks::check_username;
use sphare_core_common::constants::{MAX_SEARCH_QUERY_LENGTH, MAX_USERNAME_LENGTH, SCROLL_LOAD_THROTTLE_DELAY};
use sphare_core_common::routes::{SEARCH_ROUTE, SEARCH_TAB_QUERY_PARAM};
use sphare_core_common::unpack::{handle_additional_load, handle_initial_load};
use sphare_core_content::search::{is_content_search_valid, SearchState};

use sphare_iface_content::search::{search_comments, search_posts};
use sphare_iface_user::user::get_matching_user_header_vec;

use sphare_cmp_base::comment::CommentMiniatureList;
use sphare_cmp_base::post::PostListWithIndicators;
use sphare_cmp_base::search::{SearchForm, SearchSpheres};
use sphare_cmp_common::state::SphereState;
use sphare_cmp_common::user::UserHeaderLink;
use sphare_cmp_utils::icons::MagnifierIcon;
use sphare_cmp_utils::unpack::TransitionUnpack;
use sphare_cmp_utils::view::ToView;
use sphare_cmp_utils::widget::{EnumQueryTabs, NotFoundWidget};

use crate::sidebar::HomeSidebar;

#[derive(Clone, Copy, Debug, Default, Display, EnumIter, EnumString, Eq, IntoStaticStr, Hash, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub enum SearchType {
    #[default]
    Spheres,
    Posts,
    Comments,
    Users,
}

#[derive(Clone, Copy, Debug, Default, Display, EnumIter, EnumString, Eq, IntoStaticStr, Hash, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub enum SphereSearchType {
    #[default]
    Posts,
    Comments,
}

impl Into<Signal<String>> for SearchType {
    fn into(self) -> Signal<String> {
        match self {
            SearchType::Spheres => move_tr!("spheres"),
            SearchType::Posts => move_tr!("posts"),
            SearchType::Comments => move_tr!("comments"),
            SearchType::Users => move_tr!("users"),
        }
    }
}

impl ToView for SearchType {
    fn to_view(self) -> impl IntoView + 'static {
        match self {
            SearchType::Spheres => EitherOf4::A(view! { <SearchSpheresWithContext/> }),
            SearchType::Posts => EitherOf4::B(view! { <SearchPosts/> }),
            SearchType::Comments => EitherOf4::C(view! { <SearchComments/> }),
            SearchType::Users => EitherOf4::D(view! { <SearchUsers/> }),
        }
    }
}

impl Into<Signal<String>> for SphereSearchType {
    fn into(self) -> Signal<String> {
        match self {
            SphereSearchType::Posts => move_tr!("posts"),
            SphereSearchType::Comments => move_tr!("comments"),
        }
    }
}

impl ToView for SphereSearchType {
    fn to_view(self) -> impl IntoView + 'static {
        match self {
            SphereSearchType::Posts => Either::Left(view! { <SearchPosts/> }),
            SphereSearchType::Comments => Either::Right(view! { <SearchComments/> }),
        }
    }
}

/// Button to navigate to the search page
#[component]
pub fn SearchButton(
    #[prop(default="button-rounded-ghost")]
    class: &'static str,
) -> impl IntoView
{
    let tab: &'static str = SearchType::default().into();
    view! {
        <Form method="GET" action=SEARCH_ROUTE>
            <input name=SEARCH_TAB_QUERY_PARAM value=tab class="hidden"/>
            <button class=class>
                <MagnifierIcon/>
            </button>
        </Form>
    }
}

/// Component to search spheres, posts, comments and users
#[component]
pub fn Search() -> impl IntoView
{
    provide_context(SearchState::default());
    view! {
        <div class="w-full flex justify-center">
            <div class="w-full 2xl:w-4/5 4xl:w-2/3 flex flex-col">
                <EnumQueryTabs
                    query_param=SEARCH_TAB_QUERY_PARAM
                    query_enum_iter=SearchType::iter()
                />
            </div>
        </div>
        <HomeSidebar/>
    }
}

/// Component to search posts, comments in a sphere
#[component]
pub fn SphereSearch() -> impl IntoView
{
    provide_context(SearchState::default());
    view! {
        <div class="w-full flex justify-center">
            <div class="w-full flex flex-col">
                <EnumQueryTabs
                    query_param=SEARCH_TAB_QUERY_PARAM
                    query_enum_iter=SphereSearchType::iter()
                />
            </div>
        </div>
    }
}

/// Component to search spheres, uses the SearchState from the context to get user input
#[component]
pub fn SearchSpheresWithContext() -> impl IntoView
{
    let search_state = expect_context::<SearchState>();
    view! {
        <SearchSpheres search_state/>
    }
}

#[component]
pub fn SearchPosts() -> impl IntoView
{
    let search_state = expect_context::<SearchState>();
    let sphere_state = use_context::<SphereState>();

    let post_vec = RwSignal::new(Vec::new());
    let additional_load_count = RwSignal::new(0);
    let is_loading = RwSignal::new(false);
    let load_error = RwSignal::new(None);
    let list_ref = NodeRef::<html::Ul>::new();

    let input_error = is_content_search_valid(search_state.search_input.into());

    let _initial_post_resource = LocalResource::new(
        move || async move {
            is_loading.set(true);
            let search_input = search_state.search_input_debounced.get();
            let initial_load = match search_input.is_empty() {
                true => Ok(Vec::new()),
                false => search_posts(
                    search_state.search_input_debounced.get(),
                    sphere_state.map(|sphere_state| sphere_state.sphere_name.get()),
                    search_state.show_spoiler.get(),
                    0,
                ).await,
            };
            handle_initial_load(initial_load, post_vec, load_error, Some(list_ref));
            is_loading.set(false);
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
                let additional_load = search_posts(
                    search_state.search_input_debounced.get(),
                    sphere_state.map(|sphere_state| sphere_state.sphere_name.get()),
                    search_state.show_spoiler.get(),
                    post_vec.read_untracked().len(),
                ).await;
                handle_additional_load(additional_load, post_vec, load_error);
                is_loading.set(false);
            }
        }
    );
    view! {
        <SearchForm
            search_state=search_state.clone()
            show_spoiler_checkbox=true
            maxlength=Some(MAX_SEARCH_QUERY_LENGTH)
            input_error
        />
        { move || match (post_vec.read().is_empty(), search_state.search_input_debounced.get_untracked().is_empty()) {
            (true, true) => None,
            (true, false) => Some(Either::Left(view! { <NotFoundWidget message=move_tr!("search-no-post-found")/> })),
            (false, _) => Some(Either::Right(
                view! {
                    <PostListWithIndicators
                        post_vec
                        is_loading
                        load_error
                        additional_load_count
                        list_ref
                    />
                }
            ))
        }}
    }
}

#[component]
pub fn SearchComments() -> impl IntoView
{
    let search_state = expect_context::<SearchState>();
    let sphere_state = use_context::<SphereState>();

    let comment_vec = RwSignal::new(Vec::new());
    let additional_load_count = RwSignal::new(0);
    let is_loading = RwSignal::new(false);
    let load_error = RwSignal::new(None);
    let list_ref = NodeRef::<html::Ul>::new();

    let input_error = is_content_search_valid(search_state.search_input.into());

    let _initial_comment_resource = LocalResource::new(
        move || async move {
            is_loading.set(true);
            let search_input = search_state.search_input_debounced.get();
            let initial_load = match search_input.is_empty() {
                true => Ok(Vec::new()),
                false => search_comments(
                    search_state.search_input_debounced.get(),
                    sphere_state.map(|sphere_state| sphere_state.sphere_name.get()),
                    0,
                ).await,
            };
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
                let additional_load = search_comments(
                    search_state.search_input_debounced.get_untracked(),
                    sphere_state.map(|sphere_state| sphere_state.sphere_name.get()),
                    comment_vec.read_untracked().len()
                ).await;
                handle_additional_load(additional_load, comment_vec, load_error);
                is_loading.set(false);
            }
        }
    );
    view! {
        <SearchForm
            search_state=search_state.clone()
            show_spoiler_checkbox=false
            maxlength=Some(MAX_SEARCH_QUERY_LENGTH)
            input_error
        />
        { move || match (comment_vec.read().is_empty(), search_state.search_input_debounced.get_untracked().is_empty()) {
            (true, true) => None,
            (true, false) => Some(Either::Left(view! { <NotFoundWidget message=move_tr!("search-no-comment-found")/> })),
            (false, _) => Some(Either::Right(
                view! {
                    <CommentMiniatureList
                        comment_vec
                        is_loading
                        load_error
                        additional_load_count
                        list_ref
                    />
                }
            ))
        }}

    }
}

#[component]
pub fn SearchUsers() -> impl IntoView
{
    let search_state = expect_context::<SearchState>();
    let search_user_resource = Resource::new(
        move || search_state.search_input_debounced.get(),
        move |search_input| async move {
            match check_username(&search_input, false) {
                Err(_) => Ok(Vec::new()),
                Ok(()) => get_matching_user_header_vec(search_input, None, 50).await,
            }
        }
    );

    let input_error = Signal::derive(move || check_username(&search_state.search_input.read(), true).err());

    view! {
        <SearchForm
            search_state=search_state.clone()
            show_spoiler_checkbox=false
            maxlength=Some(MAX_USERNAME_LENGTH)
            input_error
        />
        <TransitionUnpack resource=search_user_resource let:user_header_vec>
        { match (user_header_vec.is_empty(), search_state.search_input_debounced.read_untracked().is_empty())  {
            (true, true) => None,
            (true, false) => Some(Either::Left(view! { <NotFoundWidget message=move_tr!("search-no-user-found")/> })),
            (false, _) => {
                let user_header_link_list = user_header_vec.iter().map(|user_header| view! {
                    <li><UserHeaderLink user_header=user_header.clone()/></li>
                }).collect_view();
                Some(Either::Right(view! {
                    <ul class="flex flex-col self-center p-2 overflow-y-auto max-h-full w-4/5 lg:w-full 2xl:w-1/2 divide-y divide-base-content/20">
                        {user_header_link_list}
                    </ul>
                }))
            },
        }}
        </TransitionUnpack>
    }
}
