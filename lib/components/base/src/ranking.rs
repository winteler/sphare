use leptos::children::ChildrenFn;
use leptos::prelude::*;
use leptos::server_fn::const_format::concatcp;
use leptos::{component, view, IntoView};
use leptos_fluent::move_tr;

use sphare_core_content::ranking::{CommentSortType, PostSortType, SortType};

use sphare_cmp_utils::icons::{FlameIcon, GraphIcon, HourglassIcon, PodiumIcon};

/// Component to show a sorting option
#[component]
pub fn SortWidgetOption(
    sort_type: SortType,
    sort_signal: RwSignal<SortType>,
    #[prop(into)]
    datatip: Signal<String>,
    is_tooltip_bottom: bool,
    children: ChildrenFn,
) -> impl IntoView {
    let is_selected = move || sort_signal.read() == sort_type;
    let class = move || {
        match is_selected() {
            true => "button-ghost !rounded-none px-2 xl:px-3 border border-1 border-white join-item",
            false => "button-ghost !rounded-none px-2 xl:px-3 border border-1 border-base-100 hover:border-white join-item",
        }
    };
    const BASE_CLASS: &str = "rounded-none tooltip";
    let tooltip_class = match is_tooltip_bottom {
        true => concatcp!(BASE_CLASS, " tooltip-bottom"),
        false => BASE_CLASS,
    };

    view! {
        <div class=tooltip_class data-tip=datatip>
            <button
                class=class
                on:click=move |_| {
                    if sort_signal.get_untracked() != sort_type {
                        sort_signal.set(sort_type);
                    }
                }
            >
                {children()}
            </button>
        </div>
    }.into_any()
}

/// Component to indicate how to sort posts
#[component]
pub fn PostSortWidget(
    sort_signal: RwSignal<SortType>,
    #[prop(optional)]
    is_tooltip_bottom: bool,
) -> impl IntoView {
    view! {
        <div class="join rounded-none w-fit">
            <SortWidgetOption sort_type=SortType::Post(PostSortType::Hot) sort_signal datatip=move_tr!("hot") is_tooltip_bottom>
                <FlameIcon/>
            </SortWidgetOption>
            <SortWidgetOption sort_type=SortType::Post(PostSortType::Trending) sort_signal datatip=move_tr!("trending") is_tooltip_bottom>
                <GraphIcon/>
            </SortWidgetOption>
            <SortWidgetOption sort_type=SortType::Post(PostSortType::Best) sort_signal datatip=move_tr!("best") is_tooltip_bottom>
                <PodiumIcon/>
            </SortWidgetOption>
            <SortWidgetOption sort_type=SortType::Post(PostSortType::Recent) sort_signal datatip=move_tr!("recent") is_tooltip_bottom>
                <HourglassIcon/>
            </SortWidgetOption>
        </div>
    }.into_any()
}

/// Component to indicate how to sort comments
#[component]
pub fn CommentSortWidget(
    sort_signal: RwSignal<SortType>
) -> impl IntoView {
    view! {
        <div class="join rounded-none w-fit">
            <SortWidgetOption sort_type=SortType::Comment(CommentSortType::Best) sort_signal datatip=move_tr!("best") is_tooltip_bottom=true>
                <PodiumIcon/>
            </SortWidgetOption>
            <SortWidgetOption sort_type=SortType::Comment(CommentSortType::Recent) sort_signal datatip=move_tr!("recent") is_tooltip_bottom=true>
                <HourglassIcon/>
            </SortWidgetOption>
        </div>
    }.into_any()
}