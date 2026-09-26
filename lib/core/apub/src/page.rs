use activitypub_federation::protocol::helpers::deserialize_one_or_many;
use activitypub_federation::{
    config::Data,
    fetch::object_id::ObjectId,
    kinds::{object::PageType},
    protocol::{
        helpers::{deserialize_skip_error},
        verification::verify_domains_match
    },
    traits::Object,
};
use activitypub_federation::kinds::link::LinkType;
use activitypub_federation::kinds::object::{DocumentType, ImageType};
use activitypub_federation::protocol::values::MediaTypeMarkdownOrHtml;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_with::{skip_serializing_none};
use sqlx::PgPool;
use url::Url;
use sphare_core_common::activity_pub::{ApubHelper, AttributedTo};
use sphare_core_common::errors::AppError;
use sphare_core_common::to_app_error;
use sphare_core_content::post::Post;
use sphare_core_user::user::ssr::get_admin_function_user;
use sphare_core_user::user::User;
use crate::group::{ApubSphere};
use crate::person::ApubPerson;
use crate::tag::ApubTag;
use crate::utils::{ImageObject, LanguageTag, Source};


#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApubLink {
    href: Url,
    media_type: Option<String>,
    r#type: LinkType,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApubImage {
    #[serde(rename = "type")]
    kind: ImageType,
    url: Url,
    /// Used for alt_text
    name: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApubDocument {
    #[serde(rename = "type")]
    kind: DocumentType,
    url: Url,
    media_type: Option<String>,
    /// Used for alt_text
    name: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Attachment {
    Link(ApubLink),
    Image(ApubImage),
    Document(ApubDocument),
}

#[skip_serializing_none]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Page {
    #[serde(rename = "type")]
    pub(crate) kind: PageType,
    pub id: ObjectId<ApubPost>,
    pub(crate) attributed_to: AttributedTo,
    #[serde(deserialize_with = "deserialize_one_or_many", default)]
    pub(crate) to: Vec<Url>,
    // If there is inReplyTo field this is actually a comment and must not be parsed
    #[serde(deserialize_with = "deserialize_skip_error", default)]
    pub(crate) in_reply_to: Option<String>,
    pub(crate) name: Option<String>,
    #[serde(deserialize_with = "deserialize_one_or_many", default)]
    pub(crate) cc: Vec<Url>,
    pub(crate) content: Option<String>,
    pub(crate) media_type: Option<MediaTypeMarkdownOrHtml>,
    #[serde(deserialize_with = "deserialize_skip_error", default)]
    pub(crate) source: Option<Source>,
    /// most software uses array type for attachment field, so we do the same. nevertheless, we only
    /// use the first item
    #[serde(default)]
    pub(crate) attachment: Vec<Attachment>,
    pub(crate) image: Option<ImageObject>,
    pub(crate) sensitive: Option<bool>,
    pub(crate) published: Option<DateTime<Utc>>,
    pub(crate) updated: Option<DateTime<Utc>>,
    pub(crate) language: Option<LanguageTag>,
    pub(crate) audience: Option<ObjectId<ApubSphere>>,
    // TODO add field for sattelite
    /// Contains hashtags and post tags.
    /// https://www.w3.org/TR/activitystreams-vocabulary/#dfn-tag
    #[serde(deserialize_with = "deserialize_skip_error", default)]
    pub tag: Vec<ApubTag>,
    pub(crate) context: Option<String>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct ApubPost {
    apub_id: ObjectId<ApubPost>,
    sphere_apub_id: ObjectId<ApubSphere>,
    person_id: ObjectId<ApubPerson>,
    title: String,
    content: String,
}

#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize, sqlx::FromRow)]
pub struct PostJoinApubInfo {
    #[cfg_attr(feature = "ssr", sqlx(flatten))]
    pub post: Post,
    pub sphere_apub_id: String,
    pub category_apub_id: Option<String>,
}

impl TryFrom<PostJoinApubInfo> for ApubPost {
    type Error = AppError;

    fn try_from(post: PostJoinApubInfo) -> Result<Self, Self::Error> {
        Ok(Self {
            apub_id: Url::parse(&post.post.post_apub_id)?.into(),
            sphere_apub_id: Url::parse(&post.sphere_apub_id)?.into(),
            person_id: Url::parse(&post.post.creator_apub_id)?.into(),
            title: post.post.title,
            content: post.post.body,
        })
    }
}

#[async_trait::async_trait]
impl Object for ApubPost {
    type DataType = ApubHelper;
    type Kind = Page;
    type Error = AppError;

    fn id(&self) -> &Url {
        self.apub_id.inner()
    }

    async fn read_from_id(
        object_id: Url,
        data: &Data<Self::DataType>,
    ) -> Result<Option<Self>, Self::Error> {
        let post = load_post_by_apub_id(&object_id, data.get_db_pool()).await?;
        let post = match post {
            Some(post) => Some(post.try_into()?),
            None => None,
        };
        Ok(post)
    }

    async fn into_json(self, data: &Data<Self::DataType>) -> Result<Self::Kind, Self::Error> {
        let creator = self.person_id.dereference_local(data).await.map_err(to_app_error!("Failed domain verification"))?;
        Ok(Page {
            kind: Default::default(),
            id: self.apub_id,
            attributed_to: AttributedTo::Forum(creator.apub_id.inner().clone().into()),
            to: vec![],
            in_reply_to: None,
            name: Some(self.title),
            cc: vec![],
            content: Some(self.content),
            media_type: None,
            source: None,
            attachment: vec![],
            image: None,
            sensitive: None,
            published: None,
            updated: None,
            language: None,
            audience: None,
            tag: vec![],
            context: None,
        })
    }

    async fn verify(
        json: &Self::Kind,
        expected_domain: &Url,
        _data: &Data<Self::DataType>,
    ) -> Result<(), Self::Error> {
        verify_domains_match(json.id.inner(), expected_domain).map_err(to_app_error!("Failed domain verification"))?;
        json.check_valid_post()?;
        Ok(())
    }

    async fn from_json(json: Self::Kind, data: &Data<Self::DataType>) -> Result<Self, Self::Error> {
        let function_user = get_admin_function_user(data.app_data().get_db_pool()).await?;
        let post = insert_or_update_post(&json, &function_user, data.app_data().get_db_pool()).await?;
        Ok(post.try_into()?)
    }
}

impl Page {
    pub fn check_valid_post(&self) -> Result<(), AppError> {
        match self.name {
            None | Some(name) if name.is_empty() => return Err(AppError::new("Invalid apub post: title missing.")),
            _ => ()
        };
        self.check_valid_post_content()
    }

    fn check_valid_post_content(&self) -> Result<(), AppError> {
        match (self.content, self.media_type) {
            (None, _) | (Some(content), _) if content.is_empty() => Err(AppError::new("Post without content, abort load.")),
            (_, None) | (_, Some(MediaTypeMarkdownOrHtml::Markdown)) => Ok(()),
            (_, Some(MediaTypeMarkdownOrHtml::Html)) => Err(AppError::new("Post with html content, abort load."))
        }
    }
}

pub async fn load_post_by_apub_id(
    post_apub_id: &Url,
    db_pool: &PgPool
) -> Result<Option<PostJoinApubInfo>, AppError> {
    let post = sqlx::query_as::<_, PostJoinApubInfo>(
        "SELECT p.*, s.sphere_apub_id, sc.category_apub_id
        FROM posts p
        JOIN spheres s ON p.sphere_id = s.sphere_id
        LEFT JOIN sphere_categories sc ON p.sphere_id = sc.category_id
        WHERE post_apub_id = $1",
    )
        .bind(post_apub_id.to_string())
        .fetch_optional(db_pool)
        .await?;

    Ok(post)
}

pub async fn insert_or_update_post(
    page: &Page,
    function_user: &User,
    db_pool: &PgPool,
) -> Result<PostJoinApubInfo, AppError> {
    // TODO dereference sphere, sphere category and person(s)

    let post = sqlx::query_as::<_, PostJoinApubInfo>(
        "WITH upserted_post AS (
                INSERT INTO posts (
                    post_apub_id, title, body, markdown_body, link_type, link_url, link_embed, link_thumbnail_url, is_nsfw, is_spoiler, category_id,
                    sphere_id, satellite_id, is_pinned, creator_id, is_creator_moderator
                )
                VALUES (
                    $1, $2, $3, $4, $5, $6, $7, $8,
                    (
                        CASE
                            WHEN $10 THEN TRUE
                            ELSE (
                                (SELECT is_nsfw FROM spheres s WHERE s.sphere_name = $13) OR
                                COALESCE(
                                    (SELECT is_nsfw FROM satellites sa WHERE sa.satellite_id = $14),
                                    FALSE
                                )
                            )
                        END
                    ),
                    (
                        CASE
                            WHEN $11 THEN TRUE
                            ELSE COALESCE(
                                (SELECT is_spoiler FROM satellites sa WHERE sa.satellite_id = $14),
                                FALSE
                            )
                        END
                    ),
                    $12,
                    (SELECT sphere_id FROM spheres s WHERE s.sphere_name = $13),
                    $14, $15, $16, $17
                )
                ON CONFLICT (post_apub_id)
                DO UPDATE SET
                    title = EXCLUDED.title,
                    body = EXCLUDED.body,
                    markdown_body = EXCLUDED.markdown_body,
                    link_type = EXCLUDED.link_type,
                    link_url = EXCLUDED.link_url,
                    link_embed = EXCLUDED.link_embed,
                    link_thumbnail_url = EXCLUDED.link_thumbnail_url,
                    is_nsfw = EXCLUDED.is_nsfw,
                    is_spoiler = EXCLUDED.is_spoiler,
                    creator_id = EXCLUDED.creator_id,
                    sphere_id = EXCLUDED.sphere_id,
                    satellite_id = EXCLUDED.satellite_id,
                    is_pinned = EXCLUDED.is_pinned,
                    creator_id = EXCLUDED.creator_id,
                    is_creator_moderator = EXCLUDED.is_creator_moderator
                RETURNING *
            )
            SELECT p.*, s.sphere_apub_id, sc.category_apub_id
            FROM upserted_post p
            JOIN spheres s ON p.sphere_id = s.sphere_id
            LEFT JOIN sphere_categories sc ON p.sphere_id = sc.category_id",
    )
        .bind(page.id.inner().to_string())
        .bind(page.name.ok_or(AppError::new("Apub post is missing a title."))?)
        .bind(page.content.ok_or(AppError::new("Apub post is missing a title."))?)
        .fetch_one(db_pool)
        .await?;

    Ok(post)
}