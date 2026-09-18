use activitypub_federation::config::Data;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use url::Url;
use sphare_core_common::activity_pub::ApubHelper;
use sphare_core_common::colors::Color;
use sphare_core_common::errors::AppError;
use sphare_core_sphere::sphere_category::SphereCategory;
use sphare_core_user::user::FunctionUserType;
use crate::group::Group;

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

pub async fn load_group_categories(
    group: &Group,
    context: &Data<ApubHelper>,
) -> Result<(), AppError> {

    let apub_id_vec: Vec<String> = group.tags.iter().map(|t| t.id.to_string()).collect();
    let category_name_vec: Vec<String> = group.tags.iter().map(|t| t.preferred_username.clone()).collect();
    let color_vec: Vec<i16> = group.tags.iter().map(|t| t.color.unwrap_or_default() as i16).collect();
    let description_vec: Vec<String> = group.tags.iter().map(|t| t.content.clone().unwrap_or_default()).collect();

    let function_user_type: &'static str = FunctionUserType::AdminBot.into();

    // Upsert categories and delete those no longer presents
    sqlx::query!(
        r#"
        WITH community_tags AS (
            SELECT *
            FROM UNNEST($1::text[], $2::text[], $3::smallint[], $4::text[]) AS t(category_apub_id, category_name, category_color, description)
        ),
        remote_sphere AS (
            SELECT *
            FROM spheres
            WHERE sphere_apub_id = $5
        ),
        upserted_categories AS (
            INSERT INTO sphere_categories
                (category_apub_id, sphere_id, category_name, category_color, description, is_active, creator_id)
            SELECT
                c.category_apub_id,
                s.sphere_id,
                c.category_name,
                c.category_color,
                c.description,
                TRUE,
                (
                    SELECT p.person_id FROM persons p
                    JOIN users u ON u.person_id = p.person_id
                    WHERE function_user_type = $6
                )
            FROM community_tags c, remote_sphere s
            ON CONFLICT (sphere_id, category_name) DO UPDATE
            SET category_apub_id = EXCLUDED.category_apub_id,
                description = EXCLUDED.description,
                category_color = EXCLUDED.category_color,
                creator_id = EXCLUDED.creator_id,
                is_active = EXCLUDED.is_active
            RETURNING category_id
        )
        UPDATE sphere_categories c
        SET is_active = FALSE
        WHERE c.sphere_id = (SELECT sphere_id FROM remote_sphere) AND NOT (c.category_id = ANY(SELECT category_id FROM upserted_categories))
        "#,
        &apub_id_vec,
        &category_name_vec,
        &color_vec,
        &description_vec,
        group.id.inner().to_string(),
        function_user_type,
    )
        .execute(context.get_db_pool())
        .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use sphare_core_common::colors::Color;
    use sphare_core_sphere::sphere_category::SphereCategory;
    use crate::tag::{ApubCommunityTag, CommunityTagType};

    #[test]
    fn test_apub_community_tag_try_from_sphere_category() {
        let sphere_category = SphereCategory {
            category_id: 0,
            category_apub_id: "https://www.sphare.space/c/test/category/news".to_string(),
            sphere_id: 0,
            category_name: "news".to_string(),
            category_color: Color::None,
            description: "This is a test category".to_string(),
            is_active: false,
            creator_id: 0,
            timestamp: Default::default(),
            delete_timestamp: None,
        };

        let apub_community_tag = ApubCommunityTag::try_from(sphere_category.clone()).expect("Should convert SphereCategory to ApubCommunityTag");
        assert_eq!(apub_community_tag.preferred_username, sphere_category.category_name);
        assert_eq!(apub_community_tag.name, None);
        assert_eq!(apub_community_tag.content, Some(sphere_category.description));
        assert_eq!(apub_community_tag.color, Some(sphere_category.category_color));
        assert_eq!(apub_community_tag.kind, CommunityTagType::default());
    }
}