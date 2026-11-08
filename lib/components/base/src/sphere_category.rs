use leptos::prelude::*;
use leptos_fluent::move_tr;

use sphare_core_common::common::SphereCategoryHeader;
use sphare_core_common::errors::AppError;
use sphare_core_sphere::sphere_category::SphereCategory;

use sphare_cmp_utils::unpack::TransitionUnpack;
use sphare_cmp_utils::widget::{RotatingArrow};

use crate::filter::SphereCategoryToggle;

/// Component to display a badge with sphere category's name
#[component]
pub fn SphereCategoryBadge(
    #[prop(into)]
    category_header: SphereCategoryHeader,
) -> impl IntoView {
    let class = format!(
        "flex items-center {} px-2 pt-1 pb-1.5 rounded-full text-xs lg:text-sm leading-none",
        category_header.category_color.to_bg_class()
    );
    view! {
        <div class=class>{category_header.category_name}</div>
    }
}

/// Dialog to select a sphere category
#[component]
pub fn SphereCategoryDropdown(
    category_vec_resource: Resource<Result<Vec<SphereCategory>, AppError>>,
    #[prop(optional)]
    init_category_vec: Vec<SphereCategoryHeader>,
    #[prop(default = true)]
    show_inactive: bool,
    #[prop(default = "")]
    name: &'static str,
) -> impl IntoView {
    let selected_category: RwSignal<Vec<SphereCategoryHeader>> = RwSignal::new(init_category_vec);

    view! {
        <TransitionUnpack resource=category_vec_resource let:sphere_category_vec>
        {
            if sphere_category_vec.is_empty() || (!show_inactive && !sphere_category_vec.iter().any(|sphere_category| sphere_category.is_active)) {
                log::debug!("No category to display.");
                return ().into_any()
            }
            let sphere_category_vec = StoredValue::new(sphere_category_vec.clone());
            view! {
                <div class="flex justify-between">
                    <span class="label text-white">{move_tr!("category")}</span>
                    <select
                        name=name
                        multiple
                        class="select_input"
                    >
                    {
                        sphere_category_vec.read_value().iter().map(|category| match show_inactive || category.is_active {
                            true => {
                                let category_id = category.category_id;
                                Some(view! {
                                    <option
                                        selected=move || selected_category.read().iter().find(|category_header| category_header.category_id == category_id).is_some()
                                        value={category_id}
                                    >
                                        <SphereCategoryBadge category_header=category/>
                                    </option>
                                })
                            },
                            false => None,
                        }.into_any()).collect_view()
                    }
                    </select>
                </div>
            }.into_any()
        }
        </TransitionUnpack>
    }
}

/// Dialog to select a sphere category
#[component]
fn NoSphereCategory() -> impl IntoView {
    view! {
        <span class="text-gray-400">{move_tr!("category-none")}</span>
    }
}

/// Collapse with a sphere category name as title, showing its description when opened and with an additional toggle to filter on this category
#[component]
pub fn SphereCategoryCollapseWithFilter(sphere_category: SphereCategory) -> impl IntoView {
    let category_id = sphere_category.category_id;
    let description = sphere_category.description.clone();
    let show_description = RwSignal::new(false);
    let collapse_class = move || match show_description.get() {
        true => "transition-all duration-500 overflow-hidden",
        false => "transition-all duration-500 overflow-hidden h-0",
    };
    let collapse_class_inner = move || match show_description.get() {
        true => "transition-all duration-500 opacity-100 visible",
        false => "transition-all duration-500 opacity-0 invisible",
    };

    view! {
        <div class="flex flex-col gap-1">
            <div class="flex justify-between items-center gap-2">
                <button
                    class="grow p-1 rounded-md flex justify-between items-center hover:bg-base-content/20"
                    on:click=move |_| show_description.update(|value| *value = !*value)
                >
                    <SphereCategoryBadge category_header=sphere_category/>
                    <RotatingArrow point_up=show_description/>
                </button>
                <SphereCategoryToggle category_id/>
            </div>
            <div class=collapse_class>
                <div class=collapse_class_inner>
                    <div class="pl-2 pb-2 text-sm">{description}</div>
                </div>
            </div>
        </div>
    }
}