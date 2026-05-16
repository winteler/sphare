use leptos::html;
use leptos::prelude::*;
use leptos_fluent::move_tr;

use sphare_core_common::checks::check_string_length;
use sphare_core_common::constants::MAX_MOD_MESSAGE_LENGTH;
use sphare_core_common::editor::TextareaData;
use sphare_core_common::errors::AppError;
use sphare_core_common::unpack::handle_dialog_action_result;
use sphare_core_content::comment::Comment;
use sphare_core_content::moderation::{Content, ModerationInfo};
use sphare_core_sphere::rule::get_rule_title;
use sphare_core_user::role::PermissionLevel;

use sphare_iface_content::moderation::ModerateComment;
use sphare_iface_sphere::rule::get_rule_by_id;

use sphare_cmp_base::moderation::ModerationInfoDialog;
use sphare_cmp_common::role::AuthorizedShow;
use sphare_cmp_common::state::{GlobalState, SphereState};
use sphare_cmp_utils::editor::FormTextEditor;
use sphare_cmp_utils::icons::{HammerIcon, MagnifierIcon};
use sphare_cmp_utils::unpack::{ActionError, SuspenseUnpack, TransitionUnpack};
use sphare_cmp_utils::widget::{ModalDialog, ModalFormButtons};

/// Component to moderate a post
#[component]
pub fn ModerateButton(show_dialog: RwSignal<bool>) -> impl IntoView {
    let edit_button_class = move || match show_dialog.get() {
        true => "button-rounded-primary",
        false => "button-rounded-neutral",
    };
    view! {
        <button
            class=edit_button_class
            aria-expanded=move || show_dialog.get().to_string()
            aria-haspopup="dialog"
            on:click=move |_| show_dialog.set(true)
        >
            <HammerIcon/>
        </button>
    }.into_any()
}

/// Component to access a post's moderation dialog
#[component]
pub fn ModeratePostButton(post_id: i64) -> impl IntoView {
    let show_dialog = RwSignal::new(false);
    let sphere_name = expect_context::<SphereState>().sphere_name;
    view! {
        <AuthorizedShow sphere_name permission_level=PermissionLevel::Moderate>
            <div>
                <ModerateButton show_dialog/>
                <ModeratePostDialog
                    post_id
                    show_dialog
                />
            </div>
        </AuthorizedShow>
    }.into_any()
}

/// Component to access a comment's moderation dialog
#[component]
pub fn ModerateCommentButton(comment_id: i64, comment: RwSignal<Comment>) -> impl IntoView {
    let show_dialog = RwSignal::new(false);
    let sphere_name = expect_context::<SphereState>().sphere_name;
    view! {
        <AuthorizedShow sphere_name permission_level=PermissionLevel::Moderate>
            <div>
                <ModerateButton show_dialog/>
                <ModerateCommentDialog
                    comment_id
                    comment
                    show_dialog
                />
            </div>
        </AuthorizedShow>
    }.into_any()
}

/// Dialog to moderate a post
#[component]
pub fn ModeratePostDialog(
    post_id: i64,
    show_dialog: RwSignal<bool>
) -> impl IntoView {
    let state = expect_context::<GlobalState>();

    let textarea_ref = NodeRef::<html::Textarea>::new();
    let body_data = TextareaData {
        content: RwSignal::new(String::new()),
        textarea_ref,
    };
    let is_text_empty = Signal::derive(move || body_data.content.read().is_empty());

    view! {
        <ModalDialog
            class="w-full max-w-xl"
            show_dialog
        >
            <div class="bg-base-100 shadow-xl p-3 rounded-xs flex flex-col gap-3">
                <div class="text-center font-bold text-2xl">{move_tr!("moderate-post")}</div>
                <ActionForm action=state.moderate_post_action>
                    <div class="flex flex-col gap-3 w-full">
                        <input
                            type="text"
                            name="post_id"
                            class="hidden"
                            value=post_id
                        />
                        <RuleSelect name="rule_id"/>
                        <FormTextEditor
                            name="moderator_message"
                            placeholder={move_tr!("message")}
                            data=body_data
                            maxlength=Some(MAX_MOD_MESSAGE_LENGTH)
                        />
                        <BanMenu/>
                        <ModalFormButtons
                            disable_publish=is_text_empty
                            show_form=show_dialog
                        />
                    </div>
                </ActionForm>
                <ActionError action=state.moderate_post_action.into()/>
            </div>
        </ModalDialog>
    }.into_any()
}

/// Dialog to moderate a comment
#[component]
pub fn ModerateCommentDialog(
    comment_id: i64,
    comment: RwSignal<Comment>,
    show_dialog: RwSignal<bool>,
) -> impl IntoView {
    let textarea_ref = NodeRef::<html::Textarea>::new();
    let comment_data = TextareaData{
        content: RwSignal::new(String::new()),
        textarea_ref,
    };
    let is_form_invalid = Signal::derive(move || {
        check_string_length(&*comment_data.content.read(), "Moderator message", MAX_MOD_MESSAGE_LENGTH, false).is_err()
    });

    let moderate_comment_action = ServerAction::<ModerateComment>::new();

    let moderate_result = moderate_comment_action.value();

    Effect::new(move |_| handle_dialog_action_result(moderate_result.get(), comment, show_dialog));

    view! {
        <ModalDialog
            class="w-full max-w-xl"
            show_dialog
        >
            <div class="bg-base-100 shadow-xl p-3 rounded-xs flex flex-col gap-3">
                <div class="text-center font-bold text-2xl">{move_tr!("moderate-comment")}</div>
                <ActionForm action=moderate_comment_action>
                    <div class="flex flex-col gap-3 w-full">
                        <input
                            type="text"
                            name="comment_id"
                            class="hidden"
                            value=comment_id
                        />
                        <RuleSelect name="rule_id"/>
                        <FormTextEditor
                            name="moderator_message"
                            placeholder=move_tr!("message")
                            data=comment_data
                            maxlength=Some(MAX_MOD_MESSAGE_LENGTH)
                        />
                        <BanMenu/>
                        <ModalFormButtons
                            disable_publish=is_form_invalid
                            show_form=show_dialog
                        />
                    </div>
                </ActionForm>
                <ActionError action=moderate_comment_action.into()/>
            </div>
        </ModalDialog>
    }.into_any()
}

/// Dialog to select infringed rule
#[component]
pub fn RuleSelect(
    name: &'static str,
) -> impl IntoView {
    let sphere_state = expect_context::<SphereState>();
    view! {
        <div class="flex flex-col gap-1 w-full">
            <span class="text-xl font-semibold">{move_tr!("infringed-rule")}</span>
            <select
                class="select_input max-w-full"
                name=name
            >
                <TransitionUnpack resource=sphere_state.sphere_rules_resource let:rules_vec>
                {
                    rules_vec.iter().map(|rule| {
                        let title = get_rule_title(&rule.title, rule.sphere_id.is_some());
                        view! {
                            <option value=rule.rule_id>
                                {title}
                            </option>
                        }
                    }).collect_view()
                }
                </TransitionUnpack>
            </select>
        </div>
    }.into_any()
}

/// Dialog to input number of banned days
#[component]
pub fn BanMenu() -> impl IntoView {
    let ban_value = RwSignal::new(0);
    let is_permanent_ban = RwSignal::new(false);
    let sphere_name = expect_context::<SphereState>().sphere_name;
    view! {
        <input
            type="number"
            name="ban_duration_days"
            class="hidden"
            value=ban_value
            disabled=is_permanent_ban
        />
        <AuthorizedShow sphere_name permission_level=PermissionLevel::Ban>
            <div class="flex items-center justify-between w-full">
                <span class="text-xl font-semibold">{move_tr!("ban-duration")}</span>
                <select
                    class="select_input"
                    on:change=move |ev| {
                        let value = event_target_value(&ev);
                        if let Ok(num_days_banned) = value.parse::<i32>() {
                            ban_value.set(num_days_banned);
                            is_permanent_ban.set(false);
                        } else {
                            ban_value.set(0);
                            is_permanent_ban.set(true);
                        }
                    }
                >
                    <option>"0"</option>
                    <option>"1"</option>
                    <option>"7"</option>
                    <option>"30"</option>
                    <option>"180"</option>
                    <option>"365"</option>
                    <option value="">{move_tr!("permanent")}</option>
                </select>
            </div>
        </AuthorizedShow>
    }.into_any()
}

/// Displays the body of a moderated post or comment
#[component]
pub fn ModerationInfoButton(
    #[prop(into)]
    content: Signal<Content>,
) -> impl IntoView {
    let state = expect_context::<GlobalState>();
    let sphere_state = expect_context::<SphereState>();
    let show_button = move || {
        let (is_moderated, creator_id) = match &*content.read() {
            Content::Post(post) => (post.infringed_rule_id.is_some(), post.creator_id),
            Content::Comment(comment) => (comment.infringed_rule_id.is_some(), comment.creator_id),
        };
        let is_author = match &(*state.user.read()) {
            Some(Ok(Some(user))) => user.user_id == creator_id,
            _ => false
        };
        let is_moderator = *sphere_state.permission_level.read() >= PermissionLevel::Moderate;
        is_moderated && (is_author || is_moderator)
    };
    let show_dialog = RwSignal::new(false);
    let button_class = move || match show_dialog.get() {
        true => "button-rounded-primary",
        false => "button-rounded-neutral",
    };
    
    view! {
        <Suspense>
            <Show when=show_button>
                <button
                    class=button_class
                    on:click=move |_| show_dialog.update(|value| *value = !*value)
                >
                    <MagnifierIcon class="content-toolbar-icon-size"/>
                </button>
                <ModalDialog
                    class="w-full max-w-xl"
                    show_dialog
                >
                    <div class="bg-base-100 shadow-xl p-3 rounded-xs flex flex-col gap-3">
                        <ContentModerationInfo content=content/>
                        <button
                            type="button"
                            class="p-1 h-full rounded-xs bg-error hover:bg-error/75 active:scale-y-90 transition duration-250"
                            on:click=move |_| show_dialog.set(false)
                        >
                            {move_tr!("close")}
                        </button>
                    </div>
                </ModalDialog>
            </Show>
        </Suspense>
    }.into_any()
}

/// Component to display a button opening a modal dialog with a ban's details
#[component]
pub fn ContentModerationInfo(
    #[prop(into)]
    content: Signal<Content>,
) -> impl IntoView {
    let mod_info_resource = Resource::new(
        move || content.get(),
        move |content| async move {
            let rule_id = match &content {
                Content::Post(post) => post.infringed_rule_id,
                Content::Comment(comment) => comment.infringed_rule_id
            };
            match rule_id {
                Some(rule_id) => {
                    let rule = get_rule_by_id(rule_id).await?;
                    Ok(ModerationInfo {
                        content,
                        rule,
                    })
                },
                None => Err(AppError::new("Content is not moderated.")),
            }
        }
    );

    view! {
        <SuspenseUnpack resource=mod_info_resource let:moderation_info>
            <ModerationInfoDialog
                moderated_content=moderation_info.content.clone()
                rule_title=moderation_info.rule.title.clone()
                rule_description=moderation_info.rule.description.clone()
                is_sphere_rule=moderation_info.rule.sphere_id.is_some()
            />
        </SuspenseUnpack>
    }
}
