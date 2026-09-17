use std::fs::File;
use activitypub_federation::fetch::object_id::ObjectId;
use activitypub_federation::traits::{Actor, Object};
use leptos::serde_json;
use url::Url;
use sphare_core_apub::group::{get_sphere_by_apub_id, insert_or_update_sphere, ApubSphere};
use sphare_core_apub::person::ApubPerson;
use sphare_core_sphere::sphere::ssr::create_sphere;
use sphare_core_user::user::ssr::get_admin_function_user;
use crate::apub_factory::{mock_apub_group, mock_apub_group_moderators, mock_apub_persons};
use crate::apub_utils::test_sphere_role_vec;
use crate::common::{create_test_user, get_db_pool};
use crate::utils::{get_apub_sphere, init_local_instance_and_get_apub_config};

mod apub_factory;
mod apub_utils;
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
    let function_user = get_admin_function_user(&db_pool).await.expect("Should get admin");

    let group_id_str = "https://enterprise.lemmy.ml/c/tenforward";
    let group_id = Url::parse(group_id_str).expect("Should be valid url");

    assert_eq!(get_sphere_by_apub_id(&group_id, &db_pool).await.expect("Should get option"), None);
    let group_file = File::open("assets/apub/lemmy/group.json").expect("Should open group.json");
    let mut group = serde_json::from_reader(group_file).expect("Should deserialize Group");

    let sphere = insert_or_update_sphere(&group, &function_user, &db_pool).await.expect("Should get Sphere");
    assert_eq!(sphere.sphere_name, group.preferred_username);
    assert_eq!(sphere.sphere_apub_id, group.id.inner().to_string());
    assert_eq!(sphere.description, group.description.clone().expect("Should have a description"));
    assert_eq!(sphere.is_nsfw, group.sensitive.unwrap_or_default());
    assert_eq!(sphere.inbox, group.endpoints.clone().expect("Should have an endpoint").shared_inbox.to_string());
    assert_eq!(sphere.followers_endpoint, group.followers.clone().map(|url| url.to_string()));
    assert_eq!(sphere.moderators_endpoint, group.attributed_to.clone().expect("Should get group moderators").url_string());
    assert_eq!(sphere.public_key, group.public_key.public_key_pem);

    let result_sphere = get_sphere_by_apub_id(&group_id, &db_pool).await.expect("Should get option").expect("Sphere should be some");
    assert_eq!(result_sphere, sphere);

    group.preferred_username = String::from("tenforward_updated");

    let updated_sphere = insert_or_update_sphere(&group, &function_user, &db_pool).await.expect("Should get sphere");
    assert_eq!(updated_sphere.sphere_name, group.preferred_username);
    assert_eq!(updated_sphere.sphere_apub_id, group.id.inner().to_string());
    assert_eq!(updated_sphere.description, group.description.clone().expect("Should have a description"));
    assert_eq!(updated_sphere.is_nsfw, group.sensitive.unwrap_or_default());
    assert_eq!(updated_sphere.inbox, group.endpoints.clone().expect("Should have an endpoint").shared_inbox.to_string());
    assert_eq!(updated_sphere.followers_endpoint, group.followers.clone().map(|url| url.to_string()));
    assert_eq!(updated_sphere.moderators_endpoint, group.attributed_to.clone().expect("Should get group moderators").url_string());
    assert_eq!(updated_sphere.public_key, group.public_key.public_key_pem);

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
    let function_user = get_admin_function_user(&db_pool).await.expect("Should get admin");

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
    let sphere = insert_or_update_sphere(&group, &function_user, &db_pool).await.expect("Should get Sphere");
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

    let mock_server = wiremock::MockServer::start().await; // binds 127.0.0.1:<ephemeral port>

    let mut group = mock_apub_group(&mock_server).await;

    let mod_name_vec = ["alice", "bob", "charles"];
    let mod_apub_id_vec: Vec<ObjectId<ApubPerson>> = mock_apub_persons(&mod_name_vec, &mock_server).await;

    mock_apub_group_moderators(&group.id, &mod_apub_id_vec[0..2], &mock_server).await;

    assert!(get_sphere_by_apub_id(group.id.inner(), &db_pool).await.expect("Should get option").is_none());

    let apub_sphere = ApubSphere::from_json(group.clone(), &apub_data).await.expect("Should get ApubSphere");
    let sphere = get_sphere_by_apub_id(group.id.inner(), &db_pool).await.expect("Should get option").expect("Sphere should be some");
    assert_eq!(apub_sphere, sphere.clone().try_into().expect("Should convert to ApubSphere"));
    test_sphere_role_vec(&sphere, &mod_name_vec[0..2], &mod_apub_id_vec[0..2], &db_pool).await;

    mock_server.reset().await;

    let mod_apub_id_vec: Vec<ObjectId<ApubPerson>> = mock_apub_persons(&mod_name_vec, &mock_server).await;
    mock_apub_group_moderators(&group.id, &mod_apub_id_vec[1..3], &mock_server).await;

    group.preferred_username = String::from("updated_username");

    let updated_apub_sphere = ApubSphere::from_json(group.clone(), &apub_data).await.expect("Should get sphere");
    let sphere = get_sphere_by_apub_id(group.id.inner(), &db_pool).await.expect("Should get option").expect("Sphere should be some");
    assert_eq!(updated_apub_sphere, sphere.clone().try_into().expect("Should convert to ApubSphere"));
    test_sphere_role_vec(&sphere, &mod_name_vec[1..3], &mod_apub_id_vec[1..3], &db_pool).await;
}