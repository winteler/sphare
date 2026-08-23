use std::fs::File;
use activitypub_federation::traits::{Actor, Object};
use leptos::serde_json;
use url::Url;
use sphare_core_apub::group::{get_sphere_by_apub_id, insert_or_update_sphere, ApubSphere};
use sphare_core_sphere::sphere::ssr::create_sphere;
use crate::common::{create_test_user, get_db_pool};
use crate::utils::{get_apub_sphere, init_local_instance_and_get_apub_config};

mod common;
mod data_factory;
mod utils;

#[tokio::test]
async fn test_get_sphere_by_apub_id() {
    let db_pool = get_db_pool().await;
    let user = create_test_user(&db_pool).await;
    let sphere = create_sphere("Activity", "Pub", false, &user, &db_pool).await.expect("Should create sphere");
    let sphere_apub_id = Url::parse(&sphere.sphere_apub_id).expect("Should parse sphere URL");
    let result_sphere = get_sphere_by_apub_id(&sphere_apub_id, &db_pool).await.expect("Should get sphere by apub");
    assert_eq!(result_sphere, Some(sphere));

    let unknown_apub_id = Url::parse("https://unknown.apub/id").expect("Should create unknown URL");
    let result_sphere = get_sphere_by_apub_id(&unknown_apub_id, &db_pool).await.expect("Should get sphere by apub");
    assert_eq!(result_sphere, None);
}

#[tokio::test]
async fn test_insert_or_update_sphere() {
    let db_pool = get_db_pool().await;

    let group_id_str = "https://enterprise.lemmy.ml/c/tenforward";
    let group_id = Url::parse(group_id_str).expect("Should be valid url");

    assert_eq!(get_sphere_by_apub_id(&group_id, &db_pool).await.expect("Should get option"), None);
    let group_file = File::open("assets/apub/lemmy/group.json").expect("Should open group.json");
    let mut group = serde_json::from_reader(group_file).expect("Should deserialize Group");

    let sphere = insert_or_update_sphere(&group, &db_pool).await.expect("Should get Sphere");
    assert_eq!(sphere.sphere_name, "tenforward");
    assert_eq!(sphere.sphere_apub_id, "https://enterprise.lemmy.ml/c/tenforward");
    assert_eq!(sphere.description, "A description of ten forward.");
    assert_eq!(sphere.is_nsfw, false);
    assert_eq!(sphere.inbox, "https://enterprise.lemmy.ml/inbox");
    assert_eq!(sphere.public_key, "-----BEGIN PUBLIC KEY-----\nMIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEAzRjKTNtvDCmugplwEh+g\nx1bhKm6BHUZfXfpscgMMm7tXFswSDzUQirMgfkxa9ubfr1PDFKffA2vQ9x6CyuO/\n70xTafdOHyV1tSqzgKz0ZvFZ/VCOo6qy1mYWVkrtBm/fKzM+87MdkKYB/zI4VyEJ\nLfLQgjwxBAEYUH3CBG71U0gO0TwbimWNN0vqlfp0QfThNe1WYObF88ZVzMLgFbr7\nRHBItZjlZ/d8foPDidlIR3l2dJjy0EsD8F9JM340jtX7LXqFmU4j1AQKNHTDLnUF\nwYVhzuQGNJ504l5LZkFG54XfIFT7dx2QwuuM9bSnfPv/98RYrq1Si6tCkxEt1cVe\n4wIDAQAB\n-----END PUBLIC KEY-----\n");

    let result_sphere = get_sphere_by_apub_id(&group_id, &db_pool).await.expect("Should get option").expect("Sphere should be some");
    assert_eq!(result_sphere, sphere);

    group.preferred_username = String::from("tenforward_updated");

    let updated_sphere = insert_or_update_sphere(&group, &db_pool).await.expect("Should get sphere");
    assert_eq!(updated_sphere.sphere_name, "tenforward_updated");
    assert_eq!(updated_sphere.sphere_apub_id, "https://enterprise.lemmy.ml/c/tenforward");
    assert_eq!(updated_sphere.description, "A description of ten forward.");
    assert_eq!(updated_sphere.is_nsfw, false);
    assert_eq!(updated_sphere.inbox, "https://enterprise.lemmy.ml/inbox");
    assert_eq!(sphere.public_key, "-----BEGIN PUBLIC KEY-----\nMIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEAzRjKTNtvDCmugplwEh+g\nx1bhKm6BHUZfXfpscgMMm7tXFswSDzUQirMgfkxa9ubfr1PDFKffA2vQ9x6CyuO/\n70xTafdOHyV1tSqzgKz0ZvFZ/VCOo6qy1mYWVkrtBm/fKzM+87MdkKYB/zI4VyEJ\nLfLQgjwxBAEYUH3CBG71U0gO0TwbimWNN0vqlfp0QfThNe1WYObF88ZVzMLgFbr7\nRHBItZjlZ/d8foPDidlIR3l2dJjy0EsD8F9JM340jtX7LXqFmU4j1AQKNHTDLnUF\nwYVhzuQGNJ504l5LZkFG54XfIFT7dx2QwuuM9bSnfPv/98RYrq1Si6tCkxEt1cVe\n4wIDAQAB\n-----END PUBLIC KEY-----\n");


    let result_updated_sphere = get_sphere_by_apub_id(&group_id, &db_pool).await.expect("Should get option").expect("Sphere should be some");
    assert_eq!(result_updated_sphere, updated_sphere);
}

#[tokio::test]
async fn test_apub_sphere_object_id() {
    let apub_sphere = get_apub_sphere();
    assert_eq!(apub_sphere.id(), apub_sphere.id());
}

#[tokio::test]
async fn test_apub_sphere_object_read_from_id() {
    let db_pool = get_db_pool().await;

    let group_id_str = "https://www.sphare.space/c/Sphare";
    let group_id = Url::parse(group_id_str).expect("Should be valid url");

    let (_, apub_config) = init_local_instance_and_get_apub_config(&db_pool).await;
    let apub_data = apub_config.to_request_data();
    assert_eq!(ApubSphere::read_from_id(group_id.clone(), &apub_data).await, Ok(None));

    let group_json = r#"{
        "id": "https://www.sphare.space/c/Sphare",
        "type": "Group",
        "preferredUsername": "SphareDev",
        "name": "Sphare",
        "inbox": "https://www.sphare.space/inbox",
        "outbox": "https://www.sphare.space/c/Sphare/outbox",
        "publicKey": {
            "id": "https://www.sphare.space/c/Sphare#main-key",
            "owner": "https://www.sphare.space/c/Sphare",
            "publicKeyPem": "12345"
        }
    }"#;
    let group = serde_json::from_str(group_json).expect("Should deserialize Group");
    let sphere = insert_or_update_sphere(&group, &db_pool).await.expect("Should get Sphere");
    let apub_sphere = sphere.try_into().expect("Should convert to ApubSphere");
    assert_eq!(ApubSphere::read_from_id(group_id.clone(), &apub_data).await, Ok(Some(apub_sphere)));
}

#[tokio::test]
async fn test_apub_sphere_object_into_json() {
    let db_pool = get_db_pool().await;
    let (_, apub_config) = init_local_instance_and_get_apub_config(&db_pool).await;
    let apub_sphere = get_apub_sphere();
    let public_key = apub_sphere.public_key_pem();
    let group_json = apub_sphere.clone().into_json(&apub_config.to_request_data()).await.expect("Should get Group json");

    let expected_json = serde_json::json!({
        "type": "Group",
        "id": "https://www.sphare.space/c/SomeSphere",
        "preferredUsername": "SomeSphere",
        "publicKey": {
            "id": "https://www.sphare.space/c/SomeSphere#main-key",
            "owner": "https://www.sphare.space/c/SomeSphere",
            "publicKeyPem": public_key
        },
        "description": "The description",
        "sensitive": false,
        "inbox": "https://www.sphare.space/inbox",
        "outbox": "https://www.sphare.space/c/SomeSphere/outbox",
        "language": []
    });
    assert_eq!(
        serde_json::to_string(&group_json).expect("Should serialize Group json"),
        serde_json::to_string(&expected_json).expect("Should serialize expected_json")
    );
}

#[tokio::test]
async fn test_apub_sphere_object_verify() {
    let db_pool = get_db_pool().await;
    let (_, apub_config) = init_local_instance_and_get_apub_config(&db_pool).await;
    let apub_data = apub_config.to_request_data();
    let apub_sphere = get_apub_sphere();
    let valid_url = apub_sphere.apub_id.inner().clone();
    let group = apub_sphere.into_json(&apub_data).await.expect("Should get group");
    let invalid_url = Url::parse("https://sample.net/abc").expect("Should be valid url");
    assert!(ApubSphere::verify(&group, &valid_url, &apub_data).await.is_ok());
    assert!(ApubSphere::verify(&group, &invalid_url, &apub_data).await.is_err());
}

#[tokio::test]
async fn test_apub_sphere_object_from_json() {
    let db_pool = get_db_pool().await;
    let (_, apub_config) = init_local_instance_and_get_apub_config(&db_pool).await;
    let apub_data = apub_config.to_request_data();

    let group_id_str = "https://mastodon.social/users/SphareDev";
    let group_id = Url::parse(group_id_str).expect("Should be valid url");

    assert!(get_sphere_by_apub_id(&group_id, &db_pool).await.expect("Should get option").is_none());
    let group_json = r#"{
        "id": "https://mastodon.social/users/SphareDev",
        "type": "Group",
        "preferredUsername": "SphareDev",
        "name": "Sphare",
        "inbox": "https://mastodon.social/users/SphareDev/inbox",
        "outbox": "https://mastodon.social/users/SphareDev/outbox",
        "publicKey": {
            "id": "https://mastodon.social/users/SphareDev#main-key",
            "owner": "https://mastodon.social/users/SphareDev",
            "publicKeyPem": "12345"
        }
    }"#;
    let group = serde_json::from_str(group_json).expect("Should deserialize Group");

    let apub_sphere = ApubSphere::from_json(group, &apub_data).await.expect("Should get ApubSphere");
    let sphere = get_sphere_by_apub_id(&group_id, &db_pool).await.expect("Should get option").expect("Sphere should be some");
    assert_eq!(apub_sphere, sphere.try_into().expect("Should convert to ApubSphere"));

    let updated_group_json = r#"{
        "id": "https://mastodon.social/users/SphareDev",
        "type": "Group",
        "preferredUsername": "SphareDev",
        "name": "SphareUpdated",
        "inbox": "https://mastodon.social/users/SphareDev/inbox",
        "outbox": "https://mastodon.social/users/SphareDev/outbox",
        "publicKey": {
            "id": "https://mastodon.social/users/SphareDev#main-key",
            "owner": "https://mastodon.social/users/SphareDev",
            "publicKeyPem": "54321"
        }
    }"#;
    let updated_group = serde_json::from_str(updated_group_json).expect("Should deserialize group");

    let updated_apub_sphere = insert_or_update_sphere(&updated_group, &db_pool).await.expect("Should get sphere");
    let sphere = get_sphere_by_apub_id(&group_id, &db_pool).await.expect("Should get option").expect("Sphere should be some");
    assert_eq!(updated_apub_sphere, sphere);
}