use activitypub_federation::{
    config::Data,
    fetch::object_id::ObjectId,
    kinds::actor::PersonType,
    protocol::{
        public_key::PublicKey,
        verification::verify_domains_match
    },
    traits::{Actor, Object},
};
use rsa::pkcs1::LineEnding;
use rsa::pkcs8::{DecodePrivateKey, EncodePrivateKey};
use rsa::RsaPrivateKey;
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;
use sqlx::PgPool;
use url::Url;

use sphare_core_common::activity_pub::ApHelper;
use sphare_core_common::errors::AppError;
use sphare_core_common::instance::ssr::get_or_insert_instance;
use sphare_core_common::to_app_error;

#[skip_serializing_none]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Person {
    #[serde(rename = "type")]
    pub(crate) kind: PersonType,
    pub(crate) id: ObjectId<ApubPerson>,
    /// username, set at account creation and usually fixed after that
    pub(crate) preferred_username: String,
    /// displayname
    pub(crate) name: Option<String>,
    pub(crate) inbox: Url,
    /// mandatory field in activitypub, sphare currently serves an empty outbox
    pub(crate) outbox: Url,
    pub(crate) public_key: PublicKey,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ApubPerson {
    pub apub_id: ObjectId<ApubPerson>,
    /// username, set at account creation and usually fixed after that
    pub preferred_username: String,
    /// displayname
    pub name: Option<String>,
    inbox: Url,
    outbox: Url,
    public_key: String,
    private_key: Option<RsaPrivateKey>,
}

#[derive(sqlx::FromRow, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DbPerson {
    pub person_id: i64,
    pub instance_id: i64,
    pub username: String,
    pub display_name: Option<String>,
    pub is_nsfw: bool,
    pub actor_id: String,
    pub inbox: String,
    pub outbox: String,
    pub is_local: bool,
    pub public_key: String,
    pub private_key: Option<String>,
    pub last_refreshed_at: chrono::DateTime<chrono::Utc>,
    pub delete_timestamp: Option<chrono::DateTime<chrono::Utc>>,
}

impl ApubPerson {
    pub fn new(
        apub_id: ObjectId<ApubPerson>,
        preferred_username: String,
        name: Option<String>,
        inbox: Url,
        outbox: Url,
        public_key: String,
        private_key: Option<RsaPrivateKey>,
    ) -> ApubPerson {
        ApubPerson {
            apub_id,
            preferred_username,
            name,
            inbox,
            outbox,
            public_key,
            private_key,
        }
    }
}

impl TryFrom<DbPerson> for ApubPerson {
    type Error = AppError;
    fn try_from(db_person: DbPerson) -> Result<Self, Self::Error> {
        let private_key = match db_person.private_key {
            Some(private_key_pem) => RsaPrivateKey::from_pkcs8_pem(&private_key_pem).ok(),
            None => None,
        };
        Ok(ApubPerson {
            preferred_username: db_person.username,
            name: db_person.display_name,
            apub_id: Url::parse(&db_person.actor_id)?.into(),
            inbox: Url::parse(&db_person.inbox)?,
            outbox: Url::parse(&db_person.outbox)?,
            public_key: db_person.public_key,
            private_key,
        })
    }
}

impl Actor for ApubPerson {
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
impl Object for ApubPerson {
    type DataType = ApHelper;
    type Kind = Person;
    type Error = AppError;

    fn id(&self) -> &Url {
        self.apub_id.inner()
    }

    async fn read_from_id(actor_id: Url, data: &Data<Self::DataType>) -> Result<Option<Self>, Self::Error> {
        let db_person = get_person_by_actor_id(&actor_id, data.app_data().get_db_pool()).await?;
        let person = match db_person {
            Some(db_person) => Some(db_person.try_into()?),
            None => None,
        };
        Ok(person)
    }

    async fn into_json(self, _data: &Data<Self::DataType>) -> Result<Self::Kind, Self::Error> {
        Ok(Self::Kind {
            kind: Default::default(),
            name: self.name.clone(),
            preferred_username: self.preferred_username.clone(),
            id: self.apub_id.clone(),
            inbox: self.inbox.clone(),
            outbox: self.outbox.clone(),
            public_key: self.public_key(),
        })
    }

    async fn verify(json: &Self::Kind, expected_domain: &Url, _data: &Data<Self::DataType>) -> Result<(), Self::Error> {
        verify_domains_match(json.id.inner(), expected_domain).map_err(to_app_error!("Failed domain verification"))
    }

    async fn from_json(json: Self::Kind, data: &Data<Self::DataType>) -> Result<Self, Self::Error> {
        let db_person = insert_or_update_person(json, data.app_data().get_db_pool()).await?;
        db_person.try_into()
    }
}

pub async fn get_person_by_username(
    username: &str,
    db_pool: &PgPool,
) -> Result<DbPerson, AppError> {
    let person = sqlx::query_as!(
        DbPerson,
        "SELECT p.*, u.private_key FROM persons p
        JOIN users u ON u.person_id = p.person_id
        WHERE p.username = $1 AND p.is_local = TRUE",
        username,
    ).fetch_one(db_pool).await?;

    Ok(person)
}

pub async fn get_person_by_actor_id(
    actor_id: &Url,
    db_pool: &PgPool,
) -> Result<Option<DbPerson>, AppError> {
    let person = sqlx::query_as!(
        DbPerson,
        r#"
        SELECT
            p.person_id as "person_id!",
            p.instance_id as "instance_id!",
            p.username as "username!",
            p.display_name as "display_name!",
            p.is_nsfw as "is_nsfw!",
            p.actor_id as "actor_id!",
            p.inbox as "inbox!",
            p.outbox as "outbox!",
            p.is_local as "is_local!",
            p.public_key as "public_key!",
            u.private_key as "private_key?",
            p.last_refreshed_at as "last_refreshed_at!",
            p.delete_timestamp as "delete_timestamp?"
        FROM persons p
        LEFT JOIN users u ON u.person_id = p.person_id
        WHERE p.actor_id = $1
        "#,
        actor_id.to_string(),
    ).fetch_optional(db_pool).await?;

    Ok(person)
}

pub async fn insert_or_update_person(
    person: Person,
    db_pool: &PgPool,
) -> Result<DbPerson, AppError> {

    let instance = get_or_insert_instance(person.id.inner(), db_pool).await?;
    let person = sqlx::query_as!(
        DbPerson,
        "INSERT INTO persons
        (instance_id, username, display_name, actor_id, inbox, outbox, is_local, public_key)
            VALUES (
                $1, $2, $3, $4, $5, $6, FALSE, $7
            )
        ON CONFLICT (actor_id) WHERE delete_timestamp IS NULL
        DO UPDATE SET
            username = EXCLUDED.username,
            display_name = EXCLUDED.display_name,
            inbox = EXCLUDED.inbox,
            outbox = EXCLUDED.outbox,
            public_key = EXCLUDED.public_key,
            last_refreshed_at = NOW()
        RETURNING *, NULL as private_key",
        instance.instance_id,
        person.preferred_username,
        person.name,
        person.id.inner().to_string(),
        person.inbox.to_string(),
        person.outbox.to_string(),
        person.public_key.public_key_pem,
    ).fetch_one(db_pool).await?;

    Ok(person)
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
    use crate::person::ApubPerson;

    fn get_apub_person() -> ApubPerson {
        let mut rng = StdRng::try_from_rng(&mut SysRng).expect("Should get rng");
        let priv_key = RsaPrivateKey::new(&mut rng, RSA_KEY_SIZE).expect("Should get private key");
        let pub_key_pem = RsaPublicKey::from(&priv_key).to_public_key_pem(LineEnding::default()).expect("Should get public key pem");

        ApubPerson {
            apub_id: Url::parse("https://mastodon.social/users/SphareDev").expect("Should be valid apub_id").into(),
            preferred_username: "SphareDev".to_string(),
            name: None,
            inbox: Url::parse("https://mastodon.social/users/SphareDev/inbox").expect("Should be valid inbox url"),
            outbox: Url::parse("https://mastodon.social/users/SphareDev/outbox").expect("Should be valid outbox url"),
            public_key: pub_key_pem,
            private_key: Some(priv_key),
        }
    }
    #[test]
    fn test_apub_person_actor_public_key_pem() {
        let apub_person = get_apub_person();
        assert_eq!(apub_person.public_key_pem(), apub_person.public_key);
    }

    #[test]
    fn test_apub_person_actor_private_key_pem() {
        let apub_person = get_apub_person();
        assert_eq!(
            apub_person.private_key_pem(),
            apub_person.private_key.map(|private_key| private_key.to_pkcs8_pem(LineEnding::default()).expect("Should create private key pem").to_string())
        );
    }

    #[test]
    fn test_apub_person_actor_inbox() {
        let apub_person = get_apub_person();
        assert_eq!(apub_person.inbox(), apub_person.inbox);
    }
}