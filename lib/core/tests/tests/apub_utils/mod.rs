#![allow(dead_code)]

use activitypub_federation::fetch::object_id::ObjectId;
use sqlx::PgPool;

use sphare_core_apub::person::{get_person_by_actor_id, DbPerson, ApubPerson};
use sphare_core_apub::tag::ApubCommunityTag;
use sphare_core_sphere::sphere::Sphere;
use sphare_core_sphere::sphere_category::ssr::get_sphere_category_vec;
use sphare_core_user::role::{PermissionLevel, UserSphereRole};
use sphare_core_user::role::ssr::get_sphere_role_vec;

pub fn test_moderator(
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

pub async fn test_sphere_role_vec(
    sphere: &Sphere,
    expected_mod_names: &[&str],
    expected_mod_apub_id: &[ObjectId<ApubPerson>],
    db_pool: &PgPool,
) {
    let sphere_mod_vec = get_sphere_role_vec(&sphere.sphere_name, db_pool).await.expect("Should get sphere moderators");
    assert_eq!(sphere_mod_vec.len(), expected_mod_names.len());

    // Test persons and corresponding roles were created
    for (username, person_apub_id) in expected_mod_names.iter().zip(expected_mod_apub_id.iter()) {
        println!("Testing moderator {} with id {}", username, person_apub_id.inner());
        let person = get_person_by_actor_id(person_apub_id.inner(), db_pool).await.expect("Should get optional").expect("Should get person");
        assert_eq!(person.actor_id, person_apub_id.inner().to_string());
        assert_eq!(person.username, *username);

        test_moderator(&person, sphere, &sphere_mod_vec);
    }
}

pub async fn test_sphere_category_vec(
    sphere: &Sphere,
    expected_sphere_categories: &[ApubCommunityTag],
    db_pool: &PgPool,
) {
    println!("Testing sphere categories {expected_sphere_categories:?}");
    let sphere_category_vec = get_sphere_category_vec(&sphere.sphere_name, db_pool).await.expect("Should get sphere categories");
    assert_eq!(sphere_category_vec.len(), expected_sphere_categories.len());

    // Test persons and corresponding roles were created
    for expected_category in expected_sphere_categories {
        let sphere_category = sphere_category_vec.iter().find(
            |category| category.category_apub_id == expected_category.id.to_string()
        ).expect("Should find sphere category");
        println!("Testing sphere category {} with apub id {}", sphere_category.category_name, sphere_category.category_apub_id);
        assert_eq!(sphere_category.category_apub_id, expected_category.id.to_string());
        assert_eq!(sphere_category.category_name, expected_category.name);
        assert_eq!(sphere_category.description, expected_category.content.clone().unwrap_or_default());
        assert_eq!(sphere_category.category_color, expected_category.color.unwrap_or_default());
    }
}