use leptos::html;
use leptos::prelude::*;
use leptos_fluent::{move_tr, tr};
use url::Url;

use sphare_core_content::embed::{select_embed_type, verify_link_and_get_embed, EmbedType, Link, LinkType};

use sphare_cmp_utils::errors::ErrorDetail;
use sphare_cmp_utils::icons::LinkIcon;
use sphare_core_common::errors::AppError;

const DEFAULT_MEDIA_CLASS: &str = "h-fit w-fit max-h-160 max-w-full object-contain";
const THUMBNAIL_CLASS: &str = "h-16 w-16 object-contain";

/// Component to safely embed content at the url `link-input`.
/// It will try to infer the content type using the oembed API. If the provider of the url
/// is not in the whitelisted list of providers, it will instead try to naively embed the
/// content using file extension in the url and fallback to a simple link.
#[component]
pub fn EmbedPreview(
    embed_type_input: RwSignal<EmbedType>,
    #[prop(into)]
    link_input: Signal<String>,
    #[prop(into)]
    select_trigger: Signal<usize>,
    title_input: RwSignal<String>,
    select_ref: NodeRef<html::Select>,
) -> impl IntoView {
    let link_resource = Resource::new(
        move || (select_trigger.get(), link_input.get()),
        move |(_, url)| async move {
            verify_link_and_get_embed(embed_type_input.get_untracked(), &url).await
        },
    );

    view! {
        <Suspense>
        { move || link_resource.read().clone().map(|(link, title)| {
                title_input.update(|title_input| if title_input.is_empty() {
                    *title_input = title.unwrap_or_default();
                });
                select_embed_type(link.link_type, embed_type_input, select_ref);
                view! { <Embed link align_center=true/> }
            })
        }
        </Suspense>
    }
}

/// Component to safely embed external content
#[component]
pub fn Embed(
    link: Link,
    #[prop(default = false)]
    align_center: bool,
) -> impl IntoView {
    match (link.link_type, link.link_url, link.link_embed, link.link_thumbnail_url) {
        (LinkType::None, _, _, _) => None,
        (_, None, _, _) => None,
        (LinkType::Link, Some(link_url), None, thumbnail_url) => Url::parse(&link_url).ok().map(|url| view! {
            <LinkEmbed url thumbnail_url align_center/>
        }.into_any()),
        (link_type, Some(link_url), None, _) => Some(view! {
            <NaiveEmbed link_input=link_url link_type align_center/>
        }.into_any()),
        (_, Some(_), Some(link_embed), _) => Some(view! {
            <HtmlEmbed html=link_embed align_center/>
        }.into_any()),
    }
}

/// Component to naively and safely embed external content
#[component]
pub fn NaiveEmbed(
    #[prop(into)]
    link_input: Signal<String>,
    link_type: LinkType,
    #[prop(default = false)]
    align_center: bool,
) -> impl IntoView {
    view! {
        { move || {
            match (link_type, Url::parse(&link_input.read())) {
                (LinkType::None, _) => None,
                (_, Err(e)) => Some(view! { <ErrorDetail error=AppError::new(format!("{}: {e}", tr!("invalid-link")))/> }.into_any()),
                (LinkType::Link, Ok(url)) => Some(view! { <LinkEmbed url align_center/> }.into_any()),
                (LinkType::Image, Ok(url)) => Some(view! { <ImageEmbed url=url.to_string() align_center/> }.into_any()),
                (LinkType::Video, Ok(url)) => Some(view! { <VideoEmbed url=url.to_string() align_center/> }.into_any()),
                (LinkType::Rich, Ok(url)) => Some(view! { <LinkEmbed url align_center/> }.into_any()),
            }
        }}
    }
}

/// Component to embed a link with an optional thumbnail
#[component]
pub fn LinkEmbed(
    url: Url,
    #[prop(default = None)]
    thumbnail_url: Option<String>,
    #[prop(default = false)]
    align_center: bool,
) -> impl IntoView {
    match url.domain() {
        Some(domain) => {
            let class = match align_center {
                true => "flex justify-center items-center h-fit w-full",
                false => "flex justify-center lg:justify-start items-center h-fit w-full",
            };
            let clean_domain = match domain.starts_with("www.") {
                true => domain[4..].to_string(),
                false => domain.to_string(),
            };
            view! {
                <div class=class>
                    <a
                        href=url.to_string()
                        target="_blank"
                        class="w-fit flex items-center gap-2 px-2 py-1 bg-primary rounded-sm hover:bg-base-content/50">
                        { match thumbnail_url {
                            Some(thumbnail_url) => view! { <img src=thumbnail_url class=THUMBNAIL_CLASS/> }.into_any(),
                            None => view! { <LinkIcon/> }.into_any(),
                        }}
                        <div>{clean_domain}</div>
                    </a>
                </div>
            }.into_any()
        },
        None => view! { <ErrorDetail error=AppError::new(tr!("invalid-domain-name"))/> }.into_any(),
    }
}

/// Component to embed an image
#[component]
pub fn ImageEmbed(
    url: String,
    #[prop(default = false)]
    align_center: bool,
) -> impl IntoView {
    let class = match align_center {
        true => "flex justify-center items-center h-fit w-full",
        false => "flex justify-center lg:justify-start items-center h-fit w-full",
    };
    view! {
        <div class=class>
            <img src=url class=DEFAULT_MEDIA_CLASS/>
        </div>
    }
}

/// Component to embed a video
#[component]
pub fn VideoEmbed(
    url: String,
    #[prop(default = false)]
    align_center: bool,
) -> impl IntoView {
    let class = match align_center {
        true => "flex justify-center items-center h-fit w-full",
        false => "flex justify-center lg:justify-start items-center h-fit w-full",
    };
    view! {
        <div class=class>
            <video
                src=url
                class=DEFAULT_MEDIA_CLASS
                controls
            >
            {
                move_tr!("invalid-video-format")
            }
            </video>
        </div>
    }
}

/// Component to embed html
#[component]
pub fn HtmlEmbed(
    html: String,
    #[prop(default = false)]
    align_center: bool,
) -> impl IntoView {
    let class = match align_center {
        true => "flex justify-center items-center h-fit w-full",
        false => "flex justify-center lg:justify-start items-center h-fit w-full",
    };
    view! {
        <div class=class inner_html=html/>
    }
}