use activitypub_federation::protocol::verification::verify_domains_match;
use activitypub_federation::traits::Actor;
use activitypub_federation::{
    config::Data,
    fetch::object_id::ObjectId,
    kinds::actor::GroupType,
    protocol::{
        helpers::{deserialize_last, deserialize_skip_error},
        public_key::PublicKey,
        values::MediaTypeHtml,
    },
    traits::Object,
};
use chrono::{DateTime, Utc};
use rsa::pkcs1::LineEnding;
use rsa::pkcs8::EncodePrivateKey;
use rsa::RsaPrivateKey;
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;
use sqlx::PgPool;
use url::Url;

use sphare_core_common::activity_pub::ApHelper;
use sphare_core_common::errors::AppError;
use sphare_core_common::instance::ssr::get_or_insert_instance;
use sphare_core_common::to_app_error;
use sphare_core_sphere::sphere::Sphere;
use sphare_core_user::role::ssr::set_user_sphere_role;
use sphare_core_user::role::PermissionLevel;
use sphare_core_user::user::ssr::get_admin_function_user;

use crate::person::ApubPerson;
use crate::utils::{generate_outbox_url, Endpoints, ImageObject, LanguageTag, Source};

#[skip_serializing_none]
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Group {
    #[serde(rename = "type")]
    pub(crate) kind: GroupType,
    pub id: ObjectId<ApubSphere>,
    /// username, set at account creation and usually fixed after that
    pub preferred_username: String,
    pub followers: Option<Url>,
    pub public_key: PublicKey,
    /// title / display name
    pub name: Option<String>,
    // short description
    pub(crate) description: Option<String>,
    /// sidebar
    #[serde(deserialize_with = "deserialize_skip_error", default)]
    pub source: Option<Source>,
    pub(crate) media_type: Option<MediaTypeHtml>,
    // sidebar
    pub summary: Option<String>,
    #[serde(deserialize_with = "deserialize_last", default)]
    pub icon: Option<ImageObject>,
    /// banner
    #[serde(deserialize_with = "deserialize_last", default)]
    pub image: Option<ImageObject>,
    // lemmy extension
    pub sensitive: Option<bool>,
    #[serde(deserialize_with = "deserialize_skip_error", default)]
    pub attributed_to: Option<Vec<ObjectId<ApubPerson>>>,
    // lemmy extension
    pub posting_restricted_to_mods: Option<bool>,
    pub inbox: Url,
    pub outbox: Url,
    pub endpoints: Option<Endpoints>,
    pub featured: Option<Url>,
    #[serde(default)]
    pub(crate) language: Vec<LanguageTag>,
    /// True if this is a private community
    pub(crate) manually_approves_followers: Option<bool>,
    pub published: Option<DateTime<Utc>>,
    pub updated: Option<DateTime<Utc>>,
    /// https://docs.joinmastodon.org/spec/activitypub/#discoverable
    pub(crate) discoverable: Option<bool>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ApubSphere {
    pub apub_id: ObjectId<ApubSphere>,
    name: String,
    description: String,
    icon: Option<ImageObject>,
    banner: Option<ImageObject>,
    is_nsfw: bool,
    inbox: Url,
    public_key: String,
    private_key: Option<RsaPrivateKey>,
}

impl ApubSphere {
    pub fn new(
        apub_id: ObjectId<ApubSphere>,
        name: String,
        description: String,
        is_nsfw: bool,
        inbox: Url,
        public_key: String,
        private_key: Option<RsaPrivateKey>,
    ) -> Self {
        Self {
            apub_id,
            name,
            description,
            icon: None,
            banner: None,
            is_nsfw,
            inbox,
            public_key,
            private_key,
        }
    }
}

impl TryFrom<Sphere> for ApubSphere {
    type Error = AppError;

    fn try_from(sphere: Sphere) -> Result<Self, Self::Error> {
        let apub_id = Url::parse(&sphere.sphere_apub_id)?;
        let icon_url = match sphere.icon_url {
            Some(icon_url) => Some(Url::parse(&icon_url)?),
            None => None,
        };
        let banner_url = match sphere.banner_url {
            Some(banner_url) => Some(Url::parse(&banner_url)?),
            None => None,
        };
        Ok(Self {
            apub_id: apub_id.clone().into(),
            name: sphere.sphere_name,
            description: sphere.description,
            icon: icon_url.map(ImageObject::new),
            banner: banner_url.map(ImageObject::new),
            is_nsfw: sphere.is_nsfw,
            inbox: Url::parse(&sphere.inbox)?,
            public_key: sphere.public_key,
            private_key: None,
        })
    }
}

impl Actor for ApubSphere {
    fn public_key_pem(&self) -> &str {
        &self.public_key
    }

    fn private_key_pem(&self) -> Option<String> {
        self.private_key.clone().map(|pk| pk.to_pkcs8_pem(LineEnding::default()).unwrap().to_string())
    }

    fn inbox(&self) -> Url {
        self.inbox.clone()
    }
}

#[async_trait::async_trait]
impl Object for ApubSphere {
    type DataType = ApHelper;
    type Kind = Group;
    type Error = AppError;

    fn id(&self) -> &Url {
        self.apub_id.inner()
    }

    async fn read_from_id(
        sphere_apub_id: Url,
        data: &Data<Self::DataType>,
    ) -> Result<Option<Self>, Self::Error> {
        let sphere = get_sphere_by_apub_id(&sphere_apub_id, data.app_data().get_db_pool()).await?;
        let sphere = match sphere {
            Some(sphere) => Some(sphere.try_into()?),
            None => None,
        };
        Ok(sphere)
    }

    async fn into_json(self, _data: &Data<Self::DataType>) -> Result<Self::Kind, Self::Error> {
        Ok(Self::Kind {
            kind: Default::default(),
            id: self.apub_id.clone(),
            preferred_username: self.name.clone(),
            followers: None,
            public_key: self.public_key(),
            name: None,
            description: Some(self.description),
            source: None,
            media_type: None,
            summary: None,
            icon: self.icon,
            image: self.banner,
            sensitive: Some(self.is_nsfw),
            attributed_to: None,
            posting_restricted_to_mods: None,
            inbox: self.inbox,
            outbox: generate_outbox_url(self.apub_id.inner())?,
            endpoints: None,
            featured: None,
            language: Vec::new(),
            manually_approves_followers: None,
            published: None,
            updated: None,
            discoverable: None,
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
        let moderators = json.attributed_to.clone();
        println!("Insert or update sphere");
        let sphere = insert_or_update_sphere(&json, data.app_data().get_db_pool()).await?;
        println!("get_admin_function_user");
        let function_user = get_admin_function_user(data.app_data().get_db_pool()).await?;
        if let Some(moderators) = moderators {
            for moderator in moderators {
                println!("get moderator");
                let person = moderator.dereference(data).await.map_err(to_app_error!("Failed to get moderator"))?;
                println!("set_user_sphere_role");
                set_user_sphere_role(&person.preferred_username, &sphere.sphere_name, PermissionLevel::Moderate, &function_user, data.app_data().get_db_pool()).await?;
            }
        }
        sphere.try_into()
    }
}

pub async fn get_sphere_by_apub_id(sphere_apub_id: &Url, db_pool: &PgPool) -> Result<Option<Sphere>, AppError> {
    let sphere = sqlx::query_as::<_, Sphere>(
        "SELECT *
        FROM spheres
        WHERE sphere_apub_id = $1",
    )
        .bind(sphere_apub_id.to_string())
        .fetch_optional(db_pool)
        .await?;

    Ok(sphere)
}

pub async fn insert_or_update_sphere(
    group: &Group,
    db_pool: &PgPool,
) -> Result<Sphere, AppError> {
    let instance = get_or_insert_instance(group.id.inner(), db_pool).await?;
    let inbox = match &group.endpoints {
        Some(endpoints) => endpoints.shared_inbox.clone(),
        None => group.inbox.clone(),
    };
    let sphere = sqlx::query_as::<_, Sphere>(
        "INSERT INTO spheres (sphere_name, sphere_apub_id, instance_id, description, is_nsfw, creator_id, is_local, inbox, public_key)
            VALUES (
                $1, $2, $3, $4, $5, $6, TRUE, $7, $8
            )
            ON CONFLICT (sphere_apub_id)
            DO UPDATE SET
                sphere_name = EXCLUDED.sphere_name,
                instance_id = EXCLUDED.instance_id,
                description = EXCLUDED.description,
                is_nsfw = EXCLUDED.is_nsfw,
                creator_id = EXCLUDED.creator_id,
                inbox = EXCLUDED.inbox,
                public_key = EXCLUDED.public_key,
                timestamp = NOW()
            RETURNING *"
    )
        .bind(&group.preferred_username)
        .bind(group.id.inner().as_str())
        .bind(instance.instance_id)
        .bind(group.description.clone().unwrap_or_default())
        .bind(group.sensitive.unwrap_or(false))
        .bind(1)
        .bind(inbox.as_str())
        .bind(group.public_key.public_key_pem.clone())
        .fetch_one(db_pool)
        .await?;

    Ok(sphere)
}

#[cfg(test)]
mod tests {
    use activitypub_federation::traits::{Actor};
    use rand::prelude::StdRng;
    use rand::rngs::SysRng;
    use rand::SeedableRng;
    use rsa::pkcs1::LineEnding;
    use rsa::{RsaPrivateKey, RsaPublicKey};
    use rsa::pkcs8::{EncodePrivateKey, EncodePublicKey};
    use url::Url;
    use sphare_core_common::constants::RSA_KEY_SIZE;
    use crate::group::ApubSphere;

    fn get_apub_sphere() -> ApubSphere {
        let mut rng = StdRng::try_from_rng(&mut SysRng).expect("Should get rng");
        let priv_key = RsaPrivateKey::new(&mut rng, RSA_KEY_SIZE).expect("Should get private key");
        let pub_key_pem = RsaPublicKey::from(&priv_key).to_public_key_pem(LineEnding::default()).expect("Should get public key pem");

        ApubSphere {
            apub_id: Url::parse("https://www.sphare.space/c/SomeSphere").expect("Should be valid apub_id").into(),
            name: String::from("SomeSphere"),
            description: String::from("The description"),
            icon: None,
            banner: None,
            is_nsfw: false,
            inbox: Url::parse("https://www.sphare.space/inbox").expect("Should be valid inbox").into(),
            public_key: pub_key_pem,
            private_key: Some(priv_key),
        }
    }
    #[test]
    fn test_apub_sphere_actor_public_key_pem() {
        let apub_sphere = get_apub_sphere();
        assert_eq!(apub_sphere.public_key_pem(), apub_sphere.public_key);
    }

    #[test]
    fn test_apub_sphere_actor_private_key_pem() {
        let apub_sphere = get_apub_sphere();
        assert_eq!(
            apub_sphere.private_key_pem(),
            apub_sphere.private_key.map(|private_key| private_key.to_pkcs8_pem(LineEnding::default()).expect("Should create private key pem").to_string())
        );
    }

    #[test]
    fn test_apub_sphere_actor_inbox() {
        let apub_sphere = get_apub_sphere();
        assert_eq!(apub_sphere.inbox(), apub_sphere.inbox);
    }
}