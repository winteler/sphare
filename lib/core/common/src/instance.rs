use serde::{Deserialize, Serialize};

#[cfg_attr(feature = "ssr", derive(sqlx::FromRow))]
#[derive(Clone, Debug, PartialEq, Eq, Ord, PartialOrd, Serialize, Deserialize)]
pub struct Instance {
    pub instance_id: i64,
    pub instance_apub_id: String,
    pub is_local: bool,
    pub public_key: Option<String>,
    pub private_key: Option<String>,
    pub last_refreshed_at: chrono::DateTime<chrono::offset::Utc>,
}

#[cfg(feature = "ssr")]
pub mod ssr {
    use rand::prelude::StdRng;
    use rand::rngs::SysRng;
    use rand::SeedableRng;
    use rsa::pkcs8::{EncodePrivateKey, EncodePublicKey, LineEnding};
    use rsa::{RsaPrivateKey, RsaPublicKey};
    use sqlx::PgPool;
    use url::Url;

    use crate::constants::RSA_KEY_SIZE;
    use crate::errors::AppError;
    use crate::to_app_error;

    use crate::instance::Instance;

    /// Returns the ActivityPub instance id from a given url
    ///
    /// ```
    /// use url::Url;
    /// use sphare_core_common::instance::ssr::get_instance_apub_id_from_url;
    ///
    /// let url = Url::parse("https://mysite.com/some/path").expect("Should be valid url");
    /// assert_eq!(get_instance_apub_id_from_url(url).to_string(), "https://mysite.com/");
    /// ```
    pub fn get_instance_apub_id_from_url(mut url: Url) -> Url {
        url.set_fragment(None);
        url.set_path("");
        url.set_query(None);
        url
    }

    pub async fn init_local_instance(instance_url: &Url, db_pool: &PgPool) -> Result<Instance, AppError> {
        let existing = sqlx::query_as!(
            Instance,
            "SELECT * FROM instances WHERE is_local = TRUE"
        ).fetch_optional(db_pool).await?;

        if let Some(instance) = existing && !instance.instance_apub_id.is_empty() {
            return Ok(instance);
        }

        let mut rng = StdRng::try_from_rng(&mut SysRng).map_err(to_app_error!("Failed to get rng"))?;
        let priv_key = RsaPrivateKey::new(&mut rng, RSA_KEY_SIZE).map_err(to_app_error!("Failed to generate private key"))?;
        let priv_key_pem = priv_key.to_pkcs8_pem(LineEnding::default()).map_err(to_app_error!("Failed to create private key pem"))?;
        let pub_key_pem = RsaPublicKey::from(&priv_key).to_public_key_pem(LineEnding::default()).map_err(to_app_error!("Failed to generate public key pem"))?;

        let person = sqlx::query_as!(
            Instance,
            "INSERT INTO instances (instance_apub_id, is_local, public_key, private_key)
            VALUES ($1, TRUE, $2, $3)
            ON CONFLICT (is_local) WHERE is_local = TRUE
            DO UPDATE
            SET instance_apub_id = EXCLUDED.instance_apub_id,
                public_key = EXCLUDED.public_key,
                private_key = EXCLUDED.private_key,
                last_refreshed_at = NOW()
            RETURNING *",
            instance_url.to_string(),
            pub_key_pem,
            priv_key_pem.to_string()
        ).fetch_one(db_pool).await?;

        Ok(person)
    }

    pub async fn get_or_insert_instance(instance_url: &Url, db_pool: &PgPool) -> Result<Instance, AppError> {
        let instance_apub_id = get_instance_apub_id_from_url(instance_url.clone()).to_string();
        let instance = sqlx::query_as!(
            Instance,
            "SELECT * FROM instances
            WHERE instance_apub_id = $1",
            instance_apub_id,
        ).fetch_optional(db_pool).await?;

        match instance {
            Some(instance) => Ok(instance),
            None => {
                let instance = sqlx::query_as!(
                    Instance,
                    "INSERT INTO instances (instance_apub_id, is_local, public_key, private_key)
                    VALUES ($1, FALSE, NULL, NULL)
                    RETURNING *",
                    instance_apub_id,
                ).fetch_one(db_pool).await?;
                Ok(instance)
            }
        }
    }
}