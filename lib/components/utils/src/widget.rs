use std::fmt::Debug;
use std::marker::PhantomData;
use std::str::FromStr;

use const_format::concatcp;
use leptos::html;
use leptos::prelude::codee::{Decoder, Encoder};
use leptos::prelude::*;
use leptos_fluent::move_tr;
use leptos_router::components::Form;
use leptos_router::hooks::use_query_map;
use leptos_use::BreakpointsTailwind::Lg;
use leptos_use::{breakpoints_tailwind, use_breakpoints, use_clipboard};
use strum::IntoEnumIterator;

#[cfg(feature = "hydrate")]
use leptos_use::on_click_outside;

use sphare_core_common::errors::AppError;
use sphare_core_common::time::get_elapsed_time_string;
use sphare_core_common::traits::ToLocalizedStr;
use crate::errors::ErrorDisplay;
use crate::icons::{ArrowUpIcon, ClockIcon, CommentIcon, DotMenuIcon, EditTimeIcon, HelpIcon, LoadingIcon, MaximizeIcon, MinimizeIcon, ModeratorIcon, NotFoundIcon, NsfwIcon, PinnedIcon, RefreshIcon, ScoreIcon, ShareIcon, SphereIcon, SpoilerIcon};
use crate::view::ToView;

pub const SPHERE_NAME_PARAM: &str = "sphere_name";
pub const IMAGE_FILE_PARAM: &str = "image";


/// Component that displays its children in a modal dialog
#[component]
pub fn ModalDialog(
    #[prop(default = "")]
    class: &'static str,
    show_dialog: RwSignal<bool>,
    children: ChildrenFn,
    #[prop(optional)]
    modal_ref: NodeRef<html::Div>,
) -> impl IntoView {
    let dialog_class =
        move || format!("relative transform overflow-visible rounded-sm transition-all {class}");
    view! {
        <Show when=show_dialog>
            <div
                class="relative z-20"
                aria-labelledby="modal-title"
                role="dialog"
                aria-modal="true"
            >
                <div class="fixed inset-0 bg-base-200/75 transition-opacity"></div>
                <div class="fixed inset-0 z-20 w-screen overflow-auto">
                    <div class="flex min-h-full justify-center items-center">
                        <div class=dialog_class node_ref=modal_ref>
                            {children()}
                        </div>
                    </div>
                </div>
            </div>
        </Show>
    }.into_any()
}

/// Button that displays its children in a dropdown when clicked
#[component]
pub fn DropdownButton<C: IntoView + 'static>(
    #[prop(default="button-rounded-neutral")]
    button_class: &'static str,
    #[prop(default="button-rounded-primary")]
    activated_button_class: &'static str,
    #[prop(into)]
    button_content: ViewFn,
    #[prop(optional)]
    align_right: bool,
    #[prop(default = true)]
    open_down: bool,
    #[prop(optional)]
    show_dropdown: RwSignal<bool>,
    children: TypedChildrenFn<C>,
    #[prop(optional)]
    dropdown_ref: NodeRef<html::Div>,
) -> impl IntoView {
    #[cfg(feature = "hydrate")]
    {
        // only enable with "hydrate" to avoid server side "Dropped SendWrapper" error
        let _ = on_click_outside(dropdown_ref, move |_| show_dropdown.set(false));
    }
    let button_class = move || match show_dropdown.get() {
        true => activated_button_class,
        false => button_class,
    };

    view! {
        <div class="h-full relative" node_ref=dropdown_ref>
            <button
                class=button_class
                type="button"
                on:click= move |_| show_dropdown.update(|value| *value = !*value)
            >
                {button_content.run()}
            </button>
            <Dropdown show_dropdown align_right open_down children/>
        </div>
    }.into_any()
}

/// Component that displays its children in a dropdown when the input show_dropdown is true
#[component]
pub fn Dropdown<C: IntoView + 'static>(
    show_dropdown: RwSignal<bool>,
    #[prop(optional)]
    align_right: bool,
    #[prop(default = true)]
    open_down: bool,
    children: TypedChildrenFn<C>,
) -> impl IntoView {
    let children = children.into_inner();
    let class = match (align_right, open_down) {
        (true, true) => "absolute z-10 right-0 min-w-max",
        (false, true) => "absolute z-10 left-0 min-w-max",
        (true, false) => "absolute z-10 bottom-full right-0 min-w-max",
        (false, false) => "absolute z-10 bottom-full left-0 min-w-max",
    };
    view! {
        <Show when=show_dropdown>
            <div class=class>
            {
                children()
            }
            </div>
        </Show>
    }.into_any()
}

/// Form to update query parameter `query_param` with the value `title` upon clicking
#[component]
fn QueryTab(
    query_param: &'static str,
    query_value: &'static str,
    #[prop(into)]
    query_display: Signal<String>,
    is_selected: Signal<bool>,
) -> impl IntoView {
    let tab_class = move || match is_selected.get() {
        true => "w-full p-1 px-2 text-sm lg:text-base text-center bg-base-content/20 hover:bg-base-content/50",
        false => "w-full p-1 px-2 text-sm lg:text-base text-center hover:bg-base-content/50",
    };
    view! {
        <Form method="GET" action="" attr:class="w-full">
            <input type="search" class="hidden" name=query_param value=query_value/>
            <button type="submit" class=tab_class>
                {query_display}
            </button>
        </Form>
    }
}

/// Component to display a QueryTab based on the input query_to_view_map
#[component]
fn QueryTabs<I, T>(
    query_param: &'static str,
    query_enum_iter: I,
) -> impl IntoView
where
    I: IntoIterator<Item = T>,
    T: Copy + Default + FromStr + Into<&'static str> + Into<Signal<String>> + IntoEnumIterator + PartialEq + Send + Sync + 'static
{
    let query = use_query_map();
    let selected_enum = Signal::derive(move || T::from_str(&query.read().get(query_param).unwrap_or_default()).unwrap_or_default());
    view! {
        <div class="w-full flex justify-stretch divide-x divide-base-content/20 border border-1 border-base-content/20">
        {
            // TODO try styling first and last element differently
            query_enum_iter.into_iter().map(|enum_value| {
                let is_selected = Signal::derive(move || selected_enum.get() == enum_value);
                let query_display: Signal<String> = enum_value.into();
                view! {
                    <QueryTab query_param query_value=enum_value.into() query_display is_selected/>
                }
            }.into_any()).collect_view()
        }
        </div>
    }.into_any()
}

/// Component to display the view of the enum selected by the query parameter `query_param`
#[component]
fn QueryShow<T>(
    query_param: &'static str,
    _enum_type: PhantomData<T>,
) -> impl IntoView
where
    T: Copy + Default + FromStr + Default + IntoEnumIterator + PartialEq + ToView
{
    let query = use_query_map();
    view! {
        { move || {
            T::from_str(
                &query.read().get(query_param).unwrap_or_default()
            )
                .unwrap_or_default()
                .to_view()
        }}
    }.into_any()
}

/// Component to display tabs based on the `query_enum_iter` and upon clicking them, update
/// the query parameter `query_param` with the enum value and display the view using the ToView trait
#[component]
pub fn EnumQueryTabs<I, T>(
    query_param: &'static str,
    query_enum_iter: I,
) -> impl IntoView
where
    I: IntoIterator<Item = T> + Clone + Send + Sync + 'static,
    T: Copy + Default + FromStr + Into<&'static str> + Into<Signal<String>> + IntoEnumIterator + PartialEq + ToView + Send + Sync + 'static
{
    let _enum_type = PhantomData::<T>;
    view! {
        <div class="flex flex-col gap-4 pt-2 px-2 w-full h-full">
            <QueryTabs query_param query_enum_iter=query_enum_iter.clone()/>
            <QueryShow query_param _enum_type/>
        </div>
    }
}

/// Component to create a dropdown based on a given strum::EnumIter
#[component]
pub fn EnumDropdown<I, T>(
    name: &'static str,
    enum_iter: I,
    #[prop(default = "select_input w-fit")]
    class: &'static str,
    select_ref: NodeRef<html::Select>,
) -> impl IntoView
where
    I: IntoIterator<Item = T>,
    T: FromStr + Into<&'static str> + IntoEnumIterator
{
    view! {
        <select
            name=name
            class=class
            node_ref=select_ref
        >
        {
            enum_iter.into_iter().map(|enum_val| view! {<option>{enum_val.into()}</option>}.into_any()).collect_view()
        }
        </select>
    }.into_any()
}

/// Component to create a dropdown based on a given strum::EnumIter, display text using ToLocalizedStr Trait
#[component]
pub fn LocalizedEnumDropdown<I, T>(
    name: &'static str,
    enum_iter: I,
    #[prop(default = "select_input w-fit")]
    class: &'static str,
    select_ref: NodeRef<html::Select>,
) -> impl IntoView
where
    I: IntoIterator<Item = T>,
    T: FromStr + Into<&'static str> + IntoEnumIterator + ToLocalizedStr
{
    view! {
        <select
            name=name
            class=class
            node_ref=select_ref
        >
        {
            enum_iter.into_iter().map(|enum_val| view! {<option value=enum_val.into()>{enum_val.to_localized_str()}</option>}.into_any()).collect_view()
        }
        </select>
    }.into_any()
}

/// Component to display a button with a three-dot icon opening a menu displaying the children of the component when clicked
#[component]
pub fn DotMenu<C: IntoView + 'static>(
    children: TypedChildrenFn<C>,
) -> impl IntoView {
    let children = children.into_inner();
    view! {
        <DropdownButton
            button_content=move || view! { <DotMenuIcon/> }
            button_class="button-rounded-neutral px-1.5 py-1"
            activated_button_class="button-rounded-primary px-1.5 py-1"
        >
            <div class="bg-base-200 shadow-sm rounded-sm mt-1 p-1 flex flex-col gap-1">
            {
                children()
            }
            </div>
        </DropdownButton>
    }.into_any()
}

/// Component to display a badge, i.e. an icon with associated text
/// For simplicity, the icon is passed as a child
#[component]
pub fn Badge<C>(
    #[prop(into)]
    text: Signal<String>,
    children: TypedChildren<C>,
) -> impl IntoView
where
    C: IntoView + 'static,
{
    view! {
        <div class="flex gap-1.5 items-center">
            {children.into_inner()()}
            <div class="text-xs lg:text-sm">{move || text.get()}</div>
        </div>
    }.into_any()
}

#[component]
pub fn SpoilerBadge() -> impl IntoView {
    view! {
        <div class="h-fit w-fit px-1 py-0.5 bg-black rounded-full">
            <SpoilerIcon/>
        </div>
    }
}

/// Component to display the number of comments in a post
#[component]
pub fn CommentCountWidget(
    count: i32,
) -> impl IntoView {
    view! {
        <div class="flex gap-1.5 items-center text-xs lg:text-sm px-1">
            <CommentIcon/>
            {count}
        </div>
    }.into_any()
}

/// Component to display the moderator of a post or comment
#[component]
pub fn ModeratorWidget(
    #[prop(into)]
    moderator: Signal<Option<String>>
) -> impl IntoView {
    view! {
        <Show when=move || moderator.read().is_some()>
            <div class="flex px-1 gap-1.5 items-center text-xs lg:text-sm">
                <ModeratorIcon/>
                {
                    move || moderator.get().unwrap_or_default()
                }
            </div>
        </Show>
    }.into_any()
}

/// Component to conditionally display a pin icon
#[component]
pub fn IsPinnedWidget(
    #[prop(into)]
    is_pinned: Signal<bool>,
) -> impl IntoView {
    view! {
        { move || match is_pinned.get() {
            true => Some(view! { <div class="px-1"><PinnedIcon/></div>}),
            false => None
        }}
    }
}

/// Component to display a content's tags (spoiler, nsfw, ...)
#[component]
pub fn TagsWidget(
    is_nsfw: bool,
    is_spoiler: bool,
    #[prop(default = false)]
    is_pinned: bool,
) -> impl IntoView {
    view! {
        <div class="flex gap-1">
        {
            match is_spoiler {
                true => Some(view! { <div class="h-fit w-fit px-1 py-0.5 bg-black rounded-full"><SpoilerIcon/></div> }),
                false => None
            }
        }
        {
            match is_nsfw {
                true => Some(view! { <NsfwIcon/>}),
                false => None
            }
        }
        <IsPinnedWidget is_pinned/>
        </div>
    }
}

/// Component to display the creation time of a post
#[component]
pub fn TimeSinceWidget(
    #[prop(into)]
    timestamp: Signal<chrono::DateTime<chrono::Utc>>,
    #[prop(into, optional)]
    is_grayed_out: Signal<bool>,
) -> impl IntoView {
    let use_fullname = use_breakpoints(breakpoints_tailwind()).ge(Lg);
    view! {
        <div
            class="flex gap-1.5 items-center text-xs lg:text-sm px-1"
            class:text-gray-400=is_grayed_out
        >
            <ClockIcon is_grayed_out/>
            {
                move || get_elapsed_time_string(timestamp.get(), use_fullname.get())
            }
        </div>
    }.into_any()
}

/// Component to display the edit time of a post or comment
#[component]
pub fn TimeSinceEditWidget(
    #[prop(into)]
    edit_timestamp: Signal<Option<chrono::DateTime<chrono::Utc>>>
) -> impl IntoView {
    let use_fullname = use_breakpoints(breakpoints_tailwind()).ge(Lg);
    view! {
        <Show when=move || edit_timestamp.read().is_some()>
            <div class="flex gap-1.5 items-center text-xs lg:text-sm px-1">
                <EditTimeIcon/>
                {
                    move || get_elapsed_time_string(edit_timestamp.get().unwrap(), use_fullname.get())
                }
            </div>
        </Show>
    }
}

/// Button to share content that copies the input `link` to the clipboard
#[component]
pub fn ShareButton(
    link: String,
) -> impl IntoView {
    let link = StoredValue::new(link);
    let use_clipboard = use_clipboard();
    let show_notification = RwSignal::new(false);

    view! {
        <button
            type="button"
            class="button-rounded-neutral"
            on:click= move |_| {
                show_notification.set(true);
                if use_clipboard.is_supported.get() {
                    log::info!("Copied link to clipboard: {}", link.read_value());
                    (use_clipboard.copy)(&*link.read_value());
                } else {
                    log::warn!("Clipboard API not supported in your browser.");
                }
                set_timeout(move || show_notification.set(false), std::time::Duration::from_secs(3));
            }
        >
            <ShareIcon/>
        </button>
        <Show
            when=move || use_clipboard.is_supported.get()
            fallback=move || view! {
                <div class="toast toast-center">
                    <div class="alert alert-error" class=("hidden", move || !show_notification.get())>
                        <span>{move_tr!("clipboard-error-message")}</span>
                    </div>
                </div>
            }
        >
            <div class="toast toast-center">
                <div class="alert alert-success" class=("hidden", move || !show_notification.get())>
                    <span>{move_tr!("copy-link-to-clipboard-message")}</span>
                </div>
            </div>
        </Show>
    }
}

/// Displays the body of a content given as input with correct styling for markdown
#[component]
pub fn ContentBody(
    #[prop(into)]
    body: Signal<String>,
    is_markdown: bool,
) -> impl IntoView {
    let class = match is_markdown {
        true => "",
        false => "whitespace-pre-wrap text-wrap wrap-anywhere",
    };

    view! {
        <div
            class=class
            inner_html=body
        />
    }.into_any()
}

/// Component to display a post's score
#[component]
pub fn ScoreIndicator(score: i32) -> impl IntoView {
    view! {
        <div class="w-fit px-1 flex gap-1 items-center">
            <ScoreIcon/>
            <div class="text-xs lg:text-sm">
                {score}
            </div>
        </div>
    }.into_any()
}

/// Component to display a "minimize" or "maximize" icon with transitions
#[component]
pub fn MinimizeMaximizeWidget(
    is_maximized: RwSignal<bool>,
) -> impl IntoView {
    let invisible_class = "transition-transform opacity-0 invisible h-0 w-0 order-2";
    let visible_class = "transition-transform rotate-90 duration-300 opacity-100 visible order-1";
    let minimize_class = move || match is_maximized.get() {
        true => visible_class,
        false => invisible_class,
    };
    let maximize_class = move || match is_maximized.get() {
        true => invisible_class,
        false => visible_class,
    };
    view! {
        <div class="flex flex-col">
            <div class=minimize_class>
                <MinimizeIcon/>
            </div>
            <div class=maximize_class>
                <MaximizeIcon/>
            </div>
        </div>
    }
}

/// Reload button updating a signal upon clicking
#[component]
pub fn RefreshButton(
    /// signal counting the number of reloads
    refresh_count: RwSignal<usize>,
    #[prop(optional)]
    is_tooltip_bottom: bool,
) -> impl IntoView {
    const BASE_CLASS: &str = "button-rounded-ghost w-fit tooltip";
    let button_class = match is_tooltip_bottom {
        true => concatcp!(BASE_CLASS, " tooltip-bottom"),
        false => BASE_CLASS,
    };
    view! {
        <button
            class=button_class
            data-tip=move_tr!("refresh")
            on:click=move |_| refresh_count.update(|count| *count += 1)
        >
            <RefreshIcon/>
        </button>
    }
}

/// Reload button refetching a resource upon clicking
#[component]
pub fn RefreshResourceButton<T, Ser>(
    resource: Resource<T, Ser>,
    #[prop(optional)]
    is_tooltip_bottom: bool,
) -> impl IntoView
where
    Ser: Encoder<T> + Decoder<T>,
    <Ser as Encoder<T>>::Error: Debug,
    <Ser as Decoder<T>>::Error: Debug,
    <<Ser as Decoder<T>>::Encoded as FromEncodedStr>::DecodingError: Debug,
    <Ser as Encoder<T>>::Encoded: IntoEncodedString,
    <Ser as Decoder<T>>::Encoded: FromEncodedStr,
    T: Send + Sync + 'static,
{
    const BASE_CLASS: &str = "button-rounded-ghost w-fit tooltip";
    let button_class = match is_tooltip_bottom {
        true => concatcp!(BASE_CLASS, " tooltip-bottom"),
        false => BASE_CLASS,
    };
    view! {
        <button
            class=button_class
            data-tip=move_tr!("refresh")
            on:click=move |_| resource.refetch()
        >
            <RefreshIcon/>
        </button>
    }
}

/// Component to render cancel and publish buttons for a modal Form
#[component]
pub fn ModalFormButtons(
    /// functions returning whether the publish buttons should be disabled
    #[prop(into)]
    disable_publish: Signal<bool>,
    /// signal to hide the form upon submitting or cancelling
    show_form: RwSignal<bool>,
) -> impl IntoView {
    view! {
        <div class="flex justify-between gap-2">
            <button
                type="button"
                class="button-error"
                on:click=move |_| show_form.set(false)
            >
                {move_tr!("cancel")}
            </button>
            <button
                type="submit"
                class="button-secondary"
                disabled=disable_publish
            >
                {move_tr!("submit")}
            </button>
        </div>
    }
}

/// Component to render cancel and publish buttons for a modal Form
#[component]
pub fn RotatingArrow(
    #[prop(into)]
    point_up: Signal<bool>,
    #[prop(default = "h-5 w-5 p-1")]
    class: &'static str,
) -> impl IntoView {
    let arrow_class = Signal::derive(move || match point_up.get() {
        true => format!("{class} transition duration-200"),
        false => format!("{class} transition duration-200 rotate-180"),
    });
    
    view! {
        <ArrowUpIcon class=arrow_class/>
    }
}

/// Component to render cancel and publish buttons for a modal Form
#[component]
pub fn Collapse<C>(
    #[prop(into)]
    title_view: ViewFnOnce,
    #[prop(default = true)]
    is_open: bool,
    children: TypedChildrenFn<C>,
) -> impl IntoView
where
    C : IntoView + 'static 
{
    let children = children.into_inner();
    let show_children = RwSignal::new(is_open);
    let children_class = move || match show_children.get() {
        true => "transition-all duration-500 overflow-hidden",
        false => "transition-all duration-500 overflow-hidden h-0",
    };
    let children_class_inner = move || match show_children.get() {
        true => "transition-all duration-500 opacity-100 visible",
        false => "transition-all duration-500 opacity-0 invisible",
    };
    
    view! {
        <div class="flex flex-col gap-1">
            <button
                class="p-1 rounded-md hover:bg-base-content/20"
                on:click=move |_| show_children.update(|value| *value = !*value)
            >
                <div class="flex justify-between items-center">
                   {title_view.run()}
                    <RotatingArrow point_up=show_children/>
                </div>
            </button>
            <div class=children_class>
                <div class=children_class_inner>
                {
                    children()
                }
                </div>
            </div>
        </div>
    }
}


/// Component to display a title with collapsable children
#[component]
pub fn TitleCollapse<C: IntoView + 'static>(
    #[prop(into)]
    title: Signal<String>,
    #[prop(default = "text- text-left font-semibold")]
    title_class: &'static str,
    #[prop(default = true)]
    is_open: bool,
    children: TypedChildrenFn<C>,
) -> impl IntoView {
    let children = children.into_inner();
    let show_children = RwSignal::new(is_open);
    let children_class = move || match show_children.get() {
        true => "transition duration-500 opacity-100 visible",
        false => "opacity-0 invisible h-0 max-h-0 overflow-hidden",
    };

    view! {
        <div class="flex flex-col shrink-0 relative">
            <button
                class="p-1 rounded-md hover:bg-base-content/20"
                on:click=move |_| show_children.update(|value| *value = !*value)
            >
                <div class="flex justify-between items-center">
                    <div class=title_class>{title}</div>
                    <RotatingArrow point_up=show_children/>
                </div>
            </button>
            <div class=children_class>
                {children()}
            </div>
        </div>
    }
}

/// Component to render a help button, displaying its children in a model window when clicked
#[component]
pub fn HelpButton<C: IntoView + 'static>(
    #[prop(default = "absolute bottom-full right-0 z-10 mb-1 -mr-1 p-2 w-86 lg:w-128 bg-base-200 rounded-sm")]
    modal_class: &'static str,
    #[prop(default = "editor-button-size")]
    icon_class: &'static str,
    children: TypedChildrenFn<C>,
) -> impl IntoView {
    let children = children.into_inner();
    let show_help = RwSignal::new(false);
    let node_ref = NodeRef::<html::Div>::new();
    #[cfg(feature = "hydrate")]
    {
        // only enable with "hydrate" to avoid server side "Dropped SendWrapper" error
        let _ = on_click_outside(node_ref, move |_| show_help.set(false));
    }

    view! {
        <div
            class="h-full w-fit rounded-full bg-base-300 relative inline-block"
            node_ref=node_ref
        >
            <Show when=show_help>
                <div class=modal_class>
                    {children()}
                </div>
            </Show>
            <button
                type="button"
                class="button-rounded-ghost p-2"
                on:click=move |_| show_help.update(|value| *value = !*value)
            >
                <HelpIcon class=icon_class/>
            </button>
        </div>
    }
}

/// Component to indicate no matching content was found
#[component]
pub fn NotFoundWidget(
    message: Signal<String>,
    #[prop(default = true)]
    is_main_content: bool
) -> impl IntoView {
    let (icon_class, not_found_class, message_class) = match is_main_content {
        true => ("h-28 w-28", "text-2xl", "text-xl"),
        false => ("h-12 w-12 xl:h-16 xl:w-16 4xl:h-22 4xl:w-22", "text-base xl:text-lg 4xl:text-xl", "text-sm xl:text-base 4xl:text-lg"),
    };
    view! {
        <div class="w-full py-2 flex items-center gap-2 justify-center">
            <NotFoundIcon class=icon_class/>
            <div class="flex flex-col max-w-2/3">
                <h2 class=not_found_class>{move_tr!("not-found-message")}</h2>
                <h3 class=message_class>{message}</h3>
            </div>
        </div>
    }
}

/// Component to display a loading indicator or error depending on the input signals
#[component]
pub fn LoadIndicators(
    #[prop(into)]
    is_loading: Signal<bool>,
    #[prop(into)]
    load_error: Signal<Option<AppError>>,
    #[prop(default = "w-full min-h-9 lg:min-h-17")]
    load_div_class: &'static str,
) -> impl IntoView {
    view! {
        <Show when=move || load_error.read().is_some()>
        {
            let error = load_error.get_untracked().unwrap();
            view! {
                <div class="flex justify-start py-4"><ErrorDisplay error/></div>
            }
        }
        </Show>
        <div class=load_div_class>
            <Show
                when=is_loading
            >
                <LoadingIcon/>
            </Show>
        </div>
    }
}

/// Component to display the content of a banner
#[component]
pub fn BannerContent(
    #[prop(into)]
    title: Signal<String>,
    icon_url: Option<String>,
    banner_url: Option<String>,
    #[prop(default = "h-8 w-8 2xl:h-12 2xl:w-12")]
    sphere_icon_class: &'static str,
) -> impl IntoView {
    let default_icon_index = title.read_untracked().as_bytes().first().cloned().unwrap_or_default();
    view! {
        <img
            src=banner_url.unwrap_or(String::from("/banner.jpg"))
            class="w-full h-full object-cover object-left"
            alt=move_tr!("sphere-banner")
        />
        <div class="absolute inset-0 flex items-center justify-center">
            <div class="p-3 backdrop-blur-sm bg-black/50 rounded-xs flex items-center gap-3 max-w-full overflow-hidden">
                <SphereIcon icon_url default_icon_index class=sphere_icon_class/>
                <span class="text-2xl 2xl:text-4xl inline-block w-fit">{title}</span>
            </div>
        </div>
    }.into_any()
}