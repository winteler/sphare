use activitypub_federation::traits::Object;
use url::Url;
use sphare_core_apub::tag::load_group_categories;
use sphare_core_sphere::sphere::ssr::create_sphere;
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