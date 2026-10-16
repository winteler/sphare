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

const H1_CLASS: &str = "text-4xl mb-3";
const H2_CLASS: &str = "text-2xl mb-3";
const H3_CLASS: &str = "text-xl mb-2.5";
const P_CLASS: &str = "mb-2.5";
const A_CLASS: &str = "link text-primary";
const UL_CLASS: &str = "list-inside list-disc mb-2.5";
const OL_CLASS: &str = "list-inside list-decimal mb-2.5";
const BLOCKQUOTE_CLASS: &str = "w-fit p-1 mb-2.5 border-s-4 rounded-sm border-slate-400 bg-slate-600";
const TABLE_CLASS: &str = "table mb-2.5";
const CODE_CLASS: &str = "inline-block w-fit rounded-md bg-black px-1 py-0.5 mx-0.5 mb-2.5";
const SPOILER_SPAN_CLASS: &str = "transition-all duration-300 ease-in-out h-fit rounded-md bg-white p-1 mx-0.5 text-white spoiler-text";

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

#[derive(Default)]
struct SpoilerState {
    /// Number of currently open elements
    element_depth: usize,
    /// Element depth at which the currently open spoiler was started, if any
    spoiler_depth: Option<usize>,
    /// True right after a spoiler was opened, until some content is written
    trim_start: bool,
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

impl SpoilerState {
    fn start_element(&mut self) {
        self.element_depth += 1;
        self.trim_start = false;
    }

    /// Call before writing the End event
    fn end_element(&mut self, writer: &mut Writer<Cursor<Vec<u8>>>) -> Result<(), AppError> {
        // The parent element of an unclosed spoiler ends: close it first
        if self.spoiler_depth == Some(self.element_depth) {
            close_spoiler(writer)?;
            self.spoiler_depth = None;
        }
        self.element_depth = self.element_depth.saturating_sub(1);
        self.trim_start = false;
        Ok(())
    }

    /// Empty elements, entity references and any other non-text event
    fn other_event(&mut self) {
        self.trim_start = false;
    }

    fn finish(&mut self, writer: &mut Writer<Cursor<Vec<u8>>>) -> Result<(), AppError> {
        if self.spoiler_depth.take().is_some() {
            close_spoiler(writer)?;
        }
        Ok(())
    }

    fn process_spoilers(
        &mut self,
        writer: &mut Writer<Cursor<Vec<u8>>>,
        text: &str,
    ) -> Result<(), AppError> {
        let mut parts = text.split(SPOILER_TAG).peekable();
        while let Some(mut part) = parts.next() {
            // Every part except the last one is followed by a SPOILER_TAG
            let is_followed_by_tag = parts.peek().is_some();
            let closes = is_followed_by_tag && self.spoiler_depth == Some(self.element_depth);
            let opens = is_followed_by_tag && self.spoiler_depth.is_none();

            if self.spoiler_depth.is_some() {
                if self.trim_start {
                    part = part.trim_start();
                }
                if closes {
                    part = part.trim_end();
                }
            }
            if !part.is_empty() {
                writer.write_event(Event::Text(BytesText::new(part)))?;
                self.trim_start = false;
            }

            if closes {
                close_spoiler(writer)?;
                self.spoiler_depth = None;
            } else if opens {
                open_spoiler(writer)?;
                self.spoiler_depth = Some(self.element_depth);
                self.trim_start = true;
            } else if is_followed_by_tag {
                // Tag inside a nested element of the spoiler: keep it as text
                writer.write_event(Event::Text(BytesText::new(SPOILER_TAG)))?;
                self.trim_start = false;
            }
        }
        Ok(())
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
    println!("Markdown as html: {html_from_markdown}");

    // Add styling, will be done by parsing the html which is a bit ugly. Would be better
    // if the styling could be added directly when generating the html from markdown
    let styled_html_output = style_html_user_content(html_from_markdown.as_str())?;
    Ok(styled_html_output)
}

fn open_spoiler(writer: &mut Writer<Cursor<Vec<u8>>>) -> Result<(), AppError> {
    writer.write_event(Event::Start(BytesStart::new("label")))?;
    let mut checkbox_elem = BytesStart::new("input");
    checkbox_elem.push_attribute(("type", "checkbox"));
    checkbox_elem.push_attribute(("class", "spoiler-checkbox hidden"));
    writer.write_event(Event::Empty(checkbox_elem))?;
    let mut span = BytesStart::new("span");
    span.push_attribute(("class", SPOILER_SPAN_CLASS));
    writer.write_event(Event::Start(span))?;
    Ok(())
}

fn close_spoiler(writer: &mut Writer<Cursor<Vec<u8>>>) -> Result<(), AppError> {
    writer.write_event(Event::End(BytesEnd::new("span")))?;
    writer.write_event(Event::End(BytesEnd::new("label")))?;
    Ok(())
}

pub fn style_html_user_content(user_content: &str) -> Result<String, AppError> {
    let mut reader = Reader::from_str(user_content);
    let mut writer = Writer::new(Cursor::new(Vec::new()));
    let mut is_in_block = false;
    let mut is_in_code = false;

    let mut spoilers_state = SpoilerState::default();

    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) => {
                let mut elem = e.into_owned();
                match elem.name().as_ref() {
                    b"h1" => elem.push_attribute(("class", H1_CLASS)),
                    b"h2" => elem.push_attribute(("class", H2_CLASS)),
                    b"h3" => elem.push_attribute(("class", H3_CLASS)),
                    b"p" if !is_in_block => elem.push_attribute(("class", P_CLASS)),
                    b"a" => elem.push_attribute(("class", A_CLASS)),
                    b"ul" => elem.push_attribute(("class", UL_CLASS)),
                    b"ol" => elem.push_attribute(("class", OL_CLASS)),
                    b"code" => {
                        elem.push_attribute(("class", CODE_CLASS))
                    }
                    b"table" => elem.push_attribute(("class", TABLE_CLASS)),
                    b"blockquote" => {
                        is_in_block = true;
                        elem.push_attribute((
                            "class",
                            BLOCKQUOTE_CLASS,
                        ))
                    },
                    _ => (),
                }
                is_in_code |= elem.name().as_ref() == b"code";
                spoilers_state.start_element();
                writer.write_event(Event::Start(elem))?;
            }
            Ok(Event::End(e)) => {
                spoilers_state.end_element(&mut writer)?;
                match e.name().as_ref() {
                    b"blockquote" => is_in_block = false,
                    b"code" => is_in_code = false,
                    _ => (),
                }
                writer.write_event(Event::End(e))?;
            }
            Ok(Event::Empty(e)) => {
                let mut elem = e.into_owned();

                if elem.name().as_ref() == b"hr" {
                    elem.push_attribute(("class", "my-2"))
                }
                spoilers_state.other_event();
                writer.write_event(Event::Empty(elem))?;
            }
            Ok(Event::Text(e)) => {
                let text = e.decode().map_err(|e| AppError::new(format!("Error while decoding text: {e}")))?.into_owned();

                match is_in_code {
                    true => writer.write_event(Event::Text(BytesText::new(&text)))?,
                    false => spoilers_state.process_spoilers(&mut writer, &text)?,
                }
            }
            Ok(Event::Eof) => {
                spoilers_state.finish(&mut writer)?;
                break
            },
            // we can either move or borrow the event to write, depending on your use-case
            Ok(e) => {
                spoilers_state.other_event();
                writer.write_event(e)?
            },
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

    // ---------- style_html_user_content ----------

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
    fn test_style_html_user_content_untouched_elements() {
        for html in [
            "<h4></h4>",
            "<strong>x</strong>",
            "<br/>",
            r#"<img src="a" alt="b"/>"#,
            "<pre></pre>",
        ] {
            assert_eq!(style_html_user_content(html).expect("styled"), html);
        }
    }

    #[test]
    fn test_style_html_user_content_paragraph_and_nesting() {
        assert_eq!(
            style_html_user_content("<p>text</p>").expect("styled"),
            r#"<p class="mb-2.5">text</p>"#
        );
        assert_eq!(
            style_html_user_content("<ul><li>a</li></ul>").expect("styled"),
            r#"<ul class="list-inside list-disc mb-2.5"><li>a</li></ul>"#
        );
        // <p> inside a blockquote is not styled, but the one after it is
        assert_eq!(
            style_html_user_content("<blockquote><p>a</p></blockquote><p>b</p>").expect("styled"),
            r#"<blockquote class="w-fit p-1 mb-2.5 border-s-4 rounded-sm border-slate-400 bg-slate-600"><p>a</p></blockquote><p class="mb-2.5">b</p>"#
        );
    }

    #[test]
    fn test_style_html_user_content_keeps_entities_escaped() {
        // Must not be double-escaped (&amp;amp;) nor unescaped
        assert_eq!(
            style_html_user_content("<p>a &amp; b &lt;c&gt; &quot;d&quot;</p>").expect("styled"),
            r#"<p class="mb-2.5">a &amp; b &lt;c&gt; &quot;d&quot;</p>"#
        );
    }

    #[test]
    fn test_style_html_user_content_multiple_spoilers() {
        assert_eq!(
            style_html_user_content("<p>a ||b|| c ||d||</p>").expect("styled"),
            r#"<p class="mb-2.5">a <label><input type="checkbox" class="spoiler-checkbox hidden"/><span class="transition-all duration-300 ease-in-out rounded-md bg-white p-0.5 px-1 mx-0.5 text-white spoiler-text">b</span></label> c <label><input type="checkbox" class="spoiler-checkbox hidden"/><span class="transition-all duration-300 ease-in-out rounded-md bg-white p-0.5 px-1 mx-0.5 text-white spoiler-text">d</span></label></p>"#
        );
        // Empty spoiler produces nothing
        assert_eq!(
            style_html_user_content("<p>||||</p>").expect("styled"),
            r#"<p class="mb-2.5"><label><input type="checkbox" class="spoiler-checkbox hidden"/><span class="transition-all duration-300 ease-in-out rounded-md bg-white p-0.5 px-1 mx-0.5 text-white spoiler-text"></span></label></p>"#
        );
    }

    #[test]
    fn test_style_html_user_content_malformed_html_is_error() {
        assert!(style_html_user_content("<p></div>").is_err());
        assert!(style_html_user_content("<p").is_err());
        // Unclosed void elements are not supported (markdown-rs emits <br />)
        assert!(style_html_user_content("<p>a<br>b</p>").is_err());
    }

    // ---------- get_styled_html_from_markdown: formatting ----------

    #[test]
    fn test_get_styled_html_from_markdown() {
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
            get_styled_html_from_markdown(markdown).expect("Should get styled html"),
            expected_html
        );

        let markdown = indoc! {r#"
            `code blocks`
        "#};
        let expected_html = indoc! {r#"
            <p class="mb-2.5"><code class="block w-fit rounded-md bg-black px-1 py-0.5 mx-0.5 mb-2.5">code blocks</code></p>
        "#};
        assert_eq!(
            get_styled_html_from_markdown(markdown).expect("Should get styled html"),
            expected_html
        );

        let markdown = indoc! {r#"
            || Spoilers ||
        "#};
        let expected_html = indoc! {r#"
            <p class="mb-2.5"><label><input type="checkbox" class="spoiler-checkbox hidden"/><span class="transition-all duration-300 ease-in-out rounded-md bg-white p-0.5 px-1 mx-0.5 text-white spoiler-text">Spoilers</span></label></p>
        "#};
        assert_eq!(
            get_styled_html_from_markdown(markdown).expect("Should get styled html"),
            expected_html
        );

        let markdown = indoc! {r#"
            **bold**, *italic*, combined emphasis with **asterisks and _underscores_**.
        "#};
        let expected_html = indoc! {r#"
            <p class="mb-2.5"><strong>bold</strong>, <em>italic</em>, combined emphasis with <strong>asterisks and <em>underscores</em></strong>.</p>
        "#};
        assert_eq!(
            get_styled_html_from_markdown(markdown).expect("Should get styled html"),
            expected_html
        );

        let markdown = indoc! {r#"
            Strikethrough uses two tildes. ~~Scratch this.~~
        "#};
        let expected_html = indoc! {r#"
            <p class="mb-2.5">Strikethrough uses two tildes. <del>Scratch this.</del></p>
        "#};
        assert_eq!(
            get_styled_html_from_markdown(markdown).expect("Should get styled html"),
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
            get_styled_html_from_markdown(markdown).expect("Should get styled html"),
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
            get_styled_html_from_markdown(markdown).expect("Should get styled html"),
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
            get_styled_html_from_markdown(markdown).expect("Should get styled html"),
            expected_html
        );

        let markdown = indoc! {r#"
            Finally, we can add links [link text](https://www.example.com), images ![alt text](https://github.com/adam-p/markdown-here/raw/master/src/utils/images/icon48.png "Logo Title Text 1")
        "#};
        let expected_html = indoc! {r#"
            <p class="mb-2.5">Finally, we can add links <a href="https://www.example.com" class="link text-primary">link text</a>, images <img src="https://github.com/adam-p/markdown-here/raw/master/src/utils/images/icon48.png" alt="alt text" title="Logo Title Text 1" /></p>
        "#};
        assert_eq!(
            get_styled_html_from_markdown(markdown).expect("Should get styled html"),
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
            get_styled_html_from_markdown(markdown).expect("Should get styled html"),
            expected_html
        );
    }

    #[test]
    fn test_get_styled_html_from_markdown_empty() {
        assert_eq!(get_styled_html_from_markdown("").expect("Should get styled html"), "");
    }

    #[test]
    fn test_get_styled_html_from_markdown_fenced_code_block() {
        let markdown = "```\nlet x = 1;\n```";
        let expected_html = indoc! {r#"
            <pre><code class="block w-fit rounded-md bg-black px-1 py-0.5 mx-0.5 mb-2.5">let x = 1;
            </code></pre>"#
        };
        assert_eq!(get_styled_html_from_markdown(markdown).expect("Should get styled html"), expected_html);
    }

    #[test]
    fn test_get_styled_html_from_markdown_hard_break() {
        assert_eq!(
            get_styled_html_from_markdown("a  \nb").expect("Should get styled html"),
            "<p class=\"mb-2.5\">a<br />\nb</p>"
        );
    }

    #[test]
    fn test_get_styled_html_from_markdown_autolink() {
        assert_eq!(
            get_styled_html_from_markdown("https://example.com").expect("Should get styled html"),
            "<p class=\"mb-2.5\"><a href=\"https://example.com\" class=\"link text-primary\">https://example.com</a></p>"
        );
    }

    #[test]
    fn test_get_styled_html_from_markdown_escapes_special_chars() {
        assert_eq!(
            get_styled_html_from_markdown(r#"a < b & c > d "q""#).expect("Should get styled html"),
            "<p class=\"mb-2.5\">a &lt; b &amp; c &gt; d &quot;q&quot;</p>"
        );
    }

    #[test]
    fn test_get_styled_html_from_markdown_spoiler_with_special_chars() {
        // The whole spoiler content must end up inside a single label/span
        assert_eq!(
            get_styled_html_from_markdown("|| a & b ||").expect("Should get styled html"),
            "<p class=\"mb-2.5\"><label><input type=\"checkbox\" class=\"spoiler-checkbox hidden\"/><span class=\"transition-all duration-300 ease-in-out rounded-md bg-white p-0.5 px-1 mx-0.5 text-white spoiler-text\">a &amp; b</span></label></p>"
        );
    }

    #[test]
    fn test_style_html_user_content_spoiler_with_entities() {
        assert_eq!(
            style_html_user_content("<p>|| a &amp; b &lt; c ||</p>").expect("styled"),
            r#"<p class="mb-2.5"><label><input type="checkbox" class="spoiler-checkbox hidden"/><span class="transition-all duration-300 ease-in-out rounded-md bg-white p-0.5 px-1 mx-0.5 text-white spoiler-text">a &amp; b &lt; c</span></label></p>"#
        );
    }

    #[test]
    fn test_style_html_user_content_spoiler_with_inline_elements() {
        assert_eq!(
            style_html_user_content("<p>||<strong>bold</strong> text||</p>").expect("styled"),
            r#"<p class="mb-2.5"><label><input type="checkbox" class="spoiler-checkbox hidden"/><span class="transition-all duration-300 ease-in-out rounded-md bg-white p-0.5 px-1 mx-0.5 text-white spoiler-text"><strong>bold</strong> text</span></label></p>"#
        );
    }

    #[test]
    fn test_style_html_user_content_unclosed_spoiler_ends_with_parent() {
        assert_eq!(
            style_html_user_content("<p>a ||b</p><p>c</p>").expect("styled"),
            r#"<p class="mb-2.5">a <label><input type="checkbox" class="spoiler-checkbox hidden"/><span class="transition-all duration-300 ease-in-out rounded-md bg-white p-0.5 px-1 mx-0.5 text-white spoiler-text">b</span></label></p><p class="mb-2.5">c</p>"#
        );
    }

    #[test]
    fn test_style_html_user_content_spoiler_tag_in_nested_element_is_literal() {
        // Closing inside <em> would produce misnested tags, so it stays as text
        assert_eq!(
            style_html_user_content("<p>||a <em>b||</em></p>").expect("styled"),
            r#"<p class="mb-2.5"><label><input type="checkbox" class="spoiler-checkbox hidden"/><span class="transition-all duration-300 ease-in-out rounded-md bg-white p-0.5 px-1 mx-0.5 text-white spoiler-text">a <em>b||</em></span></label></p>"#
        );
    }

    #[test]
    fn test_style_html_user_content_no_spoiler_in_code() {
        assert_eq!(
            style_html_user_content("<p><code>a || b</code></p>").expect("styled"),
            r#"<p class="mb-2.5"><code class="block w-fit rounded-md bg-black px-1 py-0.5 mx-0.5 mb-2.5">a || b</code></p>"#
        );
    }

    // ---------- get_styled_html_from_markdown: security ----------

    #[test]
    fn test_get_styled_html_from_markdown_raw_html_is_escaped() {
        assert_eq!(
            get_styled_html_from_markdown("<script>alert(1)</script>").expect("Should get styled html"),
            "&lt;script&gt;alert(1)&lt;/script&gt;"
        );
        assert_eq!(
            get_styled_html_from_markdown("<img src=x onerror=alert(1)>").expect("Should get styled html"),
            "&lt;img src=x onerror=alert(1)&gt;"
        );
        for (payload, forbidden) in [
            (r#"<iframe src="https://evil.example"></iframe>"#, "<iframe"),
            ("<style>*{display:none}</style>", "<style"),
            (r#"<a href="javascript:alert(1)">x</a>"#, "<a href=\"javascript"),
            ("<!-- comment -->", "<!--"),
            ("<svg onload=alert(1)>", "<svg"),
        ] {
            let html = get_styled_html_from_markdown(payload).expect("Should get styled html");
            assert!(!html.contains(forbidden), "{payload} -> {html}");
        }
    }

    #[test]
    fn test_get_styled_html_from_markdown_dangerous_urls_are_dropped() {
        for (payload, scheme) in [
            ("[x](javascript:alert(1))", "javascript:"),
            ("[x](JaVaScRiPt:alert(1))", "javascript:"),
            ("[x](vbscript:msgbox(1))", "vbscript:"),
            ("[x](data:text/html;base64,PHNjcmlwdD4=)", "data:"),
            ("![x](javascript:alert(1))", "javascript:"),
            ("![x](data:image/svg+xml;base64,PHN2Zz4=)", "data:"),
            ("<javascript:alert(1)>", "href=\"javascript:"),
        ] {
            let html = get_styled_html_from_markdown(payload).expect("Should get styled html").to_lowercase();
            assert!(!html.contains(scheme), "{payload} -> {html}");
        }
    }

    #[test]
    fn test_get_styled_html_from_markdown_safe_urls_are_kept() {
        assert_eq!(
            get_styled_html_from_markdown("[x](mailto:a@example.com)").expect("Should get styled html"),
            "<p class=\"mb-2.5\"><a href=\"mailto:a@example.com\" class=\"link text-primary\">x</a></p>"
        );
    }

    #[test]
    fn test_get_styled_html_from_markdown_attribute_breakout() {
        let html = get_styled_html_from_markdown(r#"[x](https://a.example/"onmouseover="alert(1))"#)
            .expect("Should get styled html");
        assert!(!html.contains("\"onmouseover"), "{html}");

        let html = get_styled_html_from_markdown(r#"[x](https://a.example "t\" onmouseover=\"alert(1)")"#)
            .expect("Should get styled html");
        assert!(!html.contains("\" onmouseover"), "{html}");

        let html = get_styled_html_from_markdown(r#"![a" onerror="alert(1)](https://a.example/x.png)"#)
            .expect("Should get styled html");
        assert!(!html.contains("\" onerror"), "{html}");
    }

    #[test]
    fn test_get_styled_html_from_markdown_deep_nesting_does_not_panic() {
        assert!(get_styled_html_from_markdown(&format!("{}x", "> ".repeat(1000))).is_ok());
        assert!(get_styled_html_from_markdown(&"[".repeat(10_000)).is_ok());
        assert!(get_styled_html_from_markdown(&"*a ".repeat(10_000)).is_ok());
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
