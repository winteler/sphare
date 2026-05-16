use leptos::prelude::*;
use leptos_fluent::move_tr;
use leptos_router::hooks::use_navigate;
use leptos_router::NavigateOptions;

use sphare_core_common::common::SphereHeader;
use sphare_core_common::routes::get_sphere_path;

use sphare_cmp_utils::icons::SphereIcon;
use sphare_cmp_utils::widget::Badge;

/// Component to display a sphere's header
#[component]
pub fn SphereHeader(
    sphere_header: SphereHeader
) -> impl IntoView {
    let default_icon_index = sphere_header.sphere_name.as_bytes().first().cloned().unwrap_or_default();
    view! {
        <Badge text=sphere_header.sphere_name>
            <SphereIcon
                icon_url=sphere_header.icon_url
                default_icon_index
                class="content-toolbar-icon-size"
            />
        </Badge>
    }
}

/// Component to display a sphere's header that navigates to it upon clicking
#[component]
pub fn SphereHeaderLink(
    sphere_header: SphereHeader
) -> impl IntoView {
    // use navigate and prevent default to handle case where sphere header is in another <a>
    let navigate = use_navigate();
    let sphere_name = StoredValue::new(sphere_header.sphere_name.clone());
    let sphere_path = get_sphere_path(&sphere_header.sphere_name);
    let aria_label = move_tr!("navigate-sphere", {"sphere_name" => sphere_name.get_value()});
    view! {
        <button
            class="button-rounded-neutral px-2 py-1 flex items-center"
            on:click=move |ev| {
                ev.prevent_default();
                navigate(sphere_path.as_str(), NavigateOptions::default());
            }
            aria-label=aria_label
        >
            <SphereHeader sphere_header/>
        </button>
    }
}
