extern crate core;

use activitypub_federation::fetch::object_id::ObjectId;
use activitypub_federation::protocol::values::{MediaTypeMarkdown, MediaTypeMarkdownOrHtml};
use activitypub_federation::traits::{Object};
use url::Url;

use sphare_core_apub::group::ApubSphere;
use sphare_core_apub::page::{insert_or_update_post, ApubPost, Page, load_post_by_apub_id};
use sphare_core_apub::person::{get_person_by_username, ApubPerson};
use sphare_core_apub::utils::{Source};
use sphare_core_common::activity_pub::{AttributedTo};
use sphare_core_content::post::Post;
use sphare_core_content::post::ssr::{get_post_by_id, get_post_with_sphere_info_by_id};
use sphare_core_sphere::sphere::ssr::create_sphere;
use sphare_core_sphere::sphere_category::ssr::get_sphere_category_apub_id;
use crate::apub_factory::{mock_apub_group, mock_apub_group_moderators, mock_apub_persons};
use crate::common::{create_test_user, get_db_pool};
use crate::data_factory::create_sphere_with_post;
use crate::utils::{get_apub_post, get_updated_apub_post, init_local_instance_and_get_apub_config};

mod apub_factory;
mod apub_utils;
mod common;
mod data_factory;
mod utils;

fn check_apub_post_match_page(
    apub_post: &ApubPost,
    page: &Page,
) {
    let (expected_html_content, expected_markdown_content) = apub_post.get_html_and_markdown_content();
    let expected_plaintext_content = match apub_post.markdown_content {
        Some(_) => None,
        None => Some(apub_post.content.clone()),
    };
    assert_eq!(page.id, apub_post.apub_id);
    assert_eq!(page.attributed_to, AttributedTo::Forum(apub_post.person_id.inner().clone().into()));
    assert!(page.to.is_empty());
    assert_eq!(page.in_reply_to, None);
    assert_eq!(page.name, Some(apub_post.title.clone()));
    assert!(page.cc.is_empty());
    assert_eq!(page.content, Some(expected_html_content));
    assert_eq!(page.media_type, Some(MediaTypeMarkdownOrHtml::Html));
    assert_eq!(page.source, Some(Source {
        content: expected_markdown_content,
        media_type: MediaTypeMarkdown::Markdown,
    }));
    assert_eq!(page.plaintext_source, expected_plaintext_content);
    assert!(page.attachment.is_empty());
    assert_eq!(page.image, None);
    assert_eq!(page.sensitive, Some(apub_post.is_nsfw));
    assert_eq!(page.spoiler, Some(apub_post.is_spoiler));
    assert_eq!(page.published, None);
    assert_eq!(page.updated, apub_post.update_timestamp);
    assert_eq!(page.language, None);
    assert_eq!(page.audience, Some(apub_post.sphere_apub_id.clone()));
    assert!(page.tag.is_empty());
    assert_eq!(page.context, None);
}

fn check_apub_post_match_post(
    apub_post: &ApubPost,
    post: &Post,
) {
    assert_eq!(post.post_apub_id, apub_post.apub_id.to_string());
    assert_eq!(post.title, apub_post.title);
    assert_eq!(post.body, apub_post.content);
    assert_eq!(post.markdown_body, apub_post.markdown_content);
    assert_eq!(post.is_nsfw, apub_post.is_nsfw);
    assert_eq!(post.is_spoiler, apub_post.is_spoiler);
    assert_eq!(post.creator_apub_id, apub_post.person_id.to_string());
    assert_eq!(post.edit_timestamp, apub_post.update_timestamp);
}

#[tokio::test]
async fn test_apub_post_read_from_id() {
    let db_pool = get_db_pool().await;
    let (_, apub_config) = init_local_instance_and_get_apub_config(&db_pool).await;
    let apub_data = apub_config.to_request_data();

    let mut user = create_test_user(&db_pool).await;
    let (_, expected_post) = create_sphere_with_post("news", &mut user, &db_pool).await;
    let apub_post = ApubPost::read_from_id(
        Url::parse(&expected_post.post_apub_id).expect("Should parse apub ID"),
        &apub_data
    ).await.expect("Should load post").expect("Post should exist");
    check_apub_post_match_post(&apub_post, &expected_post);
}

#[tokio::test]
async fn test_apub_post_object_verify() {
    let db_pool = get_db_pool().await;
    let (_, apub_config) = init_local_instance_and_get_apub_config(&db_pool).await;
    let apub_data = apub_config.to_request_data();

    let user = create_test_user(&db_pool).await;
    let person = get_person_by_username(&user.username, &db_pool).await.expect("Person should be in DB.");
    let person_apub_id: ObjectId<ApubPerson> = Url::parse(&person.actor_id).expect("Person apub ID should be valid url").into();
    let mut apub_post = get_apub_post(&Url::parse("https://www.sphare.space/c/news").expect("Should get valid url"), &person_apub_id);
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
    let db_pool = get_db_pool().await;
    let (_, apub_config) = init_local_instance_and_get_apub_config(&db_pool).await;
    let apub_data = apub_config.to_request_data();

    let user = create_test_user(&db_pool).await;
    let person = get_person_by_username(&user.username, &db_pool).await.expect("Person should be in DB.");
    let person_apub_id: ObjectId<ApubPerson> = Url::parse(&person.actor_id).expect("Person apub ID should be valid url").into();

    let apub_post = get_apub_post(&Url::parse("https://www.sphare.space/c/news").expect("Should get valid url"), &person_apub_id);
    let page = apub_post.clone().into_json(&apub_data).await.expect("Should get page");
    check_apub_post_match_page(&apub_post, &page);

    let apub_post = get_updated_apub_post(&Url::parse("https://www.sphare.space/").expect("Should get valid url"), &person_apub_id);
    let page = apub_post.clone().into_json(&apub_data).await.expect("Should get page");
    check_apub_post_match_page(&apub_post, &page);
}

#[tokio::test]
async fn test_apub_post_from_json() {
    let db_pool = get_db_pool().await;
    let (_, apub_config) = init_local_instance_and_get_apub_config(&db_pool).await;
    let apub_data = apub_config.to_request_data();

    let mock_server = wiremock::MockServer::start().await; // binds 127.0.0.1:<ephemeral port>

    let group = mock_apub_group(&mock_server).await;

    let mod_name_vec = ["alice", "bob", "charles"];
    let mod_apub_id_vec: Vec<ObjectId<ApubPerson>> = mock_apub_persons(&mod_name_vec, &mock_server).await;

    mock_apub_group_moderators(&group.id, &mod_apub_id_vec[0..2], &mock_server).await;

    let author_apub_id = mock_apub_persons(&["dylan"], &mock_server).await[0].clone();

    let apub_post = get_apub_post(group.id.inner(), &author_apub_id);
    let page = apub_post.clone().into_json(&apub_data).await.expect("Should get page");

    let result_apub_post = ApubPost::from_json(page, &apub_data).await.expect("Should load page");
    assert_eq!(result_apub_post, apub_post);

    let post = load_post_by_apub_id(result_apub_post.apub_id.inner(), &db_pool).await.expect("Should load post").expect("Post should exist");
    check_apub_post_match_post(&apub_post, &post.post);
}

#[tokio::test]
async fn test_load_post_by_apub_id() {
    let db_pool = get_db_pool().await;
    let mut user = create_test_user(&db_pool).await;
    let (sphere, expected_post) = create_sphere_with_post("news", &mut user, &db_pool).await;
    let post = load_post_by_apub_id(
        &Url::parse(&expected_post.post_apub_id).expect("Should parse apub ID"),
        &db_pool
    ).await.expect("Should load post").expect("Post should exist");
    let post_with_sphere_info = get_post_with_sphere_info_by_id(expected_post.post_id, &db_pool).await.expect("Should load post with sphere info");
    assert_eq!(post.post, expected_post);
    assert_eq!(post.sphere_apub_id, sphere.sphere_apub_id);
    assert_eq!(
        post.category_apub_id,
        post_with_sphere_info.sphere_category.map(|category| {
            get_sphere_category_apub_id(&post_with_sphere_info.sphere_name, &category.category_name).expect("Should get sphere category apub ID").to_string()
        })
    );
}

#[tokio::test]
async fn test_insert_or_update_post() {
    let db_pool = get_db_pool().await;
    let (_, apub_config) = init_local_instance_and_get_apub_config(&db_pool).await;
    let apub_data = apub_config.to_request_data();

    let user = create_test_user(&db_pool).await;
    let person = get_person_by_username(&user.username, &db_pool).await.expect("Person should be in DB.");
    let person_apub_id: ObjectId<ApubPerson> = Url::parse(&person.actor_id).expect("Person apub ID should be valid url").into();
    let apub_person = person.try_into().expect("Should convert person to apub person");

    let sphere = create_sphere("sphere", "description", false, &user, &db_pool).await.expect("Should create sphere");
    let apub_sphere: ApubSphere = sphere.clone().try_into().expect("Should convert sphere to apub sphere");

    let mut apub_post = get_apub_post(apub_sphere.apub_id.inner(), &person_apub_id);
    let page = apub_post.clone().into_json(&apub_data).await.expect("Should get page");

    let inserted_post_with_apub_info = insert_or_update_post(&page, &apub_person, &apub_sphere, &db_pool).await.expect("Should insert or update page");
    let post = get_post_by_id(inserted_post_with_apub_info.post.post_id, &db_pool).await.expect("Post should be in DB.");

    assert_eq!(inserted_post_with_apub_info.post, post);
    check_apub_post_match_post(&apub_post, &post);
    assert_eq!(post.sphere_id, sphere.sphere_id);
    assert_eq!(inserted_post_with_apub_info.sphere_apub_id, apub_post.sphere_apub_id.to_string());
    assert_eq!(
        inserted_post_with_apub_info.category_apub_id,
        page.get_sphere_category().expect("Should get category").map(|category| category.category_apub_id.to_string())
    );

    apub_post.content = String::from("Updated content");
    let page = apub_post.clone().into_json(&apub_data).await.expect("Should get page");

    let inserted_post_with_apub_info = insert_or_update_post(&page, &apub_person, &apub_sphere, &db_pool).await.expect("Should insert or update page");
    let post = get_post_by_id(inserted_post_with_apub_info.post.post_id, &db_pool).await.expect("Post should be in DB.");

    assert_eq!(inserted_post_with_apub_info.post, post);
    check_apub_post_match_post(&apub_post, &post);
    assert_eq!(post.sphere_id, sphere.sphere_id);
    assert_eq!(inserted_post_with_apub_info.sphere_apub_id, apub_post.sphere_apub_id.to_string());
    assert_eq!(
        inserted_post_with_apub_info.category_apub_id,
        page.get_sphere_category().expect("Should get category").map(|category| category.category_apub_id.to_string())
    );
}