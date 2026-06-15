use leptos::prelude::window;
use leptos_router::hooks::use_navigate;
use leptos_router::NavigateOptions;

pub fn browser_redirect(redirect_url: &str) {
    if let Err(e) = window().location().set_href(redirect_url) {
        log::error!("Failed to redirect to auth provider: {}", e.as_string().unwrap_or_default());
    }
}

pub fn router_redirect(redirect_url: &str) {
    let navigate = use_navigate();
    navigate(redirect_url, NavigateOptions::default());
}