use activitypub_federation::fetch::object_id::ObjectId;
use activitypub_federation::traits::{Collection, Object};
use leptos::serde_json::json;
use sphare_core_apub::community_moderator::{handle_community_moderators, ApubCommunityModerators, GroupModerators};
use sphare_core_sphere::sphere::ssr::create_sphere;
use sphare_core_user::role::ssr::get_sphere_role_vec;
use url::Url;
use wiremock::{Mock, MockServer, ResponseTemplate};
use wiremock::matchers::{method, path};
use sphare_core_apub::group::{insert_or_update_sphere};
use sphare_core_apub::person::{get_person_by_actor_id, ApubPerson, DbPerson};
use sphare_core_common::activity_pub::generate_rsa_keys_pem;
use sphare_core_sphere::sphere::Sphere;
use sphare_core_user::role::{PermissionLevel, UserSphereRole};
use sphare_core_user::user::ssr::get_admin_function_user;
use crate::common::{create_user_and_get_person, get_db_pool};
use crate::utils::{get_mocked_apub_sphere, init_local_instance_and_get_apub_config};

mod common;
mod data_factory;
mod utils;

fn test_apub_role(
    person: &DbPerson,
    sphere: &Sphere,
    user_sphere_role_vec: &[UserSphereRole],
) {
    let role = user_sphere_role_vec.iter().find(|moderator| moderator.person_id == person.person_id).expect("Should find person");
    assert_eq!(role.sphere_id, sphere.sphere_id);
    assert_eq!(role.sphere_name, sphere.sphere_name);
    assert_eq!(role.username, person.username);
    assert_eq!(role.permission_level, PermissionLevel::Manage);
    assert!(role.delete_timestamp.is_none());

}

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
    test_apub_role(&person_1, &sphere, &result_sphere_moderators);
    test_apub_role(&person_2, &sphere, &result_sphere_moderators);

    let moderator_vec = vec![person_2.actor_id.clone(), person_3.actor_id.clone()].into_iter().map(|actor_id| Url::parse(&actor_id).expect("Should parse actor_id").into()).collect();

    handle_community_moderators(&moderator_vec, &apub_sphere, &apub_config.to_request_data()).await.expect("Should add community moderators");

    let result_sphere_moderators = get_sphere_role_vec(&sphere.sphere_name, &db_pool).await.expect("Should get sphere moderators");

    assert_eq!(result_sphere_moderators.len(), 2);
    test_apub_role(&person_2, &sphere, &result_sphere_moderators);
    test_apub_role(&person_3, &sphere, &result_sphere_moderators);
}

#[tokio::test]
async fn test_community_moderators_from_json() {
    let db_pool = get_db_pool().await;
    let (_, apub_config) = init_local_instance_and_get_apub_config(&db_pool).await;
    let apub_data = apub_config.to_request_data();

    let mock_server = MockServer::start().await;
    let mock_server_url = Url::parse(&format!("http://localhost:{}", mock_server.address().port())).expect("Mock server uri should be valid");

    let actor_name_vec = vec!["alice", "bob"];
    let actor_id_vec: Vec<ObjectId<ApubPerson>> = actor_name_vec
        .iter()
        .map(|name| mock_server_url.join(&format!("/actors/{}", name)).expect("Should join actor path").into())
        .collect();

    println!("Actor id vec: {actor_id_vec:?}");

    for (name, actor_id) in actor_name_vec.iter().zip(actor_id_vec.iter()) {
        let (pub_key_pem, _priv_key_pem) = generate_rsa_keys_pem().expect("Should get keys");
        let actor_json = json!({
            "id": actor_id.inner().to_string(),
            "type": "Person",
            "preferredUsername": name,
            "inbox": format!("{}/inbox", actor_id),
            "outbox": format!("{}/outbox", actor_id),
            "publicKey": {
                "id": format!("{}/#main-key", actor_id),
                "owner": actor_id,
                "publicKeyPem": pub_key_pem,
            },
        });

        Mock::given(method("GET"))
            .and(path(format!("/actors/{}", name)))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_raw(actor_json.to_string(), "application/activity+json")
            )
            .mount(&mock_server)
            .await;
    }

    let group_moderators = GroupModerators {
        r#type: Default::default(),
        apub_id: mock_server_url,
        ordered_items: actor_id_vec.clone(),
    };

    let apub_sphere = get_mocked_apub_sphere(&mock_server.uri());
    let group = apub_sphere.clone().into_json(&apub_data).await.expect("Should get group from apub_sphere");
    let function_user = get_admin_function_user(&db_pool).await.expect("Should get function user");
    let sphere = insert_or_update_sphere(&group, &function_user, &db_pool).await.expect("Should get Sphere");

    ApubCommunityModerators::from_json(group_moderators, &apub_sphere, &apub_data).await.expect("Should load moderators from json");

    let result_sphere_moderators = get_sphere_role_vec(&sphere.sphere_name, &db_pool).await.expect("Should get sphere moderators");
    assert_eq!(result_sphere_moderators.len(), 2);

    // Test persons and corresponding roles were created
    for (username, person_apub_id) in actor_name_vec.iter().zip(actor_id_vec.iter()) {
        let person = get_person_by_actor_id(person_apub_id.inner(), &db_pool).await.expect("Should get optional").expect("Should get person");
        assert_eq!(person.actor_id, person_apub_id.inner().to_string());
        assert_eq!(person.username, *username);

        test_apub_role(&person, &sphere, &result_sphere_moderators);
    }
}