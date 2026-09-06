use sphare_core_apub::community_moderator::handle_community_moderators;
use sphare_core_sphere::sphere::ssr::create_sphere;
use sphare_core_user::role::ssr::get_sphere_role_vec;
use url::Url;

use crate::common::{create_user_and_get_person, get_db_pool};
use crate::utils::init_local_instance_and_get_apub_config;

mod common;
mod data_factory;
mod utils;

#[tokio::test]
async fn test_handle_community_moderators() {
    let db_pool = get_db_pool().await;
    let (_, apub_config) = init_local_instance_and_get_apub_config(&db_pool).await;

    let (user_1, person_1) = create_user_and_get_person("1", &db_pool).await;
    let (_, person_2) = create_user_and_get_person("2", &db_pool).await;
    let (_, person_3) = create_user_and_get_person("3", &db_pool).await;

    let moderator_vec = vec![person_1.actor_id.clone(), person_2.actor_id.clone()].into_iter().map(|actor_id| Url::parse(&actor_id).expect("Should parse actor_id").into()).collect();

    let sphere = create_sphere("a", "sphere", false, &user_1, &db_pool).await.expect("Should create sphere");
    let apub_sphere = sphere.clone().try_into().expect("Should convert sphere to apub_sphere");

    handle_community_moderators(&moderator_vec, &apub_sphere, &apub_config.to_request_data()).await.expect("Should add community moderators");

    let result_sphere_moderators = get_sphere_role_vec(&sphere.sphere_name, &db_pool).await.expect("Should get sphere moderators");

    assert_eq!(result_sphere_moderators.len(), 2);
    assert!(result_sphere_moderators.iter().find(|moderator| moderator.person_id == person_1.person_id).is_some());
    assert!(result_sphere_moderators.iter().find(|moderator| moderator.person_id == person_2.person_id).is_some());

    let moderator_vec = vec![person_1.actor_id, person_2.actor_id, person_3.actor_id].into_iter().map(|actor_id| Url::parse(&actor_id).expect("Should parse actor_id").into()).collect();

    handle_community_moderators(&moderator_vec, &apub_sphere, &apub_config.to_request_data()).await.expect("Should add community moderators");

    let result_sphere_moderators = get_sphere_role_vec(&sphere.sphere_name, &db_pool).await.expect("Should get sphere moderators");

    assert_eq!(result_sphere_moderators.len(), 3);
    assert!(result_sphere_moderators.iter().find(|moderator| moderator.person_id == person_1.person_id).is_some());
    assert!(result_sphere_moderators.iter().find(|moderator| moderator.person_id == person_2.person_id).is_some());
    assert!(result_sphere_moderators.iter().find(|moderator| moderator.person_id == person_3.person_id).is_some());

}