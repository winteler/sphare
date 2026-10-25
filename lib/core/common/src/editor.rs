use std::borrow::Cow;
use std::io::Cursor;

use leptos::html::{Textarea};
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
const UL_CLASS: &str = "list-inside list-disc mb-2.5 [&_ul]:ms-4 [&_ol]:ms-4 [&_ul]:mb-0 [&_ol]:mb-0";
const OL_CLASS: &str = "list-inside list-decimal mb-2.5 [&_ul]:ms-4 [&_ol]:ms-4 [&_ul]:mb-0 [&_ol]:mb-0";
const BLOCKQUOTE_CLASS: &str = "editor-quote w-fit p-1 mb-2.5 border-s-4 border-1 border-slate-400 bg-slate-600";
const TABLE_CLASS: &str = "table mb-2.5";
const INLINE_CODE_CLASS: &str =
    "border border-neutral-600 bg-neutral-800 px-1 py-0.5 mx-0.5 text-sm text-neutral-100 break-words";
const CODE_BLOCK_CLASS: &str =
    "block w-fit max-w-full overflow-x-auto border border-neutral-600 bg-neutral-800 p-2 mb-2.5 text-sm text-neutral-100";
const SPOILER_SPAN_CLASS: &str =
    "spoiler-content cursor-pointer relative inline-block max-w-full rounded-md bg-black p-0.5 px-1 mx-0.5 text-white";
const SPOILER_BLOCK_CLASS: &str =
    "spoiler-block cursor-pointer isolate relative grid w-fit max-w-full rounded-md bg-black mb-2.5 text-white";
const SPOILER_BODY_CLASS: &str = "spoiler-body col-start-1 row-start-1 min-w-0 p-1";
const SPOILER_COVER_CLASS: &str =
    "spoiler-cover col-start-1 row-start-1 z-10 rounded-md bg-white px-2 flex min-w-0 items-center justify-center text-center text-black transition-opacity duration-300 ease-in-out";
const SPOILER_CHECKBOX_CLASS: &str = "spoiler-checkbox sr-only";
const SPOILER_TEXT_CLASS: &str = "spoiler-text";
const DEFAULT_SPOILER_TITLE: &str = "Spoiler";

#[derive(Clone, Debug, PartialEq)]
enum SpoilerBlockMarker {
    Open { colons: usize, title: String },
    Close { colons: usize },
}

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
    /// Open block spoilers as (element depth, number of colons of the opener)
    blocks: Vec<(usize, usize)>,
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
        if self.spoiler_depth == Some(self.element_depth) {
            close_spoiler(writer)?;
            self.spoiler_depth = None;
        }
        // The parent of unclosed block spoilers ends (e.g. the blockquote they are in)
        while self.blocks.last().is_some_and(|b| b.0 == self.element_depth) {
            self.blocks.pop();
            close_block_spoiler(writer)?;
        }
        self.element_depth = self.element_depth.saturating_sub(1);
        self.trim_start = false;
        Ok(())
    }


    /// Empty elements, entity references and any other non-text event
    fn other_event(&mut self) {
        self.trim_start = false;
    }

    fn open_block(
        &mut self,
        writer: &mut Writer<Cursor<Vec<u8>>>,
        title: &str,
        colons: usize,
    ) -> Result<(), AppError> {
        open_block_spoiler(writer, title)?;
        self.blocks.push((self.element_depth, colons));
        self.trim_start = false;
        Ok(())
    }

    /// Outermost block at the current depth that this closer is long enough to close.
    /// Blocks at another depth can't be closed from here without misnesting the tags.
    fn block_to_close(&self, colons: usize) -> Option<usize> {
        self.blocks
            .iter()
            .position(|&(depth, opener)| depth == self.element_depth && opener <= colons)
    }

    /// Closes the block at `index` and every block nested in it
    fn close_blocks_from(
        &mut self,
        writer: &mut Writer<Cursor<Vec<u8>>>,
        index: usize,
    ) -> Result<(), AppError> {
        while self.blocks.len() > index {
            self.blocks.pop();
            close_block_spoiler(writer)?;
        }
        self.trim_start = false;
        Ok(())
    }
    fn finish(&mut self, writer: &mut Writer<Cursor<Vec<u8>>>) -> Result<(), AppError> {
        if self.spoiler_depth.take().is_some() {
            close_spoiler(writer)?;
        }
        self.close_blocks_from(writer, 0)
    }

    fn process_inline_spoilers(
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
    use crate::editor::render_markdown;
    use crate::errors::AppError;

    pub fn get_html_and_markdown_strings(body: &str, is_markdown: bool) -> Result<(String, Option<&str>), AppError> {
        match is_markdown {
            true => Ok((
                render_markdown(body)?,
                Some(body),
            )),
            false => Ok((String::from(body), None)),
        }
    }
}

pub fn render_markdown(
    markdown_input: &str,
) -> Result<String, AppError> {
    let html_from_markdown = markdown::to_html_with_options(
        &separate_spoiler_markers(markdown_input),
        &Options::gfm()
    ).map_err(AppError::new)?;
    log::debug!("Markdown as html: {html_from_markdown}");
    
    let styled_html_output = style_html_user_content(html_from_markdown.as_str())?;
    Ok(styled_html_output)
}

/// `::: spoiler [title]` (at least three `:`) opens a spoiler block, `:::` (or the same number of `:` if opened with more) closes it.
fn parse_spoiler_block_marker(text: &str) -> Option<SpoilerBlockMarker> {
    let text = text.trim();
    let colons = text.chars().take_while(|c| *c == ':').count();
    if colons < 3 {
        return None;
    }
    let rest = &text[colons..];
    if rest.trim().is_empty() {
        return Some(SpoilerBlockMarker::Close { colons });
    }
    let title = rest.trim_start().strip_prefix("spoiler")?;
    let keyword_ends = title.is_empty() || title.starts_with(char::is_whitespace);
    // A second tag means an inline spoiler; a newline means the title isn't alone on its line
    if !keyword_ends || title.contains(SPOILER_TAG) || title.contains('\n') {
        return None;
    }
    Some(SpoilerBlockMarker::Open { colons, title: title.trim().to_string() })
}

/// Splits leading blockquote markers (`> `, `>> `, ...) from the rest of the line
fn split_quote_prefix(line: &str) -> (&str, &str) {
    let mut rest = line;
    loop {
        let trimmed = rest.trim_start_matches(' ');
        if rest.len() - trimmed.len() >= 4 {
            break;
        }
        let Some(after) = trimmed.strip_prefix('>') else { break };
        rest = after.strip_prefix(' ').unwrap_or(after);
    }
    (&line[..line.len() - rest.len()], rest)
}

fn parse_code_fence_opener(line: &str) -> Option<(char, usize)> {
    // The first character must be a backtick or a tilde, otherwise this isn't a code block fence.
    let c = line.chars().next().filter(|c| *c == '`' || *c == '~')?;
    // Count how many times that same character repeats at the start of the line
    let len = line.chars().take_while(|x| *x == c).count();
    // "```code```" on one line is inline code, not a fence
    (len >= 3 && !(c == '`' && line[len..].contains('`'))).then_some((c, len))
}

fn parse_code_fence_closer(content: &str, c: char, len: usize) -> bool {
    content.len() >= len && content.chars().all(|x| x == c)
}

fn separate_spoiler_markers(markdown: &str) -> Cow<'_, str> {
    if !markdown.contains(SPOILER_TAG) {
        return Cow::Borrowed(markdown);
    }
    let mut out = String::with_capacity(markdown.len() + 16);
    // Some((fence char, fence length)) while inside a fenced code block
    let mut fence: Option<(char, usize)> = None;
    for line in markdown.lines() {
        let (prefix, rest) = split_quote_prefix(line);
        // Leading spaces after the quote prefix. 4 or more means an indented code block,
        // which can't be a marker or a fence
        let indent = rest.len() - rest.trim_start_matches(' ').len();
        let content = rest.trim();
        // A line with only the quote markers keeps us inside the same blockquote
        let blank = prefix.trim_end();
        match fence {
            // Case 1: inside a fenced code block.
            Some((c, len)) => {
                // Check if fenced block is closed
                if indent < 4 && parse_code_fence_closer(content, c, len) {
                    fence = None;
                }
                out.push_str(line);
                out.push('\n');
            }
            // Case 2: a spoiler block tag outside a code block
            None if indent < 4 && parse_spoiler_block_marker(content).is_some() => {
                // Write the spoiler tag separated from the rest of content, so that it ends up in its own <p> after html conversion
                // and can be easily detected to add a new element to encapsulate what's in the spoiler.
                out.push_str(blank);
                out.push('\n');
                out.push_str(prefix);
                out.push_str(content);
                out.push('\n');
                out.push_str(blank);
                out.push('\n');
            }
            // Case 3: any other line outside a code block
            None => {
                // check if it opens a code block
                if indent < 4 {
                    fence = parse_code_fence_opener(content);
                }
                // copy its content as is
                out.push_str(line);
                out.push('\n');
            }
        }
    }
    if !markdown.ends_with('\n') {
        while out.ends_with('\n') {
            out.pop();
        }
    }
    Cow::Owned(out)
}

/// Visually hidden but focusable checkbox that toggles the spoiler. Its `aria-label` is
/// what a screen reader announces, so that it never reads the content of the spoiler as the name.
fn write_spoiler_checkbox(writer: &mut Writer<Cursor<Vec<u8>>>, label: &str) -> Result<(), AppError> {
    let mut checkbox_elem = BytesStart::new("input");
    checkbox_elem.push_attribute(("type", "checkbox"));
    checkbox_elem.push_attribute(("class", SPOILER_CHECKBOX_CLASS));
    let aria_label = match label  {
        label if label == DEFAULT_SPOILER_TITLE => label.to_string(),
        title => format!("{DEFAULT_SPOILER_TITLE}: {title}"),
    };
    checkbox_elem.push_attribute(("aria-label", aria_label.as_str()));
    writer.write_event(Event::Empty(checkbox_elem))?;
    Ok(())
}

fn open_spoiler(writer: &mut Writer<Cursor<Vec<u8>>>) -> Result<(), AppError> {
    writer.write_event(Event::Start(BytesStart::new("label")))?;
    write_spoiler_checkbox(writer, DEFAULT_SPOILER_TITLE)?;
    // The span is the cover, the inner span holds the text that is hidden until the spoiler is revealed
    let mut span = BytesStart::new("span");
    span.push_attribute(("class", SPOILER_SPAN_CLASS));
    writer.write_event(Event::Start(span))?;
    let mut text = BytesStart::new("span");
    text.push_attribute(("class", SPOILER_TEXT_CLASS));
    writer.write_event(Event::Start(text))?;
    Ok(())
}

fn close_spoiler(writer: &mut Writer<Cursor<Vec<u8>>>) -> Result<(), AppError> {
    writer.write_event(Event::End(BytesEnd::new("span")))?;
    writer.write_event(Event::End(BytesEnd::new("span")))?;
    writer.write_event(Event::End(BytesEnd::new("label")))?;
    Ok(())
}

fn open_block_spoiler(writer: &mut Writer<Cursor<Vec<u8>>>, title: &str) -> Result<(), AppError> {
    let title = if title.is_empty() { DEFAULT_SPOILER_TITLE } else { title };
    let mut label = BytesStart::new("label");
    label.push_attribute(("class", "block"));
    writer.write_event(Event::Start(label))?;
    write_spoiler_checkbox(writer, title)?;
    let mut div = BytesStart::new("div");
    div.push_attribute(("class", SPOILER_BLOCK_CLASS));
    writer.write_event(Event::Start(div))?;
    // The cover and the body share the same grid cell, so the block is as big as the biggest of them
    let mut cover = BytesStart::new("span");
    cover.push_attribute(("class", SPOILER_COVER_CLASS));
    writer.write_event(Event::Start(cover))?;
    writer.write_event(Event::Text(BytesText::new(title)))?;
    writer.write_event(Event::End(BytesEnd::new("span")))?;
    let mut body = BytesStart::new("div");
    body.push_attribute(("class", SPOILER_BODY_CLASS));
    writer.write_event(Event::Start(body))?;
    Ok(())
}

fn close_block_spoiler(writer: &mut Writer<Cursor<Vec<u8>>>) -> Result<(), AppError> {
    writer.write_event(Event::End(BytesEnd::new("div")))?; // body
    writer.write_event(Event::End(BytesEnd::new("div")))?; // block
    writer.write_event(Event::End(BytesEnd::new("label")))?;
    Ok(())
}

/// Called right after a `<p>` start event. If the paragraph is a spoiler marker, returns it
/// together with a reader positioned after the paragraph.
fn read_spoiler_block_marker<'a>(reader: &Reader<&'a [u8]>) -> Option<(SpoilerBlockMarker, Reader<&'a [u8]>)> {
    let mut lookahead = reader.clone();
    let mut text = String::new();
    loop {
        match lookahead.read_event().ok()? {
            Event::Text(t) => text.push_str(&t.decode().ok()?),
            Event::GeneralRef(r) => {
                if let Ok(Some(c)) = r.resolve_char_ref() {
                    text.push(c);
                } else {
                    text.push_str(quick_xml::escape::resolve_xml_entity(&r.decode().ok()?)?);
                }
            }
            Event::End(e) if e.name().as_ref() == b"p" => break,
            // Formatting in the title is dropped, only its text is kept
            Event::Start(_) | Event::End(_) | Event::Empty(_) => {}
            _ => return None,
        }
    }
    Some((parse_spoiler_block_marker(&text)?, lookahead))
}

/// Adds `class` to the element. If it already has a class attribute (e.g. `language-rust`
/// on code blocks) the class is appended, because a second `class` attribute is invalid
/// html and browsers ignore it.
fn append_class_to_elem(elem: &mut BytesStart<'static>, class: &str) -> Result<(), AppError> {
    let mut has_class = false;
    let mut content = String::from_utf8_lossy(elem.name().as_ref()).into_owned();
    let name_len = content.len();
    for attr in elem.html_attributes() {
        let attr = attr.map_err(AppError::new)?;
        let key = String::from_utf8_lossy(attr.key.as_ref());
        // The value is still escaped here; only the quote needs care because of the double-quote rewrite
        let mut value = String::from_utf8_lossy(&attr.value).replace('"', "&quot;");
        if key == "class" {
            has_class = true;
            value = format!("{value} {class}");
        }
        content.push_str(&format!(" {key}=\"{value}\""));
    }
    if !has_class {
        content.push_str(&format!(" class=\"{class}\""));
    }
    *elem = BytesStart::from_content(content, name_len);
    Ok(())
}

fn process_elem(
    elem: &mut BytesStart<'static>,
    block_depth: &mut usize,
    is_in_code: &mut bool,
    is_in_pre: &mut bool,
) -> Result<(), AppError> {
    let class = match elem.name().as_ref() {
        b"h1" => Some(H1_CLASS),
        b"h2" => Some(H2_CLASS),
        b"h3" => Some(H3_CLASS),
        b"p" if *block_depth == 0 => Some(P_CLASS),
        b"a" => Some(A_CLASS),
        b"ul" => Some(UL_CLASS),
        b"ol" => Some(OL_CLASS),
        b"pre" => {
            *is_in_pre = true;
            Some(CODE_BLOCK_CLASS)
        },
        b"code" if !*is_in_pre => Some(INLINE_CODE_CLASS),
        b"table" => Some(TABLE_CLASS),
        b"blockquote" => {
            *block_depth += 1;
            Some(BLOCKQUOTE_CLASS)
        },
        _ => None,
    };

    if let Some(class) = class {
        append_class_to_elem(elem, class)?;
    }
    *is_in_code |= elem.name().as_ref() == b"code";
    Ok(())
}

pub fn style_html_user_content(user_content: &str) -> Result<String, AppError> {
    let mut reader = Reader::from_str(user_content);
    let mut writer = Writer::new(Cursor::new(Vec::new()));
    let mut block_depth = 0;
    let mut is_in_code = false;
    let mut is_in_pre = false;

    let mut spoilers_state = SpoilerState::default();

    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) => {
                if e.name().as_ref() == b"p" && let Some((marker, advanced)) = read_spoiler_block_marker(&reader) {
                    match marker {
                        SpoilerBlockMarker::Open { colons, title } => {
                            spoilers_state.open_block(&mut writer, &title, colons)?;
                            reader = advanced; // skip the marker paragraph
                            continue;
                        }
                        SpoilerBlockMarker::Close { colons } => {
                            if let Some(index) = spoilers_state.block_to_close(colons) {
                                spoilers_state.close_blocks_from(&mut writer, index)?;
                                reader = advanced;
                                continue;
                            }
                        }
                    }
                }
                let mut elem = e.into_owned();
                process_elem(
                    &mut elem,
                    &mut block_depth,
                    &mut is_in_code,
                    &mut is_in_pre
                )?;
                spoilers_state.start_element();
                writer.write_event(Event::Start(elem))?;
            }
            Ok(Event::End(e)) => {
                spoilers_state.end_element(&mut writer)?;
                match e.name().as_ref() {
                    b"blockquote" => block_depth = block_depth.saturating_sub(1),
                    b"code" => is_in_code = false,
                    b"pre" => is_in_pre = false,
                    _ => (),
                }
                writer.write_event(Event::End(e))?;
            }
            Ok(Event::Empty(e)) => {
                let mut elem = e.into_owned();

                if elem.name().as_ref() == b"hr" {
                    append_class_to_elem(&mut elem, "my-2")?;
                }
                spoilers_state.other_event();
                writer.write_event(Event::Empty(elem))?;
            }
            Ok(Event::Text(e)) => {
                let text = e.decode().map_err(|e| AppError::new(format!("Error while decoding text: {e}")))?.into_owned();

                match is_in_code {
                    true => writer.write_event(Event::Text(BytesText::new(&text)))?,
                    false => spoilers_state.process_inline_spoilers(&mut writer, &text)?,
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
    use std::assert_matches;
use indoc::{formatdoc};
    use quick_xml::events::Event;
    use quick_xml::Reader;
    use crate::editor::ssr::get_html_and_markdown_strings;
    use super::*;

    fn new_writer() -> Writer<Cursor<Vec<u8>>> {
        Writer::new(Cursor::new(Vec::new()))
    }

    fn written(writer: Writer<Cursor<Vec<u8>>>) -> String {
        String::from_utf8(writer.into_inner().into_inner()).expect("Should be valid UTF-8")
    }

    /// Reads the first event (the opening `<p>`) and then calls `read_spoiler_block_marker`.
    fn marker_after_first_p(html: &str) -> Option<(SpoilerBlockMarker, Reader<&[u8]>)> {
        let mut reader = Reader::from_str(html);
        let p_event  =reader.read_event().expect("Should read the opening <p> event");
        assert_matches!(p_event, Event::Start(ref e) if e.name().as_ref() == b"p");
        read_spoiler_block_marker(&reader)
    }

    /// Every opened element must be closed again and never closed twice.
    fn assert_balanced_html(html: &str) {
        let mut reader = Reader::from_str(html);
        let mut depth: i64 = 0;
        loop {
            match reader.read_event().expect("Should produce well-formed html") {
                Event::Start(_) => depth += 1,
                Event::End(_) => depth -= 1,
                Event::Eof => break,
                _ => {}
            }
            assert!(depth >= 0, "Should never close more elements than were opened in: {html}");
        }
        assert_eq!(depth, 0, "Should close every opened element in: {html}");
    }

    // ---------------------------------------------------------------------
    // get_html_and_markdown_strings
    // ---------------------------------------------------------------------

    #[test]
    fn test_get_html_and_markdown_strings_for_plain_text() {
        let text_body = "hello world";

        let (html, markdown) = get_html_and_markdown_strings(text_body, false).expect("Should get text body");

        assert_eq!(html, text_body, "Should return the text unchanged as html");
        assert_eq!(markdown, None, "Should not return a markdown string for plain text");
    }

    #[test]
    fn test_get_html_and_markdown_strings_for_markdown() {
        let markdown_body = "# this is a header";

        let (html, markdown) = get_html_and_markdown_strings(markdown_body, true).expect("Should get markdown body");

        let expected_html = render_markdown(markdown_body).expect("Should render the markdown to html");
        assert_eq!(html, expected_html, "Should return the styled html");
        assert_eq!(markdown, Some(markdown_body), "Should return the original markdown string");
    }

    // ---------------------------------------------------------------------
    // format_textarea_content
    // ---------------------------------------------------------------------

    // Bold
    #[test]
    fn test_format_textarea_content_bold_without_selection() {
        let mut content = String::from("This is some user text ");

        let cursor_position = format_textarea_content(&mut content, 23, 23, FormatType::Bold);

        assert_eq!(cursor_position, Some(25), "Should place the cursor between the bold markers");
        assert_eq!(content, "This is some user text ****", "Should append empty bold markers");
    }

    #[test]
    fn test_format_textarea_content_bold_with_selection() {
        let mut content = String::from("This is some user text ");

        let cursor_position = format_textarea_content(&mut content, 8, 12, FormatType::Bold);

        assert_eq!(cursor_position, None, "Should not move the cursor");
        assert_eq!(content, "This is **some** user text ", "Should wrap the selection in bold markers");
    }

    // Italic
    #[test]
    fn test_format_textarea_content_italic_without_selection() {
        let mut content = String::from("This is some user text ");

        let cursor_position = format_textarea_content(&mut content, 23, 23, FormatType::Italic);

        assert_eq!(cursor_position, Some(24), "Should place the cursor between the italic markers");
        assert_eq!(content, "This is some user text **", "Should append empty italic markers");
    }

    #[test]
    fn test_format_textarea_content_italic_with_selection() {
        let mut content = String::from("This is some user text ");

        let cursor_position = format_textarea_content(&mut content, 8, 12, FormatType::Italic);

        assert_eq!(cursor_position, None, "Should not move the cursor");
        assert_eq!(content, "This is *some* user text ", "Should wrap the selection in italic markers");
    }

    // Strikethrough
    #[test]
    fn test_format_textarea_content_strikethrough_without_selection() {
        let mut content = String::from("This is some user text ");

        let cursor_position = format_textarea_content(&mut content, 23, 23, FormatType::Strikethrough);

        assert_eq!(cursor_position, Some(25), "Should place the cursor between the strikethrough markers");
        assert_eq!(content, "This is some user text ~~~~", "Should append empty strikethrough markers");
    }

    #[test]
    fn test_format_textarea_content_strikethrough_with_selection() {
        let mut content = String::from("This is some user text ");

        let cursor_position = format_textarea_content(&mut content, 8, 12, FormatType::Strikethrough);

        assert_eq!(cursor_position, None, "Should not move the cursor");
        assert_eq!(content, "This is ~~some~~ user text ", "Should wrap the selection in strikethrough markers");
    }

    // Header1
    #[test]
    fn test_format_textarea_content_header1_without_selection() {
        let mut content = String::from("This is some user text ");

        let cursor_position = format_textarea_content(&mut content, 23, 23, FormatType::Header1);

        assert_eq!(cursor_position, Some(25), "Should shift the cursor by the length of the prefix");
        assert_eq!(content, "# This is some user text ", "Should prefix the line with a header 1 marker");
    }

    #[test]
    fn test_format_textarea_content_header1_with_selection() {
        let mut content = String::from("This is some user text ");

        let cursor_position = format_textarea_content(&mut content, 8, 12, FormatType::Header1);

        assert_eq!(cursor_position, None, "Should not move the cursor");
        assert_eq!(content, "# This is some user text ", "Should prefix the whole line with a header 1 marker");
    }

    // Header2
    #[test]
    fn test_format_textarea_content_header2_without_selection() {
        let mut content = String::from("This is some user text ");

        let cursor_position = format_textarea_content(&mut content, 23, 23, FormatType::Header2);

        assert_eq!(cursor_position, Some(26), "Should shift the cursor by the length of the prefix");
        assert_eq!(content, "## This is some user text ", "Should prefix the line with a header 2 marker");
    }

    #[test]
    fn test_format_textarea_content_header2_with_selection() {
        let mut content = String::from("This is some user text ");

        let cursor_position = format_textarea_content(&mut content, 8, 12, FormatType::Header2);

        assert_eq!(cursor_position, None, "Should not move the cursor");
        assert_eq!(content, "## This is some user text ", "Should prefix the whole line with a header 2 marker");
    }

    // List
    #[test]
    fn test_format_textarea_content_list_without_selection() {
        let mut content = String::from("This is some user text ");

        let cursor_position = format_textarea_content(&mut content, 23, 23, FormatType::List);

        assert_eq!(cursor_position, Some(25), "Should shift the cursor by the length of the prefix");
        assert_eq!(content, "* This is some user text ", "Should prefix the line with a list marker");
    }

    #[test]
    fn test_format_textarea_content_list_with_selection() {
        let mut content = String::from("This is some user text ");

        let cursor_position = format_textarea_content(&mut content, 8, 12, FormatType::List);

        assert_eq!(cursor_position, None, "Should not move the cursor");
        assert_eq!(content, "* This is some user text ", "Should prefix the whole line with a list marker");
    }

    // NumberedList
    #[test]
    fn test_format_textarea_content_numbered_list_without_selection() {
        let mut content = String::from("This is some user text ");

        let cursor_position = format_textarea_content(&mut content, 23, 23, FormatType::NumberedList);

        assert_eq!(cursor_position, Some(26), "Should shift the cursor by the length of the prefix");
        assert_eq!(content, "1. This is some user text ", "Should prefix the line with a numbered list marker");
    }

    #[test]
    fn test_format_textarea_content_numbered_list_with_selection() {
        let mut content = String::from("This is some user text ");

        let cursor_position = format_textarea_content(&mut content, 8, 12, FormatType::NumberedList);

        assert_eq!(cursor_position, None, "Should not move the cursor");
        assert_eq!(content, "1. This is some user text ", "Should prefix the whole line with a numbered list marker");
    }

    // CodeBlock
    #[test]
    fn test_format_textarea_content_code_block_without_selection() {
        let mut content = String::from("This is some user text ");

        let cursor_position = format_textarea_content(&mut content, 23, 23, FormatType::CodeBlock);

        assert_eq!(cursor_position, Some(26), "Should place the cursor between the code fences");
        assert_eq!(content, "This is some user text ``````", "Should append empty code fences");
    }

    #[test]
    fn test_format_textarea_content_code_block_with_selection() {
        let mut content = String::from("This is some user text ");

        let cursor_position = format_textarea_content(&mut content, 8, 12, FormatType::CodeBlock);

        assert_eq!(cursor_position, None, "Should not move the cursor");
        assert_eq!(content, "This is ```some``` user text ", "Should wrap the selection in code fences");
    }

    // Spoiler
    #[test]
    fn test_format_textarea_content_spoiler_without_selection() {
        let mut content = String::from("This is some user text ");

        let cursor_position = format_textarea_content(&mut content, 23, 23, FormatType::Spoiler);

        assert_eq!(cursor_position, Some(26), "Should place the cursor between the spoiler tags");
        assert_eq!(content, "This is some user text ::::::", "Should append empty spoiler tags");
    }

    #[test]
    fn test_format_textarea_content_spoiler_with_selection() {
        let mut content = String::from("This is some user text ");

        let cursor_position = format_textarea_content(&mut content, 8, 12, FormatType::Spoiler);

        assert_eq!(cursor_position, None, "Should not move the cursor");
        assert_eq!(content, "This is :::some::: user text ", "Should wrap the selection in spoiler tags");
    }

    // BlockQuote
    #[test]
    fn test_format_textarea_content_block_quote_without_selection() {
        let mut content = String::from("This is some user text ");

        let cursor_position = format_textarea_content(&mut content, 23, 23, FormatType::BlockQuote);

        assert_eq!(cursor_position, Some(25), "Should shift the cursor by the length of the prefix");
        assert_eq!(content, "> This is some user text ", "Should prefix the line with a block quote marker");
    }

    #[test]
    fn test_format_textarea_content_block_quote_with_selection() {
        let mut content = String::from("This is some user text ");

        let cursor_position = format_textarea_content(&mut content, 8, 12, FormatType::BlockQuote);

        assert_eq!(cursor_position, None, "Should not move the cursor");
        assert_eq!(content, "> This is some user text ", "Should prefix the whole line with a block quote marker");
    }

    // Link
    #[test]
    fn test_format_textarea_content_link_without_selection() {
        let mut content = String::from("This is some user text ");

        let cursor_position = format_textarea_content(&mut content, 23, 23, FormatType::Link);

        assert_eq!(cursor_position, Some(24), "Should place the cursor at the start of the link text");
        assert_eq!(
            content,
            "This is some user text [link text](https://www.your_link.com)",
            "Should append a link with placeholder text and url"
        );
    }

    #[test]
    fn test_format_textarea_content_link_with_selection() {
        let mut content = String::from("This is some user text ");

        let cursor_position = format_textarea_content(&mut content, 8, 12, FormatType::Link);

        assert_eq!(cursor_position, Some(9), "Should place the cursor at the start of the link text");
        assert_eq!(
            content,
            "This is [link text](some) user text ",
            "Should use the selection as the url and add placeholder link text"
        );
    }

    // Image
    #[test]
    fn test_format_textarea_content_image_without_selection() {
        let mut content = String::from("This is some user text ");

        let cursor_position = format_textarea_content(&mut content, 23, 23, FormatType::Image);

        assert_eq!(cursor_position, Some(25), "Should place the cursor between the brackets of the alt text");
        assert_eq!(
            content,
            "This is some user text ![](https://image_url.png)",
            "Should append an image with a placeholder url"
        );
    }

    #[test]
    fn test_format_textarea_content_image_with_selection() {
        let mut content = String::from("This is some user text ");

        let cursor_position = format_textarea_content(&mut content, 8, 12, FormatType::Image);

        assert_eq!(cursor_position, Some(10), "Should place the cursor between the brackets of the alt text");
        assert_eq!(content, "This is ![](some) user text ", "Should use the selection as the image url");
    }

    // ---------------------------------------------------------------------
    // render_markdown: standard markdown styling
    // ---------------------------------------------------------------------

    #[test]
    fn test_render_empty_markdown() {
        let markdown = "";

        let result = render_markdown(markdown).expect("Should render the markdown to html");

        assert_eq!(result, "", "Should render nothing for empty input");
    }

    #[test]
    fn test_render_headings_h1_to_h3() {
        let markdown = "# T1\n\n## T2\n\n### T3";

        let result = render_markdown(markdown).expect("Should render the markdown to html");

        let expected = formatdoc! {r#"
            <h1 class="{H1_CLASS}">T1</h1>
            <h2 class="{H2_CLASS}">T2</h2>
            <h3 class="{H3_CLASS}">T3</h3>"#};
        assert_eq!(result, expected, "Should add classes to h1, h2 and h3");
    }

    #[test]
    fn test_render_paragraph_with_bold_and_link() {
        let markdown = "para with **bold** and [link](https://example.com)";

        let result = render_markdown(markdown).expect("Should render the markdown to html");

        let expected = formatdoc! {r#"
            <p class="{P_CLASS}">para with <strong>bold</strong> and <a href="https://example.com" class="{A_CLASS}">link</a></p>"#};
        assert_eq!(result, expected, "Should add a class to the paragraph and the link");
    }

    #[test]
    fn test_render_unordered_and_ordered_lists() {
        let markdown = "- a\n- b\n\n1. x\n2. y";

        let result = render_markdown(markdown).expect("Should render the markdown to html");

        let expected = formatdoc! {r#"
            <ul class="{UL_CLASS}">
            <li>a</li>
            <li>b</li>
            </ul>
            <ol class="{OL_CLASS}">
            <li>x</li>
            <li>y</li>
            </ol>"#};
        assert_eq!(result, expected, "Should add classes to ul and ol");
    }

    #[test]
    fn test_render_blockquote_followed_by_paragraph() {
        let markdown = "> quote\n\ntext";

        let result = render_markdown(markdown).expect("Should render the markdown to html");

        let expected = formatdoc! {r#"
            <blockquote class="{BLOCKQUOTE_CLASS}">
            <p>quote</p>
            </blockquote>
            <p class="{P_CLASS}">text</p>"#};
        assert_eq!(
            result,
            expected,
            "Should style the blockquote but not paragraphs inside it, and style paragraphs after it again"
        );
    }

    #[test]
    fn test_render_table() {
        let markdown = "| a | b |\n|---|---|\n| 1 | 2 |";

        let result = render_markdown(markdown).expect("Should render the markdown to html");

        let expected = formatdoc! {r#"
            <table class="{TABLE_CLASS}">
            <thead>
            <tr>
            <th>a</th>
            <th>b</th>
            </tr>
            </thead>
            <tbody>
            <tr>
            <td>1</td>
            <td>2</td>
            </tr>
            </tbody>
            </table>"#};
        assert_eq!(result, expected, "Should add a class to the table");
    }

    #[test]
    fn test_render_inline_code() {
        let markdown = "`code` and text";

        let result = render_markdown(markdown).expect("Should render the markdown to html");

        let expected = formatdoc! {r#"
            <p class="{P_CLASS}"><code class="{INLINE_CODE_CLASS}">code</code> and text</p>"#};
        assert_eq!(result, expected, "Should add a class to inline code");
    }

    #[test]
    fn test_render_fenced_code_block() {
        let markdown = "```\nlet x = 1;\n```";

        let result = render_markdown(markdown).expect("Should render the markdown to html");

        let expected = formatdoc! {r#"
            <pre class="{CODE_BLOCK_CLASS}"><code>let x = 1;
            </code></pre>"#};
        assert_eq!(result, expected, "Should add the code class to the code block");
    }

    #[test]
    fn test_render_fenced_code_block_with_language() {
        let markdown = "```rust\nfn main() {}\n```";
        let result = render_markdown(markdown).expect("Should render the markdown to html");
        let expected = formatdoc! {r#"
            <pre class="{CODE_BLOCK_CLASS}"><code class="language-rust">fn main() {{}}
            </code></pre>"#
        };
        assert_eq!(
            result,
            expected,
            "Should add the code class to the existing language class instead of a second class attribute"
        );
    }

    #[test]
    fn test_render_inline_code_after_fenced_code_block() {
        let markdown = "```\nx\n```\n\n`y`";

        let result = render_markdown(markdown).expect("Should render the markdown to html");

        let expected = formatdoc! {r#"
            <pre class="{CODE_BLOCK_CLASS}"><code>x
            </code></pre>
            <p class="{P_CLASS}"><code class="{INLINE_CODE_CLASS}">y</code></p>"#
        };
        assert_eq!(result, expected, "Should style inline code again after the code block ended");
    }

    #[test]
    fn test_render_code_keeps_html_special_characters_escaped() {
        let markdown = "`<b> & \"q\"`";

        let result = render_markdown(markdown).expect("Should render the markdown to html");

        let expected = formatdoc! {r#"
            <p class="{P_CLASS}"><code class="{INLINE_CODE_CLASS}">&lt;b&gt; &amp; &quot;q&quot;</code></p>"#};
        assert_eq!(result, expected, "Should keep html special characters escaped in code");
    }

    #[test]
    fn test_render_horizontal_rule() {
        let markdown = "---";
        let result = render_markdown(markdown).expect("Should render the markdown to html");

        assert_eq!(
            result,
            r#"<hr class="my-2"/>"#,
            "Should add the my-2 class to the horizontal rule"
        );
    }

    #[test]
    fn test_render_gfm_strikethrough() {
        let markdown = "~~strike~~";

        let result = render_markdown(markdown).expect("Should render the markdown to html");

        let expected = formatdoc! {r#"
            <p class="{P_CLASS}"><del>strike</del></p>"#};
        assert_eq!(result, expected, "Should render GFM strikethrough");
    }

    #[test]
    fn test_render_gfm_task_list() {
        let markdown = "- [x] done";

        let result = render_markdown(markdown).expect("Should render the markdown to html");

        let expected = formatdoc! {r#"
            <ul class="{UL_CLASS}">
            <li><input type="checkbox" disabled="" checked="" /> done</li>
            </ul>"#};
        assert_eq!(result, expected, "Should render a GFM task list item");
    }

    #[test]
    fn test_render_image() {
        let markdown = "![img](https://example.com/a.png)";

        let result = render_markdown(markdown).expect("Should render the markdown to html");

        let expected = formatdoc! {r#"
            <p class="{P_CLASS}"><img src="https://example.com/a.png" alt="img" /></p>"#};
        assert_eq!(result, expected, "Should render the image as is");
    }

    #[test]
    fn test_render_markdown_without_spoilers() {
        let markdown = "# Title\n\nline one\nline two";

        let result = render_markdown(markdown).expect("Should render the markdown to html");

        let expected = formatdoc! {r#"
            <h1 class="{H1_CLASS}">Title</h1>
            <p class="{P_CLASS}">line one
            line two</p>"#};
        assert_eq!(result, expected, "Should render like plain markdown and keep soft line breaks");
    }

    // ---------------------------------------------------------------------
    // Spoiler markup constants
    // ---------------------------------------------------------------------

    #[test]
    fn test_spoiler_checkbox_stays_in_the_tab_order() {
        let classes: Vec<&str> = SPOILER_CHECKBOX_CLASS.split_whitespace().collect();

        assert!(classes.contains(&"sr-only"), "Should hide the checkbox visually only");
        for class in ["hidden", "invisible"] {
            assert!(!classes.contains(&class), "Should not use {class}, it removes the checkbox from the tab order");
        }
    }

    #[test]
    fn test_spoiler_cover_and_body_share_the_same_grid_cell() {
        let block_classes: Vec<&str> = SPOILER_BLOCK_CLASS.split_whitespace().collect();
        assert!(block_classes.contains(&"grid"), "Should lay out the block spoiler as a grid");

        for class in [SPOILER_COVER_CLASS, SPOILER_BODY_CLASS] {
            let classes: Vec<&str> = class.split_whitespace().collect();
            assert!(
                classes.contains(&"col-start-1") && classes.contains(&"row-start-1"),
                "Should place the element with classes {class:?} in the first grid cell"
            );
        }
    }

    // ---------------------------------------------------------------------
    // render_markdown: inline spoilers
    // ---------------------------------------------------------------------

    #[test]
    fn test_render_inline_spoiler() {
        let markdown = "hello ::: secret ::: world";

        let result = render_markdown(markdown).expect("Should render the markdown to html");

        let expected = formatdoc! {r#"
            <p class="{P_CLASS}">hello <label><input type="checkbox" class="{SPOILER_CHECKBOX_CLASS}" aria-label="{DEFAULT_SPOILER_TITLE}"/><span class="{SPOILER_SPAN_CLASS}"><span class="{SPOILER_TEXT_CLASS}">secret</span></span></label> world</p>"#};
        assert_eq!(result, expected, "Should wrap the text between two tags in a spoiler");
    }

    #[test]
    fn test_render_inline_spoiler_trims_whitespace_inside_the_tags() {
        let markdown = "a :::    b    ::: c";

        let result = render_markdown(markdown).expect("Should render the markdown to html");

        let expected = formatdoc! {r#"
            <p class="{P_CLASS}">a <label><input type="checkbox" class="{SPOILER_CHECKBOX_CLASS}" aria-label="{DEFAULT_SPOILER_TITLE}"/><span class="{SPOILER_SPAN_CLASS}"><span class="{SPOILER_TEXT_CLASS}">b</span></span></label> c</p>"#};
        assert_eq!(result, expected, "Should trim spaces at both ends of the spoiler content");
    }

    #[test]
    fn test_render_inline_spoiler_as_whole_paragraph() {
        let markdown = ":::secret:::";

        let result = render_markdown(markdown).expect("Should render the markdown to html");

        let expected = formatdoc! {r#"
            <p class="{P_CLASS}"><label><input type="checkbox" class="{SPOILER_CHECKBOX_CLASS}" aria-label="{DEFAULT_SPOILER_TITLE}"/><span class="{SPOILER_SPAN_CLASS}"><span class="{SPOILER_TEXT_CLASS}">secret</span></span></label></p>"#};
        assert_eq!(result, expected, "Should support a spoiler that is the whole paragraph");
    }

    #[test]
    fn test_render_multiple_inline_spoilers_in_one_paragraph() {
        let markdown = "a ::: b ::: c ::: d";

        let result = render_markdown(markdown).expect("Should render the markdown to html");

        let expected = formatdoc! {r#"
            <p class="{P_CLASS}">a <label><input type="checkbox" class="{SPOILER_CHECKBOX_CLASS}" aria-label="{DEFAULT_SPOILER_TITLE}"/><span class="{SPOILER_SPAN_CLASS}"><span class="{SPOILER_TEXT_CLASS}">b</span></span></label> c <label><input type="checkbox" class="{SPOILER_CHECKBOX_CLASS}" aria-label="{DEFAULT_SPOILER_TITLE}"/><span class="{SPOILER_SPAN_CLASS}"><span class="{SPOILER_TEXT_CLASS}">d</span></span></label></p>"#};
        assert_eq!(
            result,
            expected,
            "Should open a second spoiler after the first one was closed, and close it at the end of the paragraph"
        );
    }

    #[test]
    fn test_render_unclosed_inline_spoiler_is_closed_at_the_end_of_the_paragraph() {
        let markdown = ":::unclosed\n\nnext paragraph";

        let result = render_markdown(markdown).expect("Should render the markdown to html");

        let expected = formatdoc! {r#"
            <p class="{P_CLASS}"><label><input type="checkbox" class="{SPOILER_CHECKBOX_CLASS}" aria-label="{DEFAULT_SPOILER_TITLE}"/><span class="{SPOILER_SPAN_CLASS}"><span class="{SPOILER_TEXT_CLASS}">unclosed</span></span></label></p>
            <p class="{P_CLASS}">next paragraph</p>"#};
        assert_eq!(result, expected, "Should not let an unclosed spoiler leak into the next paragraph");
    }

    #[test]
    fn test_render_inline_spoiler_with_formatting_inside() {
        let markdown = ":::a **b** c:::";

        let result = render_markdown(markdown).expect("Should render the markdown to html");

        let expected = formatdoc! {r#"
            <p class="{P_CLASS}"><label><input type="checkbox" class="{SPOILER_CHECKBOX_CLASS}" aria-label="{DEFAULT_SPOILER_TITLE}"/><span class="{SPOILER_SPAN_CLASS}"><span class="{SPOILER_TEXT_CLASS}">a <strong>b</strong> c</span></span></label></p>"#};
        assert_eq!(result, expected, "Should keep nested formatting inside the spoiler");
    }

    #[test]
    fn test_render_inline_spoiler_crossing_an_element_boundary_stays_well_nested() {
        // The spoiler is closed where the <strong> ends, so the tags never overlap
        let markdown = "**bold :::spoiler** text:::";

        let result = render_markdown(markdown).expect("Should render the markdown to html");

        let expected = formatdoc! {r#"
            <p class="{P_CLASS}"><strong>bold <label><input type="checkbox" class="{SPOILER_CHECKBOX_CLASS}" aria-label="{DEFAULT_SPOILER_TITLE}"/><span class="{SPOILER_SPAN_CLASS}"><span class="{SPOILER_TEXT_CLASS}">spoiler</span></span></label></strong> text<label><input type="checkbox" class="{SPOILER_CHECKBOX_CLASS}" aria-label="{DEFAULT_SPOILER_TITLE}"/><span class="{SPOILER_SPAN_CLASS}"><span class="{SPOILER_TEXT_CLASS}"></span></span></label></p>"#};
        assert_eq!(result, expected, "Should close the spoiler together with the element it was opened in");
    }

    #[test]
    fn test_render_empty_inline_spoiler() {
        let markdown = "a ::: ::: b";

        let result = render_markdown(markdown).expect("Should render the markdown to html");

        let expected = formatdoc! {r#"
            <p class="{P_CLASS}">a <label><input type="checkbox" class="{SPOILER_CHECKBOX_CLASS}" aria-label="{DEFAULT_SPOILER_TITLE}"/><span class="{SPOILER_SPAN_CLASS}"><span class="{SPOILER_TEXT_CLASS}"></span></span></label> b</p>"#};
        assert_eq!(result, expected, "Should render an empty spoiler for two adjacent tags");
    }

    #[test]
    fn test_render_inline_spoiler_in_heading() {
        let markdown = "# ::: head ::: here";

        let result = render_markdown(markdown).expect("Should render the markdown to html");

        let expected = formatdoc! {r#"
            <h1 class="{H1_CLASS}"><label><input type="checkbox" class="{SPOILER_CHECKBOX_CLASS}" aria-label="{DEFAULT_SPOILER_TITLE}"/><span class="{SPOILER_SPAN_CLASS}"><span class="{SPOILER_TEXT_CLASS}">head</span></span></label> here</h1>"#};
        assert_eq!(result, expected, "Should support spoilers in headings");
    }

    #[test]
    fn test_render_inline_spoiler_in_table_cell() {
        let markdown = "| a |\n|---|\n| ::: x ::: |";

        let result = render_markdown(markdown).expect("Should render the markdown to html");

        let expected = formatdoc! {r#"
            <table class="{TABLE_CLASS}">
            <thead>
            <tr>
            <th>a</th>
            </tr>
            </thead>
            <tbody>
            <tr>
            <td><label><input type="checkbox" class="{SPOILER_CHECKBOX_CLASS}" aria-label="{DEFAULT_SPOILER_TITLE}"/><span class="{SPOILER_SPAN_CLASS}"><span class="{SPOILER_TEXT_CLASS}">x</span></span></label></td>
            </tr>
            </tbody>
            </table>"#};
        assert_eq!(result, expected, "Should support spoilers in table cells");
    }

    #[test]
    fn test_render_inline_spoiler_in_list_item() {
        let markdown = "- item ::: hidden :::";

        let result = render_markdown(markdown).expect("Should render the markdown to html");

        let expected = formatdoc! {r#"
            <ul class="{UL_CLASS}">
            <li>item <label><input type="checkbox" class="{SPOILER_CHECKBOX_CLASS}" aria-label="{DEFAULT_SPOILER_TITLE}"/><span class="{SPOILER_SPAN_CLASS}"><span class="{SPOILER_TEXT_CLASS}">hidden</span></span></label></li>
            </ul>"#};
        assert_eq!(result, expected, "Should support spoilers in list items");
    }

    #[test]
    fn test_render_inline_spoiler_after_link() {
        let markdown = "[a](https://a.example) ::: z :::";

        let result = render_markdown(markdown).expect("Should render the markdown to html");

        let expected = formatdoc! {r#"
            <p class="{P_CLASS}"><a href="https://a.example" class="{A_CLASS}">a</a> <label><input type="checkbox" class="{SPOILER_CHECKBOX_CLASS}" aria-label="{DEFAULT_SPOILER_TITLE}"/><span class="{SPOILER_SPAN_CLASS}"><span class="{SPOILER_TEXT_CLASS}">z</span></span></label></p>"#};
        assert_eq!(result, expected, "Should handle a spoiler after a link");
    }

    #[test]
    fn test_render_html_entities_inside_inline_spoiler_stay_escaped() {
        let markdown = "& ::: a < ::: b";

        let result = render_markdown(markdown).expect("Should render the markdown to html");

        let expected = formatdoc! {r#"
            <p class="{P_CLASS}">&amp; <label><input type="checkbox" class="{SPOILER_CHECKBOX_CLASS}" aria-label="{DEFAULT_SPOILER_TITLE}"/><span class="{SPOILER_SPAN_CLASS}"><span class="{SPOILER_TEXT_CLASS}">a &lt;</span></span></label> b</p>"#};
        assert_eq!(result, expected, "Should keep entities escaped inside and outside the spoiler");
    }

    #[test]
    fn test_render_inline_spoiler_after_inline_code() {
        let markdown = "`:::` ::: s :::";

        let result = render_markdown(markdown).expect("Should render the markdown to html");

        let expected = formatdoc! {r#"
            <p class="{P_CLASS}"><code class="{INLINE_CODE_CLASS}">:::</code> <label><input type="checkbox" class="{SPOILER_CHECKBOX_CLASS}" aria-label="{DEFAULT_SPOILER_TITLE}"/><span class="{SPOILER_SPAN_CLASS}"><span class="{SPOILER_TEXT_CLASS}">s</span></span></label></p>"#};
        assert_eq!(result, expected, "Should leave code mode after the code element ends");
    }

    #[test]
    fn test_render_inline_spoiler_with_unicode() {
        let markdown = "é ::: héllo 🎉 ::: ü";

        let result = render_markdown(markdown).expect("Should render the markdown to html");

        let expected = formatdoc! {r#"
            <p class="{P_CLASS}">é <label><input type="checkbox" class="{SPOILER_CHECKBOX_CLASS}" aria-label="{DEFAULT_SPOILER_TITLE}"/><span class="{SPOILER_SPAN_CLASS}"><span class="{SPOILER_TEXT_CLASS}">héllo 🎉</span></span></label> ü</p>"#};
        assert_eq!(result, expected, "Should handle non-ASCII text");
    }

    // ---------------------------------------------------------------------
    // render_markdown: block spoilers
    // ---------------------------------------------------------------------

    #[test]
    fn test_render_block_spoiler_with_title() {
        let markdown = "::: spoiler Title\nhidden\n:::";
        let result = render_markdown(markdown).expect("Should render the markdown to html");

        let expected = formatdoc! {r#"
            <label class="block"><input type="checkbox" class="{SPOILER_CHECKBOX_CLASS}" aria-label="{DEFAULT_SPOILER_TITLE}: Title"/><div class="{SPOILER_BLOCK_CLASS}"><span class="{SPOILER_COVER_CLASS}">Title</span><div class="{SPOILER_BODY_CLASS}">
            <p class="{P_CLASS}">hidden</p>
            </div></div></label>"#
        };
        assert_eq!(result, expected, "Should wrap the content in a block spoiler with the title");
    }

    #[test]
    fn test_render_block_spoiler_without_title_uses_default_title() {
        let markdown = "::: spoiler\nhidden\n:::";
        let result = render_markdown(markdown).expect("Should render the markdown to html");

        let expected = formatdoc! {r#"
            <label class="block"><input type="checkbox" class="{SPOILER_CHECKBOX_CLASS}" aria-label="{DEFAULT_SPOILER_TITLE}"/><div class="{SPOILER_BLOCK_CLASS}"><span class="{SPOILER_COVER_CLASS}">Spoiler</span><div class="{SPOILER_BODY_CLASS}">
            <p class="{P_CLASS}">hidden</p>
            </div></div></label>"#
        };
        assert_eq!(result, expected, "Should fall back to the default title");
    }

    #[test]
    fn test_render_block_spoiler_with_markdown_content() {
        let markdown = "::: spoiler Title\n\n# Head\n\nsome **bold** text\n\n:::";
        let result = render_markdown(markdown).expect("Should render the markdown to html");

        let expected = formatdoc! {r#"
            <label class="block"><input type="checkbox" class="{SPOILER_CHECKBOX_CLASS}" aria-label="{DEFAULT_SPOILER_TITLE}: Title"/><div class="{SPOILER_BLOCK_CLASS}"><span class="{SPOILER_COVER_CLASS}">Title</span><div class="{SPOILER_BODY_CLASS}">
            <h1 class="{H1_CLASS}">Head</h1>
            <p class="{P_CLASS}">some <strong>bold</strong> text</p>
            </div></div></label>"#
        };
        assert_eq!(result, expected, "Should style the markdown inside the block spoiler");
    }

    #[test]
    fn test_render_block_spoiler_title_drops_formatting() {
        let markdown = "::: spoiler **bold** title\nx\n:::";
        let result = render_markdown(markdown).expect("Should render the markdown to html");

        let expected = formatdoc! {r#"
            <label class="block"><input type="checkbox" class="{SPOILER_CHECKBOX_CLASS}" aria-label="{DEFAULT_SPOILER_TITLE}: bold title"/><div class="{SPOILER_BLOCK_CLASS}"><span class="{SPOILER_COVER_CLASS}">bold title</span><div class="{SPOILER_BODY_CLASS}">
            <p class="{P_CLASS}">x</p>
            </div></div></label>"#
        };
        assert_eq!(result, expected, "Should only keep the text of a formatted title");
    }

    #[test]
    fn test_render_block_spoiler_title_escapes_special_characters() {
        let markdown = "::: spoiler a\"b&c\nx\n:::";
        let result = render_markdown(markdown).expect("Should render the markdown to html");

        let expected = formatdoc! {r#"
            <label class="block"><input type="checkbox" class="{SPOILER_CHECKBOX_CLASS}" aria-label="{DEFAULT_SPOILER_TITLE}: a&quot;b&amp;c"/><div class="{SPOILER_BLOCK_CLASS}"><span class="{SPOILER_COVER_CLASS}">a&quot;b&amp;c</span><div class="{SPOILER_BODY_CLASS}">
            <p class="{P_CLASS}">x</p>
            </div></div></label>"#
        };
        assert_eq!(result, expected, "Should escape quotes and ampersands in the title and the aria-label");
    }

    #[test]
    fn test_render_block_spoiler_title_decodes_entities_once_and_escapes() {
        let markdown = "::: spoiler a &amp; b &#35;\nx\n:::";
        let result = render_markdown(markdown).expect("Should render the markdown to html");

        let expected = formatdoc! {r#"
            <label class="block"><input type="checkbox" class="{SPOILER_CHECKBOX_CLASS}" aria-label="{DEFAULT_SPOILER_TITLE}: a &amp; b #"/><div class="{SPOILER_BLOCK_CLASS}"><span class="{SPOILER_COVER_CLASS}">a &amp; b #</span><div class="{SPOILER_BODY_CLASS}">
            <p class="{P_CLASS}">x</p>
            </div></div></label>"#
        };
        assert_eq!(
            result,
            expected,
            "Should decode named and numeric entities in the title and escape the result"
        );
    }

    #[test]
    fn test_render_block_spoiler_title_with_unicode() {
        let markdown = "::: spoiler Ünïcödé 🎉\nxé\n:::";
        let result = render_markdown(markdown).expect("Should render the markdown to html");

        let expected = formatdoc! {r#"
            <label class="block"><input type="checkbox" class="{SPOILER_CHECKBOX_CLASS}" aria-label="{DEFAULT_SPOILER_TITLE}: Ünïcödé 🎉"/><div class="{SPOILER_BLOCK_CLASS}"><span class="{SPOILER_COVER_CLASS}">Ünïcödé 🎉</span><div class="{SPOILER_BODY_CLASS}">
            <p class="{P_CLASS}">xé</p>
            </div></div></label>"#
        };
        assert_eq!(result, expected, "Should handle non-ASCII titles and content");
    }

    #[test]
    fn test_render_unclosed_block_spoiler_is_closed_at_the_end_of_the_document() {
        let markdown = "::: spoiler unclosed\ntext";
        let result = render_markdown(markdown).expect("Should render the markdown to html");

        let expected = formatdoc! {r#"
            <label class="block"><input type="checkbox" class="{SPOILER_CHECKBOX_CLASS}" aria-label="{DEFAULT_SPOILER_TITLE}: unclosed"/><div class="{SPOILER_BLOCK_CLASS}"><span class="{SPOILER_COVER_CLASS}">unclosed</span><div class="{SPOILER_BODY_CLASS}">
            <p class="{P_CLASS}">text</p></div></div></label>"#
        };
        assert_eq!(result, expected, "Should close an unclosed block spoiler at the end");
    }

    #[test]
    fn test_render_two_consecutive_block_spoilers() {
        let markdown = "::: spoiler A\nx\n:::\n\n::: spoiler B\ny\n:::";
        let result = render_markdown(markdown).expect("Should render the markdown to html");

        let expected = formatdoc! {r#"
            <label class="block"><input type="checkbox" class="{SPOILER_CHECKBOX_CLASS}" aria-label="{DEFAULT_SPOILER_TITLE}: A"/><div class="{SPOILER_BLOCK_CLASS}"><span class="{SPOILER_COVER_CLASS}">A</span><div class="{SPOILER_BODY_CLASS}">
            <p class="{P_CLASS}">x</p>
            </div></div></label>
            <label class="block"><input type="checkbox" class="{SPOILER_CHECKBOX_CLASS}" aria-label="{DEFAULT_SPOILER_TITLE}: B"/><div class="{SPOILER_BLOCK_CLASS}"><span class="{SPOILER_COVER_CLASS}">B</span><div class="{SPOILER_BODY_CLASS}">
            <p class="{P_CLASS}">y</p>
            </div></div></label>"#
        };
        assert_eq!(result, expected, "Should render two separate block spoilers");
    }

    #[test]
    fn test_render_block_spoiler_between_text_without_blank_lines() {
        let markdown = "text\n::: spoiler T\nx\n:::\nmore";
        let result = render_markdown(markdown).expect("Should render the markdown to html");

        let expected = formatdoc! {r#"
            <p class="{P_CLASS}">text</p>
            <label class="block"><input type="checkbox" class="{SPOILER_CHECKBOX_CLASS}" aria-label="{DEFAULT_SPOILER_TITLE}: T"/><div class="{SPOILER_BLOCK_CLASS}"><span class="{SPOILER_COVER_CLASS}">T</span><div class="{SPOILER_BODY_CLASS}">
            <p class="{P_CLASS}">x</p>
            </div></div></label>
            <p class="{P_CLASS}">more</p>"#
        };
        assert_eq!(
            result,
            expected,
            "Should split the paragraphs around the markers even without blank lines"
        );
    }

    #[test]
    fn test_render_nested_block_spoilers_with_longer_outer_marker() {
        let markdown = ":::: spoiler outer\n::: spoiler inner\nx\n:::\ny\n::::";
        let result = render_markdown(markdown).expect("Should render the markdown to html");

        let expected = formatdoc! {r#"
            <label class="block"><input type="checkbox" class="{SPOILER_CHECKBOX_CLASS}" aria-label="{DEFAULT_SPOILER_TITLE}: outer"/><div class="{SPOILER_BLOCK_CLASS}"><span class="{SPOILER_COVER_CLASS}">outer</span><div class="{SPOILER_BODY_CLASS}">
            <label class="block"><input type="checkbox" class="{SPOILER_CHECKBOX_CLASS}" aria-label="{DEFAULT_SPOILER_TITLE}: inner"/><div class="{SPOILER_BLOCK_CLASS}"><span class="{SPOILER_COVER_CLASS}">inner</span><div class="{SPOILER_BODY_CLASS}">
            <p class="{P_CLASS}">x</p>
            </div></div></label>
            <p class="{P_CLASS}">y</p>
            </div></div></label>"#
        };
        assert_eq!(result, expected, "Should close only the inner spoiler with a short closer");
    }

    #[test]
    fn test_render_longer_closer_closes_nested_block_spoilers() {
        let markdown = ":::: spoiler outer\n::: spoiler inner\nx\n::::\ny";
        let result = render_markdown(markdown).expect("Should render the markdown to html");

        let expected = formatdoc! {r#"
            <label class="block"><input type="checkbox" class="{SPOILER_CHECKBOX_CLASS}" aria-label="{DEFAULT_SPOILER_TITLE}: outer"/><div class="{SPOILER_BLOCK_CLASS}"><span class="{SPOILER_COVER_CLASS}">outer</span><div class="{SPOILER_BODY_CLASS}">
            <label class="block"><input type="checkbox" class="{SPOILER_CHECKBOX_CLASS}" aria-label="{DEFAULT_SPOILER_TITLE}: inner"/><div class="{SPOILER_BLOCK_CLASS}"><span class="{SPOILER_COVER_CLASS}">inner</span><div class="{SPOILER_BODY_CLASS}">
            <p class="{P_CLASS}">x</p>
            </div></div></label></div></div></label>
            <p class="{P_CLASS}">y</p>"#};
        assert_eq!(result, expected, "Should close the inner and the outer spoiler with the long closer");
    }

    #[test]
    fn test_render_shorter_closer_does_not_close_block_spoiler_opened_with_more_colons() {
        // The `:::` can't close a spoiler opened with four colons, so the block stays open
        // until the end of the document and the closer ends up as an (empty) inline spoiler.
        let markdown = ":::: spoiler\nx\n:::";
        let result = render_markdown(markdown).expect("Should render the markdown to html");

        let expected = formatdoc! {r#"
            <label class="block"><input type="checkbox" class="{SPOILER_CHECKBOX_CLASS}" aria-label="{DEFAULT_SPOILER_TITLE}"/><div class="{SPOILER_BLOCK_CLASS}"><span class="{SPOILER_COVER_CLASS}">Spoiler</span><div class="{SPOILER_BODY_CLASS}">
            <p class="{P_CLASS}">x</p>
            <p class="{P_CLASS}"><label><input type="checkbox" class="{SPOILER_CHECKBOX_CLASS}" aria-label="{DEFAULT_SPOILER_TITLE}"/><span class="{SPOILER_SPAN_CLASS}"><span class="{SPOILER_TEXT_CLASS}"></span></span></label></p></div></div></label>"#
        };
        assert_eq!(
            result,
            expected,
            "Should keep a block spoiler open when the closer has fewer colons than the opener"
        );
    }

    #[test]
    fn test_render_block_spoiler_inside_blockquote() {
        let markdown = "> ::: spoiler Q\n> hidden\n> :::\n\nafter";
        let result = render_markdown(markdown).expect("Should render the markdown to html");

        let expected = formatdoc! {r#"
            <blockquote class="{BLOCKQUOTE_CLASS}">
            <label class="block"><input type="checkbox" class="{SPOILER_CHECKBOX_CLASS}" aria-label="{DEFAULT_SPOILER_TITLE}: Q"/><div class="{SPOILER_BLOCK_CLASS}"><span class="{SPOILER_COVER_CLASS}">Q</span><div class="{SPOILER_BODY_CLASS}">
            <p>hidden</p>
            </div></div></label>
            </blockquote>
            <p class="{P_CLASS}">after</p>"#
        };
        assert_eq!(result, expected, "Should support a complete block spoiler inside a blockquote");
    }

    #[test]
    fn test_render_unclosed_block_spoiler_inside_blockquote_is_closed_with_the_blockquote() {
        let markdown = "> ::: spoiler Q\n> hidden\n\nafter";
        let result = render_markdown(markdown).expect("Should render the markdown to html");

        let expected = formatdoc! {r#"
            <blockquote class="{BLOCKQUOTE_CLASS}">
            <label class="block"><input type="checkbox" class="{SPOILER_CHECKBOX_CLASS}" aria-label="{DEFAULT_SPOILER_TITLE}: Q"/><div class="{SPOILER_BLOCK_CLASS}"><span class="{SPOILER_COVER_CLASS}">Q</span><div class="{SPOILER_BODY_CLASS}">
            <p>hidden</p>
            </div></div></label></blockquote>
            <p class="{P_CLASS}">after</p>"#
        };
        assert_eq!(result, expected, "Should close the block spoiler before the blockquote ends");
    }

    #[test]
    fn test_render_block_spoiler_with_crlf_line_endings() {
        let markdown = "::: spoiler Title\r\nhidden\r\n:::";
        let result = render_markdown(markdown).expect("Should render the markdown to html");

        let expected = formatdoc! {r#"
            <label class="block"><input type="checkbox" class="{SPOILER_CHECKBOX_CLASS}" aria-label="{DEFAULT_SPOILER_TITLE}: Title"/><div class="{SPOILER_BLOCK_CLASS}"><span class="{SPOILER_COVER_CLASS}">Title</span><div class="{SPOILER_BODY_CLASS}">
            <p class="{P_CLASS}">hidden</p>
            </div></div></label>"#
        };
        assert_eq!(result, expected, "Should handle Windows line endings");
    }

    #[test]
    fn test_render_block_spoiler_marker_after_longer_code_fence() {
        // The ``` inside the ```` fence doesn't close it, only the ```` does
        let markdown = "````\n```\n::: spoiler T\n````\n::: spoiler T\nx\n:::";
        let result = render_markdown(markdown).expect("Should render the markdown to html");

        let expected = formatdoc! {r#"
            <pre class="{CODE_BLOCK_CLASS}"><code>```
            ::: spoiler T
            </code></pre>
            <label class="block"><input type="checkbox" class="{SPOILER_CHECKBOX_CLASS}" aria-label="{DEFAULT_SPOILER_TITLE}: T"/><div class="{SPOILER_BLOCK_CLASS}"><span class="{SPOILER_COVER_CLASS}">T</span><div class="{SPOILER_BODY_CLASS}">
            <p class="{P_CLASS}">x</p>
            </div></div></label>"#
        };
        assert_eq!(
            result,
            expected,
            "Should only treat a fence of the same character and at least the same length as closing"
        );
    }

    #[test]
    fn test_render_block_spoiler_marker_after_tilde_fence() {
        let markdown = "~~~\n:::\n~~~\n::: spoiler T\nx\n:::";
        let result = render_markdown(markdown).expect("Should render the markdown to html");

        let expected = formatdoc! {r#"
            <pre class="{CODE_BLOCK_CLASS}"><code>:::
            </code></pre>
            <label class="block"><input type="checkbox" class="{SPOILER_CHECKBOX_CLASS}" aria-label="{DEFAULT_SPOILER_TITLE}: T"/><div class="{SPOILER_BLOCK_CLASS}"><span class="{SPOILER_COVER_CLASS}">T</span><div class="{SPOILER_BODY_CLASS}">
            <p class="{P_CLASS}">x</p>
            </div></div></label>"#
        };
        assert_eq!(result, expected, "Should support tilde fences");
    }

    #[test]
    fn test_render_inline_tag_in_block_marker_title_is_not_a_block_spoiler() {
        // `::: spoiler Title :::x:::` contains a second tag, so it is handled as inline spoilers
        let markdown = "::: spoiler Title :::x:::\nhidden";

        let result = render_markdown(markdown).expect("Should render the markdown to html");

        let expected = formatdoc! {r#"
            <p class="{P_CLASS}"><label><input type="checkbox" class="{SPOILER_CHECKBOX_CLASS}" aria-label="{DEFAULT_SPOILER_TITLE}"/><span class="{SPOILER_SPAN_CLASS}"><span class="{SPOILER_TEXT_CLASS}">spoiler Title</span></span></label>x<label><input type="checkbox" class="{SPOILER_CHECKBOX_CLASS}" aria-label="{DEFAULT_SPOILER_TITLE}"/><span class="{SPOILER_SPAN_CLASS}"><span class="{SPOILER_TEXT_CLASS}">hidden</span></span></label></p>"#};
        assert_eq!(result, expected, "Should not create a block spoiler for a line with a second tag");
    }

    // ---------------------------------------------------------------------
    // render_markdown: security
    // ---------------------------------------------------------------------

    #[test]
    fn test_render_raw_script_tag_is_escaped() {
        let markdown = "<script>alert(1)</script>";

        let result = render_markdown(markdown).expect("Should render the markdown to html");

        assert_eq!(result, "&lt;script&gt;alert(1)&lt;/script&gt;", "Should escape a raw script tag");
    }

    #[test]
    fn test_render_raw_html_inside_a_paragraph_is_escaped() {
        let markdown = "before <b onclick=\"x()\">bold</b> after";

        let result = render_markdown(markdown).expect("Should render the markdown to html");

        let expected = formatdoc! {r#"
            <p class="{P_CLASS}">before &lt;b onclick=&quot;x()&quot;&gt;bold&lt;/b&gt; after</p>"#
        };
        assert_eq!(result, expected, "Should escape inline raw html");
    }

    #[test]
    fn test_render_javascript_link_is_neutralized() {
        let markdown = "[x](javascript:alert(1))";

        let result = render_markdown(markdown).expect("Should render the markdown to html");

        let expected = formatdoc! {r#"
            <p class="{P_CLASS}"><a href="" class="{A_CLASS}">x</a></p>"#
        };
        assert_eq!(result, expected, "Should drop the javascript: url from the link");
    }

    #[test]
    fn test_render_javascript_image_source_is_neutralized() {
        let markdown = "![x](javascript:alert(1))";

        let result = render_markdown(markdown).expect("Should render the markdown to html");

        let expected = formatdoc! {r#"
            <p class="{P_CLASS}"><img src="" alt="x" /></p>"#
        };
        assert_eq!(result, expected, "Should drop the javascript: url from the image");
    }

    #[test]
    fn test_render_link_title_cannot_break_out_of_its_attribute() {
        let markdown = "[x](https://example.com \"t\\\" onmouseover=\\\"alert(1)\")";

        let result = render_markdown(markdown).expect("Should render the markdown to html");

        let expected = formatdoc! {r#"
            <p class="{P_CLASS}"><a href="https://example.com" title="t&quot; onmouseover=&quot;alert(1)" class="{A_CLASS}">x</a></p>"#
        };
        assert_eq!(result, expected, "Should escape quotes in the link title");
    }

    #[test]
    fn test_render_script_in_inline_spoiler_is_escaped() {
        let markdown = ":::<script>alert(1)</script>:::";

        let result = render_markdown(markdown).expect("Should render the markdown to html");

        let expected = formatdoc! {r#"
            <p class="{P_CLASS}"><label><input type="checkbox" class="{SPOILER_CHECKBOX_CLASS}" aria-label="{DEFAULT_SPOILER_TITLE}"/><span class="{SPOILER_SPAN_CLASS}"><span class="{SPOILER_TEXT_CLASS}">&lt;script&gt;alert(1)&lt;/script&gt;</span></span></label></p>"#
        };
        assert_eq!(result, expected, "Should escape html inside an inline spoiler");
    }

    #[test]
    fn test_render_script_in_block_spoiler_title_is_escaped() {
        let markdown = "::: spoiler <b>t</b>\nx\n:::";

        let result = render_markdown(markdown).expect("Should render the markdown to html");

        let expected = formatdoc! {r#"
            <label class="block"><input type="checkbox" class="{SPOILER_CHECKBOX_CLASS}" aria-label="{DEFAULT_SPOILER_TITLE}: &lt;b&gt;t&lt;/b&gt;"/><div class="{SPOILER_BLOCK_CLASS}"><span class="{SPOILER_COVER_CLASS}">&lt;b&gt;t&lt;/b&gt;</span><div class="{SPOILER_BODY_CLASS}">
            <p class="{P_CLASS}">x</p>
            </div></div></label>"#
        };
        assert_eq!(result, expected, "Should escape html in the title and the aria-label");
    }

    #[test]
    fn test_render_block_spoiler_title_cannot_break_out_of_the_aria_label() {
        let markdown = "::: spoiler x\" onfocus=\"alert(1)\ny\n:::";

        let result = render_markdown(markdown).expect("Should render the markdown to html");

        let expected = formatdoc! {r#"
            <label class="block"><input type="checkbox" class="{SPOILER_CHECKBOX_CLASS}" aria-label="{DEFAULT_SPOILER_TITLE}: x&quot; onfocus=&quot;alert(1)"/><div class="{SPOILER_BLOCK_CLASS}"><span class="{SPOILER_COVER_CLASS}">x&quot; onfocus=&quot;alert(1)</span><div class="{SPOILER_BODY_CLASS}">
            <p class="{P_CLASS}">y</p>
            </div></div></label>"#
        };
        assert_eq!(result, expected, "Should escape the quotes of the title in the aria-label attribute");
    }

    #[test]
    fn test_render_raw_html_in_block_spoiler_body_is_escaped() {
        let markdown = "::: spoiler T\n\n<script>alert(1)</script>\n\n:::";
        let result = render_markdown(markdown).expect("Should render the markdown to html");
        let expected = formatdoc! {r#"
            <label class="block"><input type="checkbox" class="{SPOILER_CHECKBOX_CLASS}" aria-label="{DEFAULT_SPOILER_TITLE}: T"/><div class="{SPOILER_BLOCK_CLASS}"><span class="{SPOILER_COVER_CLASS}">T</span><div class="{SPOILER_BODY_CLASS}">
            &lt;script&gt;alert(1)&lt;/script&gt;
            </div></div></label>"#
        };
        assert_eq!(result, expected, "Should escape raw html in the body of a block spoiler");
    }

    #[test]
    fn test_render_gfm_footnotes() {
        // The markdown crate adds class="sr-only" to the footnote heading and
        // class="data-footnote-backref" to the back link, so the css classes must be merged into them
        let markdown = "text[^1]\n\n[^1]: note";
        let result = render_markdown(markdown).expect("Should render the markdown to html");

        let expected = formatdoc! {r##"
            <p class="{P_CLASS}">text<sup><a href="#user-content-fn-1" id="user-content-fnref-1" data-footnote-ref="" aria-describedby="footnote-label" class="{A_CLASS}">1</a></sup></p>
            <section data-footnotes="" class="footnotes"><h2 id="footnote-label" class="sr-only {H2_CLASS}">Footnotes</h2>
            <ol class="{OL_CLASS}">
            <li id="user-content-fn-1">
            <p class="{P_CLASS}">note <a href="#user-content-fnref-1" data-footnote-backref="" aria-label="Back to content" class="data-footnote-backref {A_CLASS}">↩</a></p>
            </li>
            </ol>
            </section>
            "##};
        assert_eq!(result, expected, "Should append the css classes to the classes added by the footnote html");
    }

    #[test]
    fn test_render_pathological_inputs_do_not_panic_and_produce_balanced_html() {
        let long_colons = ":".repeat(10_000);
        let many_openers = "::: spoiler\n".repeat(200);
        let deep_quotes = format!("{} ::: spoiler Q\n{} x", ">".repeat(100), ">".repeat(100));
        let inputs: Vec<&str> = vec![
            "",
            ":",
            "::",
            ":::",
            "::::::",
            ":::\n:::\n:::",
            "::: spoiler",
            "::: spoiler\n",
            ":::: spoiler\n::: spoiler\n:::\n",
            "```\n::: spoiler\n",
            "~~~\n:::",
            "> > > ::: spoiler\n> > x",
            "- ::: spoiler\n- x",
            "a\r\n:::\r\nb",
            "::: spoiler 🎉\n🎉\n:::",
            ":::🎉:::🎉:::",
            "\u{0}:::\u{0}",
            "<!-- ::: spoiler -->",
            "&amp;&amp;::: &#xZZ; :::",
            &long_colons,
            &many_openers,
            &deep_quotes,
        ];

        for input in inputs {
            let result = render_markdown(input).expect("Should not fail for unusual input");

            assert_balanced_html(&result);
        }
    }

    // ---------------------------------------------------------------------
    // style_html_user_content
    // ---------------------------------------------------------------------

    #[test]
    fn test_style_html_leaves_other_empty_elements_unchanged() {
        let html = r#"<br/><img src="a.png"/>"#;

        let result = style_html_user_content(html).expect("Should style the html");

        assert_eq!(result, html, "Should not add classes to br and img");
    }

    #[test]
    fn test_style_html_passes_comments_through() {
        let html = "<!-- note --><p>a</p>";

        let result = style_html_user_content(html).expect("Should style the html");

        let expected = formatdoc! {r#"
            <!-- note --><p class="{P_CLASS}">a</p>"#
        };
        assert_eq!(result, expected, "Should keep comments as they are");
    }

    #[test]
    fn test_style_html_appends_class_to_existing_class_attribute() {
        let html = r#"<code class="language-rust">x</code>"#;
        let result = style_html_user_content(html).expect("Should style the html");
        let expected = formatdoc! {r#"
            <code class="language-rust {INLINE_CODE_CLASS}">x</code>"#
        };
        assert_eq!(result, expected, "Should append to the existing class attribute");
    }

    #[test]
    fn test_style_html_mismatched_end_tag_returns_error() {
        let html = "<p>text</div>";

        let result = style_html_user_content(html);

        assert!(result.is_err(), "Should fail for a mismatched end tag");
    }

    #[test]
    fn test_style_html_inline_spoiler_inside_a_paragraph() {
        let html = "<p>a ::: b ::: c</p>";

        let result = style_html_user_content(html).expect("Should style the html");

        let expected = formatdoc! {r#"
            <p class="{P_CLASS}">a <label><input type="checkbox" class="{SPOILER_CHECKBOX_CLASS}" aria-label="{DEFAULT_SPOILER_TITLE}"/><span class="{SPOILER_SPAN_CLASS}"><span class="{SPOILER_TEXT_CLASS}">b</span></span></label> c</p>"#
        };
        assert_eq!(result, expected, "Should convert the tags in a text node into a spoiler");
    }

    #[test]
    fn test_style_html_block_marker_paragraph_is_replaced_by_spoiler() {
        let html = "<p>::: spoiler T</p><p>x</p><p>:::</p>";

        let result = style_html_user_content(html).expect("Should style the html");

        let expected = formatdoc! {r#"
            <label class="block"><input type="checkbox" class="{SPOILER_CHECKBOX_CLASS}" aria-label="{DEFAULT_SPOILER_TITLE}: T"/><div class="{SPOILER_BLOCK_CLASS}"><span class="{SPOILER_COVER_CLASS}">T</span><div class="{SPOILER_BODY_CLASS}"><p class="{P_CLASS}">x</p></div></div></label>"#
        };
        assert_eq!(result, expected, "Should replace the marker paragraphs by the spoiler elements");
    }

    #[test]
    fn test_style_html_closer_without_open_block_is_a_normal_paragraph_with_spoiler_tag() {
        // Without an open block spoiler a `:::` paragraph is not a closer, it is handled as text
        let html = "<p>:::</p>";

        let result = style_html_user_content(html).expect("Should style the html");

        let expected = formatdoc! {r#"
            <p class="{P_CLASS}"><label><input type="checkbox" class="{SPOILER_CHECKBOX_CLASS}" aria-label="{DEFAULT_SPOILER_TITLE}"/><span class="{SPOILER_SPAN_CLASS}"><span class="{SPOILER_TEXT_CLASS}"></span></span></label></p>"#
        };
        assert_eq!(result, expected, "Should not treat a closer as a block closer when no block is open");
    }

    /// Builds an element from the content of a start tag, e.g. `a href="x"`.
    /// `String::from_utf8_lossy(&elem)` gives that content back, without the surrounding `<` and `>`.
    fn elem(content: &str) -> BytesStart<'static> {
        // from_content takes the row content between the `<` and `>` and the second attribute is the start of the attributes, that we get by searching for the first space.
        BytesStart::from_content(content.to_string(), content.find(' ').unwrap_or(content.len()))
    }

    // ---------------------------------------------------------------------
    // append_class_to_elem
    // ---------------------------------------------------------------------

    #[test]
    fn test_append_class_builds_expected_start_tag() {
        let cases = [

            ( // No attributes
                "p",
                r#"p class="foo""#
            ),
            ( // The class is added after the existing attributes
                r#"a href="x" title="t""#,
                r#"a href="x" title="t" class="foo""#
            ),
            ( // An existing class is extended and keeps its position
                r#"a href="x" class="a b" title="t""#,
                r#"a href="x" class="a b foo" title="t""#
            ),
            ( // Values are not escaped again, names with dashes and colons and unicode are kept
                r##"a data-ref="" xlink:href="#a?b=1&amp;c=2" title="Ünï &quot;🎉&quot;""##,
                r##"a data-ref="" xlink:href="#a?b=1&amp;c=2" title="Ünï &quot;🎉&quot;" class="foo""##,
            ),
            ( // Single quoted values are rewritten with double quotes
                "a href='x'",
                r#"a href="x" class="foo""#
            ),
            ( // A double quote in a single quoted value can't break out of the attribute
                r#"a title='x" onmouseover="y'"#,
                r#"a title="x&quot; onmouseover=&quot;y" class="foo""#
            ),
            ( // A double quote in a single quoted value can't break out of the attribute
                r#"p class='a" onclick="y'"#,
                r#"p class="a&quot; onclick=&quot;y foo""#
            ),
            ( // Html attribute without a value
                "input disabled",
                r#"input disabled="" class="foo""#
            ),
            ( // `<hr />` is parsed with a trailing space, which must not end up before the class
                "hr ",
                r#"hr class="foo""#
            ),
        ];
        for (content, expected) in cases {
            let mut elem = elem(content);
            append_class_to_elem(&mut elem, "foo").expect("Should append the class");
            assert_eq!(String::from_utf8_lossy(&elem), expected, "Should build the expected tag for: {content}");
        }
    }

    #[test]
    fn test_append_class_twice_keeps_a_single_class_attribute() {
        let mut elem = elem("p");
        append_class_to_elem(&mut elem, "a").expect("Should append the first class");
        append_class_to_elem(&mut elem, "b").expect("Should append the second class");
        assert_eq!(String::from_utf8_lossy(&elem), r#"p class="a b""#, "Should append to the class added before");
    }

    #[test]
    fn test_append_class_with_invalid_attributes_returns_error() {
        let invalid = [
            r#"a href="x"#, // Unclosed quote
            r#"p class="a" class="b""#, // Duplicate attribute
        ];
        for content in invalid {
            append_class_to_elem(&mut elem(content), "foo").expect_err("Should fail for invalid attributes");
        }
    }

    // ---------------------------------------------------------------------
    // process_elem
    // ---------------------------------------------------------------------

    #[test]
    fn test_process_elem_adds_css_class_to_styled_elements() {
        let styled = [
            ("h1", H1_CLASS),
            ("h2", H2_CLASS),
            ("h3", H3_CLASS),
            ("p", P_CLASS),
            ("a", A_CLASS),
            ("ul", UL_CLASS),
            ("ol", OL_CLASS),
            ("pre", CODE_BLOCK_CLASS),
            ("code", INLINE_CODE_CLASS),
            ("table", TABLE_CLASS),
            ("blockquote", BLOCKQUOTE_CLASS),
        ];
        for (name, class) in styled {
            let mut elem = elem(name);
            let mut block_depth = 0;
            let (mut is_in_code, mut is_in_pre) = (false, false);
            process_elem(&mut elem, &mut block_depth, &mut is_in_code, &mut is_in_pre).expect("Should process the element");
            assert_eq!(
                String::from_utf8_lossy(&elem),
                format!(r#"{name} class="{class}""#),
                "Should add the css class of {name}"
            );
        }
    }

    #[test]
    fn test_process_elem_leaves_other_elements_and_flags_unchanged() {
        let unstyled = [
            "h4",
            "div",
            "li",
            "strong",
            r#"li id="user-content-fn-1""#,
            r#"section data-footnotes="" class="footnotes""#,
            // Only whole, lowercase element names are matched
            "abbr",
            "param",
            "preview",
            "codex",
            "tablex",
            "H1",
        ];
        for content in unstyled {
            let mut elem = elem(content);
            let mut block_depth = 1;
            let (mut is_in_code, mut is_in_pre) = (true, true);
            process_elem(&mut elem, &mut block_depth, &mut is_in_code, &mut is_in_pre).expect("Should process the element");
            assert_eq!(String::from_utf8_lossy(&elem), content, "Should not change the element: {content}");
            assert!(block_depth == 1 && is_in_code && is_in_pre, "Should not reset any flag for: {content}");
        }
    }

    #[test]
    fn test_process_elem_handles_blockquote_flag() {
        let cases = [
            ( // (start tag, block_depth before, expected start tag, block_depth after)
                "p", 0,
                format!(r#"p class="{P_CLASS}""#), 0
            ),
            ( // Paragraphs in a blockquote are not styled
                "p", 1,
                "p".to_string(), 1
            ),
            (
                "blockquote", 0,
                format!(r#"blockquote class="{BLOCKQUOTE_CLASS}""#), 1
            ),
            (
                "blockquote", 1,
                format!(r#"blockquote class="{BLOCKQUOTE_CLASS}""#), 2
            ),
            ( // Everything else in a blockquote is still styled
                "ul", 1,
                format!(r#"ul class="{UL_CLASS}""#), 1
            ),
        ];
        for (content, mut block_depth, expected, expected_block_depth) in cases {
            let mut elem = elem(content);
            let (mut is_in_code, mut is_in_pre) = (false, false);
            process_elem(&mut elem, &mut block_depth, &mut is_in_code, &mut is_in_pre).expect("Should process the element");
            assert_eq!(String::from_utf8_lossy(&elem), expected, "Should build the expected tag for: {content}");
            assert_eq!(block_depth, expected_block_depth, "Should update the blockquote depth for: {content}");
            assert!(!is_in_code && !is_in_pre, "Should not touch the other flags for: {content}");
        }
    }

    #[test]
    fn test_process_elem_handles_code_and_pre_flags() {
        let cases = [
            // (start tag, is_in_pre before, expected start tag, is_in_code after, is_in_pre after)
            ("pre", false, format!(r#"pre class="{CODE_BLOCK_CLASS}""#), false, true),
            ("code", false, format!(r#"code class="{INLINE_CODE_CLASS}""#), true, false),
            // Code in a code block is not styled, not even when it has its own (language) class
            ("code", true, "code".to_string(), true, true),
            (r#"code class="language-rust""#, true, r#"code class="language-rust""#.to_string(), true, true),
        ];
        for (content, in_pre, expected, expected_in_code, expected_in_pre) in cases {
            let mut elem = elem(content);
            let mut block_depth = 0;
            let (mut is_in_code, mut is_in_pre) = (false, in_pre);
            process_elem(&mut elem, &mut block_depth, &mut is_in_code, &mut is_in_pre).expect("Should process the element");
            assert_eq!(String::from_utf8_lossy(&elem), expected, "Should build the expected tag for: {content}");
            assert_eq!(is_in_code, expected_in_code, "Should update the code flag for: {content}");
            assert_eq!(is_in_pre, expected_in_pre, "Should update the pre flag for: {content}");
            assert_eq!(block_depth, 0, "Should not touch the blockquote depth for: {content}");
        }
    }

    #[test]
    fn test_process_elem_only_parses_attributes_of_styled_elements() {
        let mut block_depth = 0;
        let (mut is_in_code, mut is_in_pre) = (false, false);

        let mut styled = elem(r#"a href="x"#);
        process_elem(&mut styled, &mut block_depth, &mut is_in_code, &mut is_in_pre)
            .expect_err("Should fail when the attributes of a styled element are invalid");

        let mut unstyled = elem(r#"div title="x"#);
        process_elem(&mut unstyled, &mut block_depth, &mut is_in_code, &mut is_in_pre)
            .expect("Should not look at the attributes of an element without class");
        assert_eq!(String::from_utf8_lossy(&unstyled), r#"div title="x"#, "Should leave the unstyled element as it is");
    }

    // ---------------------------------------------------------------------
    // separate_spoiler_markers
    // ---------------------------------------------------------------------

    #[test]
    fn test_separate_markers_without_spoiler_tag_returns_input_borrowed() {
        let markdown = "plain text\nsecond line";

        let result = separate_spoiler_markers(markdown);

        assert_matches!(
            result,
            Cow::Borrowed(_),
            "Should not allocate when the input has no spoiler tag"
        );
    }

    #[test]
    fn test_separate_markers_puts_block_markers_on_their_own_paragraph() {
        let markdown = "a\n::: spoiler T\nb\n:::";

        let result = separate_spoiler_markers(markdown);

        assert_eq!(
            result,
            "a\n\n::: spoiler T\n\nb\n\n:::",
            "Should surround the opener and closer with blank lines"
        );
    }

    #[test]
    fn test_separate_markers_trims_marker_lines() {
        let markdown = "   ::: spoiler T   ";

        let result = separate_spoiler_markers(markdown);

        assert_eq!(result, "\n::: spoiler T", "Should remove whitespace around the marker");
    }

    #[test]
    fn test_separate_markers_keeps_blockquote_prefix() {
        let markdown = "> ::: spoiler Q\n> x";

        let result = separate_spoiler_markers(markdown);

        assert_eq!(
            result,
            ">\n> ::: spoiler Q\n>\n> x",
            "Should keep the blockquote markers on the added blank lines"
        );
    }

    #[test]
    fn test_separate_markers_keeps_nested_blockquote_prefix() {
        let markdown = "> > :::";

        let result = separate_spoiler_markers(markdown);

        assert_eq!(
            result,
            "> >\n> > :::\n> >",
            "Should keep all blockquote markers on the added blank lines"
        );
    }

    #[test]
    fn test_separate_markers_converts_crlf_to_lf() {
        let markdown = "a\r\n:::\r\nb";

        let result = separate_spoiler_markers(markdown);

        assert_eq!(result, "a\n\n:::\n\nb", "Should normalize line endings");
    }

    #[test]
    fn test_separate_markers_leaves_inline_spoilers_untouched() {
        let markdown = "a ::: b ::: c";

        let result = separate_spoiler_markers(markdown);

        assert_eq!(result, markdown, "Should not change lines that are not a block marker");
    }

    #[test]
    fn test_separate_markers_leaves_fenced_code_untouched() {
        let markdown = "```\n::: spoiler T\n:::\n```";

        let result = separate_spoiler_markers(markdown);

        assert_eq!(result, markdown, "Should not touch markers inside a code fence");
    }

    #[test]
    fn test_separate_markers_leaves_indented_code_untouched() {
        let markdown = "    ::: spoiler T";

        let result = separate_spoiler_markers(markdown);

        assert_eq!(result, markdown, "Should not touch markers in an indented code block");
    }

    #[test]
    fn test_separate_markers_fence_is_only_closed_by_same_character() {
        let markdown = "```\n~~~\n::: spoiler T\n```\n::: spoiler T";

        let result = separate_spoiler_markers(markdown);

        assert_eq!(
            result,
            "```\n~~~\n::: spoiler T\n```\n\n::: spoiler T",
            "Should keep the first marker inside the fence and separate the second one"
        );
    }

    #[test]
    fn test_separate_markers_fence_needs_enough_characters_to_close() {
        let markdown = "````\n```\n::: spoiler T\n````";

        let result = separate_spoiler_markers(markdown);

        assert_eq!(result, markdown, "Should stay in the fence when the closing line is too short");
    }

    #[test]
    fn test_separate_markers_inline_code_on_one_line_is_not_a_fence() {
        let markdown = "```code```\n::: spoiler T";

        let result = separate_spoiler_markers(markdown);

        assert_eq!(
            result,
            "```code```\n\n::: spoiler T",
            "Should not enter fence mode for ```code``` on one line"
        );
    }

    #[test]
    fn test_separate_markers_unclosed_fence_swallows_the_rest() {
        let markdown = "```\n::: spoiler T\nx\n:::";

        let result = separate_spoiler_markers(markdown);

        assert_eq!(result, markdown, "Should leave everything after an unclosed fence unchanged");
    }

    #[test]
    fn test_separate_markers_keeps_a_single_trailing_newline() {
        let markdown = "a ::: b\n";

        let result = separate_spoiler_markers(markdown);

        assert_eq!(result, markdown, "Should keep a single trailing newline");
    }

    #[test]
    fn test_separate_markers_does_not_add_a_trailing_newline() {
        let markdown = "a ::: b";

        let result = separate_spoiler_markers(markdown);

        assert_eq!(result, markdown, "Should not add a trailing newline");
    }

    // ---------------------------------------------------------------------
    // parse_spoiler_block_marker
    // ---------------------------------------------------------------------

    #[test]
    fn test_parse_marker_opener_with_title() {
        let text = "::: spoiler My title";

        let result = parse_spoiler_block_marker(text);

        assert_eq!(result, Some(SpoilerBlockMarker::Open { colons: 3, title: String::from("My title") }), "Should parse the colons and the title");
    }

    #[test]
    fn test_parse_marker_opener_without_title_has_empty_title() {
        let text = "::: spoiler";

        let result = parse_spoiler_block_marker(text);

        assert_eq!(result, Some(SpoilerBlockMarker::Open { colons: 3, title: String::from("") }), "Should parse an opener without title");
    }

    #[test]
    fn test_parse_marker_opener_counts_all_colons() {
        let text = ":::::: spoiler T";

        let result = parse_spoiler_block_marker(text);

        assert_eq!(result, Some(SpoilerBlockMarker::Open { colons: 6, title: String::from("T") }), "Should report the number of colons");
    }

    #[test]
    fn test_parse_marker_opener_trims_surrounding_whitespace() {
        let text = "   ::: spoiler   T   ";

        let result = parse_spoiler_block_marker(text);

        assert_eq!(result, Some(SpoilerBlockMarker::Open { colons: 3, title: String::from("T") }), "Should trim the line and the title");
    }

    #[test]
    fn test_parse_marker_opener_without_space_after_colons() {
        let text = ":::spoiler T";

        let result = parse_spoiler_block_marker(text);

        assert_eq!(
            result,
            Some(SpoilerBlockMarker::Open { colons: 3, title: String::from("T") }),
            "Should accept the keyword directly after the colons"
        );
    }

    #[test]
    fn test_parse_marker_opener_with_tab_before_title() {
        let text = "::: spoiler\ttab title";

        let result = parse_spoiler_block_marker(text);

        assert_eq!(
            result,
            Some(SpoilerBlockMarker::Open { colons: 3, title: String::from("tab title") }),
            "Should accept any whitespace between keyword and title"
        );
    }

    #[test]
    fn test_parse_marker_opener_with_unicode_title() {
        let text = "::: spoiler Ünï 🎉";

        let result = parse_spoiler_block_marker(text);

        assert_eq!(
            result,
            Some(SpoilerBlockMarker::Open { colons: 3, title: String::from("Ünï 🎉") }),
            "Should keep non-ASCII characters in the title"
        );
    }

    #[test]
    fn test_parse_marker_closer() {
        for (text, expected_colons) in [(":::", 3), ("::::: ", 5)] {
            let result = parse_spoiler_block_marker(text);

            assert_eq!(result, Some(SpoilerBlockMarker::Close { colons: expected_colons }), "Should parse {text:?} as a closer");
        }
    }

    #[test]
    fn test_parse_marker_with_two_colons_is_not_a_marker() {
        for text in [":: spoiler T", "::"] {
            let result = parse_spoiler_block_marker(text);

            assert!(result.is_none(), "Should need at least three colons in {text:?}");
        }
    }

    #[test]
    fn test_parse_marker_with_other_keyword_is_not_a_marker() {
        for text in ["::: warning", "::: Spoiler T"] {
            let result = parse_spoiler_block_marker(text);

            assert!(result.is_none(), "Should only accept the lowercase spoiler keyword in {text:?}");
        }
    }

    #[test]
    fn test_parse_marker_with_longer_keyword_is_not_a_marker() {
        let text = "::: spoilerx";

        let result = parse_spoiler_block_marker(text);

        assert!(result.is_none(), "Should require the keyword to end after 'spoiler'");
    }

    #[test]
    fn test_parse_marker_with_second_tag_in_title_is_not_a_marker() {
        let text = "::: spoiler a ::: b";

        let result = parse_spoiler_block_marker(text);

        assert!(result.is_none(), "Should leave a line with a second tag to the inline spoiler");
    }

    #[test]
    fn test_parse_marker_with_newline_in_title_is_not_a_marker() {
        let text = "::: spoiler a\nb";

        let result = parse_spoiler_block_marker(text);

        assert!(result.is_none(), "Should require the title on a single line");
    }

    #[test]
    fn test_parse_marker_plain_text_and_empty_are_not_markers() {
        for text in ["hello", "", "text ::: spoiler"] {
            let result = parse_spoiler_block_marker(text);

            assert!(result.is_none(), "Should not parse {text:?} as a marker");
        }
    }

    // ---------------------------------------------------------------------
    // split_quote_prefix
    // ---------------------------------------------------------------------

    #[test]
    fn test_split_quote_prefix_single_quote() {
        let line = "> text";

        let result = split_quote_prefix(line);

        assert_eq!(result, ("> ", "text"), "Should split '> ' from the text");
    }

    #[test]
    fn test_split_quote_prefix_without_space() {
        let line = ">text";

        let result = split_quote_prefix(line);

        assert_eq!(result, (">", "text"), "Should split '>' without a space");
    }

    #[test]
    fn test_split_quote_prefix_nested_quotes() {
        for (line, expected) in [("> > text", ("> > ", "text")), (">> text", (">> ", "text"))] {
            let result = split_quote_prefix(line);

            assert_eq!(result, expected, "Should split the nested quote prefix of {line:?}");
        }
    }

    #[test]
    fn test_split_quote_prefix_without_quote() {
        for (line, expected) in [("text", ("", "text")), ("", ("", ""))] {
            let result = split_quote_prefix(line);

            assert_eq!(result, expected, "Should return an empty prefix for {line:?}");
        }
    }

    #[test]
    fn test_split_quote_prefix_with_up_to_three_leading_spaces() {
        let line = "  > text";

        let result = split_quote_prefix(line);

        assert_eq!(result, ("  > ", "text"), "Should allow up to three spaces before '>'");
    }

    #[test]
    fn test_split_quote_prefix_with_four_leading_spaces_is_code() {
        let line = "    > text";

        let result = split_quote_prefix(line);

        assert_eq!(result, ("", "    > text"), "Should not see a quote in an indented code line");
    }

    #[test]
    fn test_split_quote_prefix_only_marker() {
        let line = ">";

        let result = split_quote_prefix(line);

        assert_eq!(result, (">", ""), "Should handle a line with only a marker");
    }

    #[test]
    fn test_split_quote_prefix_removes_only_one_space_after_marker() {
        let line = ">  text";

        let result = split_quote_prefix(line);

        assert_eq!(result, ("> ", " text"), "Should keep the additional space in the rest");
    }

    // ---------------------------------------------------------------------
    // parse_code_fence_opener / parse_code_fence_closer
    // ---------------------------------------------------------------------

    #[test]
    fn test_code_fence_opener_with_backticks() {
        for (line, expected) in [("```", ('`', 3)), ("```rust", ('`', 3)), ("`````", ('`', 5))] {
            let result = parse_code_fence_opener(line);

            assert_eq!(result, Some(expected), "Should detect a backtick fence and its length in {line:?}");
        }
    }

    #[test]
    fn test_code_fence_opener_with_tildes() {
        for (line, expected) in [("~~~~", ('~', 4)), ("~~~a`b", ('~', 3))] {
            let result = parse_code_fence_opener(line);

            assert_eq!(result, Some(expected), "Should detect a tilde fence and its length in {line:?}");
        }
    }

    #[test]
    fn test_code_fence_opener_rejects_short_fences() {
        for line in ["``", "~~"] {
            let result = parse_code_fence_opener(line);

            assert_eq!(result, None, "Should need at least three fence characters in {line:?}");
        }
    }

    #[test]
    fn test_code_fence_opener_rejects_inline_code_on_one_line() {
        for line in ["```code```", "```a`b"] {
            let result = parse_code_fence_opener(line);

            assert_eq!(result, None, "Should not allow a backtick in a backtick fence info string: {line:?}");
        }
    }

    #[test]
    fn test_code_fence_opener_rejects_other_text() {
        for line in ["text", "", "a```"] {
            let result = parse_code_fence_opener(line);

            assert_eq!(result, None, "Should not detect a fence in {line:?}");
        }
    }

    #[test]
    fn test_code_fence_closer_matches_same_character_and_enough_length() {
        for (line, fence_char, fence_len) in [("```", '`', 3), ("````", '`', 3), ("~~~", '~', 3)] {
            let result = parse_code_fence_closer(line, fence_char, fence_len);

            assert!(result, "Should close a {fence_char}-fence of length {fence_len} with {line:?}");
        }
    }

    #[test]
    fn test_code_fence_closer_rejects_short_or_different_lines() {
        for (line, fence_char, fence_len) in [("``", '`', 3), ("~~~", '`', 3), ("```x", '`', 3), ("", '`', 3)] {
            let result = parse_code_fence_closer(line, fence_char, fence_len);

            assert!(!result, "Should not close a {fence_char}-fence of length {fence_len} with {line:?}");
        }
    }

    // ---------------------------------------------------------------------
    // open_spoiler / close_spoiler / open_block_spoiler / close_block_spoiler
    // ---------------------------------------------------------------------

    #[test]
    fn test_open_spoiler_writes_label_checkbox_cover_and_text_span() {
        let mut writer = new_writer();

        open_spoiler(&mut writer).expect("Should write the spoiler opening");

        let result = written(writer);
        let expected = formatdoc! {r#"
            <label><input type="checkbox" class="{SPOILER_CHECKBOX_CLASS}" aria-label="{DEFAULT_SPOILER_TITLE}"/><span class="{SPOILER_SPAN_CLASS}"><span class="{SPOILER_TEXT_CLASS}">"#};
        assert_eq!(result, expected, "Should open label, labelled checkbox, cover span and text span");
    }

    #[test]
    fn test_close_spoiler_closes_text_span_cover_span_and_label() {
        let mut writer = new_writer();

        close_spoiler(&mut writer).expect("Should write the spoiler closing");

        let result = written(writer);
        assert_eq!(result, "</span></span></label>", "Should close text span, cover span and label");
    }

    #[test]
    fn test_open_block_spoiler_writes_cover_with_title_and_body() {
        let mut writer = new_writer();

        open_block_spoiler(&mut writer, "My title").expect("Should write the block spoiler opening");

        let result = written(writer);
        let expected = formatdoc! {r#"
            <label class="block"><input type="checkbox" class="{SPOILER_CHECKBOX_CLASS}" aria-label="{DEFAULT_SPOILER_TITLE}: My title"/><div class="{SPOILER_BLOCK_CLASS}"><span class="{SPOILER_COVER_CLASS}">My title</span><div class="{SPOILER_BODY_CLASS}">"#};
        assert_eq!(result, expected, "Should open label, checkbox labelled with the title, block, cover and body");
    }

    #[test]
    fn test_open_block_spoiler_with_empty_title_uses_default() {
        let mut writer = new_writer();

        open_block_spoiler(&mut writer, "").expect("Should write the block spoiler opening");

        let result = written(writer);
        let expected = formatdoc! {r#"
            <label class="block"><input type="checkbox" class="{SPOILER_CHECKBOX_CLASS}" aria-label="{DEFAULT_SPOILER_TITLE}"/><div class="{SPOILER_BLOCK_CLASS}"><span class="{SPOILER_COVER_CLASS}">{DEFAULT_SPOILER_TITLE}</span><div class="{SPOILER_BODY_CLASS}">"#};
        assert_eq!(result, expected, "Should use the default title for the cover and the aria-label");
    }

    #[test]
    fn test_open_block_spoiler_escapes_the_title() {
        let mut writer = new_writer();
        let title = "<img src=x onerror=alert(1)> & \"q\"";

        open_block_spoiler(&mut writer, title).expect("Should write the block spoiler opening");

        let result = written(writer);
        let expected = formatdoc! {r#"
            <label class="block"><input type="checkbox" class="{SPOILER_CHECKBOX_CLASS}" aria-label="{DEFAULT_SPOILER_TITLE}: &lt;img src=x onerror=alert(1)&gt; &amp; &quot;q&quot;"/><div class="{SPOILER_BLOCK_CLASS}"><span class="{SPOILER_COVER_CLASS}">&lt;img src=x onerror=alert(1)&gt; &amp; &quot;q&quot;</span><div class="{SPOILER_BODY_CLASS}">"#};
        assert_eq!(result, expected, "Should escape html in the title and the aria-label");
    }

    #[test]
    fn test_open_block_spoiler_title_cannot_break_out_of_the_aria_label() {
        let mut writer = new_writer();

        open_block_spoiler(&mut writer, "x\" onfocus=\"y").expect("Should write the block spoiler opening");

        let result = written(writer);
        let expected = formatdoc! {r#"
            <label class="block"><input type="checkbox" class="{SPOILER_CHECKBOX_CLASS}" aria-label="{DEFAULT_SPOILER_TITLE}: x&quot; onfocus=&quot;y"/><div class="{SPOILER_BLOCK_CLASS}"><span class="{SPOILER_COVER_CLASS}">x&quot; onfocus=&quot;y</span><div class="{SPOILER_BODY_CLASS}">"#};
        assert_eq!(result, expected, "Should escape the quotes in the aria-label attribute");
    }

    #[test]
    fn test_close_block_spoiler_closes_body_block_and_label() {
        let mut writer = new_writer();

        close_block_spoiler(&mut writer).expect("Should write the block spoiler closing");

        let result = written(writer);
        assert_eq!(result, "</div></div></label>", "Should close body, block and label");
    }

    // ---------------------------------------------------------------------
    // SpoilerState
    // ---------------------------------------------------------------------

    #[test]
    fn test_state_start_element_increases_depth_and_resets_trim() {
        let mut state = SpoilerState { trim_start: true, ..Default::default() };

        state.start_element();

        assert_eq!(state.element_depth, 1, "Should count the open element");
        assert!(!state.trim_start, "Should stop trimming once an element starts");
    }

    #[test]
    fn test_state_other_event_resets_trim() {
        let mut state = SpoilerState { trim_start: true, ..Default::default() };

        state.other_event();

        assert!(!state.trim_start, "Should stop trimming after another event");
    }

    #[test]
    fn test_state_end_element_without_open_element_does_not_underflow() {
        let mut state = SpoilerState::default();
        let mut writer = new_writer();

        state.end_element(&mut writer).expect("Should handle an unexpected end element");

        let result = written(writer);
        assert_eq!(result, "", "Should not write anything");
        assert_eq!(state.element_depth, 0, "Should keep the depth at zero");
    }

    #[test]
    fn test_state_end_element_closes_inline_spoiler_opened_at_that_depth() {
        let mut state = SpoilerState { element_depth: 2, spoiler_depth: Some(2), ..Default::default() };
        let mut writer = new_writer();

        state.end_element(&mut writer).expect("Should close the spoiler");

        let result = written(writer);
        assert_eq!(result, "</span></span></label>", "Should close the inline spoiler");
        assert_eq!(state.spoiler_depth, None, "Should forget the closed spoiler");
        assert_eq!(state.element_depth, 1, "Should decrease the depth");
    }

    #[test]
    fn test_state_end_element_keeps_inline_spoiler_opened_in_a_parent() {
        let mut state = SpoilerState { element_depth: 2, spoiler_depth: Some(1), ..Default::default() };
        let mut writer = new_writer();

        state.end_element(&mut writer).expect("Should end the child element");

        let result = written(writer);
        assert_eq!(result, "", "Should not close a spoiler of a parent element");
        assert_eq!(state.spoiler_depth, Some(1), "Should keep the spoiler open");
    }

    #[test]
    fn test_state_end_element_closes_unclosed_block_spoilers_of_that_depth() {
        let mut state = SpoilerState { element_depth: 1, blocks: vec![(0, 3), (1, 3), (1, 4)], ..Default::default() };
        let mut writer = new_writer();

        state.end_element(&mut writer).expect("Should close the blocks");

        let result = written(writer);
        assert_eq!(result, "</div></div></label></div></div></label>", "Should close both blocks of depth 1");
        assert_eq!(state.blocks, vec![(0, 3)], "Should keep the block of the parent");
    }

    #[test]
    fn test_state_open_block_registers_block_at_current_depth() {
        let mut state = SpoilerState { element_depth: 2, trim_start: true, ..Default::default() };
        let mut writer = new_writer();

        state.open_block(&mut writer, "T", 4).expect("Should open the block");

        let result = written(writer);
        let expected = formatdoc! {r#"
            <label class="block"><input type="checkbox" class="{SPOILER_CHECKBOX_CLASS}" aria-label="{DEFAULT_SPOILER_TITLE}: T"/><div class="{SPOILER_BLOCK_CLASS}"><span class="{SPOILER_COVER_CLASS}">T</span><div class="{SPOILER_BODY_CLASS}">"#
        };
        assert_eq!(result, expected, "Should write the block opening");
        assert_eq!(state.blocks, vec![(2, 4)], "Should store depth and colons");
        assert!(!state.trim_start, "Should reset trimming");
    }

    #[test]
    fn test_state_block_to_close_finds_outermost_matching_block() {
        let state = SpoilerState { element_depth: 0, blocks: vec![(0, 4), (0, 3)], ..Default::default() };

        for (closer_colons, expected) in [(4, Some(0)), (3, Some(1)), (2, None)] {
            let result = state.block_to_close(closer_colons);

            assert_eq!(
                result,
                expected,
                "Should pick the outermost block a closer with {closer_colons} colons can close"
            );
        }
    }

    #[test]
    fn test_state_block_to_close_ignores_blocks_at_other_depths() {
        let state = SpoilerState { element_depth: 1, blocks: vec![(0, 3), (2, 3)], ..Default::default() };

        let result = state.block_to_close(3);

        assert_eq!(result, None, "Should not close blocks of another depth");
    }

    #[test]
    fn test_state_close_blocks_from_closes_block_and_nested_blocks() {
        let mut state = SpoilerState { blocks: vec![(0, 4), (0, 3), (0, 3)], trim_start: true, ..Default::default() };
        let mut writer = new_writer();

        state.close_blocks_from(&mut writer, 1).expect("Should close the blocks");

        let result = written(writer);
        assert_eq!(result, "</div></div></label></div></div></label>", "Should close two blocks");
        assert_eq!(state.blocks, vec![(0, 4)], "Should keep the blocks before the index");
        assert!(!state.trim_start, "Should reset trimming");
    }

    #[test]
    fn test_state_close_blocks_from_index_past_the_end_does_nothing() {
        let mut state = SpoilerState { blocks: vec![(0, 3)], ..Default::default() };
        let mut writer = new_writer();

        state.close_blocks_from(&mut writer, 5).expect("Should do nothing");

        let result = written(writer);
        assert_eq!(result, "", "Should not write anything");
        assert_eq!(state.blocks, vec![(0, 3)], "Should keep the block");
    }

    #[test]
    fn test_state_finish_closes_inline_spoiler_and_all_blocks() {
        let mut state = SpoilerState { spoiler_depth: Some(1), blocks: vec![(0, 3), (0, 3)], ..Default::default() };
        let mut writer = new_writer();

        state.finish(&mut writer).expect("Should close everything");

        let result = written(writer);
        assert_eq!(
            result,
            "</span></span></label></div></div></label></div></div></label>",
            "Should close the inline spoiler first and then the blocks"
        );
        assert_eq!(state.spoiler_depth, None, "Should forget the inline spoiler");
        assert!(state.blocks.is_empty(), "Should forget the blocks");
    }

    #[test]
    fn test_state_finish_with_nothing_open_writes_nothing() {
        let mut state = SpoilerState::default();
        let mut writer = new_writer();

        state.finish(&mut writer).expect("Should finish");

        let result = written(writer);
        assert_eq!(result, "", "Should not write anything");
    }

    #[test]
    fn test_state_process_inline_spoilers_plain_text_is_written_unchanged() {
        let mut state = SpoilerState::default();
        let mut writer = new_writer();

        state.process_inline_spoilers(&mut writer, "a < b").expect("Should write the text");

        let result = written(writer);
        assert_eq!(result, "a &lt; b", "Should write the (escaped) text");
    }

    #[test]
    fn test_state_process_inline_spoilers_opens_and_closes_in_one_text() {
        let mut state = SpoilerState::default();
        let mut writer = new_writer();

        state.process_inline_spoilers(&mut writer, "a ::: b ::: c").expect("Should process the text");

        let result = written(writer);
        let expected = formatdoc! {r#"
            a <label><input type="checkbox" class="{SPOILER_CHECKBOX_CLASS}" aria-label="{DEFAULT_SPOILER_TITLE}"/><span class="{SPOILER_SPAN_CLASS}"><span class="{SPOILER_TEXT_CLASS}">b</span></span></label> c"#
        };
        assert_eq!(result, expected, "Should wrap b in a spoiler");
        assert_eq!(state.spoiler_depth, None, "Should have closed the spoiler");
    }

    #[test]
    fn test_state_process_inline_spoilers_keeps_spoiler_open_across_text_events() {
        let mut state = SpoilerState::default();
        let mut writer = new_writer();

        state.process_inline_spoilers(&mut writer, "a ::: b ").expect("Should open the spoiler");
        assert_eq!(state.spoiler_depth, Some(0), "Should remember where the spoiler was opened");
        state.process_inline_spoilers(&mut writer, " c ::: d").expect("Should close the spoiler");

        let result = written(writer);
        let expected = formatdoc! {r#"
            a <label><input type="checkbox" class="{SPOILER_CHECKBOX_CLASS}" aria-label="{DEFAULT_SPOILER_TITLE}"/><span class="{SPOILER_SPAN_CLASS}"><span class="{SPOILER_TEXT_CLASS}">b  c</span></span></label> d"#
        };
        assert_eq!(result, expected, "Should continue the spoiler in the next text and trim its end");
    }

    #[test]
    fn test_state_process_inline_spoilers_tag_in_nested_element_is_kept_as_text() {
        let mut state = SpoilerState { element_depth: 2, spoiler_depth: Some(1), ..Default::default() };
        let mut writer = new_writer();

        state.process_inline_spoilers(&mut writer, "x ::: y").expect("Should process the text");

        let result = written(writer);
        assert_eq!(result, "x ::: y", "Should not close a spoiler from a nested element");
        assert_eq!(state.spoiler_depth, Some(1), "Should keep the spoiler open");
    }

    #[test]
    fn test_state_process_inline_spoilers_trims_start_only_right_after_opening() {
        let mut state = SpoilerState { element_depth: 0, spoiler_depth: Some(0), trim_start: false, ..Default::default() };
        let mut writer = new_writer();

        state.process_inline_spoilers(&mut writer, "  keep").expect("Should write the text");

        let result = written(writer);
        assert_eq!(result, "  keep", "Should keep leading spaces when not directly after the opening tag");
    }

    // ---------------------------------------------------------------------
    // read_spoiler_block_marker
    // ---------------------------------------------------------------------

    #[test]
    fn test_read_marker_opener_paragraph() {
        let html = "<p>::: spoiler T</p>";

        let (result, _) = marker_after_first_p(html).expect("Should find a marker");

        assert_eq!(result, SpoilerBlockMarker::Open { colons: 3, title: String::from("T") }, "Should parse the opener");
    }

    #[test]
    fn test_read_marker_closer_paragraph() {
        let html = "<p>::::</p>";

        let (result, _) = marker_after_first_p(html).expect("Should find a marker");

        assert_eq!(result, SpoilerBlockMarker::Close { colons: 4 }, "Should parse the closer");
    }

    #[test]
    fn test_read_marker_returns_reader_positioned_after_the_paragraph() {
        let html = "<p>::: spoiler T</p><p>next</p>";

        let (_, mut reader) = marker_after_first_p(html).expect("Should find a marker");

        let first_event = reader.read_event().expect("Should read the next event");
        let second_event = reader.read_event().expect("Should read the text event");
        assert_eq!(
            first_event,
            Event::Start(BytesStart::new("p")),
            "Should continue at the paragraph after the marker"
        );
        assert_eq!(
            second_event,
            Event::Text(BytesText::new("next")),
            "Should continue with the text of the next paragraph"
        );
    }

    #[test]
    fn test_read_marker_does_not_advance_the_original_reader() {
        let html = "<p>::: spoiler T</p>";
        let mut reader = Reader::from_str(html);
        reader.read_event().expect("Should read the opening <p> event");
        let position_before = reader.buffer_position();

        read_spoiler_block_marker(&reader).expect("Should find a marker");

        let position_after = reader.buffer_position();
        assert_eq!(position_after, position_before, "Should only look ahead on a clone");
    }

    #[test]
    fn test_read_marker_ignores_formatting_in_title() {
        let html = "<p>::: spoiler <em>T</em> <strong>x</strong></p>";

        let (result, _) = marker_after_first_p(html).expect("Should find a marker");

        assert_eq!(result, SpoilerBlockMarker::Open { colons: 3, title: String::from("T x") }, "Should only keep the text");
    }

    #[test]
    fn test_read_marker_decodes_entities() {
        let html = "<p>::: spoiler a &amp; b &#35; &lt;</p>";

        let (result, _) = marker_after_first_p(html).expect("Should find a marker");

        assert_eq!(
            result,
            SpoilerBlockMarker::Open { colons: 3, title: String::from("a & b # <") },
            "Should decode named and numeric entities"
        );
    }

    #[test]
    fn test_read_marker_returns_none_for_normal_paragraph() {
        let html = "<p>just text</p>";

        let result = marker_after_first_p(html);

        assert!(result.is_none(), "Should not find a marker in normal text");
    }

    #[test]
    fn test_read_marker_returns_none_for_unknown_entity() {
        let html = "<p>::: spoiler &foo;</p>";

        let result = marker_after_first_p(html);

        assert!(result.is_none(), "Should give up on an unknown entity");
    }

    #[test]
    fn test_read_marker_returns_none_when_paragraph_is_not_closed() {
        let html = "<p>::: spoiler T";

        let result = marker_after_first_p(html);

        assert!(result.is_none(), "Should give up at the end of the input");
    }

    #[test]
    fn test_read_marker_returns_none_for_comment_in_paragraph() {
        let html = "<p>::: spoiler <!-- c --></p>";

        let result = marker_after_first_p(html);

        assert!(result.is_none(), "Should give up on events that are not text, entities or elements");
    }
}
