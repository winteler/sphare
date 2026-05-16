use leptos::either::Either;
use leptos::html;
use leptos::prelude::*;
use leptos_fluent::move_tr;
use leptos_use::{signal_throttled_with_options, ThrottleOptions};

use sphare_core_common::constants::{MAX_SPHERE_NAME_LENGTH, SCROLL_LOAD_THROTTLE_DELAY};
use sphare_core_common::errors::AppError;
use sphare_core_common::unpack::{handle_additional_load, handle_initial_load};
use sphare_core_content::search::{is_content_search_valid, SearchState};

use sphare_iface_content::search::search_spheres;

use sphare_cmp_utils::editor::LengthLimitedInput;
use sphare_cmp_utils::errors::ErrorDetail;
use sphare_cmp_utils::form::LabeledSignalCheckbox;
use sphare_cmp_utils::widget::NotFoundWidget;
use crate::sphere::InfiniteSphereLinkList;

#[component]
pub fn SearchSpheres(
    search_state: SearchState,
    #[prop(default = "gap-4 w-3/4 lg:w-1/2")]
    class: &'static str,
    #[prop(default = "w-full")]
    form_class: &'static str,
    #[prop(default = true)]
    autofocus: bool,
    #[prop(optional)]
    is_dropdown_style: bool,
) -> impl IntoView
{
    let class = format!("flex flex-col self-center min-h-0 {class}");

    let sphere_header_vec = RwSignal::new(Vec::new());
    let additional_load_count = RwSignal::new(0);
    let is_loading = RwSignal::new(false);
    let load_error = RwSignal::new(None);
    let list_ref = NodeRef::<html::Ul>::new();
    let num_fetch_sphere = 50;
    let input_error = is_content_search_valid(search_state.search_input.into());

    let _initial_sphere_resource = LocalResource::new(
        move || async move {
            is_loading.set(true);
            let search_input = search_state.search_input_debounced.get();
            let initial_load = match search_input.is_empty() {
                true => Ok(Vec::new()),
                false => search_spheres(search_input, num_fetch_sphere, 0).await,
            };
            handle_initial_load(initial_load, sphere_header_vec, load_error, Some(list_ref));
            is_loading.set(false);
        }
    );

    let additional_load_count_throttled: Signal<i32> = signal_throttled_with_options(
        additional_load_count,
        SCROLL_LOAD_THROTTLE_DELAY,
        ThrottleOptions::default().leading(true).trailing(false)
    );

    let _additional_sphere_resource = LocalResource::new(
        move || async move {
            if additional_load_count_throttled.get() > 0 {
                is_loading.set(true);
                let sphere_count = sphere_header_vec.read_untracked().len();
                let search_input = search_state.search_input_debounced.get_untracked();
                let additional_load = search_spheres(search_input, num_fetch_sphere, sphere_count).await;
                handle_additional_load(additional_load, sphere_header_vec, load_error);
                is_loading.set(false);
            }
        }
    );

    let list_class = match is_dropdown_style {
        true => "rounded-sm h-fit bg-base-200",
        false => "rounded-sm min-h-0 h-fit",
    };

    view! {
        <div class=class>
            <SearchForm
                search_state=search_state.clone()
                show_spoiler_checkbox=false
                class=form_class
                autofocus
                maxlength=Some(MAX_SPHERE_NAME_LENGTH)
                input_error
            />
            { move || match (sphere_header_vec.read().is_empty(), search_state.search_input_debounced.get_untracked().is_empty()) {
                (true, true) => None,
                (true, false) => Some(Either::Left(view! { <NotFoundWidget is_main_content=!is_dropdown_style message=move_tr!("search-no-sphere-found")/> })),
                (false, _) => Some(Either::Right(
                    view! {
                        <div class=list_class>
                            <InfiniteSphereLinkList
                                sphere_header_vec
                                is_loading
                                load_error
                                additional_load_count
                                is_dropdown_style
                                list_ref
                            />
                        </div>
                    }
                ))
            }}

        </div>
    }
}

/// Form for the search dialog
#[component]
pub fn SearchForm(
    search_state: SearchState,
    show_spoiler_checkbox: bool,
    #[prop(default = "w-3/4 lg:w-1/2")]
    class: &'static str,
    #[prop(default = true)]
    autofocus: bool,
    /// Optional maximum search text length
    #[prop(default = None)]
    maxlength: Option<usize>,
    /// Error message signal
    #[prop(default = Signal::derive(|| None))]
    input_error: Signal<Option<AppError>>,
) -> impl IntoView {
    if let Some(maxlength) = maxlength {
        if search_state.search_input.read_untracked().len() > maxlength {
            search_state.search_input.write().clear();
        }
    };
    let textarea_ref = NodeRef::<html::Textarea>::new();
    if autofocus {
        Effect::new(move || if let Some(textarea_ref) = textarea_ref.get() {
            textarea_ref.focus().ok();
        });
    }
    let class = format!("flex flex-col gap-2 self-center {class}");
    view! {
        <div class=class>
            <LengthLimitedInput
                placeholder=move_tr!("search")
                content=search_state.search_input
                maxlength=maxlength
                textarea_ref
            />
            { match show_spoiler_checkbox {
                true => Some(view! {
                    <LabeledSignalCheckbox label=move_tr!("spoiler") value=search_state.show_spoiler class="pl-1"/>
                }),
                false => None,
            }}
            { move || {
                input_error.read().as_ref().map(|error| view! {
                    <ErrorDetail error=error.clone()/>
                })
            }}
        </div>
    }
}