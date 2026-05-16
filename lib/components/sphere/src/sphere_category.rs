use std::collections::HashMap;

use leptos::html;
use leptos::prelude::*;
use leptos_fluent::move_tr;

use sphare_core_common::colors::Color;
use sphare_core_common::common::SphereCategoryHeader;
use sphare_core_common::constants::{MAX_CATEGORY_DESCRIPTION_LENGTH, MAX_CATEGORY_NAME_LENGTH};
use sphare_core_common::editor::{adjust_textarea_height, TextareaData};
use sphare_core_common::errors::AppError;
use sphare_core_sphere::sphere_category::SphereCategory;
use sphare_core_user::role::PermissionLevel;

use sphare_cmp_common::role::AuthorizedShow;
use sphare_cmp_common::state::{GlobalState, SphereState};
use sphare_cmp_utils::colors::{ColorIndicator, ColorSelect};
use sphare_cmp_utils::editor::{FormTextEditor, LengthLimitedInput};
use sphare_cmp_utils::form::FormCheckbox;
use sphare_cmp_utils::icons::{CrossIcon, PauseIcon, PlayIcon, SaveIcon};
use sphare_cmp_utils::unpack::TransitionUnpack;

/// Component to manage sphere categories
#[component]
pub fn SphereCategoriesDialog() -> impl IntoView {
    let sphere_state = expect_context::<SphereState>();
    let sphere_name = sphere_state.sphere_name;

    let category_input = RwSignal::new(String::new());
    let color_input = RwSignal::new(Color::None);
    let activated_input = RwSignal::new(true);
    let name_textarea_ref = NodeRef::<html::Textarea>::new();
    let textarea_ref = NodeRef::<html::Textarea>::new();
    let description_data = TextareaData {
        content: RwSignal::new(String::new()),
        textarea_ref
    };
    view! {
        <AuthorizedShow sphere_name permission_level=PermissionLevel::Manage>
            // TODO add overflow-y-auto max-h-full?
            <div class="flex flex-col gap-1 items-center w-full h-fit bg-base-200 p-2 rounded-sm">
                <div class="text-xl text-center">{move_tr!("sphere-categories")}</div>
                <div class="w-full flex flex-col">
                    <div class="border-b border-base-content/20">
                        <div class="w-19/20 flex items-center gap-1">
                            <div class="w-3/10 lg:w-1/3 px-1 py-2 font-bold">{move_tr!("category")}</div>
                            <div class="w-12 lg:w-20 py-2 font-bold text-center">
                                <div class="max-lg:hidden">{move_tr!("color")}</div>
                            </div>
                            <div class="w-1/3 lg:w-2/5 px-1 py-2 font-bold">{move_tr!("description")}</div>
                            <div class="w-16 py-2 font-bold lg:text-center max-lg:hidden">
                                <div>{move_tr!("active")}</div>
                            </div>
                        </div>
                    </div>
                    <div class="flex flex-col gap-1 pt-1">
                        <TransitionUnpack resource=sphere_state.sphere_categories_resource let:sphere_category_vec>
                        {
                            sphere_category_vec.iter().map(|sphere_category| {
                                let category_name = sphere_category.category_name.clone();
                                let color = sphere_category.category_color;
                                let description = sphere_category.description.clone();
                                let is_active = sphere_category.is_active;
                                view! {
                                    <div
                                        class="flex justify-between items-center"
                                    >
                                        <div
                                            class="w-19/20 flex items-center gap-1 py-1 rounded-sm hover:bg-base-content/20 active:scale-y-90 transition duration-250"
                                            on:click=move |_| {
                                                category_input.set(category_name.clone());
                                                color_input.set(color);
                                                description_data.content.set(description.clone());
                                                if let Some(name_textarea_elem) = name_textarea_ref.get() {
                                                    name_textarea_elem.set_value(&category_name);
                                                }
                                                if let Some(textarea_elem) = textarea_ref.get() {
                                                    textarea_elem.set_value(&description);
                                                    adjust_textarea_height(textarea_ref);
                                                }
                                                activated_input.set(is_active);
                                            }
                                        >
                                            <div class="w-3/10 lg:w-1/3 px-2 py-1 select-none text-sm">{category_name.clone()}</div>
                                            <div class="w-12 lg:w-20 flex justify-center"><ColorIndicator color/></div>
                                            <div class="w-1/3 lg:w-2/5 px-2 select-none whitespace-pre-wrap text-sm">{description.clone()}</div>
                                            <div class="w-7 lg:w-16 flex justify-center">
                                            {
                                                match is_active {
                                                    true => view! { <div class="bg-secondary rounded-full p-1 lg:p-1.5"><PlayIcon/></div> }.into_any(),
                                                    false => view! { <PauseIcon/> }.into_any(),
                                                }
                                            }
                                            </div>
                                        </div>
                                        <DeleteCategoryButton category_name=sphere_category.category_name.clone()/>
                                    </div>
                                }
                            }).collect_view()
                        }
                        </TransitionUnpack>
                    </div>
                    <SetCategoryForm category_input color_input activated_input description_data name_textarea_ref/>
                </div>
            </div>
        </AuthorizedShow>
    }
}

/// Component to set permission levels for a sphere
#[component]
pub fn SetCategoryForm(
    category_input: RwSignal<String>,
    color_input: RwSignal<Color>,
    activated_input: RwSignal<bool>,
    description_data: TextareaData,
    name_textarea_ref: NodeRef<html::Textarea>,
) -> impl IntoView {
    let state = expect_context::<GlobalState>();
    let sphere_state = expect_context::<SphereState>();
    let sphere_name = sphere_state.sphere_name;
    let disable_submit = move || category_input.read().is_empty() || description_data.content.read().is_empty();

    view! {
        <AuthorizedShow sphere_name permission_level=PermissionLevel::Manage>
            <ActionForm action=state.set_sphere_category_action>
                <input
                    name="sphere_name"
                    class="hidden"
                    value=sphere_name
                />
                <div class="w-full py-1 flex justify-between">
                    <div class="w-19/20 flex items-center gap-1">
                        <LengthLimitedInput
                            name="category_name"
                            placeholder=move_tr!("category")
                            content=category_input
                            class="w-3/10 lg:w-1/3 text-sm"
                            minlength=Some(1)
                            maxlength=Some(MAX_CATEGORY_NAME_LENGTH)
                            textarea_ref=name_textarea_ref
                        />
                        <ColorSelect name="category_color" color_input class="h-full w-12 lg:w-20 flex justify-center"/>
                        <FormTextEditor
                            name="description"
                            placeholder=move_tr!("description")
                            data=description_data
                            class="w-1/3 lg:w-2/5"
                            maxlength=Some(MAX_CATEGORY_DESCRIPTION_LENGTH)
                        />
                        <FormCheckbox
                            name="is_active"
                            is_checked=activated_input
                            class="w-7 lg:w-16 flex justify-center"
                            checkbox_class="checkbox checkbox-primary max-lg:checkbox-sm"
                        />
                    </div>
                    <button
                        type="submit"
                        disabled=disable_submit
                        class="button-secondary self-center"
                    >
                        <SaveIcon/>
                    </button>
                </div>
            </ActionForm>
        </AuthorizedShow>
    }
}

/// Component to delete a sphere category
#[component]
pub fn DeleteCategoryButton(
    category_name: String,
) -> impl IntoView {
    let state = expect_context::<GlobalState>();
    let sphere_state = expect_context::<SphereState>();
    let sphere_name = sphere_state.sphere_name;
    let category_name = StoredValue::new(category_name);
    view! {
        <AuthorizedShow sphere_name permission_level=PermissionLevel::Manage>
            <ActionForm
                action=state.delete_sphere_category_action
                attr:class="h-fit flex justify-center"
            >
                <input
                    name="sphere_name"
                    class="hidden"
                    value=sphere_state.sphere_name
                />
                <input
                    name="category_name"
                    class="hidden"
                    value=category_name.get_value()
                />
                <button class="button-error">
                    <CrossIcon/>
                </button>
            </ActionForm>
        </AuthorizedShow>
    }
}

pub fn get_sphere_category_header_map(
    sphere_category_load: Result<Vec<SphereCategory>, AppError>
) -> HashMap<i64, SphereCategoryHeader> {
    let mut sphere_category_map = HashMap::<i64, SphereCategoryHeader>::new();
    if let Ok(sphere_category_vec) = sphere_category_load {
        for sphere_category in sphere_category_vec {
            sphere_category_map.insert(sphere_category.category_id, sphere_category.clone().into());
        }
    }
    sphere_category_map
}

#[cfg(test)]
mod tests {
    use leptos::prelude::ServerFnErrorErr;
    use sphare_core_common::colors::Color;
    use sphare_core_common::errors::AppError;

    use crate::sphere_category::{get_sphere_category_header_map, SphereCategory};

    #[test]
    fn test_get_sphere_category_header_map() {
        let category_1 = SphereCategory {
            category_id: 0,
            sphere_id: 0,
            category_name: "a".to_string(),
            category_color: Color::None,
            description: "".to_string(),
            is_active: false,
            creator_id: 0,
            timestamp: Default::default(),
            delete_timestamp: None,
        };
        let category_2 = SphereCategory {
            category_id: 1,
            sphere_id: 1,
            category_name: "b".to_string(),
            category_color: Color::None,
            description: "".to_string(),
            is_active: false,
            creator_id: 0,
            timestamp: Default::default(),
            delete_timestamp: None,
        };
        let sphere_category_vec = vec![
            category_1.clone(),
            category_2.clone(),
        ];
        
        let category_map = get_sphere_category_header_map(Ok(sphere_category_vec));
        
        assert_eq!(category_map.len(), 2);
        
        assert_eq!(category_map.get(&category_1.category_id), Some(&category_1.into()));
        assert_eq!(category_map.get(&category_2.category_id), Some(&category_2.into()));
        
        let empty_category_map = get_sphere_category_header_map(Err(AppError::CommunicationError(ServerFnErrorErr::Request(String::from("test")))));
        
        assert_eq!(empty_category_map.len(), 0);
    }
}