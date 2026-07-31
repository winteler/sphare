use activitypub_federation::{
    config::Data,
    fetch::object_id::ObjectId,
    kinds::{object::PageType},
    protocol::{verification::verify_domains_match},
    traits::Object,
};
use serde::{Deserialize, Serialize};
use url::Url;
use sphare_core_common::activity_pub::ApHelper;
use sphare_core_common::errors::AppError;
use sphare_core_common::to_app_error;
use sphare_core_content::post::Post;

use crate::person::ApubPerson;

#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Page {
    #[serde(rename = "type")]
    kind: PageType,
    id: ObjectId<ApPost>,
    pub(crate) attributed_to: ObjectId<ApubPerson>,
    title: String,
    content: String,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct ApPost {
    apub_id: ObjectId<ApPost>,
    person_id: ObjectId<ApubPerson>,
    title: String,
    content: String,
}

impl TryFrom<Post> for ApPost {
    type Error = AppError;

    fn try_from(post: Post) -> Result<Self, Self::Error> {
        Ok(Self {
            apub_id: Url::parse(&post.post_apub_id)?.into(),
            person_id: post.creator_apub_id,
            title: post.title,
            content: post.body,
        })
    }
}

#[async_trait::async_trait]
impl Object for ApPost {
    type DataType = ApHelper;
    type Kind = Page;
    type Error = AppError;

    fn id(&self) -> &Url {
        self.apub_id.inner()
    }

    async fn read_from_id(
        object_id: Url,
        data: &Data<Self::DataType>,
    ) -> Result<Option<Self>, Self::Error> {
        let post = Post::get_from_apub_id(object_id, data.get_db_pool()).await?;
        let ap_post = ApPost::try_from(post)?;
        Ok(Some(ap_post))
    }

    async fn into_json(self, data: &Data<Self::DataType>) -> Result<Self::Kind, Self::Error> {
        let creator = self.person_id.dereference_local(data).await.map_err(to_app_error!("Failed domain verification"))?;
        Ok(Page {
            kind: Default::default(),
            id: self.apub_id,
            attributed_to: creator.get_apub_id(),
            title: self.title,
            content: self.content,
        })
    }

    async fn verify(
        json: &Self::Kind,
        expected_domain: &Url,
        _data: &Data<Self::DataType>,
    ) -> Result<(), Self::Error> {
        verify_domains_match(json.id.inner(), expected_domain).map_err(to_app_error!("Failed domain verification"))?;
        Ok(())
    }

    async fn from_json(json: Self::Kind, data: &Data<Self::DataType>) -> Result<Self, Self::Error> {
        todo!()
    }
}