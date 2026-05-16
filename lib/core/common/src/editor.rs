use std::io::Cursor;

use leptos::html::Textarea;
use leptos::prelude::*;
use leptos_fluent::move_tr;
use markdown::Options;
use quick_xml::events::{BytesEnd, BytesStart, BytesText, Event};
use quick_xml::{Reader, Writer};

use crate::constants::SPOILER_TAG;
use crate::errors::AppError;
use crate::traits::ToLocalizedStr;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FormatType {
    Bold,
    Italic,
    Strikethrough,
    Header1,
    Header2,
    List,
    NumberedList,
    CodeBlock,
    Spoiler,
    BlockQuote,
    Link,
    Image,
}

#[derive(Clone, Copy, Debug)]
pub struct TextareaData {
    pub content: RwSignal<String>,
    pub textarea_ref: NodeRef<Textarea>
}

impl ToLocalizedStr for FormatType {
    fn to_localized_str(&self) -> Signal<String> {
        match self {
            FormatType::Bold => move_tr!("bold"),
            FormatType::Italic => move_tr!("italic"),
            FormatType::Strikethrough => move_tr!("strikethrough"),
            FormatType::Header1 => move_tr!("header_1"),
            FormatType::Header2 => move_tr!("header_2"),
            FormatType::List => move_tr!("list"),
            FormatType::NumberedList => move_tr!("numbered_list"),
            FormatType::CodeBlock => move_tr!("code_block"),
            FormatType::Spoiler => move_tr!("spoiler"),
            FormatType::BlockQuote => move_tr!("block_quote"),
            FormatType::Link => move_tr!("link"),
            FormatType::Image => move_tr!("image"),
        }
    }
}

#[cfg(feature = "ssr")]
pub mod ssr {
    use crate::editor::get_styled_html_from_markdown;
    use crate::errors::AppError;

    pub fn get_html_and_markdown_strings(body: &str, is_markdown: bool) -> Result<(String, Option<&str>), AppError> {
        match is_markdown {
            true => Ok((
                get_styled_html_from_markdown(body)?,
                Some(body),
            )),
            false => Ok((String::from(body), None)),
        }
    }
}

pub fn get_styled_html_from_markdown(
    markdown_input: &str,
) -> Result<String, AppError> {
    let html_from_markdown = markdown::to_html_with_options(
        markdown_input,
        &Options::gfm()
    ).map_err(AppError::new)?;
    log::debug!("Markdown as html: {html_from_markdown}");

    // Add styling, will be done by parsing the html which is a bit ugly. Would be better
    // if the styling could be added directly when generating the html from markdown
    let styled_html_output = style_html_user_content(html_from_markdown.as_str())?;
    Ok(styled_html_output)
}

pub fn style_html_user_content(user_content: &str) -> Result<String, AppError> {
    let mut reader = Reader::from_str(user_content);
    let mut writer = Writer::new(Cursor::new(Vec::new()));
    let mut is_in_block = false;

    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) => {
                let mut elem = e.into_owned();
                match elem.name().as_ref() {
                    b"h1" => elem.push_attribute(("class", "text-4xl mb-3")),
                    b"h2" => elem.push_attribute(("class", "text-2xl mb-3")),
                    b"h3" => elem.push_attribute(("class", "text-xl mb-2.5")),
                    b"p" if !is_in_block => elem.push_attribute(("class", "mb-2.5")),
                    b"a" => elem.push_attribute(("class", "link text-primary")),
                    b"ul" => elem.push_attribute(("class", "list-inside list-disc mb-2.5")),
                    b"ol" => elem.push_attribute(("class", "list-inside list-decimal mb-2.5")),
                    b"code" => {
                        elem.push_attribute(("class", "block w-fit rounded-md bg-black px-1 py-0.5 mx-0.5 mb-2.5"))
                    }
                    b"table" => elem.push_attribute(("class", "table mb-2.5")),
                    b"blockquote" => {
                        is_in_block = true;
                        elem.push_attribute((
                            "class",
                            "w-fit p-1 mb-2.5 border-s-4 rounded-sm border-slate-400 bg-slate-600",
                        ))
                    },
                    _ => (),
                }

                // writes the event to the writer
                writer.write_event(Event::Start(elem))?;
            }
            Ok(Event::End(e)) => {
                if e.name().as_ref() == b"blockquote" {
                    is_in_block = false;
                }
                writer.write_event(Event::End(e))?;
            }
            Ok(Event::Empty(e)) => {
                let mut elem = e.into_owned();

                if elem.name().as_ref() == b"hr" {
                    elem.push_attribute(("class", "my-2"))
                }
                // writes the event to the writer
                writer.write_event(Event::Empty(elem))?;
            }
            Ok(Event::Text(e)) => {
                let text = e.decode().map_err(|e| AppError::new(format!("Error while decoding text: {e}")))?.into_owned();
                let spoiler_split_text = text.split(SPOILER_TAG);
                let mut is_current_text_spoiler = None;
                for text in spoiler_split_text {
                    let is_spoiler_text = is_current_text_spoiler.unwrap_or_default();
                    if !text.is_empty() {
                        if is_spoiler_text {
                            // Add label to encapsulate spoilers and a checkbox to toggle them
                            let label = BytesStart::new("label");
                            writer.write_event(Event::Start(label))?;
                            // Add invisible checkbox to toggle spoilers
                            let mut checkbox_elem = BytesStart::new("input");
                            checkbox_elem.push_attribute(("type", "checkbox"));
                            checkbox_elem.push_attribute(("class", "spoiler-checkbox hidden"));
                            writer.write_event(Event::Empty(checkbox_elem))?;

                            let mut span = BytesStart::new("span");
                            span.push_attribute(("class", "transition-all duration-300 ease-in-out rounded-md bg-white p-0.5 px-1 mx-0.5 text-white spoiler-text"));
                            writer.write_event(Event::Start(span))?;

                            writer.write_event(Event::Text(BytesText::new(text.trim())))?;

                            let span_end = BytesEnd::new("span");
                            writer.write_event(Event::End(span_end))?;

                            let label_end = BytesEnd::new("label");
                            writer.write_event(Event::End(label_end))?;
                        } else {
                            writer.write_event(Event::Text(BytesText::new(text)))?;
                        }
                    }
                    is_current_text_spoiler = Some(!is_spoiler_text);
                }
            }
            Ok(Event::Eof) => break,
            // we can either move or borrow the event to write, depending on your use-case
            Ok(e) => writer.write_event(e)?,
            Err(e) => {
                log::error!(
                        "Error while parsing xml at position {}: {:?}",
                        reader.buffer_position(),
                        e
                    );
                return Err(AppError::from(e));
            }
        }
    }

    let styled_html_output = String::from_utf8(writer.into_inner().into_inner())?;
    log::debug!("Styled html: {styled_html_output}");
    Ok(styled_html_output)
}

/// Formats the input `content` with the Markdown syntax corresponding to `format_type`
/// If no text is selected, returns the position to set the cursor at
pub fn format_textarea_content(
    content: &mut String,
    mut selection_start: usize,
    mut selection_end: usize,
    format_type: FormatType,
) -> Option<usize> {
    let selected_content = &content[selection_start..selection_end];
    let leading_whitespace = selected_content
        .chars()
        .take_while(|ch| ch.is_whitespace())
        .count();
    let ending_whitespace = selected_content
        .chars()
        .rev()
        .take_while(|ch| ch.is_whitespace())
        .count();

    selection_start += leading_whitespace;
    selection_end -= ending_whitespace;

    let text_offset = match format_type {
        FormatType::Bold => {
            content.insert_str(selection_end, "**");
            content.insert_str(selection_start, "**");
            2
        },
        FormatType::Italic => {
            content.insert(selection_end, '*');
            content.insert(selection_start, '*');
            1
        },
        FormatType::Strikethrough => {
            content.insert_str(selection_end, "~~");
            content.insert_str(selection_start, "~~");
            2
        },
        FormatType::Header1 => {
            content.insert_str(get_line_start_for_position(content, selection_start), "# ");
            2
        },
        FormatType::Header2 => {
            content.insert_str(get_line_start_for_position(content, selection_start), "## ");
            3
        },
        FormatType::List => {
            content.insert_str(get_line_start_for_position(content, selection_start), "* ");
            2
        },
        FormatType::NumberedList => {
            content.insert_str(get_line_start_for_position(content, selection_start), "1. ");
            3
        },
        FormatType::CodeBlock => {
            content.insert_str(selection_end, "```");
            content.insert_str(selection_start, "```");
            3
        },
        FormatType::Spoiler => {
            content.insert_str(selection_end, SPOILER_TAG);
            content.insert_str(selection_start, SPOILER_TAG);
            SPOILER_TAG.len()
        },
        FormatType::BlockQuote => {
            content.insert_str(get_line_start_for_position(content, selection_start), "> ");
            2
        },
        FormatType::Link if selection_start == selection_end => {
            content.insert_str(
                selection_start,
                "[link text](https://www.your_link.com)",
            );
            1
        },
        FormatType::Link => {
            content.insert(selection_end, ')');
            content.insert_str(
                selection_start,
                "[link text](",
            );
            1
        },
        FormatType::Image if selection_start == selection_end => {
            content.insert_str(
                selection_start,
                "![](https://image_url.png)",
            );
            2
        },
        FormatType::Image => {
            content.insert(selection_end, ')');
            content.insert_str(
                selection_start,
                "![](",
            );
            2
        },
    };

    (
        selection_start == selection_end ||
        format_type == FormatType::Link ||
        format_type == FormatType::Image
    ).then_some(selection_start + text_offset)
}

/// Given the input String, returns the starting byte index of the line containing the [position] byte index.
fn get_line_start_for_position(string: &str, position: usize) -> usize {
    match string[..position].rfind('\n') {
        Some(line_start) => line_start + 1,
        None => 0,
    }
}

/// Returns input `string` without any newlines.
///
/// ```
/// use sphare_core_common::editor::clear_newlines;
///
/// assert_eq!(clear_newlines(String::from("test"), false), String::from("test"));
/// assert_eq!(clear_newlines(String::from("test\r\nsecond line\nthird line"), true), String::from("test  second line third line"));
/// assert_eq!(clear_newlines(String::from("test\r\nsecond line\nthird line"), false), String::from("testsecond linethird line"));
/// ```
pub fn clear_newlines(string: String, add_whitespace: bool) -> String {
    string.replace(
        &['\r', '\n'][..],
        match add_whitespace {
            true => " ",
            false => "",
        }
    )
}

/// Adjust the height of `textarea_ref` so that all its content is displayed without a scrollbar.
pub fn adjust_textarea_height(textarea_ref: NodeRef<Textarea>) {
    if let Some(textarea_ref) = textarea_ref.get() {
        // First get the scroll height, as it seems in some case (in a suspense?) the height is set to 0 otherwise
        let _ = textarea_ref.scroll_height();
        textarea_ref.style(("height", "auto"));
        let scroll_height = format!("{}px", textarea_ref.scroll_height());
        textarea_ref.style(("height", scroll_height));
    }
}

#[cfg(test)]
mod tests {
    use indoc::indoc;
    use leptos::prelude::ServerFnError;

    use crate::editor::ssr::get_html_and_markdown_strings;
    use crate::editor::{format_textarea_content, get_styled_html_from_markdown, style_html_user_content, FormatType};

    #[test]
    fn test_get_html_and_markdown_strings() -> Result<(), ServerFnError> {
        let text_body = "hello world";
        let markdown_body = "#this is a header";
        
        let (html_text_body, markdown_text_body) = get_html_and_markdown_strings(
            text_body,
            false
        ).expect("Should get text body");
        assert_eq!(html_text_body, text_body);
        assert_eq!(markdown_text_body, None);

        let (html_markdown_body, markdown_markdown_body) = get_html_and_markdown_strings(
            markdown_body,
            true
        ).expect("Should get text body");
        assert_eq!(html_markdown_body, get_styled_html_from_markdown(markdown_body).expect("Should get html body"));
        assert_eq!(markdown_markdown_body, Some(markdown_body));
        
        Ok(())
    }

    #[test]
    fn test_style_html_user_content() -> Result<(), ServerFnError> {
        assert_eq!(
            style_html_user_content("<h1></h1>")?,
            r#"<h1 class="text-4xl mb-3"></h1>"#
        );
        assert_eq!(
            style_html_user_content("<h2></h2>")?,
            r#"<h2 class="text-2xl mb-3"></h2>"#
        );
        assert_eq!(
            style_html_user_content("<h3></h3>")?,
            r#"<h3 class="text-xl mb-2.5"></h3>"#
        );
        assert_eq!(
            style_html_user_content("<a></a>")?,
            r#"<a class="link text-primary"></a>"#
        );
        assert_eq!(
            style_html_user_content("<ul></ul>")?,
            r#"<ul class="list-inside list-disc mb-2.5"></ul>"#
        );
        assert_eq!(
            style_html_user_content("<ol></ol>")?,
            r#"<ol class="list-inside list-decimal mb-2.5"></ol>"#
        );
        assert_eq!(
            style_html_user_content("<code></code>")?,
            r#"<code class="block w-fit rounded-md bg-black px-1 py-0.5 mx-0.5 mb-2.5"></code>"#
        );
        assert_eq!(
            style_html_user_content("<table></table>")?,
            r#"<table class="table mb-2.5"></table>"#
        );
        assert_eq!(
            style_html_user_content("<blockquote></blockquote>")?,
            r#"<blockquote class="w-fit p-1 mb-2.5 border-s-4 rounded-sm border-slate-400 bg-slate-600"></blockquote>"#
        );
        assert_eq!(style_html_user_content("<hr/>")?, r#"<hr class="my-2"/>"#);
        assert_eq!(
            style_html_user_content("<p>Test, || This is a spoiler || this is not a spoiler</p>")?,
            r#"<p class="mb-2.5">Test, <label><input type="checkbox" class="spoiler-checkbox hidden"/><span class="transition-all duration-300 ease-in-out rounded-md bg-white p-0.5 px-1 mx-0.5 text-white spoiler-text">This is a spoiler</span></label> this is not a spoiler</p>"#
        );
        Ok(())
    }

    #[test]
    fn test_get_styled_html_from_markdown() -> Result<(), ServerFnError> {
        let markdown = indoc! {r#"
            # Here is a comment with markdown
            ## header 2
            ### header 3
            #### header 4
            ---
        "#};
        let expected_html = indoc! {r#"
            <h1 class="text-4xl mb-3">Here is a comment with markdown</h1>
            <h2 class="text-2xl mb-3">header 2</h2>
            <h3 class="text-xl mb-2.5">header 3</h3>
            <h4>header 4</h4>
            <hr  class="my-2"/>
        "#};
        assert_eq!(
            get_styled_html_from_markdown(markdown)?,
            expected_html
        );

        let markdown = indoc! {r#"
            `code blocks`
        "#};
        let expected_html = indoc! {r#"
            <p class="mb-2.5"><code class="block w-fit rounded-md bg-black px-1 py-0.5 mx-0.5 mb-2.5">code blocks</code></p>
        "#};
        assert_eq!(
            get_styled_html_from_markdown(markdown)?,
            expected_html
        );

        let markdown = indoc! {r#"
            || Spoilers ||
        "#};
        let expected_html = indoc! {r#"
            <p class="mb-2.5"><label><input type="checkbox" class="spoiler-checkbox hidden"/><span class="transition-all duration-300 ease-in-out rounded-md bg-white p-0.5 px-1 mx-0.5 text-white spoiler-text">Spoilers</span></label></p>
        "#};
        assert_eq!(
            get_styled_html_from_markdown(markdown)?,
            expected_html
        );

        let markdown = indoc! {r#"
            **bold**, *italic*, combined emphasis with **asterisks and _underscores_**.
        "#};
        let expected_html = indoc! {r#"
            <p class="mb-2.5"><strong>bold</strong>, <em>italic</em>, combined emphasis with <strong>asterisks and <em>underscores</em></strong>.</p>
        "#};
        assert_eq!(
            get_styled_html_from_markdown(markdown)?,
            expected_html
        );

        let markdown = indoc! {r#"
            Strikethrough uses two tildes. ~~Scratch this.~~
        "#};
        let expected_html = indoc! {r#"
            <p class="mb-2.5">Strikethrough uses two tildes. <del>Scratch this.</del></p>
        "#};
        assert_eq!(
            get_styled_html_from_markdown(markdown)?,
            expected_html
        );

        let markdown = indoc! {r#"
            > We can also do blockquotes
        "#};
        let expected_html = indoc! {r#"
            <blockquote class="w-fit p-1 mb-2.5 border-s-4 rounded-sm border-slate-400 bg-slate-600">
            <p>We can also do blockquotes</p>
            </blockquote>
        "#};
        assert_eq!(
            get_styled_html_from_markdown(markdown)?,
            expected_html
        );

        let markdown = indoc! {r#"
            1. lists
            2. with numbers

            * lists
            * without numbers
            * and as many elements as we want
        "#};
        let expected_html = indoc! {r#"
            <ol class="list-inside list-decimal mb-2.5">
            <li>lists</li>
            <li>with numbers</li>
            </ol>
            <ul class="list-inside list-disc mb-2.5">
            <li>lists</li>
            <li>without numbers</li>
            <li>and as many elements as we want</li>
            </ul>
        "#};
        assert_eq!(
            get_styled_html_from_markdown(markdown)?,
            expected_html
        );

        let markdown = indoc! {r#"
            \
            Also, a bit more work is needed to add an empty line.
        "#};
        let expected_html = indoc! {r#"
            <p class="mb-2.5"><br />
            Also, a bit more work is needed to add an empty line.</p>
        "#};
        assert_eq!(
            get_styled_html_from_markdown(markdown)?,
            expected_html
        );

        let markdown = indoc! {r#"
            Finally, we can add links [link text](https://www.example.com), images ![alt text](https://github.com/adam-p/markdown-here/raw/master/src/utils/images/icon48.png "Logo Title Text 1")
        "#};
        let expected_html = indoc! {r#"
            <p class="mb-2.5">Finally, we can add links <a href="https://www.example.com" class="link text-primary">link text</a>, images <img src="https://github.com/adam-p/markdown-here/raw/master/src/utils/images/icon48.png" alt="alt text" title="Logo Title Text 1" /></p>
        "#};
        assert_eq!(
            get_styled_html_from_markdown(markdown)?,
            expected_html
        );

        let markdown = indoc! {r#"
            | Tables        | Are           | Cool  |
            | ------------- |:-------------:| -----:|
            | col 3 is      | right-aligned | $1600 |
            | col 2 is      | centered      |   $12 |
            | zebra stripes | are neat      |    $1 |
        "#};
        let expected_html = indoc! {r#"
            <table class="table mb-2.5">
            <thead>
            <tr>
            <th>Tables</th>
            <th align="center">Are</th>
            <th align="right">Cool</th>
            </tr>
            </thead>
            <tbody>
            <tr>
            <td>col 3 is</td>
            <td align="center">right-aligned</td>
            <td align="right">$1600</td>
            </tr>
            <tr>
            <td>col 2 is</td>
            <td align="center">centered</td>
            <td align="right">$12</td>
            </tr>
            <tr>
            <td>zebra stripes</td>
            <td align="center">are neat</td>
            <td align="right">$1</td>
            </tr>
            </tbody>
            </table>
        "#};
        assert_eq!(
            get_styled_html_from_markdown(markdown)?,
            expected_html
        );

        Ok(())
    }

    #[test]
    fn test_format_textarea_content() {
        // Bold
        let mut content = String::from("This is some user text ");
        let cursor_position = format_textarea_content(&mut content, 23, 23, FormatType::Bold);
        assert_eq!(cursor_position, Some(25));
        assert_eq!(content, "This is some user text ****");

        let mut content = String::from("This is some user text ");
        let cursor_position = format_textarea_content(&mut content, 8, 12, FormatType::Bold);
        assert_eq!(cursor_position, None);
        assert_eq!(content, "This is **some** user text ");

        // Italic
        let mut content = String::from("This is some user text ");
        let cursor_position = format_textarea_content(&mut content, 23, 23, FormatType::Italic);
        assert_eq!(cursor_position, Some(24));
        assert_eq!(content, "This is some user text **");

        let mut content = String::from("This is some user text ");
        let cursor_position = format_textarea_content(&mut content, 8, 12, FormatType::Italic);
        assert_eq!(cursor_position, None);
        assert_eq!(content, "This is *some* user text ");

        // Strikethrough,
        let mut content = String::from("This is some user text ");
        let cursor_position = format_textarea_content(&mut content, 23, 23, FormatType::Strikethrough);
        assert_eq!(cursor_position, Some(25));
        assert_eq!(content, "This is some user text ~~~~");

        let mut content = String::from("This is some user text ");
        let cursor_position = format_textarea_content(&mut content, 8, 12, FormatType::Strikethrough);
        assert_eq!(cursor_position, None);
        assert_eq!(content, "This is ~~some~~ user text ");

        // Header1
        let mut content = String::from("This is some user text ");
        let cursor_position = format_textarea_content(&mut content, 23, 23, FormatType::Header1);
        assert_eq!(cursor_position, Some(25));
        assert_eq!(content, "# This is some user text ");

        let mut content = String::from("This is some user text ");
        let cursor_position = format_textarea_content(&mut content, 8, 12, FormatType::Header1);
        assert_eq!(cursor_position, None);
        assert_eq!(content, "# This is some user text ");

        // Header2
        let mut content = String::from("This is some user text ");
        let cursor_position = format_textarea_content(&mut content, 23, 23, FormatType::Header2);
        assert_eq!(cursor_position, Some(26));
        assert_eq!(content, "## This is some user text ");

        let mut content = String::from("This is some user text ");
        let cursor_position = format_textarea_content(&mut content, 8, 12, FormatType::Header2);
        assert_eq!(cursor_position, None);
        assert_eq!(content, "## This is some user text ");

        // List
        let mut content = String::from("This is some user text ");
        let cursor_position = format_textarea_content(&mut content, 23, 23, FormatType::List);
        assert_eq!(cursor_position, Some(25));
        assert_eq!(content, "* This is some user text ");

        let mut content = String::from("This is some user text ");
        let cursor_position = format_textarea_content(&mut content, 8, 12, FormatType::List);
        assert_eq!(cursor_position, None);
        assert_eq!(content, "* This is some user text ");

        // NumberedList
        let mut content = String::from("This is some user text ");
        let cursor_position = format_textarea_content(&mut content, 23, 23, FormatType::NumberedList);
        assert_eq!(cursor_position, Some(26));
        assert_eq!(content, "1. This is some user text ");

        let mut content = String::from("This is some user text ");
        let cursor_position = format_textarea_content(&mut content, 8, 12, FormatType::NumberedList);
        assert_eq!(cursor_position, None);
        assert_eq!(content, "1. This is some user text ");

        // CodeBlock
        let mut content = String::from("This is some user text ");
        let cursor_position = format_textarea_content(&mut content, 23, 23, FormatType::CodeBlock);
        assert_eq!(cursor_position, Some(26));
        assert_eq!(content, "This is some user text ``````");

        let mut content = String::from("This is some user text ");
        let cursor_position = format_textarea_content(&mut content, 8, 12, FormatType::CodeBlock);
        assert_eq!(cursor_position, None);
        assert_eq!(content, "This is ```some``` user text ");

        // Spoiler
        let mut content = String::from("This is some user text ");
        let cursor_position = format_textarea_content(&mut content, 23, 23, FormatType::Spoiler);
        assert_eq!(cursor_position, Some(25));
        assert_eq!(content, "This is some user text ||||");

        let mut content = String::from("This is some user text ");
        let cursor_position = format_textarea_content(&mut content, 8, 12, FormatType::Spoiler);
        assert_eq!(cursor_position, None);
        assert_eq!(content, "This is ||some|| user text ");

        // BlockQuote
        let mut content = String::from("This is some user text ");
        let cursor_position = format_textarea_content(&mut content, 23, 23, FormatType::BlockQuote);
        assert_eq!(cursor_position, Some(25));
        assert_eq!(content, "> This is some user text ");

        let mut content = String::from("This is some user text ");
        let cursor_position = format_textarea_content(&mut content, 8, 12, FormatType::BlockQuote);
        assert_eq!(cursor_position, None);
        assert_eq!(content, "> This is some user text ");

        // Link
        let mut content = String::from("This is some user text ");
        let cursor_position = format_textarea_content(&mut content, 23, 23, FormatType::Link);
        assert_eq!(cursor_position, Some(24));
        assert_eq!(content, "This is some user text [link text](https://www.your_link.com)");

        let mut content = String::from("This is some user text ");
        let cursor_position = format_textarea_content(&mut content, 8, 12, FormatType::Link);
        assert_eq!(cursor_position, Some(9));
        assert_eq!(content, "This is [link text](some) user text ");

        // Image
        let mut content = String::from("This is some user text ");
        let cursor_position = format_textarea_content(&mut content, 23, 23, FormatType::Image);
        assert_eq!(cursor_position, Some(25));
        assert_eq!(content, "This is some user text ![](https://image_url.png)");

        let mut content = String::from("This is some user text ");
        let cursor_position = format_textarea_content(&mut content, 8, 12, FormatType::Image);
        assert_eq!(cursor_position, Some(10));
        assert_eq!(content, "This is ![](some) user text ");
    }
}
