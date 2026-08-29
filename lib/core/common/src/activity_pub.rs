use rand::prelude::StdRng;
use rand::{SeedableRng};
use rand::rngs::SysRng;
use rsa::{RsaPrivateKey, RsaPublicKey};
use rsa::pkcs1::der::zeroize::Zeroizing;
use rsa::pkcs1::LineEnding;
use rsa::pkcs8::{EncodePrivateKey, EncodePublicKey};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use url::Url;
use crate::constants::RSA_KEY_SIZE;
use crate::errors::AppError;
use crate::to_app_error;

#[derive(Clone, Debug)]
pub struct ApubHelper {
    db_pool: PgPool,
    domain_name: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum PersonOrGroupType {
    Person,
    Group,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct PersonOrGroupModerators(Url);

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AttributedToPeertube {
    #[serde(rename = "type")]
    pub kind: PersonOrGroupType,
    pub id: Url,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(untagged)]
pub enum AttributedTo {
    Forum(PersonOrGroupModerators),
    Peertube(Vec<AttributedToPeertube>),
}

impl ApubHelper {
    pub fn new(db_pool: PgPool, domain_name: String) -> Self {
        Self {
            db_pool,
            domain_name,
        }
    }

    pub fn get_db_pool(&self) -> &PgPool {
        &self.db_pool
    }

    pub fn get_domain_name(&self) -> &str {
        &self.domain_name
    }
}

impl AttributedTo {
    pub fn url_string(self) -> Option<String> {
        match self {
            AttributedTo::Forum(f) => Some(f.0.to_string()),
            AttributedTo::Peertube(_) => None,
        }
    }
}

pub fn generate_rsa_keys_pem() -> Result<(String, Zeroizing<String>), AppError> {
    let mut rng = StdRng::try_from_rng(&mut SysRng).map_err(to_app_error!("Failed to get rng"))?;
    let priv_key = RsaPrivateKey::new(&mut rng, RSA_KEY_SIZE).map_err(to_app_error!("Failed to generate private key"))?;
    let priv_key_pem = priv_key.to_pkcs8_pem(LineEnding::default()).map_err(to_app_error!("Failed to create private key pem"))?;
    let pub_key_pem = RsaPublicKey::from(&priv_key).to_public_key_pem(LineEnding::LF).map_err(AppError::new)?;
    Ok((pub_key_pem, priv_key_pem))
}