use activitypub_federation::kinds::object::ImageType;
use activitypub_federation::protocol::values::MediaTypeMarkdown;
use serde::{Deserialize, Serialize};
use url::Url;
use sphare_core_common::errors::AppError;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Source {
    pub content: String,
    pub media_type: MediaTypeMarkdown,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ImageObject {
    #[serde(rename = "type")]
    kind: ImageType,
    pub url: Url,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LanguageTag {
    pub identifier: String,
    pub name: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Endpoints {
    pub shared_inbox: Url,
}

pub fn generate_outbox_url(apub_id: &Url) -> Result<Url, AppError> {
    Ok(Url::parse(&format!("{apub_id}/outbox"))?)
}

impl ImageObject {
    pub(crate) fn new(url: Url) -> Self {
        ImageObject {
            kind: ImageType::Image,
            url,
        }
    }
}

fn escape_inline(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        if "\\`*_[]<&~^".contains(c) {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

fn escape_line(line: &str) -> String {
    let line = line.trim();
    let digits = line.chars().take_while(|c| c.is_ascii_digit()).count();
    let rest = &line[digits..];
    if digits > 0 && rest.starts_with(['.', ')']) {
        format!("{}\\{}", &line[..digits], escape_inline(rest))
    } else if line.starts_with(['#', '>', '-', '+', '=', '|']) {
        format!("\\{}", escape_inline(line))
    } else {
        escape_inline(line)
    }
}

pub fn convert_plaintext_to_md(text: &str) -> String {
    let mut paragraphs: Vec<Vec<String>> = Vec::new();
    let mut current: Vec<String> = Vec::new();
    for line in text.lines() {
        if line.trim().is_empty() {
            if !current.is_empty() {
                paragraphs.push(std::mem::take(&mut current));
            }
        } else {
            current.push(escape_line(line));
        }
    }
    if !current.is_empty() {
        paragraphs.push(current);
    }
    paragraphs
        .iter()
        .map(|p| p.join("\\\n"))
        .collect::<Vec<_>>()
        .join("\n\n")
}

pub fn convert_plaintext_to_html(text: &str) -> String {
    let text = text.replace("\r\n", "\n");
    text.split("\n\n")
        .filter(|p| !p.trim().is_empty())
        .map(|p| {
            let escaped = ammonia::clean_text(p.trim_matches('\n'));
            format!("<p>{}</p>", escaped.replace('\n', "<br>"))
        })
        .collect()
}