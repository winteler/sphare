use leptos::ev::{Event, SubmitEvent};
use leptos::html;
use leptos::prelude::*;
use leptos::wasm_bindgen::closure::Closure;
use leptos::wasm_bindgen::JsCast;
use leptos::web_sys::{FileReader, FormData, HtmlFormElement, HtmlInputElement};
use leptos_fluent::move_tr;
use leptos_router::components::Outlet;
use leptos_use::signal_debounced;
use strum::IntoEnumIterator;

use sphare_core_common::checks::check_username;
use sphare_core_common::constants::{MAX_SPHERE_DESCRIPTION_LENGTH, MAX_USERNAME_LENGTH};
use sphare_core_common::editor::TextareaData;
use sphare_core_common::errors::AppError;
use sphare_core_common::traits::ToLocalizedStr;
use sphare_core_user::role::PermissionLevel;

use sphare_iface_content::moderation::get_moderation_info;
use sphare_iface_sphere::sphere_management::{get_sphere_ban_vec, set_sphere_banner, set_sphere_icon, RemoveUserBan};
use sphare_iface_user::role::SetUserSphereRole;
use sphare_iface_user::user::get_matching_user_header_vec;

use sphare_cmp_base::moderation::ModerationInfoDialog;
use sphare_cmp_common::auth_widget::LoginWindow;
use sphare_cmp_common::role::AuthorizedShow;
use sphare_cmp_common::state::{GlobalState, SphereState};
use sphare_cmp_utils::editor::{FormTextEditor, LengthLimitedInput};
use sphare_cmp_utils::errors::ErrorDisplay;
use sphare_cmp_utils::icons::{CrossIcon, LoadingIcon, MagnifierIcon, SaveIcon};
use sphare_cmp_utils::unpack::{SuspenseUnpack, TransitionUnpack};
use sphare_cmp_utils::widget::{LocalizedEnumDropdown, ModalDialog, IMAGE_FILE_PARAM, SPHERE_NAME_PARAM};

use crate::rule::SphereRulesPanel;
use crate::satellite::SatellitePanel;
use crate::sphere_category::SphereCategoriesDialog;

pub const MANAGE_SPHERE_ROUTE: &str = "/manage";
pub const NONE_STR: &str = "None";
pub const DAY_STR: &str = "day";
pub const DAYS_STR: &str = "days";
pub const PERMANENT_STR: &str = "Permanent";
pub const MISSING_SPHERE_STR: &str = "Missing sphere name.";
pub const MISSING_BANNER_FILE_STR: &str = "Missing banner file.";

/// Component to guard the sphere cockpit
#[component]
pub fn SphereCockpitGuard() -> impl IntoView {
    let state = expect_context::<GlobalState>();
    let sphere_name = expect_context::<SphereState>().sphere_name;
    view! {
        <SuspenseUnpack resource=state.user let:user>
        {
            match user {
                Some(user) => {
                    match user.check_sphere_permissions_by_name(&sphere_name.read_untracked(), PermissionLevel::Moderate) {
                        Ok(_) => view! { <Outlet/> }.into_any(),
                        Err(error) => view! { <ErrorDisplay error/> }.into_any(),
                    }
                },
                None => view! { <LoginWindow/> }.into_any(),
            }
        }
        </SuspenseUnpack>
    }.into_any()
}

/// Component to manage a sphere
#[component]
pub fn SphereCockpit() -> impl IntoView {
    view! {
        <div class="flex flex-col gap-5 overflow-y-auto w-full 2xl:w-4/5 4xl:w-2/3 mx-auto pb-5">
            <div class="text-2xl text-center">{move_tr!("sphere-cockpit")}</div>
            <SphereDescriptionDialog/>
            <SphereIconDialog/>
            <SphereBannerDialog/>
            <SatellitePanel/>
            <SphereCategoriesDialog/>
            <ModeratorPanel/>
            <SphereRulesPanel/>
            <BanPanel/>
        </div>
    }
}

/// Component to edit a sphere's description
#[component]
pub fn SphereDescriptionDialog() -> impl IntoView {
    let sphere_state = expect_context::<SphereState>();
    let sphere_name = expect_context::<SphereState>().sphere_name;
    view! {
        <AuthorizedShow sphere_name permission_level=PermissionLevel::Manage>
            // TODO add overflow-y-auto max-h-full?
            <div class="flex flex-col gap-1 items-center w-full h-fit bg-base-200 p-2 rounded-sm">
                <div class="text-xl text-center">{move_tr!("sphere-description")}</div>
                <SuspenseUnpack resource=sphere_state.sphere_with_user_info_resource let:sphere_with_user_info>
                    <SphereDescriptionForm sphere_description=sphere_with_user_info.sphere.description.clone()/>
                </SuspenseUnpack>
            </div>
        </AuthorizedShow>
    }
}

/// Form to edit a sphere's description
#[component]
pub fn SphereDescriptionForm(
    sphere_description: String,
) -> impl IntoView {
    let state = expect_context::<GlobalState>();
    let sphere_state = expect_context::<SphereState>();
    let textarea_ref = NodeRef::<html::Textarea>::new();
    let description_data = TextareaData {
        content: RwSignal::new(sphere_description.clone()),
        textarea_ref
    };
    let disable_submit = move || description_data.content.read().is_empty();
    view! {
        <ActionForm
            action=state.update_sphere_desc_action
            attr:class="w-full flex flex-col gap-1"
        >
            <input
                name="sphere_name"
                class="hidden"
                value=sphere_state.sphere_name
            />
            <FormTextEditor
                name="description"
                placeholder=move_tr!("description")
                data=description_data
                maxlength=Some(MAX_SPHERE_DESCRIPTION_LENGTH)
            />
            <button
                type="submit"
                class="button-secondary self-end"
                disabled=disable_submit
            >
                <SaveIcon/>
            </button>
        </ActionForm>
    }
}

/// Component to edit a sphere's icon
#[component]
pub fn SphereIconDialog() -> impl IntoView {
    let sphere_state = expect_context::<SphereState>();
    let sphere_name = sphere_state.sphere_name;
    let set_icon_action = Action::new_local(|data: &FormData| {
        set_sphere_icon(data.clone().into())
    });

    view! {
        <AuthorizedShow sphere_name permission_level=PermissionLevel::Manage>
            // TODO add overflow-y-auto max-h-full?
            <div class="flex flex-col gap-1 items-center w-full h-fit bg-base-200 p-2 rounded-sm">
                <div class="text-xl text-center">{move_tr!("sphere-icon")}</div>
                <SphereImageForm
                    sphere_name=sphere_state.sphere_name
                    action=set_icon_action
                    preview_class="max-h-12 max-w-full object-contain"
                />
            </div>
        </AuthorizedShow>
    }
}

/// Component to edit a sphere's banner
#[component]
pub fn SphereBannerDialog() -> impl IntoView {
    let sphere_state = expect_context::<SphereState>();
    let sphere_name = sphere_state.sphere_name;
    let set_banner_action = Action::new_local(|data: &FormData| {
        set_sphere_banner(data.clone().into())
    });
    view! {
        <AuthorizedShow sphere_name permission_level=PermissionLevel::Manage>
            // TODO add overflow-y-auto max-h-full?
            <div class="flex flex-col gap-1 items-center w-full h-fit bg-base-200 p-2 rounded-sm">
                <div class="text-xl text-center">{move_tr!("sphere-banner")}</div>
                <SphereImageForm
                    sphere_name=sphere_state.sphere_name
                    action=set_banner_action
                />
            </div>
        </AuthorizedShow>
    }
}

/// Form to upload an image to the server
/// The form contains two inputs: a hidden sphere name and an image form
#[component]
pub fn SphereImageForm(
    #[prop(into)]
    sphere_name: Signal<String>,
    action: Action<FormData, Result<(), AppError>>,
    #[prop(default = "max-h-80 max-w-full object-contain")]
    preview_class: &'static str,
) -> impl IntoView {
    let on_submit = move |ev: SubmitEvent| {
        ev.prevent_default();
        if cfg!(feature = "hydrate") {
            let target = ev.target().unwrap().unchecked_into::<HtmlFormElement>();
            let form_data = FormData::new_with_form(&target).unwrap();
            action.dispatch_local(form_data);
        }
    };

    let preview_url = RwSignal::new(String::new());
    let on_file_change = move |ev| {
        let input: HtmlInputElement = event_target::<HtmlInputElement>(&ev);
        if let Some(files) = input.files() && let Some(file) = files.get(0) {
            // Try to create a FileReader, returning early if it fails
            let reader = match FileReader::new() {
                Ok(reader) => reader,
                Err(_) => {
                    log::error!("Failed to create file reader.");
                    return
                }, // Return early if FileReader creation fails
            };

            // Set up the onload callback for FileReader
            let preview_url_clone = preview_url.clone();
            let onload_callback = Closure::wrap(Box::new(move |e: Event| {
                if let Some(reader) = e.target().and_then(|t| t.dyn_into::<FileReader>().ok()) {
                    if let Ok(Some(result)) = reader.result().and_then(|r| Ok(r.as_string())) {
                        preview_url_clone.set(result); // Update the preview URL
                    }
                }
            }) as Box<dyn FnMut(_)>);

            reader.set_onload(Some(onload_callback.as_ref().unchecked_ref()));
            onload_callback.forget(); // Prevent the closure from being dropped

            // Start reading the file as a Data URL, returning early if it fails
            if let Err(e) = reader.read_as_data_url(&file) {
                let error_message = e.as_string().unwrap_or_else(|| format!("{:?}", e));
                log::error!("Error while getting preview of local image: {error_message}");
            };
        }
    };

    view! {
        <form on:submit=on_submit class="w-full flex flex-col gap-1">
            <input
                name=SPHERE_NAME_PARAM
                class="hidden"
                value=sphere_name
            />
            <input
                type="file"
                name=IMAGE_FILE_PARAM
                accept="image/*"
                class="file-input file-input-primary !outline-offset-0 w-full"
                on:change=on_file_change
            />
            <Show when=move || !preview_url.read().is_empty()>
                <img src=preview_url alt=move_tr!("image-preview") class=preview_class/>
            </Show>
            <button
                type="submit"
                class="button-secondary self-end"
            >
                <SaveIcon/>
            </button>
            {move || {
                if cfg!(feature = "hydrate") {
                    if action.pending().get()
                    {
                        view! { <LoadingIcon/> }.into_any()
                    } else {
                        match action.value().get()
                        {
                            Some(Ok(())) => {
                                if let Some(state) = use_context::<GlobalState>() {
                                    state.sphere_reload_signal.update(|value| *value += 1);
                                }
                                ().into_any()
                            }
                            Some(Err(e)) => view! { <ErrorDisplay error=e.into()/> }.into_any(),
                            None => ().into_any()
                        }
                    }
                } else {
                    ().into_any()
                }
            }}
        </form>
    }
}

/// Component to manage moderators
#[component]
pub fn ModeratorPanel() -> impl IntoView {
    let state = expect_context::<GlobalState>();
    let sphere_state = expect_context::<SphereState>();
    let sphere_name = sphere_state.sphere_name;
    let username_input = RwSignal::new(String::default());
    let select_ref = NodeRef::<html::Select>::new();

    view! {
        // TODO add overflow-y-auto max-h-full?
        <div class="flex flex-col gap-1 items-center w-full h-fit bg-base-200 p-2 rounded-sm">
            <div class="text-xl text-center">{move_tr!("moderators")}</div>
            <div class="w-full flex flex-col gap-1">
                <div class="flex gap-1 border-b border-base-content/20">
                    <div class="w-2/5 lg:w-1/2 p-2 text-left font-bold">{move_tr!("username")}</div>
                    <div class="w-1/4 p-2 text-left font-bold">{move_tr!("role")}</div>
                </div>
                <TransitionUnpack resource=sphere_state.sphere_roles_resource let:sphere_role_vec>
                {
                    sphere_role_vec.iter().map(|role| {
                        let username = role.username.clone();
                        let role_index = role.permission_level as i32;
                        view! {
                            <div
                                class="flex gap-1 py-1 rounded-sm hover:bg-base-content/20 active:scale-y-90 transition duration-250"
                                on:click=move |_| {
                                    username_input.set(username.clone());
                                    match select_ref.get_untracked() {
                                        Some(select_ref) => select_ref.set_selected_index(role_index),
                                        None => log::error!("Form permission level select failed to load."),
                                    };
                                }
                            >
                                <div class="w-2/5 lg:w-1/2 px-2 text-sm select-none">{role.username.clone()}</div>
                                <div class="w-1/4 px-2 text-sm select-none">{role.permission_level.to_localized_str()}</div>
                            </div>
                        }
                    }).collect_view()
                }
                </TransitionUnpack>
            </div>
            <PermissionLevelForm
                sphere_name
                username_input
                select_ref
                set_role_action=state.set_sphere_role_action
            />
        </div>
    }
}

/// Component to set permission levels for a sphere
#[component]
pub fn PermissionLevelForm(
    sphere_name: Memo<String>,
    username_input: RwSignal<String>,
    select_ref: NodeRef<html::Select>,
    set_role_action: ServerAction<SetUserSphereRole>
) -> impl IntoView {
    let username_debounced: Signal<String> = signal_debounced(username_input, 250.0);
    let matching_user_resource = Resource::new(
        move || username_debounced.get(),
        move |username| async {
            if username.is_empty() {
                Ok(Vec::new())
            } else {
                get_matching_user_header_vec(username, Some(true), 20).await
            }
        },
    );
    let disable_submit = move || username_input.read().is_empty();

    view! {
        <AuthorizedShow sphere_name permission_level=PermissionLevel::Manage>
            <ActionForm action=set_role_action attr:class="w-full">
                <input
                    name="sphere_name"
                    class="hidden"
                    value=sphere_name
                />
                <div class="w-full flex gap-1 items-center">
                    <div class="dropdown w-2/5 lg:w-1/2">
                        <LengthLimitedInput
                            name="username"
                            placeholder={move_tr!("username")}
                            content=username_input
                            minlength=Some(1)
                            maxlength=Some(MAX_USERNAME_LENGTH)
                        />
                        <Show when=move || !username_input.read().is_empty()>
                            <TransitionUnpack resource=matching_user_resource let:user_header_vec>
                            {
                                let user_header_vec = user_header_vec.clone();
                                view ! {
                                    <ul tabindex="0" class="menu dropdown-content z-1 p-2 shadow-sm bg-base-300 rounded-box w-full">
                                        <For
                                            each=move || user_header_vec.clone().into_iter()
                                            key=|user_header| user_header.username.clone()
                                            let(user_header)
                                        >
                                            <li>
                                                <button
                                                    type="button"
                                                    value=user_header.username
                                                    on:click=move |ev| username_input.set(event_target_value(&ev))
                                                >
                                                    {user_header.username.clone()}
                                                </button>
                                            </li>
                                        </For>
                                    </ul>
                                }
                            }
                            </TransitionUnpack>
                        </Show>
                    </div>
                    <LocalizedEnumDropdown
                        name="permission_level"
                        enum_iter=PermissionLevel::iter()
                        class="select_input w-fit bg-base-200"
                        select_ref
                    />
                    <div class="flex-grow"></div>
                    <button
                        type="submit"
                        class="button-secondary p-3"
                        disabled=disable_submit
                    >
                        {move_tr!("assign")}
                    </button>
                </div>
            </ActionForm>
        </AuthorizedShow>
    }
}

/// Component to manage ban users
#[component]
pub fn BanPanel() -> impl IntoView {
    let sphere_name = expect_context::<SphereState>().sphere_name;
    let username_input = RwSignal::new(String::default());
    let username_debounced: Signal<String> = signal_debounced(username_input, 250.0);

    let unban_action = ServerAction::<RemoveUserBan>::new();
    let banned_users_resource = Resource::new(
        move || (username_debounced.get(), unban_action.version().get()),
        move |(username, _)| async move {
            match check_username(&username, true) {
                Ok(()) => get_sphere_ban_vec(sphere_name.get_untracked(), username).await,
                Err(e) => Err(e),
            }
        }
    );

    view! {
        // TODO add overflow-y-auto max-h-full?
        <div class="flex flex-col gap-1 items-center w-full bg-base-200 p-2 rounded-sm">
            <div class="text-xl text-center">{move_tr!("banned-users")}</div>
            <div class="w-full flex flex-col gap-1">
                <div class="flex flex-col border-b border-base-content/20">
                    <div class="flex gap-4 items-center">
                        <LengthLimitedInput
                            content=username_input
                            class="w-2/5"
                            placeholder=move_tr!("username")
                            maxlength=Some(MAX_USERNAME_LENGTH)
                        />
                        <div class="w-2/5 py-2 text-left font-bold">{move_tr!("until")}</div>
                    </div>
                </div>
                <TransitionUnpack resource=banned_users_resource show_error_detail=true let:banned_user_vec>
                {
                    banned_user_vec.iter().map(|user_ban| {
                        let duration_string = match user_ban.until_timestamp {
                            Some(until_timestamp) => until_timestamp.format("%Y-%m-%d %H:%M UTC").to_string().into(),
                            None => move_tr!("permanent"),
                        };
                        let ban_id = user_ban.ban_id;
                        view! {
                            <div class="flex gap-4 items-center">
                                <div class="w-2/5 px-2 text-sm">{user_ban.username.clone()}</div>
                                <div class="w-2/5 text-sm">{duration_string}</div>
                                <div class="flex-grow flex justify-end items-center gap-1">
                                    <BanInfoButton
                                        post_id=user_ban.post_id
                                        comment_id=user_ban.comment_id
                                    />
                                    <AuthorizedShow sphere_name permission_level=PermissionLevel::Ban>
                                        <ActionForm action=unban_action attr:class="flex justify-center items-center">
                                            <input
                                                name="ban_id"
                                                class="hidden"
                                                value=ban_id
                                            />
                                            <button class="button-error">
                                                <CrossIcon/>
                                            </button>
                                        </ActionForm>
                                    </AuthorizedShow>
                                </div>
                            </div>
                        }
                    }).collect_view()
                }
                </TransitionUnpack>
            </div>
        </div>
    }
}

/// Component to display a button opening a modal dialog with a ban's details
#[component]
pub fn BanInfoButton(
    post_id: i64,
    comment_id: Option<i64>,
) -> impl IntoView {
    let show_dialog = RwSignal::new(false);

    view! {
        <button
            class="button-secondary"
            on:click=move |_| show_dialog.update(|value| *value = !*value)
        >
            <MagnifierIcon class="content-toolbar-icon-size"/>
        </button>
        <ModalDialog
            class="w-full max-w-xl"
            show_dialog
        >
            {
                let ban_detail_resource = Resource::new(
                    move || (),
                    move |_| get_moderation_info(post_id, comment_id)
                );
                view! {
                    <div class="bg-base-100 shadow-xl p-3 rounded-xs flex flex-col gap-3">
                        <SuspenseUnpack resource=ban_detail_resource let:moderation_info>
                            <ModerationInfoDialog
                                moderated_content=moderation_info.content.clone()
                                rule_title=moderation_info.rule.title.clone()
                                rule_description=moderation_info.rule.description.clone()
                                is_sphere_rule=moderation_info.rule.sphere_id.is_some()
                            />
                            <button
                                type="button"
                                class="button-error"
                                on:click=move |_| show_dialog.set(false)
                            >
                                {move_tr!("close")}
                            </button>
                        </SuspenseUnpack>
                    </div>
                }
            }
        </ModalDialog>
    }
}