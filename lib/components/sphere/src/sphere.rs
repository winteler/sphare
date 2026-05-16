use std::collections::HashMap;

use leptos::either::Either;
use leptos::html;
use leptos::prelude::*;
use leptos_fluent::move_tr;
use leptos_router::components::{Form, Outlet, A};
use leptos_router::hooks::{use_location, use_params_map};
use leptos_use::{signal_debounced, signal_throttled_with_options, use_element_hover, ThrottleOptions};

use sphare_core_common::checks::check_sphere_name;
use sphare_core_common::constants::{MAX_MOD_MESSAGE_LENGTH, MAX_SPHERE_NAME_LENGTH, POST_BATCH_SIZE, SCROLL_LOAD_THROTTLE_DELAY};
use sphare_core_common::editor::TextareaData;
use sphare_core_common::routes::{get_create_post_path, get_satellite_path, get_sphere_name_memo, get_sphere_path, CREATE_POST_ROUTE, CREATE_POST_SPHERE_QUERY_PARAM, CREATE_POST_SUFFIX, PUBLISH_ROUTE, SEARCH_ROUTE};
use sphare_core_common::unpack::{handle_additional_load, reset_additional_load};
use sphare_core_content::filter::SphereCategoryFilter;
use sphare_core_content::post::{add_sphere_info_to_post_vec, PostWithSphereInfo};
use sphare_core_content::ranking::SortType;
use sphare_core_user::role::PermissionLevel;

use sphare_iface_content::post::get_post_vec_by_sphere_name;
use sphare_iface_sphere::sphere::{is_sphere_available, Subscribe, Unsubscribe};

use sphare_cmp_base::filter::PostFiltersButton;
use sphare_cmp_base::post::PostListWithInitLoad;
use sphare_cmp_base::ranking::PostSortWidget;
use sphare_cmp_common::auth_widget::{LoginGuardButton, LoginGuardedButton};
use sphare_cmp_common::role::AuthorizedShow;
use sphare_cmp_common::state::{GlobalState, SatelliteState, SphereState};
use sphare_cmp_utils::editor::{FormTextEditor, LengthLimitedInput};
use sphare_cmp_utils::errors::ErrorDisplay;
use sphare_cmp_utils::form::LabeledFormCheckbox;
use sphare_cmp_utils::icons::{LoadingIcon, MagnifierIcon, NsfwIcon, PlusIcon, ReturnIcon, SettingsIcon, SubscribedIcon};
use sphare_cmp_utils::unpack::{ActionError, SuspenseUnpack, TransitionUnpack};
use sphare_cmp_utils::widget::{BannerContent, RefreshButton};

use crate::satellite::ActiveSatelliteList;
use crate::sphere_category::get_sphere_category_header_map;
use crate::sphere_management::MANAGE_SPHERE_ROUTE;

/// Component to display a sphere's banner
#[component]
pub fn SphereBanner() -> impl IntoView {
    let state = expect_context::<GlobalState>();
    let sphere_name = get_sphere_name_memo(use_params_map());
    let sphere_state = SphereState::new(sphere_name, state);
    provide_context(sphere_state);

    let link_ref = NodeRef::<html::A>::new();
    let is_banner_hovered = use_element_hover(link_ref);
    let sphere_path = move || get_sphere_path(&sphere_name.get());
    let is_sphere_sub_page = move || {
        let path = use_location().pathname.read();
        path.matches("/").count() > 2
    };

    Effect::new(move || {
        sphere_name.read();
        sphere_state.sphere_category_filter.set(SphereCategoryFilter::All);
    });

    view! {
        <div class="flex flex-col flex-1 w-full overflow-y-auto pt-2 px-2 xl:px-4 gap-2 overflow-hidden">
            <TransitionUnpack resource=sphere_state.sphere_with_user_info_resource let:sphere_with_user_info>
            {
                view! {
                    <a
                        href=sphere_path()
                        class="relative flex-none rounded-sm w-full h-16 2xl:h-24 4xl:h-32 flex items-center justify-center max-w-full overflow-hidden"
                        node_ref=link_ref
                    >
                        <Show when=is_sphere_sub_page>
                            <SphereReturnIcon is_banner_hovered/>
                        </Show>
                        <BannerContent
                            title=sphere_with_user_info.sphere.sphere_name.clone()
                            icon_url=sphere_with_user_info.sphere.icon_url.clone()
                            banner_url=sphere_with_user_info.sphere.banner_url.clone()
                        />
                    </a>
                }.into_any()
            }
            </TransitionUnpack>
            <Outlet/>
        </div>
    }.into_any()
}

/// Icon to indicate clicking will return to the Sphere's main page
#[component]
fn SphereReturnIcon(
    is_banner_hovered: Signal<bool>,
) -> impl IntoView {
    view! {
        <div
            class="absolute lg:top-2 left-2 p-2 rounded-full backdrop-blur-sm bg-black/50"
            class=("bg-white/20", is_banner_hovered)
        >
            <ReturnIcon/>
        </div>
    }
}

/// Component to display a sphere's contents
#[component]
pub fn SphereContents() -> impl IntoView {
    let state = expect_context::<GlobalState>();
    let sphere_state = expect_context::<SphereState>();
    let sphere_name = expect_context::<SphereState>().sphere_name;
    let additional_load_count = RwSignal::new(0);
    let additional_post_vec = RwSignal::new(Vec::<PostWithSphereInfo>::new());
    let is_loading = RwSignal::new(false);
    let load_error = RwSignal::new(None);
    let list_ref = NodeRef::<html::Ul>::new();

    let is_category_map_loaded = RwSignal::new(false);
    let sphere_category_header_map = RwSignal::new(HashMap::new());

    let post_vec_resource = Resource::new(
        move || (
            sphere_name.get(),
            sphere_state.sphere_category_filter.get(),
            state.post_sort_type.get(),
            sphere_state.post_refresh_count.get(),
        ),
        move |(sphere_name, sphere_category_filter, sort_type, _)| async move {
            #[cfg(feature = "hydrate")]
            is_loading.set(true);
            sphere_category_header_map.set(get_sphere_category_header_map(sphere_state.sphere_categories_resource.clone().await));
            is_category_map_loaded.set(true);
            // TODO check no unnecessary loads
            reset_additional_load(additional_post_vec, additional_load_count, Some(list_ref));
            let result = get_post_vec_by_sphere_name(
                sphere_name.clone(),
                sphere_category_filter,
                sort_type,
                0,
            ).await.map(|post_vec| add_sphere_info_to_post_vec(
                post_vec,
                sphere_name,
                &*sphere_category_header_map.read_untracked(),
                None)
            );
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
                if !is_category_map_loaded.get_untracked() {
                    sphere_category_header_map.set(get_sphere_category_header_map(sphere_state.sphere_categories_resource.clone().await));
                    is_category_map_loaded.set(true);
                }
                let num_post = (POST_BATCH_SIZE as usize) + additional_post_vec.read_untracked().len();
                let additional_load = get_post_vec_by_sphere_name(
                    sphere_name.get_untracked(),
                    sphere_state.sphere_category_filter.get_untracked(),
                    state.post_sort_type.get_untracked(),
                    num_post
                ).await.map(|post_vec| add_sphere_info_to_post_vec(
                    post_vec,
                    sphere_name.get_untracked(),
                    &*sphere_category_header_map.read_untracked(),
                    None)
                );
                handle_additional_load(additional_load, additional_post_vec, load_error);
                is_loading.set(false);
            }
        }
    );

    view! {
        <ActiveSatelliteList/>
        <SuspenseUnpack resource=sphere_state.sphere_with_user_info_resource let:sphere>
            <SphereToolbar
                sphere_id=sphere.sphere.sphere_id
                sphere_name=sphere.sphere.sphere_name.clone()
                subscription_id=sphere.subscription_id
                sort_signal=state.post_sort_type
            />
        </SuspenseUnpack>
        <PostListWithInitLoad
            post_vec_resource
            additional_post_vec
            is_loading
            load_error
            additional_load_count
            list_ref
            show_sphere_header=false
        />
    }.into_any()
}

/// Component to display the sphere toolbar
#[component]
pub fn SphereToolbar(
    sphere_id: i64,
    sphere_name: String,
    subscription_id: Option<i64>,
    sort_signal: RwSignal<SortType>,
) -> impl IntoView {
    let state = expect_context::<GlobalState>();
    let sphere_state = expect_context::<SphereState>();
    let satellite_state = use_context::<SatelliteState>();
    let sphere_name = RwSignal::new(sphere_name.clone());
    let is_subscribed = RwSignal::new(subscription_id.is_some());
    let manage_path = move || get_sphere_path(&sphere_name.get()) + MANAGE_SPHERE_ROUTE;

    view! {
        <div class="flex w-full justify-between items-center">
            <div class="flex items-center w-fit gap-2">
                <PostSortWidget sort_signal/>
                <PostFiltersButton/>
            </div>
            <div class="flex items-center w-fit gap-1">
                <AuthorizedShow sphere_name permission_level=PermissionLevel::Moderate>
                    <A href=manage_path attr:class="button-rounded-ghost tooltip" attr:data-tip=move_tr!("manage")>
                        <SettingsIcon class="sphere-toolbar-icon-size"/>
                    </A>
                </AuthorizedShow>
                <RefreshButton refresh_count=sphere_state.post_refresh_count/>
                <SphereSearchButton/>
                <div class="tooltip flex" data-tip=move_tr!("new")>
                    <LoginGuardButton
                        login_button_class="button-rounded-ghost"
                        login_button_content=move || view! { <PlusIcon class="sphere-toolbar-icon-size"/> }.into_any()
                        redirect_path=get_create_post_path()
                        let:_user
                    >
                    { move || match satellite_state {
                        Some(satellite_state) => {
                            let create_post_link = move || {
                                get_satellite_path(
                                    sphere_state.sphere_name.into(),
                                    satellite_state.satellite_id.get()
                                ).get() + PUBLISH_ROUTE + CREATE_POST_SUFFIX
                            };
                            Either::Left(view! {
                                <a href=create_post_link class="button-rounded-ghost">
                                    <PlusIcon class="sphere-toolbar-icon-size"/>
                                </a>
                            })
                        }
                        None => Either::Right(view! {
                            <Form method="GET" action=CREATE_POST_ROUTE attr:class="flex">
                                <input type="text" name=CREATE_POST_SPHERE_QUERY_PARAM class="hidden" value=sphere_name/>
                                <button type="submit" class="button-rounded-ghost">
                                    <PlusIcon class="sphere-toolbar-icon-size"/>
                                </button>
                            </Form>
                        }),
                    }}
                    </LoginGuardButton>
                </div>
                <div class="tooltip" data-tip=move_tr!("join")>
                    <LoginGuardedButton
                        button_class="button-rounded-ghost"
                        button_action=move |_| {
                            is_subscribed.update(|value| {
                                *value = !*value;
                                if *value {
                                    state.subscribe_action.dispatch(Subscribe { sphere_id });
                                } else {
                                    state.unsubscribe_action.dispatch(Unsubscribe { sphere_id });
                                }
                            })
                        }
                    >
                        <SubscribedIcon class="sphere-toolbar-icon-size" show_color=is_subscribed/>
                    </LoginGuardedButton>
                </div>
            </div>
        </div>
    }.into_any()
}

/// Button to navigate to the search page of a sphere
#[component]
pub fn SphereSearchButton() -> impl IntoView
{
    let sphere_state = expect_context::<SphereState>();
    let route = move || format!("{}{}", get_sphere_path(sphere_state.sphere_name.read_untracked().as_str()), SEARCH_ROUTE);
    view! {
        <a href=route class="button-rounded-ghost tooltip" data-tip=move_tr!("search")>
            <MagnifierIcon class="sphere-toolbar-icon-size"/>
        </a>
    }
}

/// Component to create new spheres
#[component]
pub fn CreateSphere() -> impl IntoView {
    let state = expect_context::<GlobalState>();

    let sphere_name = RwSignal::new(String::new());
    let sphere_name_debounced: Signal<String> = signal_debounced(sphere_name, 250.0);
    let is_sphere_available = Resource::new(
        move || sphere_name_debounced.get(),
        move |sphere_name| async {
            if sphere_name.is_empty() || check_sphere_name(&sphere_name).is_err() {
                None
            } else {
                Some(is_sphere_available(sphere_name).await)
            }
        },
    );

    let is_name_taken = RwSignal::new(false);
    let textarea_ref = NodeRef::<html::Textarea>::new();
    let description_data = TextareaData {
        content: RwSignal::new(String::new()),
        textarea_ref,
    };
    let is_valid_sphere_name = move || check_sphere_name(&*sphere_name_debounced.read());
    let are_inputs_invalid = Memo::new(move |_| {
        is_valid_sphere_name().is_err()
            || is_name_taken.get()
            || description_data.content.read().is_empty()
    });

    view! {
        <div class="w-full 2xl:w-3/5 4xl:w-2/5 p-2 mx-auto flex flex-col gap-2 overflow-auto">
            <ActionForm action=state.create_sphere_action>
                <div class="flex flex-col gap-2 w-full">
                    <h2 class="py-4 text-4xl text-center">{move_tr!("create-sphere")}</h2>
                    <div class="h-full flex gap-2 items-center">
                        <LengthLimitedInput
                            name="sphere_name"
                            placeholder=move_tr!("name")
                            content=sphere_name
                            minlength=Some(1)
                            maxlength=Some(MAX_SPHERE_NAME_LENGTH)
                            class="flex-none w-3/5"
                        />
                        <Suspense fallback=move || view! { <LoadingIcon class="h-7 w-7"/> }>
                        {
                            move || match (sphere_name_debounced.read().is_empty(), is_valid_sphere_name(), is_sphere_available.get()) {
                                (true, _, _) => ().into_any(),
                                (_, Err(e), _) => view! {
                                    <div class="alert alert-error flex items-center">
                                        <span>{format!("{}", e.code)}</span>
                                    </div>
                                }.into_any(),
                                (_, _, Some(Some(Ok(false)))) => {
                                    is_name_taken.set(true);
                                    view! {
                                        <div class="alert alert-error flex items-center justify-center">
                                            <span class="font-semibold">{move_tr!("unavailable")}</span>
                                        </div>
                                    }.into_any()
                                },
                                (_, _, Some(Some(Err(e)))) => {
                                    log::error!("Error while checking sphere existence: {e}");
                                    is_name_taken.set(true);
                                    view! {
                                        <div class="alert alert-error h-fit py-2 flex items-center justify-center">
                                            <ErrorDisplay error=e.clone()/>
                                        </div>
                                    }.into_any()
                                },
                                _ => {
                                    is_name_taken.set(false);
                                    ().into_any()
                                }
                            }
                        }
                        </Suspense>
                    </div>
                    <FormTextEditor
                        name="description"
                        placeholder="Description"
                        data=description_data
                        maxlength=Some(MAX_MOD_MESSAGE_LENGTH)
                    />
                    <LabeledFormCheckbox name="is_nsfw" label=move_tr!("nsfw-content") label_icon_view=move || view! { <NsfwIcon/> }/>
                    <Suspense fallback=move || view! { <LoadingIcon/> }>
                        <button type="submit" class="button-secondary" disabled=are_inputs_invalid>{move_tr!("create")}</button>
                    </Suspense>
                </div>
            </ActionForm>
            <ActionError action=state.create_sphere_action.into()/>
        </div>
    }
}