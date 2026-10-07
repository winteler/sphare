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