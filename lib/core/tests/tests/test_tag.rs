use url::Url;
use sphare_core_apub::tag::{identify_missing_apub_community_tags, load_group_categories};
use sphare_core_common::colors::Color;
use sphare_core_sphere::sphere::ssr::create_sphere;
use sphare_core_sphere::sphere_category::ssr::set_sphere_category;
use sphare_core_user::user::User;
use crate::apub_factory::get_apub_community_tag;
use crate::apub_utils::test_sphere_category_vec;
use crate::common::{create_test_user, get_db_pool};
use crate::utils::{init_local_instance_and_get_apub_config};

mod apub_factory;
mod apub_utils;
mod common;
mod data_factory;
mod utils;

#[tokio::test]
async fn test_load_group_categories() {
    let db_pool = get_db_pool().await;
    let (_, apub_config) = init_local_instance_and_get_apub_config(&db_pool).await;
    let apub_data = apub_config.to_request_data();

    let user = create_test_user(&db_pool).await;
    let sphere = create_sphere("a", "b", false, &user, &db_pool).await.expect("Should create sphere");
    let sphere_apub_id = Url::parse(&sphere.sphere_apub_id).expect("Should parse sphere apub id");
    let group_categories = vec![
        get_apub_community_tag("news", "For fresh content", &sphere_apub_id),
        get_apub_community_tag("funny", "In the mood for a chuckle?", &sphere_apub_id),
    ];

    load_group_categories(
        &sphere_apub_id,
        &group_categories,
        &apub_data,
    ).await.expect("Should load group categories");

    test_sphere_category_vec(&sphere, &group_categories, &db_pool).await;
}

#[tokio::test]
async fn test_identify_missing_apub_community_tags() {
    let db_pool = get_db_pool().await;
    let (_, apub_config) = init_local_instance_and_get_apub_config(&db_pool).await;
    let apub_data = apub_config.to_request_data();

    let user = create_test_user(&db_pool).await;
    let sphere = create_sphere("a", "b", false, &user, &db_pool).await.expect("Should create sphere");
    let sphere_apub_id = Url::parse(&sphere.sphere_apub_id).expect("Should parse sphere apub id");

    // Refresh user for permissions
    let user = User::get(user.user_id, &db_pool).await.expect("Should reload user");
    let existing_category = set_sphere_category(
        &sphere.sphere_name,
        "news",
        Color::Blue,
        "For fresh content",
        true,
        &user,
        &db_pool,
    ).await.expect("Should set category");
    let missing_apub_community_tag = get_apub_community_tag("funny", "In the mood for a chuckle?", &sphere_apub_id);
    let group_categories = vec![
        get_apub_community_tag(&existing_category.category_name, &existing_category.description, &sphere_apub_id),
        missing_apub_community_tag,
    ];

    let missing_category_vec = identify_missing_apub_community_tags(
        &group_categories,
        &sphere.into(),
        &db_pool,
    ).await.expect("Should identify missing categories");
    assert_eq!(missing_category_vec.len(), 1);
    assert_eq!(missing_category_vec[0], &missing_apub_community_tag);
}