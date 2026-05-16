use leptos::form::ActionForm;
use leptos::html;
use leptos::prelude::*;
use leptos_fluent::move_tr;

use sphare_core_common::common::Rule;
use sphare_core_common::constants::{MAX_MOD_MESSAGE_LENGTH, MAX_TITLE_LENGTH};
use sphare_core_common::editor::TextareaData;
use sphare_core_user::role::PermissionLevel;

use sphare_cmp_common::role::AuthorizedShow;
use sphare_cmp_common::state::{GlobalState, SphereState};
use sphare_cmp_utils::editor::{FormMarkdownEditor, FormTextEditor};
use sphare_cmp_utils::icons::{CrossIcon, EditIcon, PlusIcon};
use sphare_cmp_utils::unpack::TransitionUnpack;
use sphare_cmp_utils::widget::{ContentBody, ModalDialog, ModalFormButtons};

/// Component to manage sphere rules
#[component]
pub fn SphereRulesPanel() -> impl IntoView {
    let sphere_state = expect_context::<SphereState>();
    view! {
        // TODO add overflow-y-auto max-h-full?
        <div class="shrink-0 flex flex-col gap-1 items-center w-full h-fit bg-base-200 p-2 rounded-sm">
            <div class="text-xl text-center">{move_tr!("rules")}</div>
            <div class="w-full flex flex-col">
                <div class="border-b border-base-content/20 pl-1 flex gap-1">
                    <div class="w-1/12 py-2 font-bold">{move_tr!("number-short")}</div>
                    <div class="w-4/12 py-2 font-bold">{move_tr!("title")}</div>
                    <div class="w-5/12 lg:w-1/2 py-2 font-bold">{move_tr!("description")}</div>
                </div>
                <TransitionUnpack resource=sphere_state.sphere_rules_resource let:sphere_rule_vec>
                {
                    let sphere_rule_vec = sphere_rule_vec.clone();
                    view! {
                        <For
                            each=move || sphere_rule_vec.clone().into_iter().filter(|rule| rule.sphere_id.is_some())
                            key=|rule| rule.rule_id
                            children=move |rule| {
                                let rule = StoredValue::new(rule);
                                let show_edit_form = RwSignal::new(false);
                                view! {
                                    <div class="flex gap-1 justify-start rounded-sm pl-1 pt-1">
                                        <div class="w-1/12 select-none text-sm">{rule.read_value().priority}</div>
                                        <div class="w-4/12 select-none text-sm">{rule.read_value().title.clone()}</div>
                                        <div class="w-5/12 lg:w-5/12 select-none text-sm">
                                            <ContentBody body=rule.read_value().description.clone() is_markdown=rule.read_value().markdown_description.is_some()/>
                                        </div>
                                        <div class="flex-grow"></div>
                                        <button
                                            class="button-secondary h-fit"
                                            on:click=move |_| show_edit_form.update(|value| *value = !*value)
                                        >
                                            <EditIcon/>
                                        </button>
                                        <DeleteRuleButton rule/>
                                    </div>
                                    <ModalDialog
                                        class="w-full max-w-xl"
                                        show_dialog=show_edit_form
                                    >
                                        <EditRuleForm rule show_form=show_edit_form/>
                                    </ModalDialog>
                                }
                            }
                        />
                    }
                }
                </TransitionUnpack>
            </div>
            <CreateRuleForm/>
        </div>
    }
}

/// Component to delete a sphere rule
#[component]
pub fn DeleteRuleButton(
    rule: StoredValue<Rule>
) -> impl IntoView {
    let state = expect_context::<GlobalState>();
    let sphere_state = expect_context::<SphereState>();
    let sphere_name = sphere_state.sphere_name;
    view! {
        <AuthorizedShow sphere_name permission_level=PermissionLevel::Manage>
            <ActionForm
                action=state.remove_rule_action
                attr:class="w-fit h-fit flex justify-center button-error"
            >
                <input
                    name="sphere_name"
                    class="hidden"
                    value=sphere_state.sphere_name
                />
                <input
                    name="priority"
                    class="hidden"
                    value=rule.with_value(|rule| rule.priority)
                />
                <button>
                    <CrossIcon/>
                </button>
            </ActionForm>
        </AuthorizedShow>
    }
}

/// Component to edit a sphere rule
#[component]
pub fn EditRuleForm(
    rule: StoredValue<Rule>,
    show_form: RwSignal<bool>,
) -> impl IntoView {
    let state = expect_context::<GlobalState>();
    let sphere_state = expect_context::<SphereState>();
    let (rule_priority, title, description, is_description_markdown)  = rule.with_value(|rule| (
        rule.priority,
        rule.title.clone(),
        match &rule.markdown_description {
            Some(description) => description.clone(),
            None => rule.description.clone(),
        },
        rule.markdown_description.is_some()
    ));
    let priority = RwSignal::new(rule_priority.to_string());
    let title_ref = NodeRef::<html::Textarea>::new();
    let title_data = TextareaData {
        content: RwSignal::new(title),
        textarea_ref: title_ref,
    };
    let description_ref = NodeRef::<html::Textarea>::new();
    let description_data = TextareaData {
        content: RwSignal::new(description),
        textarea_ref: description_ref,
    };
    let invalid_inputs = Signal::derive(move || is_invalid_rule_inputs(priority, title_data.content, description_data.content));

    view! {
        <div class="bg-base-100 shadow-xl p-3 rounded-xs flex flex-col gap-3">
            <div class="text-center font-bold text-2xl">"Edit a rule"</div>
            <ActionForm action=state.update_rule_action>
                <input
                    name="sphere_name"
                    class="hidden"
                    value=sphere_state.sphere_name
                />
                <input
                    name="current_priority"
                    class="hidden"
                    value=rule_priority
                />
                <div class="flex flex-col gap-3 w-full">
                    <RuleInputs priority title_data description_data is_description_markdown/>
                    <ModalFormButtons
                        disable_publish=invalid_inputs
                        show_form
                    />
                </div>
            </ActionForm>
        </div>
    }
}

/// Component to create a sphere rule
#[component]
pub fn CreateRuleForm() -> impl IntoView {
    let state = expect_context::<GlobalState>();
    let sphere_state = expect_context::<SphereState>();
    let show_dialog = RwSignal::new(false);
    let priority = RwSignal::new(String::default());
    let title_ref = NodeRef::<html::Textarea>::new();
    let title_data = TextareaData {
        content: RwSignal::new(String::new()),
        textarea_ref: title_ref,
    };
    let description_ref = NodeRef::<html::Textarea>::new();
    let description_data = TextareaData {
        content: RwSignal::new(String::new()),
        textarea_ref: description_ref,
    };
    let invalid_inputs = Signal::derive(move || is_invalid_rule_inputs(priority, title_data.content, description_data.content));

    view! {
        <button
            class="self-end button-secondary"
            on:click=move |_| show_dialog.update(|value| *value = !*value)
        >
            <PlusIcon/>
        </button>
        <ModalDialog
            class="w-full max-w-xl"
            show_dialog
        >
            <div class="bg-base-100 shadow-xl p-3 rounded-xs flex flex-col gap-3">
            <div class="text-center font-bold text-2xl">{move_tr!("add-rule")}</div>
                <ActionForm
                    action=state.add_rule_action
                    on:submit=move |_| show_dialog.set(false)
                >
                    <input
                        name="sphere_name"
                        class="hidden"
                        value=sphere_state.sphere_name
                    />
                    <div class="flex flex-col gap-3 w-full">
                        <RuleInputs priority title_data description_data/>
                        <ModalFormButtons
                            disable_publish=invalid_inputs
                            show_form=show_dialog
                        />
                    </div>
                </ActionForm>
            </div>
        </ModalDialog>
    }
}

/// Components with inputs to create or edit a rule
#[component]
pub fn RuleInputs(
    priority: RwSignal<String>,
    title_data: TextareaData,
    description_data: TextareaData,
    #[prop(optional)]
    is_description_markdown: bool,
) -> impl IntoView {
    view! {
        <div class="flex flex-col gap-2">
            <div class="flex gap-2 items-center">
                <input
                    tabindex="0"
                    type="number"
                    name="priority"
                    placeholder=move_tr!("number-short")
                    autocomplete="off"
                    class="input_primary no-spinner px-1 w-1/12"
                    class=("input_error", move || priority.read().is_empty())
                    bind:value=priority
                />
                <FormTextEditor
                    name="title"
                    placeholder=move_tr!("title")
                    data=title_data
                    maxlength=Some(MAX_TITLE_LENGTH as usize)
                />
            </div>
            <FormMarkdownEditor
                name="description"
                is_markdown_name="is_markdown"
                placeholder=move_tr!("description")
                data=description_data
                is_markdown=is_description_markdown
                maxlength=Some(MAX_MOD_MESSAGE_LENGTH)
            />
        </div>
    }
}

fn is_invalid_rule_inputs(
    priority: RwSignal<String>,
    title: RwSignal<String>,
    description: RwSignal<String>
) -> bool {
    priority.read().is_empty() ||
        title.with(|title| title.is_empty() || title.len() > MAX_TITLE_LENGTH as usize) ||
        description.with(|desc| desc.is_empty() || desc.len() > MAX_MOD_MESSAGE_LENGTH)
}

#[cfg(test)]
mod tests {
    use crate::rule::is_invalid_rule_inputs;
    use leptos::prelude::{Owner, RwSignal, Set};

    #[test]
    fn test_is_invalid_rule_inputs() {
        let owner = Owner::new();
        owner.set();

        let priority = RwSignal::new(String::from("1"));
        let title = RwSignal::new(String::from("title"));
        let description = RwSignal::new(String::from("description"));

        let is_invalid_rule = move || is_invalid_rule_inputs(priority, title, description);

        assert_eq!(is_invalid_rule(), false);

        priority.set(String::default());

        assert_eq!(is_invalid_rule(), true);

        priority.set(String::from("1"));
        title.set(String::default());

        assert_eq!(is_invalid_rule(), true);

        title.set(String::from("title"));
        description.set(String::default());

        assert_eq!(is_invalid_rule(), true);

        priority.set(String::default());
        title.set(String::default());
        description.set(String::default());

        assert_eq!(is_invalid_rule(), true);
    }
}
