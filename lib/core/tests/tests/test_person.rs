use std::fs::File;
use activitypub_federation::traits::{Actor, Object};
use leptos::serde_json;
use url::Url;

use sphare_core_apub::person::{ApubPerson, Person};
use sphare_core_apub::person::{get_person_by_actor_id, get_person_by_username, insert_or_update_person};
use sphare_core_common::routes::get_profile_link;
use sphare_core_user::user::User;

use crate::common::{create_test_user, get_db_pool};
use crate::utils::{get_apub_person, init_local_instance_and_get_apub_config};

mod common;
mod data_factory;
mod utils;

#[tokio::test]
async fn test_get_person_by_username() {
    let db_pool = get_db_pool().await;
    let user = create_test_user(&db_pool).await;

    let person = get_person_by_username(&user.username, &db_pool).await.expect("Should get person");
    assert_eq!(person.person_id, user.person_id);
    assert_eq!(person.username, user.username);
    assert_eq!(person.display_name, Some(user.username.clone()));
    assert_eq!(person.is_nsfw, false);
    assert_eq!(person.is_local, true);
    assert_eq!(person.actor_id, get_profile_link(&user.username).expect("Should get profile link").to_string());
    assert_eq!(person.inbox, User::get_user_inbox(&user.username).expect("Should get user inbox").to_string());
    assert_eq!(person.outbox, User::get_user_outbox(&user.username).expect("Should get user outbox").to_string());
    assert!(person.delete_timestamp.is_none());
}

#[tokio::test]
async fn test_get_person_by_actor_id() {
    let db_pool = get_db_pool().await;
    let user = create_test_user(&db_pool).await;
    let user_actor_id = get_profile_link(&user.username).expect("Should get profile link");

    let person = get_person_by_actor_id(&user_actor_id, &db_pool).await.expect("Should get person").expect("Person should be some");
    assert_eq!(person.person_id, user.person_id);
    assert_eq!(person.username, user.username);
    assert_eq!(person.display_name, Some(user.username.clone()));
    assert_eq!(person.is_nsfw, false);
    assert_eq!(person.is_local, true);
    assert_eq!(person.actor_id, get_profile_link(&user.username).expect("Should get profile link").to_string());
    assert_eq!(person.inbox, User::get_user_inbox(&user.username).expect("Should get user inbox").to_string());
    assert_eq!(person.outbox, User::get_user_outbox(&user.username).expect("Should get user outbox").to_string());
    assert!(person.delete_timestamp.is_none());

    let unknown_actor_id = Url::parse("https://mastodon.social/users/SphareDev").expect("Should be valid url");
    assert!(get_person_by_actor_id(&unknown_actor_id, &db_pool).await.expect("Should get option").is_none());
}

#[tokio::test]
async fn test_insert_or_update_person() {
    let db_pool = get_db_pool().await;

    let person_file = File::open("assets/apub/lemmy/person.json").expect("Should open group.json");
    let mut person: Person = serde_json::from_reader(person_file).expect("Should deserialize Group");
    let person_id = person.id.inner();

    assert!(get_person_by_actor_id(person_id, &db_pool).await.expect("Should get option").is_none());

    let db_person = insert_or_update_person(&person, &db_pool).await.expect("Should get person");
    assert_eq!(db_person.username, person.preferred_username);
    assert_eq!(db_person.display_name, person.name);
    assert_eq!(db_person.is_nsfw, false);
    assert_eq!(db_person.is_local, false);
    assert_eq!(db_person.actor_id, person.id.inner().to_string());
    assert_eq!(db_person.inbox, person.inbox.to_string());
    assert_eq!(db_person.outbox, person.outbox.to_string());
    assert_eq!(db_person.public_key, person.public_key.public_key_pem);
    assert!(db_person.delete_timestamp.is_none());

    let result_person = get_person_by_actor_id(person_id, &db_pool).await.expect("Should get option").expect("Person should be some");
    assert_eq!(result_person, db_person);

    person.name = Some(String::from("Updated"));

    let updated_db_person = insert_or_update_person(&person, &db_pool).await.expect("Should get person");
    assert_eq!(updated_db_person.username, person.preferred_username);
    assert_eq!(updated_db_person.display_name, person.name);
    assert_eq!(updated_db_person.is_nsfw, false);
    assert_eq!(updated_db_person.is_local, false);
    assert_eq!(db_person.actor_id, person.id.inner().to_string());
    assert_eq!(db_person.inbox, person.inbox.to_string());
    assert_eq!(db_person.outbox, person.outbox.to_string());
    assert_eq!(db_person.public_key, person.public_key.public_key_pem);
    assert!(updated_db_person.delete_timestamp.is_none());

    let result_updated_person = get_person_by_actor_id(person_id, &db_pool).await.expect("Should get option").expect("Person should be some");
    assert_eq!(result_updated_person, updated_db_person);
}

#[tokio::test]
async fn test_apub_person_object_id() {
    let apub_person = get_apub_person();
    assert_eq!(apub_person.id(), apub_person.id());
}

#[tokio::test]
async fn test_apub_person_object_read_from_id() {
    let db_pool = get_db_pool().await;

    let actor_id_str = "https://mastodon.social/users/SphareDev";
    let actor_id = Url::parse(actor_id_str).expect("Should be valid url");

    let (_, apub_config) = init_local_instance_and_get_apub_config(&db_pool).await;
    let apub_data = apub_config.to_request_data();
    assert_eq!(ApubPerson::read_from_id(actor_id.clone(), &apub_data).await, Ok(None));

    let person_json = r#"{
        "id": "https://mastodon.social/users/SphareDev",
        "type": "Person",
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
    let person = serde_json::from_str(person_json).expect("Should deserialize Person");
    let db_person = insert_or_update_person(&person, &db_pool).await.expect("Should get person");
    let apub_person = db_person.try_into().expect("Should convert to ApubPerson");
    assert_eq!(ApubPerson::read_from_id(actor_id.clone(), &apub_data).await, Ok(Some(apub_person)));
}

#[tokio::test]
async fn test_apub_person_object_into_json() {
    let db_pool = get_db_pool().await;
    let (_, apub_config) = init_local_instance_and_get_apub_config(&db_pool).await;
    let apub_person = get_apub_person();
    let public_key = apub_person.public_key_pem();
    let person_json = apub_person.clone().into_json(&apub_config.to_request_data()).await.expect("Should get Person json");

    let expected_json = serde_json::json!({
        "type": "Person",
        "id": "https://mastodon.social/users/SphareDev",
        "preferredUsername": "SphareDev",
        "name": "Sphare",
        "inbox": "https://mastodon.social/users/SphareDev/inbox",
        "outbox": "https://mastodon.social/users/SphareDev/outbox",
        "publicKey": {
            "id": "https://mastodon.social/users/SphareDev#main-key",
            "owner": "https://mastodon.social/users/SphareDev",
            "publicKeyPem": public_key
        }
    });
    assert_eq!(
        serde_json::to_string(&person_json).expect("Should serialize Person json"),
        serde_json::to_string(&expected_json).expect("Should serialize expected_json")
    );
}

#[tokio::test]
async fn test_apub_person_object_verify() {
    let db_pool = get_db_pool().await;
    let (_, apub_config) = init_local_instance_and_get_apub_config(&db_pool).await;
    let apub_data = apub_config.to_request_data();
    let apub_person = get_apub_person();
    let valid_url = apub_person.apub_id.inner().clone();
    let person = apub_person.into_json(&apub_data).await.expect("Should get person");
    let invalid_url = Url::parse("https://sample.net/abc").expect("Should be valid url");
    assert!(ApubPerson::verify(&person, &valid_url, &apub_data).await.is_ok());
    assert!(ApubPerson::verify(&person, &invalid_url, &apub_data).await.is_err());
}

#[tokio::test]
async fn test_apub_person_object_from_json() {
    let db_pool = get_db_pool().await;
    let (_, apub_config) = init_local_instance_and_get_apub_config(&db_pool).await;
    let apub_data = apub_config.to_request_data();

    let actor_id_str = "https://mastodon.social/users/SphareDev";
    let actor_id = Url::parse(actor_id_str).expect("Should be valid url");

    assert!(get_person_by_actor_id(&actor_id, &db_pool).await.expect("Should get option").is_none());
    let person_json = r#"{
        "id": "https://mastodon.social/users/SphareDev",
        "type": "Person",
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
    let person = serde_json::from_str(person_json).expect("Should deserialize Person");

    let apub_person = ApubPerson::from_json(person, &apub_data).await.expect("Should get person");
    let db_person = get_person_by_actor_id(&actor_id, &db_pool).await.expect("Should get option").expect("Person should be some");
    assert_eq!(apub_person, db_person.try_into().expect("Should convert to ApubPerson"));

    let updated_person_json = r#"{
        "id": "https://mastodon.social/users/SphareDev",
        "type": "Person",
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
    let updated_person = serde_json::from_str(updated_person_json).expect("Should deserialize Person");

    let updated_apub_person = insert_or_update_person(&updated_person, &db_pool).await.expect("Should get person");
    let db_person = get_person_by_actor_id(&actor_id, &db_pool).await.expect("Should get option").expect("Person should be some");
    assert_eq!(updated_apub_person, db_person);
}