use activitypub_federation::protocol::helpers::deserialize_one_or_many;
use activitypub_federation::{
    config::Data,
    fetch::object_id::ObjectId,
    protocol::{
        helpers::{deserialize_skip_error},
        verification::verify_domains_match
    },
    traits::Object,
};
use activitypub_federation::kinds::link::LinkType as ApubLinkType;
use activitypub_federation::kinds::object::{DocumentType, ImageType};
use activitypub_federation::protocol::values::MediaTypeMarkdownOrHtml;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_with::{skip_serializing_none};
use sqlx::PgPool;
use url::Url;
use sphare_core_common::activity_pub::{ApubHelper, AttributedTo, PersonOrGroupType};
use sphare_core_common::editor::ssr::get_html_and_markdown_strings;
use sphare_core_common::errors::AppError;
use sphare_core_common::to_app_error;
use sphare_core_content::embed::{Link, LinkType};
use sphare_core_content::post::Post;
use sphare_core_sphere::sphere_category::SphereCategory;
use crate::group::{ApubSphere};
use crate::person::ApubPerson;
use crate::tag::{ApubTag};
use crate::utils::{ImageObject, LanguageTag, Source};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub enum PageType {
    Page,
    Article,
    Note,
    Video,
    Event,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApubLink {
    href: Url,
    media_type: Option<String>,
    r#type: ApubLinkType,
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
    pub(crate) spoiler: Option<bool>,
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

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct ApubPost {
    pub apub_id: ObjectId<ApubPost>,
    pub sphere_apub_id: ObjectId<ApubSphere>,
    pub person_id: ObjectId<ApubPerson>,
    pub title: String,
    pub content: String,
    pub is_nsfw: bool,
    pub is_spoiler: bool,
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
            is_nsfw: post.post.is_nsfw,
            is_spoiler: post.post.is_spoiler,
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
            kind: PageType::Page,
            id: self.apub_id,
            attributed_to: AttributedTo::Forum(creator.apub_id.inner().clone().into()),
            to: vec![],
            in_reply_to: None,
            name: Some(self.title),
            cc: vec![],
            content: Some(self.content),
            media_type: Some(MediaTypeMarkdownOrHtml::Markdown),
            source: None,
            attachment: vec![],
            image: None,
            sensitive: Some(self.is_nsfw),
            spoiler: Some(self.is_spoiler),
            published: None,
            updated: None,
            language: None,
            audience: Some(self.sphere_apub_id),
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
        let creator = json.creator()?.dereference(data).await?;
        let apub_sphere = json.apub_sphere_id()?.dereference(data).await?;
        let post = insert_or_update_post(&json, &creator, &apub_sphere, data.app_data().get_db_pool()).await?;
        Ok(post.try_into()?)
    }
}

impl Page {
    pub fn check_valid_post(&self) -> Result<(), AppError> {
        self.check_is_post()?;
        self.check_valid_post_title()?;
        self.check_valid_post_content()
    }

    fn check_is_post(&self) -> Result<(), AppError> {
        match self.in_reply_to {
            Some(_) => Err(AppError::new("Page is a comment, not a post.")),
            None => Ok(()),
        }
    }

    fn check_valid_post_title(&self) -> Result<(), AppError> {
        match &self.name {
            None => Err(AppError::new("Invalid apub post: title missing.")),
            Some(name) if name.is_empty() => Err(AppError::new("Invalid apub post: title missing.")),
            _ => Ok(())
        }
    }

    fn check_valid_post_content(&self) -> Result<(), AppError> {
        match (&self.content, &self.media_type) {
            (None, _) => Err(AppError::new("Post without content, abort load.")),
            (Some(content), _) if content.is_empty() => Err(AppError::new("Post without content, abort load.")),
            (_, None) | (_, Some(MediaTypeMarkdownOrHtml::Markdown)) => Ok(()),
            (_, Some(MediaTypeMarkdownOrHtml::Html)) => Err(AppError::new("Post with html content, abort load."))
        }
    }

    pub fn get_html_and_markdown_content(&self) -> Result<(String, Option<&str>), AppError> {
        match (&self.content, &self.media_type) {
            (Some(content), None) => Ok((content.clone(), None)),
            (Some(content), Some(MediaTypeMarkdownOrHtml::Markdown)) => {
                get_html_and_markdown_strings(&content, true)
            },
            (Some(_), Some(MediaTypeMarkdownOrHtml::Html)) => Err(AppError::new("Html content is not accepted.")),
            (None, _) => Err(AppError::new("Post without content, cannot get bodies.")),
        }
    }

    pub fn creator(&self) -> Result<ObjectId<ApubPerson>, AppError> {
        match &self.attributed_to {
            AttributedTo::Forum(l) => Ok(l.url().into()),
            AttributedTo::Peertube(p) => p
                .iter()
                .find(|a| a.kind == PersonOrGroupType::Person)
                .map(|a| ObjectId::<ApubPerson>::from(a.id.clone()))
                .ok_or_else(|| AppError::ApubError(String::from("Missing creator."))),
        }
    }

    pub fn apub_sphere_id(&self) -> Result<&ObjectId<ApubSphere>, AppError> {
        match &self.audience {
            Some(sphere_apub_id) => Ok(sphere_apub_id),
            None => Err(AppError::ApubError(String::from("Missing sphere apub id."))),
        }
    }

    pub fn get_link(&self) -> Option<Link> {
        let first_attachment = self.attachment.first();
        // TODO check handling for videos and image embedding
        if let Some(attachment) = first_attachment.cloned() {
            match attachment {
                Attachment::Document(doc) => Some(Link::new(LinkType::Link, Some(doc.url.to_string()), None, None)),
                Attachment::Link(link) => Some(Link::new(LinkType::Link, Some(link.href.to_string()), None, None)),
                Attachment::Image(image) => Some(Link::new(LinkType::Image, Some(image.url.to_string()), None, None)),
            }
        } else if self.kind == PageType::Video {
            // we cant display videos directly, so insert a link to external video page
            Some(Link::new(LinkType::Link, Some(self.id.inner().clone().to_string()), None, None))
        } else {
            None
        }
    }

    pub fn get_sphere_category(&self) -> Result<Option<SphereCategory>, AppError> {
        // TODO for now return first tag/category
        Ok(None)
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
    creator: &ApubPerson,
    sphere: &ApubSphere,
    db_pool: &PgPool,
) -> Result<PostJoinApubInfo, AppError> {
    // TODO decide whether to always use markdown
    // TODO decide how to handle categories/flair, make something separate from tags?
    // TODO dereference category
    // TODO handle satellites
    let (body, markdown_body) = page.get_html_and_markdown_content()?;
    let link = page.get_link().unwrap();

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
                            WHEN $9 THEN TRUE
                            ELSE (
                                (SELECT is_nsfw FROM spheres s WHERE s.sphere_apub_id = $12) OR
                                COALESCE(
                                    (SELECT is_nsfw FROM satellites sa WHERE sa.satellite_id = $13),
                                    FALSE
                                )
                            )
                        END
                    ),
                    (
                        CASE
                            WHEN $10 THEN TRUE
                            ELSE COALESCE(
                                (SELECT is_spoiler FROM satellites sa WHERE sa.satellite_id = $13),
                                FALSE
                            )
                        END
                    ),
                    (SELECT category_id FROM sphere_categories WHERE category_apub_id = $11),
                    (SELECT sphere_id FROM spheres s WHERE s.sphere_name_apub_id = $12),
                    $13, FALSE,
                    (SELECT person_id FROM  persons WHERE actor_id = $14),
                    EXISTS (
                        SELECT 1 FROM user_sphere_roles r
                        JOIN persons p ON p.person_id = r.person_id
                        WHERE p.actor_id = $14 AND r.permission_level != 'None'
                    )
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
        .bind(page.id.to_string())
        .bind(page.name.clone().ok_or(AppError::new("Apub post is missing a title."))?)
        .bind(body)
        .bind(markdown_body)
        .bind(link.link_type as i16)
        .bind(link.link_url)
        .bind(link.link_embed)
        .bind(link.link_thumbnail_url)
        .bind(page.sensitive.unwrap_or_default())
        .bind(page.spoiler.unwrap_or_default())
        .bind(page.get_sphere_category()?.map(|c| c.category_apub_id.to_string()))
        .bind(sphere.apub_id.to_string())
        .bind(None::<i64>) // satellite id
        .bind(creator.apub_id.inner().to_string())
        .fetch_one(db_pool)
        .await?;

    Ok(post)
}