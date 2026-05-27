use leptos::prelude::*;

use sphare_core_common::constants::{LOGO_ICON_PATH, POPULAR_ICON_PATH};

#[component]
pub fn AddCommentIcon(#[prop(default = "content-toolbar-icon-size")] class: &'static str) -> impl IntoView {
    view! {
        <img src="/svg/toolbar/add_comment.svg" class=class/>
    }
}

#[component]
pub fn AppLogo(#[prop(default = "h-8 lg:h-10")] class: &'static str) -> impl IntoView {
    view! {
        <img src="/logo.svg" class=class/>
    }
}

#[component]
pub fn ArrowUpIcon(
    #[prop(into)]
    class: Signal<String>,
) -> impl IntoView {
    view! {
        <img src="/svg/arrow_up.svg" class=class/>
    }
}

#[component]
pub fn AuthErrorIcon(#[prop(default = "h-28 w-28")] class: &'static str) -> impl IntoView {
    view! {
        <img src="/svg/errors/alien.svg" class=class/>
    }
}

#[component]
pub fn AuthorIcon(
    #[prop(default = "content-toolbar-icon-size")]
    class: &'static str,
    #[prop(into, optional)]
    is_grayed_out: Signal<bool>,
) -> impl IntoView {
    view! {
        <Show
            when=is_grayed_out
            fallback=move || view! { <img src="/svg/toolbar/author.svg" class=class/> }
        >
            <img src="/svg/notifications/author_grayed_out.svg" class=class/>
        </Show>
    }
}

#[component]
pub fn ReturnIcon(#[prop(default = "h-5 w-5 lg:h-6 lg:w-6")] class: &'static str) -> impl IntoView {
    view! {
        <img src="/svg/sphere_icons/back_arrow.svg" class=class/>
    }
}

#[component]
pub fn BannedIcon(#[prop(default = "h-20 w-20")] class: &'static str) -> impl IntoView {
    view! {
        <img src="/svg/errors/banned.svg" class=class/>
    }
}

#[component]
pub fn BoldIcon(#[prop(default = "editor-button-size")] class: &'static str) -> impl IntoView {
    view! {
        <img src="/svg/editor/bold.svg" class=class/>
    }
}

#[component]
pub fn ClockIcon(
    #[prop(default = "content-toolbar-icon-size")]
    class: &'static str,
    #[prop(into, optional)]
    is_grayed_out: Signal<bool>,
) -> impl IntoView {
    view! {
        <Show
            when=is_grayed_out
            fallback=move || view! { <img src="/svg/toolbar/clock.svg" class=class/> }
        >
            <img src="/svg/notifications/clock_grayed_out.svg" class=class/>
        </Show>
    }
}

#[component]
pub fn CodeBlockIcon(#[prop(default = "editor-button-size")] class: &'static str) -> impl IntoView {
    view! {
        <img src="/svg/editor/codeblock.svg" class=class/>
    }
}

#[component]
pub fn CommentIcon(#[prop(default = "content-toolbar-icon-size")] class: &'static str) -> impl IntoView {
    view! {
        <img src="/svg/toolbar/comment.svg" class=class/>
    }
}

#[component]
pub fn CrossIcon(#[prop(default = "content-toolbar-icon-size")] class: &'static str) -> impl IntoView {
    view! {
        <img src="/svg/cross.svg" class=class/>
    }
}

#[component]
pub fn DeleteIcon(#[prop(default = "content-toolbar-icon-size")] class: &'static str) -> impl IntoView {
    view! {
        <img src="/svg/delete.svg" class=class/>
    }
}

#[component]
pub fn DotMenuIcon(#[prop(default = "content-toolbar-icon-size")] class: &'static str) -> impl IntoView {
    view! {
        <img src="/svg/dot_menu.svg" class=class/>
    }
}

#[component]
pub fn EditIcon(#[prop(default = "content-toolbar-icon-size")] class: &'static str) -> impl IntoView {
    view! {
        <img src="/svg/toolbar/edit.svg" class=class/>
    }
}

#[component]
pub fn EditTimeIcon(#[prop(default = "content-toolbar-icon-size")] class: &'static str) -> impl IntoView {
    view! {
        <img src="/svg/toolbar/edit_time.svg" class=class/>
    }
}

#[component]
pub fn FiltersIcon(#[prop(default = "filter-icon-size")] class: &'static str) -> impl IntoView {
    view! {
        <img src="/svg/toolbar/filters.svg" class=class/>
    }
}

#[component]
pub fn FlameIcon(#[prop(default = "filter-icon-size")] class: &'static str) -> impl IntoView {
    view! {
        <img src="/svg/flame.svg" class=class/>
    }
}

#[component]
pub fn GithubIcon(#[prop(default = "link-icon-size")] class: &'static str) -> impl IntoView {
    view! {
        <img src="/svg/external/github/github-mark-white.svg" class=class/>
    }
}

#[component]
pub fn GraphIcon(#[prop(default = "filter-icon-size")] class: &'static str) -> impl IntoView {
    view! {
        <img src="/svg/graph.svg" class=class/>
    }
}

#[component]
pub fn HammerIcon(#[prop(default = "content-toolbar-icon-size")] class: &'static str) -> impl IntoView {
    view! {
        <img src="/svg/toolbar/hammer.svg" class=class/>
    }
}

#[component]
pub fn Header1Icon(#[prop(default = "editor-button-size")] class: &'static str) -> impl IntoView {
    view! {
        <img src="/svg/editor/header_1.svg" class=class/>
    }
}

#[component]
pub fn Header2Icon(#[prop(default = "editor-button-size")] class: &'static str) -> impl IntoView {
    view! {
        <img src="/svg/editor/header_2.svg" class=class/>
    }
}

#[component]
pub fn HelpIcon(#[prop(default = "editor-button-size")] class: &'static str) -> impl IntoView {
    view! {
        <img src="/svg/editor/help.svg" class=class/>
    }
}

#[component]
pub fn HomeIcon(#[prop(default = "filter-icon-size")] class: &'static str) -> impl IntoView {
    view! {
        <img src="/svg/home.svg" class=class/>
    }
}

#[component]
pub fn HourglassIcon(#[prop(default = "filter-icon-size")] class: &'static str) -> impl IntoView {
    view! {
        <img src="/svg/hourglass.svg" class=class/>
    }
}

#[component]
pub fn ImageIcon(#[prop(default = "editor-button-size")] class: &'static str) -> impl IntoView {
    view! {
        <img src="/svg/editor/image.svg" class=class/>
    }
}

#[component]
pub fn InfoIcon(#[prop(default = "navbar-icon-size")] class: &'static str) -> impl IntoView {
    view! {
        <img src="/svg/info.svg" class=class/>
    }
}

#[component]
pub fn InternalErrorIcon(#[prop(default = "h-28 w-28")] class: &'static str) -> impl IntoView {
    view! {
        <img src="/svg/errors/landing_space_capsule.svg" class=class/>
    }
}

#[component]
pub fn InvalidRequestIcon(#[prop(default = "h-28 w-28")] class: &'static str) -> impl IntoView {
    view! {
        <img src="/svg/errors/chewbacca.svg" class=class/>
    }
}

#[component]
pub fn ItalicIcon(#[prop(default = "editor-button-size")] class: &'static str) -> impl IntoView {
    view! {
        <img src="/svg/editor/italic.svg" class=class/>
    }
}

#[component]
pub fn LinkIcon(#[prop(default = "editor-button-size")] class: &'static str) -> impl IntoView {
    view! {
        <img src="/svg/editor/link.svg" class=class/>
    }
}

#[component]
pub fn ListBulletIcon(#[prop(default = "editor-button-size")] class: &'static str) -> impl IntoView {
    view! {
        <img src="/svg/editor/bullet_list.svg" class=class/>
    }
}

#[component]
pub fn ListNumberIcon(#[prop(default = "editor-button-size")] class: &'static str) -> impl IntoView {
    view! {
        <img src="/svg/editor/number_list.svg" class=class/>
    }
}

/// Renders a loading icon
#[component]
pub fn LoadingIcon(#[prop(default = "loading-icon-size")] class: &'static str) -> impl IntoView {
    view! {
        <div class="w-full flex items-center justify-center">
            <img src="/svg/loading.svg" class=class/>
        </div>
    }
}

#[component]
pub fn LogoIcon(#[prop(default = "navbar-icon-size")] class: &'static str) -> impl IntoView {
    view! {
        <img src=LOGO_ICON_PATH class=class/>
    }
}

#[component]
pub fn MagnifierIcon(#[prop(default = "navbar-icon-size")] class: &'static str) -> impl IntoView {
    view! {
        <img src="/svg/magnifier.svg" class=class/>
    }
}

#[component]
pub fn MarkdownIcon(#[prop(default = "h-4 w-8")] class: &'static str) -> impl IntoView {
    view! {
        <div class=class>
            <img src="/svg/editor/markdown.svg"/>
        </div>
    }
}

#[component]
pub fn MaximizeIcon(#[prop(default = "h-4 w-4 lg:h-6 lg:w-6")] class: &'static str) -> impl IntoView {
    view! {
        <div class=class>
            <img src="/svg/maximize.svg"/>
        </div>
    }
}

#[component]
pub fn MinimizeIcon(#[prop(default = "h-5 w-5 lg:h-6 lg:w-6")] class: &'static str) -> impl IntoView {
    view! {
        <div class=class>
            <img src="/svg/minimize.svg"/>
        </div>
    }
}

#[component]
pub fn MinusIcon(#[prop(default = "content-toolbar-icon-size")] class: &'static str) -> impl IntoView {
    view! {
        <img src="/svg/minus.svg" class=class/>
    }
}

#[component]
pub fn ModeratorIcon(
    #[prop(default = "content-toolbar-icon-size")]
    class: &'static str,
    #[prop(into, optional)]
    is_grayed_out: Signal<bool>,
) -> impl IntoView {
    view! {
        <Show
            when=is_grayed_out
            fallback=move || view! { <img src="/svg/toolbar/moderator.svg" class=class/> }
        >
            <img src="/svg/notifications/moderator_grayed_out.svg" class=class/>
        </Show>
    }
}

#[component]
pub fn NetworkErrorIcon(#[prop(default = "h-28 w-28")] class: &'static str) -> impl IntoView {
    view! {
        <img src="/svg/errors/satellite.svg" class=class/>
    }
}

#[component]
pub fn NewLineIcon(#[prop(default = "editor-button-size")] class: &'static str) -> impl IntoView {
    view! {
        <img src="/svg/editor/new_line.svg" class=class/>
    }
}

#[component]
pub fn NotAuthorizedIcon(#[prop(default = "h-28 w-28")] class: &'static str) -> impl IntoView {
    view! {
        <img src="/svg/errors/stormtrooper.svg" class=class/>
    }
}

#[component]
pub fn NotFoundIcon(#[prop(default = "h-28 w-28")] class: &'static str) -> impl IntoView {
    view! {
        <img src="/svg/errors/man_on_the_moon.svg" class=class/>
    }
}

#[component]
pub fn NotificationIcon(#[prop(default = "navbar-icon-size")] class: &'static str) -> impl IntoView {
    view! {
        <img src="/svg/notifications/notification.svg" class=class/>
    }
}

#[component]
pub fn NsfwIcon(#[prop(optional)] class: &'static str) -> impl IntoView {
    view! {
        <div class=class>
            <div class="rounded-full p-1 bg-black text-sm font-semibold leading-none w-fit h-fit">
                "18+"
            </div>
        </div>
    }
}

#[component]
pub fn PauseIcon(#[prop(default = "content-toolbar-icon-size")] class: &'static str) -> impl IntoView {
    view! {
        <img src="/svg/pause.svg" class=class/>
    }
}

#[component]
pub fn PinnedIcon(#[prop(default = "content-toolbar-icon-size")] class: &'static str) -> impl IntoView {
    view! {
        <img src="/svg/pin.svg" class=class/>
    }
}

#[component]
pub fn PlayIcon(#[prop(default = "content-toolbar-icon-size")] class: &'static str) -> impl IntoView {
    view! {
        <img src="/svg/play.svg" class=class/>
    }
}

#[component]
pub fn PlusIcon(#[prop(default = "content-toolbar-icon-size")] class: &'static str) -> impl IntoView {
    view! {
        <img src="/svg/plus.svg" class=class/>
    }
}

#[component]
pub fn PodiumIcon(#[prop(default = "filter-icon-size")] class: &'static str) -> impl IntoView {
    view! {
        <img src="/svg/podium.svg" class=class/>
    }
}

#[component]
pub fn PopularIcon(#[prop(default = "filter-icon-size")] class: &'static str) -> impl IntoView {
    view! {
        <img src=POPULAR_ICON_PATH class=class/>
    }
}

#[component]
pub fn QuoteIcon(#[prop(default = "editor-button-size")] class: &'static str) -> impl IntoView {
    view! {
        <img src="/svg/editor/quote.svg" class=class/>
    }
}

#[component]
pub fn ReadAllIcon(#[prop(default = "sphere-toolbar-icon-size")] class: &'static str) -> impl IntoView {
    view! {
        <img src="/svg/notifications/read_all.svg" class=class/>
    }
}

#[component]
pub fn ReadIcon(#[prop(default = "sphere-toolbar-icon-size")] class: &'static str) -> impl IntoView {
    view! {
        <img src="/svg/notifications/read.svg" class=class/>
    }
}

#[component]
pub fn RefreshIcon(#[prop(default = "sphere-toolbar-icon-size")] class: &'static str) -> impl IntoView {
    view! {
        <img src="/svg/refresh.svg" class=class/>
    }
}

#[component]
pub fn SaveIcon(#[prop(default = "content-toolbar-icon-size")] class: &'static str) -> impl IntoView {
    view! {
        <img src="/svg/form/save.svg" class=class/>
    }
}

#[component]
pub fn ScoreIcon(#[prop(default = "content-toolbar-icon-size")] class: &'static str) -> impl IntoView {
    view! {
        <img src="/svg/toolbar/score.svg" class=class/>
    }
}

#[component]
pub fn SelfAuthorIcon(#[prop(default = "content-toolbar-icon-size")] class: &'static str) -> impl IntoView {
    view! {
        <img src="/svg/toolbar/author_filled.svg" class=class/>
    }
}

#[component]
pub fn SelfModeratorIcon(#[prop(default = "content-toolbar-icon-size")] class: &'static str) -> impl IntoView {
    view! {
        <img src="/svg/toolbar/moderator_filled.svg" class=class/>
    }
}

#[component]
pub fn SettingsIcon(#[prop(default = "sphere-toolbar-icon-size")] class: &'static str) -> impl IntoView {
    view! {
        <img src="/svg/settings_gear.svg" class=class/>
    }
}

#[component]
pub fn ShareIcon(#[prop(default = "content-toolbar-icon-size")] class: &'static str) -> impl IntoView {
    view! {
        <img src="/svg/toolbar/share.svg" class=class/>
    }
}

#[component]
pub fn SideBarIcon(#[prop(default = "navbar-icon-size")] class: &'static str) -> impl IntoView {
    view! {
        <img src="/svg/sidebar.svg" class=class/>
    }
}

#[component]
pub fn SphereIcon(
    icon_url: Option<String>,
    #[prop(into)]
    default_icon_index: usize,
    #[prop(default = "h-7 w-7")]
    class: &'static str
) -> impl IntoView {
    const ICON_MAPPING_SIZE: usize = 7;
    const ICON_MAPPING: [&str; ICON_MAPPING_SIZE] = [
        "/svg/sphere_icons/venus.svg",
        "/svg/sphere_icons/moon.svg",
        "/svg/sphere_icons/mars.svg",
        "/svg/sphere_icons/jupiter.svg",
        "/svg/sphere_icons/saturn.svg",
        "/svg/sphere_icons/uranus.svg",
        "/svg/sphere_icons/neptune.svg",
    ];
    match icon_url {
        Some(icon_url) => {
            let class = format!("rounded-full overflow-hidden {class}");
            view! { <img src=icon_url class=class/> }.into_any()
        },
        None => {
            let icon_url: &str = ICON_MAPPING[default_icon_index % ICON_MAPPING.len()];
            view! { <img src=icon_url class=class/> }.into_any()
        },
    }
}

#[component]
pub fn SpoilerIcon(#[prop(default = "content-toolbar-icon-size")] class: &'static str) -> impl IntoView {
    view! {
        <img src="/svg/spoiler.svg" class=class/>
    }
}

#[component]
pub fn StarIcon(
    #[prop(default = "h-7 w-7")] class: &'static str,
    show_color: RwSignal<bool>,
) -> impl IntoView {
    let svg_path = move || match show_color.get() {
        true => "/svg/stars.svg",
        false => "/svg/stars_disabled.svg",
    };
    view! {
        <img src=svg_path class=class/>
    }
}

#[component]
pub fn StrikethroughIcon(#[prop(default = "editor-button-size")] class: &'static str) -> impl IntoView {
    view! {
        <img src="/svg/editor/strikethrough.svg" class=class/>
    }
}

#[component]
pub fn SubscribedIcon(
    #[prop(default = "h-7 w-7")] class: &'static str,
    show_color: RwSignal<bool>,
) -> impl IntoView {
    let svg_path = move || match show_color.get() {
        true => "/svg/toolbar/star.svg",
        false => "/svg/toolbar/star_disabled.svg",
    };
    view! {
        <img src=svg_path class=class/>
    }
}

#[component]
pub fn TooHeavyIcon(#[prop(default = "h-28 w-28")] class: &'static str) -> impl IntoView {
    view! {
        <img src="/svg/errors/weight.svg" class=class/>
    }
}

#[component]
pub fn UnreadIcon(#[prop(default = "sphere-toolbar-icon-size")] class: &'static str) -> impl IntoView {
    view! {
        <img src="/svg/notifications/unread.svg" class=class/>
    }
}

#[component]
pub fn UserIcon(#[prop(default = "navbar-icon-size")] class: &'static str) -> impl IntoView {
    view! {
        <img src="/svg/user.svg" class=class/>
    }
}

#[component]
pub fn UserSettingsIcon(#[prop(default = "h-7 w-7")] class: &'static str) -> impl IntoView {
    view! {
        <img src="/svg/user_settings.svg" class=class/>
    }
}
