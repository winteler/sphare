use leptos::prelude::*;

use sphare_core_common::routes::get_profile_path;
use sphare_core_user::user::UserHeader;

use sphare_cmp_utils::icons::{NsfwIcon, UserIcon};

/// Component to display a user header
#[component]
pub fn UserHeaderWidget(
    user_header: UserHeader,
) -> impl IntoView {
    view! {
        <div class="flex gap-1.5 items-center text-sm">
            <UserIcon/>
            {user_header.username}
            {
                match user_header.is_nsfw {
                    true => Some(view! { <NsfwIcon/> }),
                    false => None,
                }
            }
        </div>
    }.into_any()
}

/// Component to display a user header and redirect to his profile upon click
#[component]
pub fn UserHeaderLink(
    user_header: UserHeader,
) -> impl IntoView {
    let user_profile_path = get_profile_path(&user_header.username);
    view! {
        <a href=user_profile_path>
            <div class="w-full p-2 my-1 rounded-sm hover:bg-base-200">
                <UserHeaderWidget user_header/>
            </div>
        </a>
    }.into_any()
}