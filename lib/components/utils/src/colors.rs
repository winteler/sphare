use leptos::html;
use leptos::prelude::*;
use strum::IntoEnumIterator;

#[cfg(feature = "hydrate")]
use leptos_use::on_click_outside;

use sphare_core_common::colors::Color;

use crate::widget::RotatingArrow;

/// Component to display a color
#[component]
pub fn ColorIndicator(
    #[prop(into)]
    color: Signal<Color>,
    #[prop(default = "w-4 h-4 rounded-full")]
    class: &'static str,
) -> impl IntoView {
    let color_class = move || format!("{class} {}", color.get().to_bg_class());
    view! {
        <div class=color_class></div>
    }
}

/// Component to select a color
#[component]
pub fn ColorSelect(
    /// Name of the input in the form that contains this component, must correspond to the parameter of the associated server function
    name: &'static str,
    color_input: RwSignal<Color>,
    /// Label of the select
    #[prop(default = "")]
    label: &'static str,
    #[prop(default = "h-full")]
    class: &'static str,
) -> impl IntoView {
    let show_dropdown = RwSignal::new(false);
    let color_string = move || color_input.get().to_string();
    let div_class = format!("flex {class}");
    let dropdown_ref = NodeRef::<html::Div>::new();
    #[cfg(feature = "hydrate")]
    {
        // only enable with "hydrate" to avoid server side "Dropped SendWrapper" error
        let _ = on_click_outside(dropdown_ref, move |_| show_dropdown.set(false));
    }

    let label_view = match label.is_empty() {
        true => ().into_any(),
        false => view! { <span class="pr-1 label-text">{label}</span> }.into_any()
    };

    view! {
        <div class=div_class node_ref=dropdown_ref>
            <input type="text" name=name value=color_string class="hidden"/>
            {label_view}
            <div class="h-full w-full relative">
                <div
                    class="h-full flex justify-evenly items-center lg:gap-1 hover:bg-base-content/20 input_border_primary"
                    on:click=move |_| show_dropdown.update(|value| *value = !*value)
                >
                    <div class="p-1 h-fit w-fit">
                        <ColorIndicator color=color_input/>
                    </div>
                    <RotatingArrow point_up=show_dropdown class="h-2 w-2"/>
                </div>
                <Show when=show_dropdown>
                    <div class="absolute z-40 origin-bottom-left">
                        <div class="grid grid-cols-3 gap-1 shadow-sm bg-base-100 rounded-sm mt-1 w-28">
                        { move || {
                            Color::iter().map(|color: Color| {
                                view! {
                                    <div class="w-fit rounded-sm hover:bg-base-200 px-2 py-1 h-fit w-fit" on:click=move |_| {
                                        color_input.set(color);
                                        show_dropdown.set(false);
                                    }>
                                        <ColorIndicator color/>
                                    </div>
                                }.into_any()
                            }).collect_view()
                        }}
                        </div>
                    </div>
                </Show>
            </div>
        </div>
    }
}