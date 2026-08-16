use leptos::serde_json;
use sphare_core_common::routes::get_profile_link;
use sphare_core_apub::person::{get_person_by_actor_id, get_person_by_username, insert_or_update_person};
use sphare_core_user::user::User;
use crate::common::{create_test_user, get_db_pool};

mod common;
mod data_factory;

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
}

#[tokio::test]
async fn test_insert_or_update_person() {
    let db_pool = get_db_pool().await;
    let person_json = r#"{
        "id": "https://mastodon.social/users/LemmyDev",
        "type": "Person",
        "preferredUsername": "LemmyDev",
        "name": "Lemmy",
        "inbox": "https://mastodon.social/users/LemmyDev/inbox",
        "outbox": "https://mastodon.social/users/LemmyDev/outbox",
        "publicKey": {
            "id": "https://mastodon.social/users/LemmyDev#main-key",
            "owner": "https://mastodon.social/users/LemmyDev",
            "publicKeyPem": "12345"
        }
    }"#;
    let person = serde_json::from_str(person_json).expect("Should deserialize Person");

    let db_person = insert_or_update_person(person, &db_pool).await.expect("Should get person");
    assert_eq!(db_person.username, "LemmyDev");
    assert_eq!(db_person.display_name.as_deref(), Some("Lemmy"));
    assert_eq!(db_person.is_nsfw, false);
    assert_eq!(db_person.is_local, false);
    assert_eq!(db_person.actor_id, "https://mastodon.social/users/LemmyDev");
    assert_eq!(db_person.inbox, "https://mastodon.social/users/LemmyDev/inbox");
    assert_eq!(db_person.outbox, "https://mastodon.social/users/LemmyDev/outbox");
    assert_eq!(db_person.public_key, "12345");
    assert!(db_person.delete_timestamp.is_none());
}