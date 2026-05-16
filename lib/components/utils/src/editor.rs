use leptos::either::Either;
use leptos::html;
use leptos::prelude::*;
use leptos_fluent::move_tr;

use sphare_core_common::editor::{adjust_textarea_height, clear_newlines, format_textarea_content, get_styled_html_from_markdown, FormatType, TextareaData};
use sphare_core_common::traits::ToLocalizedStr;

use crate::errors::ErrorDisplay;
use crate::icons::*;
use crate::view::ToView;
use crate::widget::HelpButton;

impl ToView for FormatType {
    fn to_view(self) -> impl IntoView + 'static {
        match self {
            FormatType::Bold => view!{ <BoldIcon/> }.into_any(),
            FormatType::Italic => view!{ <ItalicIcon/> }.into_any(),
            FormatType::Strikethrough => view!{ <StrikethroughIcon/> }.into_any(),
            FormatType::Header1 => view!{ <Header1Icon/> }.into_any(),
            FormatType::Header2 => view!{ <Header2Icon/> }.into_any(),
            FormatType::List => view!{ <ListBulletIcon/> }.into_any(),
            FormatType::NumberedList => view!{ <ListNumberIcon/> }.into_any(),
            FormatType::CodeBlock => view!{ <CodeBlockIcon/> }.into_any(),
            FormatType::Spoiler => view!{ <SpoilerIcon class="editor-button-size"/> }.into_any(),
            FormatType::BlockQuote => view!{ <QuoteIcon/> }.into_any(),
            FormatType::Link => view!{ <LinkIcon/> }.into_any(),
            FormatType::Image => view!{ <ImageIcon/> }.into_any(),
        }
    }
}

/// Component to indicate the current number of characters in `content` and the maximum length
#[component]
pub fn CharLimitIndicator(
    /// Signals and node ref to control textarea content
    content: RwSignal<String>,
    /// Optional maximum text length
    #[prop(default = None)]
    maxlength: Option<usize>,
    /// css classes
    #[prop(optional)]
    class: &'static str,
) -> impl IntoView {
    view! {
        <div
            class=format!("self-end w-fit text-sm text-base-content/50 {class}")
            class=("hidden", move || maxlength.is_none() || maxlength.is_some_and(|l| content.read().len() < l*4/5))
        >
            {move || format!("{}/{}", content.read().len(), maxlength.unwrap_or(0))}
        </div>
    }
}

/// Component for an input with an optional minimum and maximum length
#[component]
pub fn LengthLimitedInput(
    #[prop(optional)]
    /// Name of the input in the form that contains this component, must correspond to the parameter of the associated server function
    name: &'static str,
    /// Placeholder for the textarea
    #[prop(into)]
    placeholder: Signal<String>,
    /// Signals and node ref to control textarea content
    content: RwSignal<String>,
    /// Set autofocus
    #[prop(default = false)]
    autofocus: bool,
    /// Set autocomplete
    #[prop(default = false)]
    autocomplete: bool,
    /// Optional minimum text length
    #[prop(default = None)]
    minlength: Option<usize>,
    /// Optional maximum text length
    #[prop(default = None)]
    maxlength: Option<usize>,
    /// Additional css classes
    #[prop(default = "w-full")]
    class: &'static str,
    /// reference to the textarea node
    #[prop(optional)]
    textarea_ref: NodeRef<html::Textarea>,
) -> impl IntoView {
    let is_length_ok = move || {
        let content_len = content.read().len();
        match (minlength, maxlength) {
            (Some(minlength), _) if content_len < minlength => false,
            (_, Some(maxlength)) if content_len > maxlength => false,
            _ => true,
        }
    };
    let autocomplete = match autocomplete {
        true => "on",
        false => "off",
    };

    Effect::new(move || {
        let content = content.read();
        if let Some(textarea_ref) = textarea_ref.get_untracked() {
            textarea_ref.set_value(&content);
        }
        adjust_textarea_height(textarea_ref);
    });

    view! {
        <div class=format!("flex flex-col gap-1 {class}")>
            <textarea
                name=name
                placeholder=placeholder
                class="input_primary resize-none"
                class=("input_error", move || !is_length_ok())
                autofocus=autofocus
                autocomplete=autocomplete
                on:input=move |ev| {
                    let input = event_target_value(&ev);
                    let input = clear_newlines(input, false);
                    content.set(input.clone());
                }
                rows=1
                minlength=minlength.map(|l| l as i32).unwrap_or(-1)
                maxlength=maxlength.map(|l| l as i32).unwrap_or(-1)
                node_ref=textarea_ref
            >
                {content}
            </textarea>
            <CharLimitIndicator content maxlength/>
        </div>
    }
}

/// Component for a textarea that can render simple text
#[component]
pub fn FormTextEditor(
    /// name of the textarea in the form that contains this component, must correspond to the parameter of the associated server function
    name: &'static str,
    /// Placeholder for the textarea
    #[prop(into)]
    placeholder: Signal<String>,
    /// Signals and node ref to control textarea content
    data: TextareaData,
    /// Optional maximum text length
    #[prop(default = None)]
    maxlength: Option<usize>,
    /// Additional css classes
    #[prop(default = "w-full")]
    class: &'static str,
    /// Indicates if a red outline should be added when the textarea is empty
    #[prop(into, default = Signal::derive(|| false))]
    is_empty_ok: Signal<bool>,
) -> impl IntoView {
    let class = format!("flex flex-col max-w-full h-full input_border_primary {class}");

    let is_border_error = move || !is_empty_ok.get() && data.content.read().is_empty();

    Effect::new(move || adjust_textarea_height(data.textarea_ref));

    view! {
        <div
            class=class
            class=("input_border_error", is_border_error)
            on:click=move |_| if let Some(textarea_ref) = data.textarea_ref.get() {
                let _ = textarea_ref.focus();
            }
        >
            <div class="w-full h-full rounded-t-lg flex items-center">
                <label for=name class="sr-only">
                    {placeholder}
                </label>
                <textarea
                    id=name
                    name=name
                    placeholder=placeholder
                    class="w-full p-2 py-3 lg:p-3 box-border outline-hidden border-none resize-none text-sm"
                    rows=1
                    on:input=move |ev| {
                        data.content.set(event_target_value(&ev));
                        adjust_textarea_height(data.textarea_ref);
                    }
                    maxlength=maxlength.map(|l| l as i32).unwrap_or(-1)
                    node_ref=data.textarea_ref
                >
                    {data.content}
                </textarea>
            </div>
            <CharLimitIndicator content=data.content maxlength class="px-1"/>
        </div>
    }.into_any()
}

/// Component for a textarea that can render markdown
#[component]
pub fn FormMarkdownEditor(
    /// name of the textarea in the form that contains this component, must correspond to the parameter of the associated server function
    name: &'static str,
    /// name of the hidden checkbox indicating whether markdown mode is enabled, must correspond to the parameter of the associated server function
    is_markdown_name: &'static str,
    /// Placeholder for the textarea
    #[prop(into)]
    placeholder: Signal<String>,
    /// Signals and node ref to control textarea content
    data: TextareaData,
    /// Initial state for markdown rendering
    #[prop(default = false)]
    is_markdown: bool,
    /// Optional maximum text length
    #[prop(default = None)]
    maxlength: Option<usize>,
    /// Indicates if a red outline should be added when the textarea is empty
    #[prop(into, default = Signal::derive(|| false))]
    is_empty_ok: Signal<bool>,
    /// Additional css classes
    #[prop(default = "w-full")]
    class: &'static str,
) -> impl IntoView {
    let is_markdown_mode = RwSignal::new(is_markdown);
    let is_markdown_mode_string = move || is_markdown_mode.get().to_string();
    let markdown_button_class = move || match is_markdown_mode.get() {
        true => "button-primary flex items-center p-2 tooltip",
        false => "button-ghost flex items-center p-2 tooltip",
    };

    let markdown_render = move || {
        match is_markdown_mode.get() {
            true => {
                get_styled_html_from_markdown(&*data.content.read())
            },
            false => Ok(String::default())
        }
    };

    Effect::new(move || {
        data.content.read();
        adjust_textarea_height(data.textarea_ref);
    });

    let is_border_error = move || !is_empty_ok.get() && data.content.read().is_empty();

    view! {
        <div class=format!("flex flex-col gap-2 {class}")>
            <div
                class="flex flex-col w-full max-w-full p-1 lg:p-2 input_border_primary"
                class=("input_border_error", is_border_error)
            >
                <div class="w-full rounded-t-lg">
                    <label for=name class="sr-only">
                        {placeholder}
                    </label>
                    <textarea
                        id=name
                        name=name
                        placeholder=placeholder
                        class="w-full box-border bg-base-100 p-1 outline-hidden resize-none text-sm"
                        rows=1
                        autofocus
                        on:input=move |ev| {
                            data.content.set(event_target_value(&ev));
                            adjust_textarea_height(data.textarea_ref);
                        }
                        maxlength=maxlength.map(|l| l as i32).unwrap_or(-1)
                        node_ref=data.textarea_ref
                    >
                        {data.content}
                    </textarea>
                </div>
                <CharLimitIndicator content=data.content maxlength class="px-1"/>
                <div class="flex justify-between items-center mt-1">
                    <div class="flex items-stretch bg-base-300 rounded-xs">
                        <label class="flex">
                            <input
                                type="text"
                                class="hidden"
                                name=is_markdown_name
                                value=is_markdown_mode_string
                                on:click=move |_| is_markdown_mode.update(|value| *value = !*value)
                            />
                            <div class=markdown_button_class data-tip=move_tr!("markdown")>
                                <MarkdownIcon/>
                            </div>
                        </label>
                        <FormatButton format_type=FormatType::Bold data is_markdown_mode/>
                        <FormatButton format_type=FormatType::Italic data is_markdown_mode/>
                        <FormatButton format_type=FormatType::Strikethrough data is_markdown_mode/>
                        <FormatButton format_type=FormatType::Header1 data is_markdown_mode hide_for_mobile=true/>
                        <FormatButton format_type=FormatType::Header2 data is_markdown_mode hide_for_mobile=true/>
                        <FormatButton format_type=FormatType::List data is_markdown_mode/>
                        <FormatButton format_type=FormatType::NumberedList data is_markdown_mode hide_for_mobile=true/>
                        <FormatButton format_type=FormatType::CodeBlock data is_markdown_mode/>
                        <FormatButton format_type=FormatType::Spoiler data is_markdown_mode/>
                        <FormatButton format_type=FormatType::BlockQuote data is_markdown_mode/>
                        <FormatButton format_type=FormatType::Link data is_markdown_mode/>
                        <FormatButton format_type=FormatType::Image data is_markdown_mode hide_for_mobile=true/>
                    </div>
                    <MarkdownHelpButton/>
                </div>
            </div>
            <Show when=is_markdown_mode>
                { move || match markdown_render() {
                    Ok(markdown_as_html) => Either::Left(view! {
                        <div class="w-full max-w-full min-h-24 max-h-96 overflow-auto overscroll-auto p-2 border border-primary bg-base-100 break-words text-sm"
                            inner_html=markdown_as_html
                        />
                    }),
                    Err(e) => Either::Right(view! { <ErrorDisplay error=e/> })
                }}
            </Show>
        </div>
    }.into_any()
}

/// Component to format the selected text in the given textarea
#[component]
pub fn FormatButton(
    /// Signals and node ref to control textarea content
    data: TextareaData,
    /// signal indicating whether markdown rendering is activated
    is_markdown_mode: RwSignal<bool>,
    /// format operation of the button
    format_type: FormatType,
    /// boolean indicating if the button is visible in mobile mode
    #[prop(optional)]
    hide_for_mobile: bool,
) -> impl IntoView {
    let class = match hide_for_mobile {
        true => "button-ghost tooltip p-2 max-lg:hidden",
        false => "button-ghost tooltip p-2",
    };
    view! {
        <button
            type="button"
            class=class
            data-tip=format_type.to_localized_str()
            on:click=move |_| {
                if let Some(textarea_ref) = data.textarea_ref.get_untracked() {
                    let selection_start = textarea_ref.selection_start();
                    let selection_end = textarea_ref.selection_end();
                    match (selection_start, selection_end) {
                        (Ok(Some(selection_start)), Ok(Some(selection_end))) => {
                            let selection_start = selection_start as usize;
                            let selection_end = selection_end as usize;
                            let cursor_position = format_textarea_content(
                                &mut data.content.write(),
                                selection_start,
                                selection_end,
                                format_type,
                            );
                            textarea_ref.set_value(&*data.content.read_untracked());
                            if !is_markdown_mode.get_untracked() {
                                is_markdown_mode.set(true);
                            }
                            let _ = textarea_ref.focus();
                            if let Some(position) = cursor_position {
                                let _ = textarea_ref.set_selection_start(Some(position as u32));
                                let _ = textarea_ref.set_selection_end(Some(position as u32));
                            }
                        },
                        _ => log::debug!("Failed to get textarea selections."),
                    };
                }
            }
        >
            {format_type.to_view()}
        </button>
    }.into_any()
}

/// Component to render editor's help button
#[component]
pub fn MarkdownHelpButton() -> impl IntoView {
    view! {
        <HelpButton>
            <div class="relative flex flex-col gap-2 leading-snug text-justify text-xs lg:text-sm">
                <p>
                    {move_tr!("markdown-help-1")}
                    <span class="inline-flex align-bottom w-fit p-1 mt-1 rounded-md bg-base-content/20"><MarkdownIcon/></span>
                </p>
                <p>
                    {move_tr!("markdown-help-2")}
                    <a class="link text-primary" href="https://github.github.com/gfm/" >"GitHub Flavored Markdown"</a>
                    {move_tr!("markdown-help-3")}
                    <span class="inline-flex align-bottom w-fit p-1 mt-1 rounded-md bg-base-content/20"><SpoilerIcon/></span>
                </p>
            </div>
        </HelpButton>
    }
}