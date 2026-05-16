use leptos::prelude::*;
use leptos::server_fn::client::Client;
use leptos::server_fn::codec::PostUrl;
use leptos::server_fn::request::ClientReq;
use leptos::server_fn::Http;
use leptos::server_fn::ServerFn;
use leptos_fluent::move_tr;
use leptos_router::hooks::{use_location, use_navigate, use_query_map};
use leptos_router::NavigateOptions;
use serde::de::DeserializeOwned;
use web_sys::{FormData, MouseEvent};

use sphare_core_common::errors::AppError;
use sphare_core_common::routes::get_profile_path;
use sphare_core_user::user::User;

use sphare_iface_user::auth::authenticate_user;

use sphare_cmp_utils::form::LabeledSignalCheckbox;
use sphare_cmp_utils::icons::{AuthErrorIcon, AuthorIcon, DeleteIcon, LoadingIcon, ModeratorIcon, SelfAuthorIcon, SelfModeratorIcon};
use sphare_cmp_utils::unpack::{ActionError, SuspenseUnpack};
use sphare_cmp_utils::widget::{ModalDialog, ModalFormButtons};

use crate::state::GlobalState;

/// Guard for a component requiring a login. If the user is logged in, the children of this component will be rendered
/// Otherwise, it will be replaced by a form/button with the same appearance redirecting to a login screen.
#[component]
pub fn LoginGuardButton<
    F: Fn(&User) -> IV + Clone + Send + Sync + 'static,
    IV: IntoView + 'static,
>(
    #[prop(default = "")]
    login_button_class: &'static str,
    #[prop(into)]
    login_button_content: ViewFn,
    #[prop(into, default=use_location().pathname.into())]
    redirect_path: Signal<String>,
    #[prop(default = "loading-icon-size")]
    loading_icon_class: &'static str,
    children: F,
) -> impl IntoView {
    let state = expect_context::<GlobalState>();
    let children = StoredValue::new(children);
    let login_button_content = StoredValue::new(login_button_content);

    view! {
        <Transition fallback=move || view! { <LoadingIcon class=loading_icon_class/> }>
        {
            move || Suspend::new(async move {
                match &state.user.await {
                    Ok(Some(user)) => children.with_value(|children| children(user)).into_any(),
                    _ => {
                        let login_button_view = login_button_content.with_value(|content| content.run());
                        view! { <LoginButton class=login_button_class redirect_path>{login_button_view}</LoginButton> }.into_any()
                    },
                }
            })
        }
        </Transition>
    }.into_any()
}

/// Login guarded button component. If the user is logged in, a button with the given class and action will be rendered.
/// Otherwise, the button will redirect the user to a login screen.
#[component]
pub fn LoginGuardedButton<A, IV>(
    #[prop(into)]
    button_class: Signal<&'static str>,
    button_action: A,
    children: TypedChildrenFn<IV>,
    #[prop(default = "loading-icon-size")]
    loading_icon_class: &'static str,
) -> impl IntoView
where
    A: Fn(MouseEvent) -> () + Clone + Send + Sync + 'static,
    IV: IntoView + 'static
{
    let state = expect_context::<GlobalState>();
    let children = StoredValue::new(children.into_inner());
    let button_action = StoredValue::new(button_action);
    view! {
        <Transition fallback=move || view! { <LoadingIcon class=loading_icon_class/> }>
        {
            move || Suspend::new(async move {
                let children_view = children.with_value(|children| children());
                match &state.user.await {
                    Ok(Some(_)) => view! {
                        <button
                            class=button_class
                            aria-haspopup="dialog"
                            on:click=button_action.get_value()
                        >
                            {children_view}
                        </button>
                    }.into_any(),
                    _ => view! { <LoginButton class=button_class redirect_path=use_location().pathname>{children_view}</LoginButton> }.into_any(),
                }
            })
        }
        </Transition>
    }
}

/// Component to display a button opening a modal dialog if the user
/// is authenticated and redirecting to a login page otherwise
#[component]
pub fn LoginGuardedOpenModalButton<IV>(
    show_dialog: RwSignal<bool>,
    #[prop(into)]
    button_class: Signal<&'static str>,
    children: TypedChildrenFn<IV>,
) -> impl IntoView
where
    IV: IntoView + 'static
{
    view! {
        <LoginGuardedButton
            button_class
            button_action=move |_| show_dialog.update(|show: &mut bool| *show = !*show)
            children
            attr:aria-expanded=move || show_dialog.get().to_string()
            attr:aria-haspopup="dialog"
        />
    }
}

#[component]
fn LoginButton(
    #[prop(into)]
    class: Signal<&'static str>,
    #[prop(into)]
    redirect_path: Signal<String>,
    children: Children,
) -> impl IntoView {
    let state = expect_context::<GlobalState>();

    view! {
        <ActionForm action=state.login_action attr:class="flex items-center">
            <input type="text" name="redirect_url" class="hidden" value=redirect_path/>
            <button type="submit" class=class>
                {children()}
            </button>
        </ActionForm>
    }.into_any()
}

/// Auth callback component
#[component]
pub fn AuthCallback() -> impl IntoView {
    let query = use_query_map();
    let code = move || query.read_untracked().get("code").unwrap_or_default().to_string();
    let auth_resource = Resource::new_blocking(
        || (),
        move |_| {
            log::trace!("Authenticate user.");
            authenticate_user(code())
        }
    );

    view! {
        <SuspenseUnpack
            resource=auth_resource
            let:_auth_result
        >
            {
                log::debug!("Authenticated successfully");
            }
        </SuspenseUnpack>
    }.into_any()
}

/// Renders a page requesting a login
#[component]
pub fn LoginWindow() -> impl IntoView {
    let state = expect_context::<GlobalState>();

    view! {
        <div class="hero">
            <div class="hero-content flex text-center">
                <AuthErrorIcon class="h-20 w-20 lg:h-44 lg:w-44"/>
                <div class="max-w-md">
                    <h1 class="text-5xl font-bold">"Not authenticated"</h1>
                    <p class="pt-4">"Sorry, we had some trouble identifying you."</p>
                    <p class="pb-4">"Please login to access this page."</p>
                    <ActionForm action=state.login_action>
                        <input type="text" name="redirect_url" class="hidden" value=use_location().pathname/>
                        <button type="submit" class="button-primary w-full">
                            {move_tr!("login")}
                        </button>
                    </ActionForm>
                </div>
            </div>
        </div>
    }
}

/// Component to display the author of a post or comment
#[component]
pub fn AuthorWidget(
    author_id: i64,
    author: String,
    is_moderator: bool,
    #[prop(into, optional)]
    is_grayed_out: Signal<bool>,
) -> impl IntoView {
    let navigate = use_navigate();
    let state = expect_context::<GlobalState>();
    let author_profile_path = get_profile_path(&author);
    let aria_label = format!("Navigate to user {}'s profile with path {}", author, author_profile_path);

    view! {
        <button
            class="button-rounded-neutral px-2 py-1 flex gap-1.5 items-center"
            on:click=move |ev| {
                ev.prevent_default();
                navigate(author_profile_path.as_str(), NavigateOptions::default());
            }
            aria-label=aria_label
        >
            <Transition fallback=move || view! { <LoadingIcon class="content-toolbar-icon-size"/> }>
            {
                move || Suspend::new(async move {
                    match (&state.user.await, is_moderator) {
                        (Ok(Some(user)), true) if author_id == user.user_id => view! { <SelfModeratorIcon/> }.into_any(),
                        (Ok(Some(user)), false) if author_id == user.user_id => view! { <SelfAuthorIcon/> }.into_any(),
                        (_, true) => view! { <ModeratorIcon is_grayed_out/> }.into_any(),
                        (_, false) => view! { <AuthorIcon is_grayed_out/> }.into_any(),
                    }
                })
            }
            </Transition>
            <span
                class="text-xs lg:text-sm"
                class:text-gray-400=is_grayed_out
            >
                {author}
            </span>
        </button>
    }.into_any()
}

/// Component to render a delete button
#[component]
pub fn DeleteButton<A, O>(
    #[prop(into)]
    title: Signal<String>,
    id: i64,
    id_name: &'static str,
    author_id: i64,
    delete_action: ServerAction<A>
) -> impl IntoView
where
    A: DeserializeOwned
    + ServerFn<Protocol = Http<PostUrl, O>, Error = AppError>
    + Clone
    + Send
    + Sync
    + 'static,
    <<A::Client as Client<A::Error>>::Request as ClientReq<
        A::Error,
    >>::FormData: From<FormData>,
    A: Send + Sync + 'static,
    A::Output: Send + Sync + 'static,
    <A as ServerFn>::Client: Client<AppError>,
    O: 'static,
{
    let state = expect_context::<GlobalState>();
    let show_form = RwSignal::new(false);
    let show_button = move || match &(*state.user.read()) {
        Some(Ok(Some(user))) => user.user_id == author_id,
        _ => false,
    };
    let delete_button_class = move || match show_form.get() {
        true => "button-rounded-error",
        false => "button-rounded-neutral",
    };
    view! {
        <Suspense>
            <Show when=show_button>
                <div>
                    <button
                        class=delete_button_class
                        aria-expanded=move || show_form.get().to_string()
                        aria-haspopup="dialog"
                        on:click=move |_| show_form.update(|show: &mut bool| *show = !*show)
                    >
                        <DeleteIcon/>
                    </button>
                    <ModalDialog
                        class="w-full flex justify-center"
                        show_dialog=show_form
                    >
                        <div class="bg-base-100 shadow-xl p-3 rounded-xs flex flex-col gap-5 w-96">
                            <div class="text-center font-bold text-2xl">{title}</div>
                            <div class="text-center font-bold text-xl">{move_tr!("delete-warning")}</div>
                            <ActionForm action=delete_action>
                                <input
                                    name=id_name
                                    class="hidden"
                                    value=id
                                />
                                <ModalFormButtons
                                    disable_publish=false
                                    show_form
                                />
                            </ActionForm>
                            <ActionError action=delete_action.into()/>
                        </div>
                    </ModalDialog>
                </div>
            </Show>
        </Suspense>
    }
}

/// Component to display a checkbox to enable or disable NSFW results.
/// If the user is not logged in or has disabled NSFW in his settings, the checkbox is hidden and deactivated.
#[component]
pub fn NsfwCheckbox(
    show_nsfw: RwSignal<bool>,
    #[prop(default = "NSFW")]
    label: &'static str,
    #[prop(default = "pl-1")]
    class: &'static str,
) -> impl IntoView {
    let state = expect_context::<GlobalState>();
    view! {
        <Transition fallback=move || view! {  <LoadingIcon/> }>
        {
            move || Suspend::new(async move {
                match state.user.await {
                    Ok(Some(user)) if user.show_nsfw => Some(view! {
                        <LabeledSignalCheckbox label value=show_nsfw class=class/>
                    }),
                    _ => {
                        show_nsfw.set(false);
                        None
                    },
                }
            })
        }
        </Transition>
    }
}