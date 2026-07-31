use sphare_core_common::routes::get_profile_link;
use sphare_core_apub::person::get_person_by_username;
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
