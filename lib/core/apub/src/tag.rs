use serde::{Deserialize, Serialize};
use serde_json::Value;
use url::Url;
use sphare_core_common::colors::Color;
use sphare_core_common::errors::AppError;
use sphare_core_sphere::sphere_category::SphereCategory;

/// Possible values in the `tag` field of a federated post or comment. Note that we don't support
/// hashtags or community tags in comments, but its easier to use the same struct for both
/// (anyway unsupported values are ignored).
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum ApubTag {
    CommunityTag(ApubCommunityTag),
    Unknown(Value),
}

/// The [ActivityStreams vocabulary](https://www.w3.org/TR/activitystreams-vocabulary/#dfn-tag)
/// defines that any object can have a list of tags associated with it.
/// Tags in AS can be of any type, so we define our own types.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Default)]
enum CommunityTagType {
    #[default]
    CommunityPostTag,
}

/// A tag that a community owns, that is added to a post.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ApubCommunityTag {
    #[serde(rename = "type")]
    kind: CommunityTagType,
    pub id: Url,
    pub name: Option<String>,
    pub preferred_username: String,
    pub content: Option<String>,
    pub color: Option<Color>,
}

impl TryFrom<SphereCategory> for ApubCommunityTag {
    type Error = AppError;

    fn try_from(value: SphereCategory) -> Result<Self, Self::Error> {
        Ok(Self {
            id: Url::parse(&value.category_apub_id)?,
            name: None,
            preferred_username: value.category_name,
            content: Some(value.description),
            color: Some(value.category_color),
            kind: Default::default(),
        })
    }
}