use activitypub_federation::fetch::object_id::ObjectId;
use activitypub_federation::traits::{Object};
use url::Url;
use sphare_core_apub::page::ApubPost;
use sphare_core_apub::person::{get_person_by_username, ApubPerson};
use crate::common::{create_test_user, get_db_pool};
use crate::utils::{get_apub_post, init_local_instance_and_get_apub_config};

mod apub_factory;
mod apub_utils;
mod common;
mod data_factory;
mod utils;
#[tokio::test]
async fn test_apub_post_object_verify() {
    let db_pool = get_db_pool().await;
    let (_, apub_config) = init_local_instance_and_get_apub_config(&db_pool).await;
    let apub_data = apub_config.to_request_data();

    let user = create_test_user(&db_pool).await;
    let person = get_person_by_username(&user.username, &db_pool).await.expect("Person should be in DB.");
    let person_apub_id: ObjectId<ApubPerson> = Url::parse(&person.actor_id).expect("Person apub ID should be valid url").into();
    let mut apub_post = get_apub_post(&Url::parse("https://www.sphare.space/").expect("Should get valid url"), &person_apub_id);
    let page = apub_post.clone().into_json(&apub_data).await.expect("Should get page");

    let valid_url = apub_post.apub_id.clone().inner().join("/").expect("Should get valid url");
    let invalid_url = Url::parse("https://sample.net/abc").expect("Should be valid url");
    assert_eq!(ApubPost::verify(&page, &valid_url, &apub_data).await, Ok(()));
    assert!(ApubPost::verify(&page, &invalid_url, &apub_data).await.is_err());

    apub_post.title = String::new();
    let missing_title_page = apub_post.into_json(&apub_data).await.expect("Should get page");
    assert!(ApubPost::verify(&missing_title_page, &valid_url, &apub_data).await.is_err());
}

#[tokio::test]
async fn test_apub_post_into_json() {

}