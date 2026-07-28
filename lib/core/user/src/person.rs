use activitypub_federation::config::Data;
use activitypub_federation::fetch::object_id::ObjectId;
use activitypub_federation::kinds::actor::PersonType;
use activitypub_federation::protocol::public_key::PublicKey;
use activitypub_federation::protocol::verification::verify_domains_match;
use activitypub_federation::traits::{Actor, Object};
use rsa::{RsaPrivateKey};
use rsa::pkcs1::LineEnding;
use rsa::pkcs8::{DecodePrivateKey, EncodePrivateKey};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use url::Url;
use sphare_core_common::activity_pub::ApHelper;
use sphare_core_common::errors::AppError;
use sphare_core_common::to_app_error;

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApPerson {
    #[serde(rename = "type")]
    kind: PersonType,
    username: String,
    preferred_username: String,
    id: ObjectId<Person>,
    inbox: Url,
    outbox: Url,
    public_key: PublicKey,
}

#[derive(Clone, Debug)]
pub struct Person {
    ap_id: ObjectId<Person>,
    preferred_username: String,
    username: String,
    inbox: Url,
    outbox: Url,
    public_key: String,
    private_key: Option<RsaPrivateKey>,
}

#[derive(sqlx::FromRow, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DbPerson {
    pub person_id: i64,
    pub username: String,
    pub display_name: String,
    pub is_nsfw: bool,
    pub federation_id: String,
    pub inbox: String,
    pub outbox: String,
    pub is_local: bool,
    pub public_key: String,
    pub private_key: Option<String>,
    pub last_refreshed_at: chrono::DateTime<chrono::Utc>,
    pub delete_timestamp: Option<chrono::DateTime<chrono::Utc>>,
}

impl TryFrom<DbPerson> for Person {
    type Error = AppError;
    fn try_from(db_person: DbPerson) -> Result<Self, Self::Error> {
        let private_key = match db_person.private_key {
            Some(private_key_pem) => RsaPrivateKey::from_pkcs8_pem(&private_key_pem).ok(),
            None => None,
        };
        Ok(Person {
            username: db_person.username,
            preferred_username: db_person.display_name,
            ap_id: Url::parse(&db_person.federation_id)?.into(),
            inbox: Url::parse(&db_person.inbox)?,
            outbox: Url::parse(&db_person.outbox)?,
            public_key: db_person.public_key,
            private_key,
        })
    }
}

impl Actor for Person {
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
impl Object for Person {
    type DataType = ApHelper;
    type Kind = ApPerson;
    type Error = AppError;

    fn id(&self) -> &Url {
        self.ap_id.inner()
    }

    async fn read_from_id(person_id: Url, data: &Data<Self::DataType>) -> Result<Option<Self>, Self::Error> {
        let db_person = get_person_by_federation_id(&person_id, data.app_data().get_db_pool()).await?;
        let person = match db_person {
            Some(db_person) => Some(db_person.try_into()?),
            None => None,
        };
        Ok(person)
    }

    async fn into_json(self, _data: &Data<Self::DataType>) -> Result<Self::Kind, Self::Error> {
        Ok(Self::Kind {
            kind: Default::default(),
            username: self.username.clone(),
            preferred_username: self.preferred_username.clone(),
            id: self.ap_id.clone(),
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

pub async fn get_person_by_federation_id(
    federation_id: &Url,
    db_pool: &PgPool,
) -> Result<Option<DbPerson>, AppError> {
    let person = sqlx::query_as!(
        DbPerson,
        r#"
        SELECT
            p.person_id as "person_id!",
            p.username as "username!",
            p.display_name as "display_name!",
            p.is_nsfw as "is_nsfw!",
            p.federation_id as "federation_id!",
            p.inbox as "inbox!",
            p.outbox as "outbox!",
            p.is_local as "is_local!",
            p.public_key as "public_key!",
            u.private_key as "private_key?",
            p.last_refreshed_at as "last_refreshed_at!",
            p.delete_timestamp as "delete_timestamp?"
        FROM persons p
        LEFT JOIN users u ON u.person_id = p.person_id
        WHERE p.federation_id = $1
        "#,
        federation_id.to_string(),
    ).fetch_optional(db_pool).await?;

    Ok(person)
}

pub async fn insert_or_update_person(
    ap_person: ApPerson,
    db_pool: &PgPool,
) -> Result<DbPerson, AppError> {
    let person = sqlx::query_as!(
        DbPerson,
        "INSERT INTO persons
        (username, display_name, federation_id, inbox, outbox, is_local, public_key)
            VALUES ($1, $2, $3, $4, $5, FALSE, $6)
        ON CONFLICT (federation_id) WHERE delete_timestamp IS NULL
        DO UPDATE SET
            username = EXCLUDED.username,
            display_name = EXCLUDED.display_name,
            inbox = EXCLUDED.inbox,
            outbox = EXCLUDED.outbox,
            public_key = EXCLUDED.public_key,
            last_refreshed_at = NOW()
        RETURNING *, NULL as private_key",
        ap_person.username,
        ap_person.preferred_username,
        ap_person.id.inner().to_string(),
        ap_person.inbox.to_string(),
        ap_person.outbox.to_string(),
        ap_person.public_key.public_key_pem,
    ).fetch_one(db_pool).await?;

    Ok(person)
}