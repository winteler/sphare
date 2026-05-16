use leptos::prelude::*;
use leptos_fluent::move_tr;
use leptos_router::components::Form;

use sphare_core_common::routes::{get_create_post_path, get_current_url, get_profile_path, get_sphere_name, CREATE_POST_ROUTE, CREATE_POST_SPHERE_QUERY_PARAM, CREATE_SPHERE_ROUTE};

use sphare_cmp_common::auth_widget::LoginGuardButton;
use sphare_cmp_common::notification::NotificationButton;
use sphare_cmp_common::state::GlobalState;
use sphare_cmp_utils::icons::*;
use sphare_cmp_utils::widget::DropdownButton;

use crate::search::SearchButton;

/// Navigation bar component
#[component]
pub fn NavigationBar() -> impl IntoView
{
    let state = expect_context::<GlobalState>();
    view! {
        <div class="flex-none flex justify-between items-center w-full p-2 bg-blue-500">
            <div class="flex items-center gap-1 lg:gap-2">
                <button
                    class="drawer-button lg:hidden button-rounded-ghost"
                    on:click=move |_| state.show_left_sidebar.update(|value| *value = !*value)
                >
                    <SideBarIcon/>
                </button>
                <a href="/" class="button-ghost flex gap-1.5 items-center">
                    <LogoIcon/>
                    <div class="lg:pt-1 lg:pb-1.5 font-semibold">"Sphare"</div>
                </a>
            </div>
            <div class="flex items-center gap-1 lg:gap-2">
                <RightSidebarButton/>
                <SearchButton class="button-rounded-ghost"/>
                <PlusMenu/>
                <NotificationButton/>
                <UserMenu/>
            </div>
        </div>
    }
}

#[component]
pub fn UserMenu() -> impl IntoView {
    view! {
        <LoginGuardButton
            login_button_class="button-rounded-ghost"
            login_button_content=move || view! { <UserIcon/> }
            let:user
        >
            <LoggedInMenu username=user.username.clone()/>
        </LoginGuardButton>
    }
}

#[component]
pub fn LoggedInMenu(
    username: String,
) -> impl IntoView {
    let state = expect_context::<GlobalState>();

    view! {
        <DropdownButton
            button_class="button-rounded-ghost"
            activated_button_class="button-navbar-activated"
            button_content=move || view! { <UserIcon/> }
            align_right=true
        >
            <ul class="mt-4 z-10 p-2 shadow-sm bg-base-200 rounded-sm flex flex-col">
                <li>
                    <a href=get_profile_path(&username) class="button-ghost-sm block w-full">{move_tr!("profile")}</a>
                </li>
                <li>
                    <ActionForm action=state.logout_action attr:class="flex">
                        <input type="text" name="redirect_url" class="hidden" value=get_current_url()/>
                        <button type="submit" class="button-ghost-sm text-left w-full">
                            {move_tr!("logout")}
                        </button>
                    </ActionForm>
                </li>
                <li class="button-ghost-sm"><span>{format!("Logged in as: {}", username)}</span></li>
            </ul>
        </DropdownButton>
    }
}

#[component]
pub fn PlusMenu() -> impl IntoView {
    let current_sphere = RwSignal::new(String::default());
    let create_sphere_str = move_tr!("create-sphere");
    let create_post_str = move_tr!("share-post");
    view! {
        <DropdownButton
            button_class="button-rounded-ghost"
            activated_button_class="button-navbar-activated"
            button_content=move || view! { <PlusIcon class="navbar-icon-size"/> }
            align_right=true
        >
            <ul class="z-10 mt-4 p-2 bg-base-200 rounded-sm w-fit flex flex-col">
                <li class="button-ghost-sm w-full">
                    <LoginGuardButton
                        login_button_content=move || view! { <span class="whitespace-nowrap">{create_sphere_str}</span> }
                        redirect_path=String::from(CREATE_SPHERE_ROUTE)
                        let:_user
                    >
                        <a href=CREATE_SPHERE_ROUTE class="whitespace-nowrap">{create_sphere_str}</a>
                    </LoginGuardButton>
                </li>
                <li class="button-ghost-sm w-full">
                    <LoginGuardButton
                        login_button_content=move || view! { <span class="whitespace-nowrap">{create_post_str}</span> }
                        redirect_path=get_create_post_path()
                        let:_user
                    >
                        <Form method="GET" action=CREATE_POST_ROUTE attr:class="flex">
                            <input type="text" name=CREATE_POST_SPHERE_QUERY_PARAM class="hidden" value=current_sphere/>
                            <button type="submit" class="whitespace-nowrap" on:click=move |_| get_sphere_name(current_sphere)>
                                {create_post_str}
                            </button>
                        </Form>
                    </LoginGuardButton>
                </li>
            </ul>
        </DropdownButton>
    }
}

#[component]
pub fn RightSidebarButton() -> impl IntoView {
    let state = expect_context::<GlobalState>();
    let button_class = move || match state.show_right_sidebar.get() {
        true => "lg:hidden button-navbar-activated",
        false => "lg:hidden button-rounded-ghost",
    };
    view! {
        <button
            class=button_class
            on:click=move |_| state.show_right_sidebar.update(|value| *value = !*value)
        >
            <InfoIcon/>
        </button>
    }
}