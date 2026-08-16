use url::Url;

use sphare_core_user::instance::ssr::{get_or_insert_instance, init_local_instance};

use crate::common::*;
use crate::utils::get_local_instance;
mod common;
mod utils;

#[tokio::test]
async fn test_init_local_instance() {
    let db_pool = get_db_pool().await;
    let initial_instance = get_local_instance(&db_pool).await;
    assert!(initial_instance.instance_apub_id.is_empty());
    assert!(initial_instance.is_local);
    assert!(initial_instance.public_key.is_none());
    assert!(initial_instance.private_key.is_none());

    let app_origin = "https://www.sphare.space/";
    let instance_url = Url::parse(app_origin).expect("App origin url should be valid");
    let updated_instance = init_local_instance(&instance_url, &db_pool).await.expect("Should update local instance");

    assert_eq!(updated_instance.instance_apub_id, app_origin);
    assert!(updated_instance.is_local);
    // TODO test key validity
    assert!(!updated_instance.public_key.as_ref().expect("Should have public key in local instance").is_empty());
    assert!(!updated_instance.private_key.as_ref().expect("Should have private key in local instance").is_empty());

    // Don't update if already initialized
    let other_origin = "https://whatever.com/";
    let other_instance_url = Url::parse(other_origin).expect("App origin url should be valid");
    let not_updated_instance = init_local_instance(&other_instance_url, &db_pool).await.expect("Should return existing local instance");
    assert_eq!(not_updated_instance, updated_instance);

}

#[tokio::test]
async fn test_get_or_insert_instance() {
    let db_pool = get_db_pool().await;
    let app_origin = "https://www.sphare.space/";
    let instance_url = Url::parse(app_origin).expect("App origin url should be valid");
    let local_instance = init_local_instance(&instance_url, &db_pool).await.expect("Should update local instance");

    let instance = get_or_insert_instance(&instance_url, &db_pool).await.expect("Should get local instance");
    assert_eq!(instance, local_instance);

    let remote_origin = "https://whatever.com/";
    let remote_instance_url = Url::parse(remote_origin).expect("Other origin url should be valid");
    let instance = get_or_insert_instance(&remote_instance_url, &db_pool).await.expect("Should get local instance");
    assert_eq!(instance.instance_apub_id, remote_origin);
    assert_eq!(instance.is_local, false);
    // TODO test key validity
    assert!(instance.public_key.is_none());
    assert!(instance.private_key.is_none());
}