use leptos::either::Either;
use leptos::prelude::*;
use leptos_fluent::move_tr;

use sphare_core_common::common::SphereCategoryHeader;
use sphare_core_common::errors::AppError;
use sphare_core_sphere::sphere_category::SphereCategory;

use sphare_cmp_utils::unpack::TransitionUnpack;
use sphare_cmp_utils::widget::{DropdownButton, RotatingArrow};

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
    #[prop(default = None)]
    init_category_id: Option<i64>,
    #[prop(default = true)]
    show_inactive: bool,
    #[prop(default = "")]
    name: &'static str,
) -> impl IntoView {
    let selected_category: RwSignal<Option<SphereCategory>> = RwSignal::new(None);
    let show_dropdown = RwSignal::new(false);

    view! {
        <TransitionUnpack resource=category_vec_resource let:sphere_category_vec>
        {
            if sphere_category_vec.is_empty() || (!show_inactive && !sphere_category_vec.iter().any(|sphere_category| sphere_category.is_active)) {
                log::debug!("No category to display.");
                return ().into_any()
            }
            if let Some(init_category_id) = init_category_id &&
                let Some(category) = sphere_category_vec.iter().find(|category| category.category_id == init_category_id) {
                selected_category.set(Some(category.clone()));
            }
            let sphere_category_vec = StoredValue::new(sphere_category_vec.clone());
            view! {
                <input
                    name=name
                    value=move || match &*selected_category.read() {
                        Some(category) => Some(category.category_id),
                        None => None,
                    }
                    class="hidden"
                />
                <div class="flex justify-between">
                    <span class="label text-white">{move_tr!("category")}</span>
                    <DropdownButton
                        button_class="input_primary flex justify-between items-center w-fit gap-2"
                        activated_button_class="input_primary flex justify-between items-center w-fit gap-2"
                        button_content=move || view! {
                            { move || match &*selected_category.read() {
                                Some(category) => Either::Left(view! {
                                    <SphereCategoryBadge category_header=category.clone()/>
                                }),
                                None => Either::Right(view! {
                                    <NoSphereCategory/>
                                })
                            }}
                            <RotatingArrow point_up=show_dropdown/>
                        }
                        align_right=true
                        open_down=false
                        show_dropdown
                    >
                        <ul class="mb-2 p-2 shadow-sm bg-base-200 rounded-sm flex flex-col gap-1">
                            <li>
                                <button
                                    type="button"
                                    class="button-ghost w-full"
                                    on:click=move |_| {
                                        selected_category.set(None);
                                        show_dropdown.set(false);
                                    }
                                >
                                    <NoSphereCategory/>
                                </button>
                            </li>
                            {
                                sphere_category_vec.read_value().iter().map(|sphere_category| {
                                    let category = StoredValue::new(sphere_category.clone());
                                    match show_inactive || sphere_category.is_active {
                                        true => Some(view! {
                                            <li>
                                                <button
                                                    type="button"
                                                    class="button-ghost"
                                                    on:click=move |_| {
                                                        selected_category.set(Some(category.get_value()));
                                                        show_dropdown.set(false);
                                                    }
                                                >
                                                    <SphereCategoryBadge category_header=sphere_category/>
                                                </button>
                                            </li>
                                        }),
                                        false => None,
                                    }
                                }).collect_view()
                            }
                        </ul>
                    </DropdownButton>
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