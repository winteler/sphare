use leptos::html::{Div, Input};
use leptos::prelude::*;
use leptos_fluent::move_tr;

#[cfg(feature = "hydrate")]
use leptos_use::on_click_outside;

use sphare_core_content::filter::{on_change_all_categories_input, on_change_category_input, on_change_only_categories_input, SphereCategoryFilter};

use sphare_cmp_common::state::SphereState;
use sphare_cmp_utils::icons::FiltersIcon;
use sphare_cmp_utils::node_utils::set_checkbox;
use sphare_cmp_utils::unpack::SuspenseUnpack;
use sphare_cmp_utils::widget::Dropdown;

use crate::sphere_category::SphereCategoryBadge;

/// Button to open post filters modal window
#[component]
pub fn PostFiltersButton() -> impl IntoView {
    let show_dropdown = RwSignal::new(false);
    let dropdown_ref = NodeRef::<Div>::new();
    #[cfg(feature = "hydrate")]
    {
        // only enable with "hydrate" to avoid server side "Dropped SendWrapper" error
        let _ = on_click_outside(dropdown_ref, move |_| show_dropdown.set(false));
    }
    let button_class = move || match show_dropdown.get() {
        true => "button-primary",
        false => "button-ghost",
    };
    view! {
        <div class="h-full relative" node_ref=dropdown_ref>
            <div class="tooltip" data-tip=move_tr!("filters")>
                <button
                    class=button_class
                    on:click=move |_| show_dropdown.update(|value| *value = !*value)
                >
                    <FiltersIcon/>
                </button>
            </div>
            <Dropdown show_dropdown>
                <div class="bg-base-200 shadow-xl my-1 p-3 rounded-xs flex flex-col gap-3">
                    <div class="text-center font-bold text-2xl">{move_tr!("category-filters")}</div>
                    <SphereCategoryFilter/>
                </div>
            </Dropdown>
        </div>
    }
}

/// Button to open post filters modal window
#[component]
pub fn SphereCategoryFilter() -> impl IntoView {
    let sphere_state = expect_context::<SphereState>();
    view! {
        <div class="flex flex-col gap-1">
            <div class="text-center font-bold text-xl whitespace-nowrap">{move_tr!("sphere-categories")}</div>
            <SuspenseUnpack resource=sphere_state.sphere_categories_resource let:sphere_category_vec>
            {
                sphere_category_vec.iter().map(|sphere_category| {
                    let category_id = sphere_category.category_id;
                    view! {
                        <label class="cursor-pointer flex justify-between">
                            <span class="label">
                                <SphereCategoryBadge category_header=sphere_category/>
                            </span>
                            <SphereCategoryToggle category_id/>
                        </label>
                    }
                }).collect_view()
            }
            </SuspenseUnpack>
            <div class="w-full border-b border-0.5 border-base-content/20"/>
            <AllCategoriesToggle/>
            <OnlyCategoriesToggle/>
        </div>
    }
}

#[component]
pub fn SphereCategoryToggle(category_id: i64) -> impl IntoView {
    let sphere_state = expect_context::<SphereState>();
    let input_ref = NodeRef::<Input>::new();
    let is_filter_active = move || {
        let is_active = match &*sphere_state.sphere_category_filter.read() {
            SphereCategoryFilter::All => false,
            SphereCategoryFilter::CategorySet(category_set) => category_set.filters.contains(&category_id),
        };
        set_checkbox(is_active, input_ref);
        is_active
    };

    view! {
        <input
            type="checkbox"
            class="toggle toggle-secondary"
            checked=is_filter_active
            on:change=move |_| on_change_category_input(sphere_state.sphere_category_filter, category_id)
            node_ref=input_ref
        />
    }
}

#[component]
pub fn AllCategoriesToggle() -> impl IntoView {
    let sphere_state = expect_context::<SphereState>();
    let input_ref = NodeRef::<Input>::new();
    let is_filter_all_categories = move || {
        let is_active = sphere_state.sphere_category_filter.read() == SphereCategoryFilter::All;
        set_checkbox(is_active, input_ref);
        is_active
    };
    view! {
        <label class="cursor-pointer flex justify-between">
            <span class="label">{move_tr!("all")}</span>
            <input
                type="checkbox"
                class="toggle toggle-primary"
                checked=is_filter_all_categories
                on:change=move |_| on_change_all_categories_input(sphere_state.sphere_category_filter)
                node_ref=input_ref
            />
        </label>
    }
}

#[component]
pub fn OnlyCategoriesToggle() -> impl IntoView {
    let sphere_state = expect_context::<SphereState>();
    let input_ref = NodeRef::<Input>::new();
    let is_filter_only_categories = move || {
        let is_active = match &*sphere_state.sphere_category_filter.read() {
            SphereCategoryFilter::All => false,
            SphereCategoryFilter::CategorySet(category_set) => category_set.only_category,
        };
        set_checkbox(is_active, input_ref);
        is_active
    };
    view! {
        <label class="cursor-pointer flex justify-between">
            <span class="label">{move_tr!("only-categories")}</span>
            <input
                type="checkbox"
                class="toggle toggle-primary"
                checked=is_filter_only_categories
                on:change=move |_| on_change_only_categories_input(sphere_state.sphere_category_filter)
                node_ref=input_ref
            />
        </label>
    }
}