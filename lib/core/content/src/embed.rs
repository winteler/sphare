use std::collections::HashSet;
use std::sync::{LazyLock};

use ammonia::Builder;
use leptos::html;
use leptos::prelude::*;
use leptos_fluent::{move_tr};
use mime_guess::{from_path, mime};
use serde::{Serialize, de::DeserializeOwned, Deserialize};
use strum_macros::{Display, EnumIter, EnumString, IntoStaticStr};
use url::Url;

#[cfg(feature = "ssr")]
use {
    http::header::{ACCEPT, USER_AGENT},
    http::{HeaderMap, HeaderValue},
    reqwest::Client,
};

use sphare_core_common::checks::check_string_length;
use sphare_core_common::constants::MAX_LINK_LENGTH;
use sphare_core_common::errors::{AppError};

static PROVIDERS: LazyLock<Option<Vec<OEmbedProvider>>> = LazyLock::new(|| {
    let parse_providers = serde_json::from_slice(include_bytes!("../embed/oembed_providers.json"));
    if let Err(e) = &parse_providers {
        log::error!("failed to parse oEmbed providers: {e}");
    }
    parse_providers.ok()
});

#[derive(Clone, Copy, Debug, Default, Display, EnumIter, EnumString, Eq, IntoStaticStr, Hash, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub enum EmbedType {
    #[default]
    None = 0,
    Link = 1,
    Embed = 2,
}

#[repr(i16)]
#[derive(Clone, Copy, Debug, Default, Display, EnumString, Eq, IntoStaticStr, Hash, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
#[cfg_attr(feature = "ssr", derive(sqlx::Type))]
pub enum LinkType {
    #[default]
    None = -1,
    Link = 0,
    Image = 1,
    Video = 2,
    Rich = 3,
}

#[cfg_attr(feature = "ssr", derive(sqlx::FromRow))]
#[derive(Clone, Debug, Default, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct Link {
    pub link_type: LinkType,
    pub link_url: Option<String>,
    pub link_embed: Option<String>,
    pub link_thumbnail_url: Option<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct OEmbedProvider {
    pub provider_name: String,
    pub provider_url: String,
    pub endpoints: Vec<OEmbedEndpoint>,
}

/// Endpoint of oEmbed provider
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct OEmbedEndpoint {
    #[serde(default)]
    pub schemes: Vec<String>,
    pub url: String,
    #[serde(default)]
    pub discovery: bool,
}

/// oEmbed type, as defined in section 2.3.4 of the [oEmbed specification][1].
///
/// [1]: https://oembed.com/
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type")]
pub enum OEmbedType {
    #[serde(rename = "link")]
    Link,
    #[serde(rename = "photo")]
    Photo(Photo),
    #[serde(rename = "video")]
    Video(Video),
    #[serde(rename = "rich")]
    Rich(Rich),
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Video {
    pub html: String,
    pub width: Option<i32>,
    pub height: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Photo {
    pub url: String,
    pub width: Option<i32>,
    pub height: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Rich {
    pub html: String,
    pub width: Option<i32>,
    pub height: Option<i32>,
}

/// oEmbed reply
/// Set version as optional to handle providers that don't respect the specification
#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct OEmbedReply {
    #[serde(flatten)]
    pub oembed_type: OEmbedType,
    pub version: Option<String>,
    pub title: Option<String>,
    pub author_name: Option<String>,
    pub author_url: Option<String>,
    pub provider_name: Option<String>,
    pub provider_url: Option<String>,
    pub cache_age: Option<i32>,
    pub thumbnail_url: Option<String>,
    pub thumbnail_width: Option<i32>,
    pub thumbnail_height: Option<i32>,
}

impl From<i16> for LinkType {
    fn from(category_color_val: i16) -> Self {
        match category_color_val {
            x if x == LinkType::Link as i16 => LinkType::Link,
            x if x == LinkType::Image as i16 => LinkType::Image,
            x if x == LinkType::Video as i16 => LinkType::Video,
            x if x == LinkType::Rich as i16 => LinkType::Rich,
            _ => LinkType::None,
        }
    }
}

impl From<LinkType> for EmbedType {
    fn from(link_type: LinkType) -> Self {
        match link_type {
            LinkType::None => EmbedType::None,
            LinkType::Link => EmbedType::Link,
            _ => EmbedType::Embed,
        }
    }
}

impl EmbedType {
    pub fn on_select(
        self,
        embed_type_input: RwSignal<EmbedType>,
        link_input: RwSignal<String>,
        select_trigger: RwSignal<usize>,
        textarea_ref: NodeRef<html::Textarea>,
    ) {
        embed_type_input.set(EmbedType::Link);
        if self == EmbedType::None {
            link_input.set(String::default());
            if let Some(link_textarea_ref) = textarea_ref.get_untracked() {
                link_textarea_ref.set_value("");
            }
        }
        *select_trigger.write() += 1;
    }

    pub fn get_localized_name(self) -> Signal<String> {
        match self {
            EmbedType::None => move_tr!("link-none"),
            EmbedType::Link => move_tr!("link-link"),
            EmbedType::Embed => move_tr!("link-embed"),
        }
    }
}

impl Link  {
    pub fn new(
        link_type: LinkType,
        link_url: Option<String>,
        link_embed: Option<String>,
        link_thumbnail_url: Option<String>,
    ) -> Self {
        Self {
            link_type,
            link_url,
            link_embed,
            link_thumbnail_url,
        }
    }
}

impl OEmbedProvider {
    /// Find an endpoint with one scheme matching the input `url` for this provider
    pub fn find_matching_endpoint(&self, url: &str) -> Option<&OEmbedEndpoint> {
        self.endpoints.iter().find(|&endpoint| endpoint.has_matching_scheme(url))
    }
}

impl OEmbedEndpoint {
    /// Find a scheme matching the input `url` for this endpoint
    pub fn has_matching_scheme(&self, url: &str) -> bool {
        self.schemes.iter().any(|scheme| url_matches_scheme(url, scheme))
    }
}

pub async fn get_oembed_data(url: String) -> Result<OEmbedReply, AppError> {
    check_string_length(&url, "Url", MAX_LINK_LENGTH as usize, false)?;
    let mut oembed_data = fetch_api::<OEmbedReply>(&url)
        .await
        .ok_or(AppError::new(format!("Cannot get oEmbed data at endpoint {url}")))?;

    match oembed_data.oembed_type {
        OEmbedType::Video(ref mut video) => video.html = clean_html(&video.html),
        OEmbedType::Rich(ref mut rich) => rich.html = clean_html(&rich.html),
        _ => ()
    };

    Ok(oembed_data)
}

/// # Check if the `scheme` matches the given `url`
///
/// ```
/// use sphare_core_content::embed::url_matches_scheme;
///
/// assert_eq!(url_matches_scheme("https://www.youtube.com/watch?v=test", "https://*.youtube.com/watch*"), true);
/// assert_eq!(url_matches_scheme("https://bsky.app/profile/test/post/testpost", "https://bsky.app/profile/*/post/*"), true);
/// assert_eq!(url_matches_scheme("https://bsky.app/profile/test", "https://*.youtube.com/watch*"), false);
/// ```
pub fn url_matches_scheme(mut url: &str, scheme: &str) -> bool {
    for (i, pattern) in scheme.split('*').enumerate() {
        if pattern.is_empty() {
            continue;
        }

        if let Some(index) = url.find(pattern) {
            if i == 0 && index > 0 {
                // the url should start with the first pattern
                return false;
            }
            url = &url[(index + pattern.len())..];
        } else {
            return false;
        }
    }
    scheme.ends_with('*') || url.is_empty()
}

/// Find the oEmbed provider and endpoint based on the URL using Sphare's providers.json
pub fn find_url_provider(url: &str) -> Option<(&OEmbedProvider, &OEmbedEndpoint)> {
    match &*PROVIDERS {
        Some(providers) => providers.iter().find_map(|provider| {
            provider.find_matching_endpoint(url).map(|endpoint| (provider, endpoint))
        }),
        None => None,
    }
}


#[cfg(not(feature = "ssr"))]
pub fn fetch_api<T>(
    path: &str,
) -> impl std::future::Future<Output = Option<T>> + Send + '_
where
    T: Serialize + DeserializeOwned,
{
    use leptos::prelude::on_cleanup;
    use send_wrapper::SendWrapper;

    SendWrapper::new(async move {
        let abort_controller =
            SendWrapper::new(web_sys::AbortController::new().ok());
        let abort_signal = abort_controller.as_ref().map(|a| a.signal());

        // abort in-flight requests if, e.g., we've navigated away from this page
        on_cleanup(move || {
            if let Some(abort_controller) = abort_controller.take() {
                abort_controller.abort()
            }
        });

        gloo_net::http::Request::get(path)
            .abort_signal(abort_signal.as_ref())
            .send()
            .await
            .map_err(|e| log::error!("API error {e}"))
            .ok()?
            .json()
            .await
            .map_err(|e| log::error!("Deserialize error {e}"))
            .ok()
    })
}

#[cfg(feature = "ssr")]
pub async fn fetch_api<T>(path: &str) -> Option<T>
where
    T: Serialize + DeserializeOwned,
{
    let client = Client::new();

    // Configure headers, as some api provider require them
    let mut headers = HeaderMap::new();
    headers.insert(ACCEPT, HeaderValue::from_static("application/json"));
    headers.insert(
        USER_AGENT,
        HeaderValue::from_static("Mozilla/5.0 (compatible; RustApp/1.0)"),
    );

    client.get(path)
        .headers(headers)
        .send()
        .await
        .map_err(|e| log::error!("Request error: {e}"))
        .ok()?
        .json()
        .await
        .map_err(|e| log::error!("Deserialize error: {e}"))
        .ok()
}

/// Select the given `link_type` in the given `list_ref` node
pub fn select_embed_type(
    link_type: LinkType,
    link_embed: RwSignal<EmbedType>,
    select_ref: NodeRef<html::Select>
) {
    let new_embed_type = link_type.into();
    link_embed.update(|embed_type| *embed_type = new_embed_type);
    if let Some(select_ref) = select_ref.get_untracked() {
        select_ref.set_selected_index(new_embed_type as i32);
    };
}

/// Check that an url is valid and infer its type
pub fn check_url_and_infer_type(
    url: &Url,
) -> LinkType {
    if url.scheme() == "https" && url.domain().is_some() {
        let mime_guess = from_path(url.as_str());
        match mime_guess.first() {
            Some(mime_guess) if mime_guess.type_() == mime::IMAGE => LinkType::Image,
            Some(mime_guess) if mime_guess.type_() == mime::VIDEO => LinkType::Video,
            _ => LinkType::Link,
        }
    } else {
        LinkType::None
    }
}

fn filter_attribute_values(attribute_value: &str, allowed_values: &[&str]) -> String {
    attribute_value
        .split(';')
        .filter_map(|v| {
            let mut parts = v.split(':').map(str::trim);
            if let Some(key) = parts.next() {
                if allowed_values.contains(&key) {
                    return match parts.next() {
                        Some(value) => Some(format!("{}:{}", key, value)),
                        None => Some(key.to_string()),
                    };
                }
            }
            None
        })
        .collect::<Vec<String>>()
        .join(";")
}

pub fn clean_html(
    html: &str,
) -> String {
    log::debug!("Html: {}", html);

    // Create a set of allowed iframe attributes
    let iframe_attributes = HashSet::from(
        [
            "width",
            "height",
            "src",
            "frameborder",
            "allowfullscreen",
            "scrollbar",
            "allow",
            "style",
            "title",
        ]
    );

    let clean_html = Builder::default()
        .add_tags(["iframe"])
        .add_tag_attributes("iframe", iframe_attributes)
        .add_tag_attribute_values(
            "iframe",
            "referrerpolicy",
            ["strict-origin", "strict-origin-when-cross-origin"]
        ).attribute_filter(|element, attribute, value| {
        match (element, attribute) {
            ("iframe", "style") => {
                Some(filter_attribute_values(value, &["border", "min-width", "min-height, width, height"]).into())
            }
            ("iframe", "allow") => {
                Some(filter_attribute_values(value, &["encrypted-media", "picture-in-picture"]).into())
            }
            _ => Some(value.into())
        }
    })
        .clean(html)
        .to_string();
    log::debug!("clean_html: {}", clean_html);
    clean_html
}

/// Check the input `link`'s validity and returns Link and an optional title.
/// If embed_type is EmbedType::Link, the link is always embedded as a simple link,
/// otherwise the link type will be inferred using the oEmbed API or the file extension.
/// If the type cannot be inferred, it will fall back to a link.
pub async fn verify_link_and_get_embed(
    embed_type: EmbedType,
    link: &str,
) -> (Link, Option<String>) {
    match (embed_type, Url::parse(link)) {
        (_, Err(_)) => (Link::default(), None),
        (EmbedType::Link, Ok(url)) => (Link::new(LinkType::Link, Some(url.to_string()), None, None), None),
        (_, Ok(url)) => {
            match find_url_provider(url.as_str()) {
                Some((_provider, endpoint)) => {
                    // TODO check values for width and height
                    let endpoint = format!("{}?url={url}&maxwidth=800&maxheight=600", endpoint.url);
                    log::debug!("Fetch oembed data: {endpoint}");
                    match get_oembed_data(endpoint).await {
                        Ok(oembed_data) => {
                            let title = oembed_data.title;
                            let thumbnail_url = oembed_data.thumbnail_url;
                            let link = match oembed_data.oembed_type {
                                OEmbedType::Link => Link::new(LinkType::Link, Some(url.to_string()), None, thumbnail_url),
                                OEmbedType::Photo(photo) => Link::new(LinkType::Image, Some(photo.url), None, thumbnail_url),
                                OEmbedType::Video(video) => Link::new(LinkType::Video, Some(url.to_string()), Some(clean_html(&video.html)), thumbnail_url),
                                OEmbedType::Rich(rich) => Link::new(LinkType::Rich, Some(url.to_string()), Some(clean_html(&rich.html)), thumbnail_url),
                            };
                            (link, title)
                        },
                        Err(e) => {
                            log::debug!("Failed to get oembed data: {}", e);
                            let inferred_type = check_url_and_infer_type(&url);
                            let link = match inferred_type {
                                LinkType::None => None,
                                _ => Some(url.to_string()),
                            };
                            (Link::new(inferred_type, link, None, None), None)
                        },
                    }
                },
                None => {
                    let inferred_type = check_url_and_infer_type(&url);
                    let link = match inferred_type {
                        LinkType::None => None,
                        _ => Some(url.to_string()),
                    };
                    (Link::new(inferred_type, link, None, None), None)
                },
            }
        },
    }
}

#[cfg(test)]
mod tests {
    use url::Url;
    use crate::embed::{check_url_and_infer_type, clean_html, find_url_provider, LinkType, OEmbedEndpoint, OEmbedProvider};

    #[test]
    fn test_link_type_from_i16() {
        assert_eq!(LinkType::from(-1), LinkType::None);
        assert_eq!(LinkType::from(0), LinkType::Link);
        assert_eq!(LinkType::from(1), LinkType::Image);
        assert_eq!(LinkType::from(2), LinkType::Video);
        assert_eq!(LinkType::from(3), LinkType::Rich);
        assert_eq!(LinkType::from(-2), LinkType::None);
        assert_eq!(LinkType::from(100), LinkType::None);
    }

    #[test]
    fn test_oembed_provider_find_matching_endpoint() {
        let endpoint1 = OEmbedEndpoint {
            schemes: vec![
                String::from("https://www.test.com/image/*"),
                String::from("https://www.test.com/profile/*/post/*"),
            ],
            url: "https://www.test.com/embed".to_string(),
            discovery: false,
        };

        let endpoint2 = OEmbedEndpoint {
            schemes: vec![
                String::from("https://www.test.com/video/*"),
            ],
            url: "https://www.test.com/embed".to_string(),
            discovery: false,
        };

        let provider = OEmbedProvider {
            provider_name: "test".to_string(),
            provider_url: "https://www.test.com".to_string(),
            endpoints: vec![endpoint1.clone(), endpoint2.clone()],
        };

        assert_eq!(provider.find_matching_endpoint("https://www.test.com/image/a"), Some(&endpoint1));
        assert_eq!(provider.find_matching_endpoint("https://www.test.com/profile/1/post/a"), Some(&endpoint1));
        assert_eq!(provider.find_matching_endpoint("https://www.test.com/video/a"), Some(&endpoint2));
        assert_eq!(provider.find_matching_endpoint("https://www.other.com/profile/1/post/a"), None);
    }

    #[test]
    fn test_oembed_endpoint_has_matching_scheme() {
        let endpoint = OEmbedEndpoint {
            schemes: vec![
                String::from("https://www.test.com/image/*"),
                String::from("https://www.test.com/profile/*/post/*"),
            ],
            url: "https://www.test.com/embed".to_string(),
            discovery: false,
        };

        assert_eq!(endpoint.has_matching_scheme("https://www.test.com/image/a"), true);
        assert_eq!(endpoint.has_matching_scheme("https://www.test.com/profile/1/post/a"), true);
        assert_eq!(endpoint.has_matching_scheme("https://www.test.com/profile/1/posts/a"), false);
        assert_eq!(endpoint.has_matching_scheme("https://www.other.com/profile/1/post/a"), false);
    }

    #[test]
    fn test_find_url_provider() {
        let (youtube_provider, youtube_endpoint) = find_url_provider("https://www.youtube.com/watch?v=test").expect("Find youtube provider");
        assert_eq!(youtube_provider.provider_name, String::from("YouTube"));
        assert_eq!(youtube_provider.provider_url, String::from("https://www.youtube.com/"));
        assert_eq!(youtube_endpoint.url, String::from("https://www.youtube.com/oembed"));
        assert_eq!(youtube_endpoint.discovery, true);

        let (giphy_provider, giphy_endpoint) = find_url_provider("https://giphy.com/gifs/test").expect("Find giphy provider");
        assert_eq!(giphy_provider.provider_name, String::from("GIPHY"));
        assert_eq!(giphy_provider.provider_url, String::from("https://giphy.com"));
        assert_eq!(giphy_endpoint.url, String::from("https://giphy.com/services/oembed"));
        assert_eq!(giphy_endpoint.discovery, true);
    }

    #[test]
    fn test_check_url_and_infer_type() {
        let no_domain_url = Url::parse("http://127.0.0.1:8000").expect("Should parse no_domain_url");
        let http_url = Url::parse("http://www.test.com/").expect("Should parse http_url");
        let https_link = Url::parse("https://www.test.com/").expect("Should parse https_link");
        let https_image = Url::parse("https://www.test.com/test.jpg").expect("Should parse https_image");
        let https_video = Url::parse("https://www.test.com/test.mp4").expect("Should parse https_video");
        assert_eq!(check_url_and_infer_type(&no_domain_url), LinkType::None);
        assert_eq!(check_url_and_infer_type(&http_url), LinkType::None);
        assert_eq!(check_url_and_infer_type(&https_link), LinkType::Link);
        assert_eq!(check_url_and_infer_type(&https_image), LinkType::Image);
        assert_eq!(check_url_and_infer_type(&https_video), LinkType::Video);
    }

    #[test]
    fn test_clean_html() {
        let input_html = r#"
            <div>Safe content</div>
            <iframe
                width="600"
                height="400"
                src="https://example.com/embed"
                style="border:1px solid black;min-width:300px;unsupported:invalid"
                allow="encrypted-media;picture-in-picture;autoplay"
                referrerpolicy="strict-origin"
                title="Example Embed">
            </iframe>
            <script>alert("malicious script");</script>
        "#;

        // Expected output after cleaning
        let expected_output = r#"
            <div>Safe content</div>
            <iframe width="600" height="400" src="https://example.com/embed" style="border:1px solid black;min-width:300px" allow="encrypted-media;picture-in-picture" referrerpolicy="strict-origin" title="Example Embed">
            </iframe>
        "#;

        assert_eq!(clean_html(input_html).trim(), expected_output.trim());

        let input_html = r#"
            <iframe
                src="https://example.com/embed"
                allow="accelerometer; gyroscope; clipboard-write;"
                referrerpolicy="unsafe-url">
            </iframe>
        "#;

        // Expected output after cleaning
        let expected_output = r#"
            <iframe src="https://example.com/embed" allow="">
            </iframe>
        "#;

        assert_eq!(clean_html(input_html).trim(), expected_output.trim());
    }
}