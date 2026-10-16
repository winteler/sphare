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

/// Split text into paragraphs of trimmed, non-empty lines.
fn paragraphs(text: &str) -> Vec<Vec<&str>> {
    let mut result = Vec::new();
    let mut current = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            if !current.is_empty() {
                result.push(std::mem::take(&mut current));
            }
        } else {
            current.push(line);
        }
    }
    if !current.is_empty() {
        result.push(current);
    }
    result
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
    paragraphs(text)
        .iter()
        .map(|p| p.iter().map(|l| escape_line(l)).collect::<Vec<_>>().join("\\\n"))
        .collect::<Vec<_>>()
        .join("\n\n")
}

pub fn convert_plaintext_to_html(text: &str) -> String {
    paragraphs(text)
        .iter()
        .map(|p| {
            let body = p
                .iter()
                .map(|l| ammonia::clean_text(l))
                .collect::<Vec<_>>()
                .join("<br>");
            format!("<p>{body}</p>")
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use indoc::indoc;
    use sphare_core_common::editor::get_styled_html_from_markdown;
    use crate::utils::{convert_plaintext_to_html, convert_plaintext_to_md};

    // ---------- convert_plaintext_to_md ----------

    #[test]
    fn test_convert_plaintext_to_md_whitespace_and_paragraphs() {
        assert_eq!(convert_plaintext_to_md(""), "");
        assert_eq!(convert_plaintext_to_md("  \n\t\n"), "");
        assert_eq!(convert_plaintext_to_md("  héllo ✓  "), "héllo ✓");
        assert_eq!(convert_plaintext_to_md("a\nb"), "a\\\nb");
        assert_eq!(convert_plaintext_to_md("a\r\n  b  \r\n"), "a\\\nb");
        assert_eq!(convert_plaintext_to_md("a\n\n\n b\n\nc"), "a\n\nb\n\nc");
        assert_eq!(convert_plaintext_to_md("a\nb\n\nc\nd"), "a\\\nb\n\nc\\\nd");
    }

    #[test]
    fn test_convert_plaintext_to_md_escapes_inline() {
        for (input, expected) in [
            ("*a* _b_", r"\*a\* \_b\_"),
            ("`c` [l](u)", r"\`c\` \[l\](u)"),
            ("<b>a&b</b>", r"\<b>a\&b\</b>"),
            ("~~s~~ ^x", r"\~\~s\~\~ \^x"),
            (r"a\b", r"a\\b"),
            ("a - b # c > d + e = f | g", "a - b # c > d + e = f | g"),
        ] {
            assert_eq!(convert_plaintext_to_md(input), expected);
        }
    }

    #[test]
    fn test_convert_plaintext_to_md_escapes_line_start() {
        for (input, expected) in [
            ("# h", r"\# h"),
            ("> q", r"\> q"),
            ("- i", r"\- i"),
            ("+ i", r"\+ i"),
            ("=== ", r"\==="),
            ("---", r"\---"),
            ("| a |", r"\| a |"),
            ("1. x", r"1\. x"),
            ("12) x", r"12\) x"),
            ("1.5", r"1\.5"),
            ("2024 was", "2024 was"),
            ("1a", "1a"),
            ("    - indented", r"\- indented"),
        ] {
            assert_eq!(convert_plaintext_to_md(input), expected);
        }
        // Applied to every line, not only the first
        assert_eq!(convert_plaintext_to_md("a\n# b"), "a\\\n\\# b");
    }

    #[test]
    fn test_convert_plaintext_to_md_is_inert_when_rendered() {
        let plaintext = "# not heading\n- not list\n1. not list\n<script>alert(1)</script>\n*not em*\n\n> not quote";
        let expected_html = indoc! {r#"
            <p class="mb-2.5"># not heading<br />
            - not list<br />
            1. not list<br />
            &lt;script&gt;alert(1)&lt;/script&gt;<br />
            *not em*</p>
            <p class="mb-2.5">&gt; not quote</p>"#
        };
        assert_eq!(
            get_styled_html_from_markdown(&convert_plaintext_to_md(plaintext)).expect("html"),
            expected_html
        );
    }

    // ---------- convert_plaintext_to_html ----------

    #[test]
    fn test_convert_plaintext_to_html_paragraphs() {
        assert_eq!(convert_plaintext_to_html(""), "");
        assert_eq!(convert_plaintext_to_html(" \n\n "), "");
        assert_eq!(convert_plaintext_to_html("hello"), "<p>hello</p>");
        assert_eq!(convert_plaintext_to_html("a\nb"), "<p>a<br>b</p>");
        assert_eq!(convert_plaintext_to_html("a\r\n  b  \r\n"), "<p>a<br>b</p>");
        assert_eq!(convert_plaintext_to_html("a\n\n\n b\n\nc"), "<p>a</p><p>b</p><p>c</p>");
        assert_eq!(convert_plaintext_to_html("a\nb\n\nc"), "<p>a<br>b</p><p>c</p>");
        assert_eq!(convert_plaintext_to_html("héllo✓"), "<p>héllo✓</p>");
    }

    #[test]
    fn test_convert_plaintext_to_html_escapes_html() {
        assert_eq!(convert_plaintext_to_html("<img>"), "<p>&lt;img&gt;</p>");
        // An already-escaped entity is escaped again, i.e. shown literally
        assert_eq!(convert_plaintext_to_html("&amp;"), "<p>&amp;amp;</p>");
        // Markdown syntax is left alone
        assert_eq!(convert_plaintext_to_html("#*_[a]"), "<p>#*_[a]</p>");
    }

    #[test]
    fn test_convert_plaintext_to_html_whitespace_is_entity_encoded() {
        // ammonia::clean_text encodes spaces; adjust if your ammonia version differs
        assert_eq!(convert_plaintext_to_html("a b"), "<p>a&#32;b</p>");
    }

    #[test]
    fn test_convert_plaintext_to_html_no_injection() {
        for payload in [
            "<script>alert(1)</script>",
            r#"<img src=x onerror=alert(1)>"#,
            r#"<a href="javascript:alert(1)">x</a>"#,
            r#""><svg onload=alert(1)>"#,
            "</p><script>alert(1)</script><p>",
        ] {
            let html = convert_plaintext_to_html(payload);
            // The only tags allowed in the output are our own <p> and <br>
            let without_own_tags = html.replace("<p>", "").replace("</p>", "").replace("<br>", "");
            assert!(!without_own_tags.contains('<'), "{payload} -> {html}");
            assert!(!without_own_tags.contains('"'), "{payload} -> {html}");
            assert!(!without_own_tags.contains('>'), "{payload} -> {html}");
        }
    }
}