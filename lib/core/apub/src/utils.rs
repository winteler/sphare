use activitypub_federation::kinds::object::ImageType;
use activitypub_federation::protocol::values::MediaTypeMarkdown;
use serde::{Deserialize, Serialize};
use url::Url;
use sphare_core_common::errors::AppError;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Source {
    pub(crate) content: String,
    pub(crate) media_type: MediaTypeMarkdown,
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
pub(crate) struct LanguageTag {
    pub(crate) identifier: String,
    pub(crate) name: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Endpoints {
    pub shared_inbox: Url,
}

pub fn generate_outbox_url(apub_id: &Url) -> Result<Url, AppError> {
    Ok(Url::parse(&format!("{apub_id}/outbox"))?.into())
}

impl ImageObject {
    pub(crate) fn new(url: Url) -> Self {
        ImageObject {
            kind: ImageType::Image,
            url,
        }
    }
}