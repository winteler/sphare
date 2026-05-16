use sphare_core_common::errors::AppError;
use sphare_core_user::user::User;

use sphare_core_sphere::satellite::ssr::{activate_satellite, create_satellite, deactivate_satellite, get_satellite_sphere, update_satellite};
use sphare_core_sphere::satellite::ssr::{get_satellite_vec_by_sphere_name, get_satellite_by_id};
use sphare_core_sphere::sphere::ssr::create_sphere;

pub use crate::common::*;
pub use crate::data_factory::*;

mod common;
mod data_factory;

#[tokio::test]
async fn test_get_satellite_by_id() -> Result<(), AppError> {
    let db_pool = get_db_pool().await;
    let mut user = create_test_user(&db_pool).await;

    let (_, satellite_vec) = create_sphere_with_satellite_vec(
        "1",
        2,
        &mut user,
        &db_pool,
    ).await.expect("Error creating sphere and satellites");

    let expected_satellite_1 = satellite_vec.first().expect("Should have satellite 1");
    let expected_satellite_2 = satellite_vec.get(1).expect("Should have satellite 2");

    let (_, expected_satellite_3) = create_sphere_with_satellite(
        "2",
        "3",
        true,
        true,
        &mut user,
        &db_pool,
    ).await.expect("Error creating sphere and satellite 3");
    
    let satellite_1 = get_satellite_by_id(expected_satellite_1.satellite_id, &db_pool).await.expect("Error getting satellite 1");
    let satellite_2 = get_satellite_by_id(expected_satellite_2.satellite_id, &db_pool).await.expect("Error getting satellite 2");
    let satellite_3 = get_satellite_by_id(expected_satellite_3.satellite_id, &db_pool).await.expect("Error getting satellite 3");

    assert_eq!(satellite_1, *expected_satellite_1);
    assert_eq!(satellite_2, *expected_satellite_2);
    assert_eq!(satellite_3, expected_satellite_3);

    Ok(())
}

#[tokio::test]
async fn test_get_satellite_vec_by_sphere_name() -> Result<(), AppError> {
    let db_pool = get_db_pool().await;
    let mut user = create_test_user(&db_pool).await;

    let (sphere, mut expected_satellite_vec) = create_sphere_with_satellite_vec(
        "sphere",
        5,
        &mut user,
        &db_pool,
    ).await.expect("Error creating sphere and satellites");

    let satellite_vec = get_satellite_vec_by_sphere_name(
        &sphere.sphere_name,
        false,
        &db_pool
    ).await.expect("Satellite vec should be loaded");
    assert_eq!(satellite_vec, expected_satellite_vec);

    let satellite_vec = get_satellite_vec_by_sphere_name(
        &sphere.sphere_name,
        true,
        &db_pool
    ).await.expect("Satellite vec should be loaded");
    assert_eq!(satellite_vec, expected_satellite_vec);

    let deactivated_satellite = expected_satellite_vec.pop().expect("Should pop satellite");
    
    deactivate_satellite(deactivated_satellite.satellite_id, &user, &db_pool).await.expect("Should disable satellite");

    let satellite_vec = get_satellite_vec_by_sphere_name(
        &sphere.sphere_name,
        false,
        &db_pool
    ).await.expect("Satellite vec should be loaded");
    assert_eq!(satellite_vec, expected_satellite_vec);

    let satellite_vec = get_satellite_vec_by_sphere_name(
        &sphere.sphere_name,
        false,
        &db_pool
    ).await.expect("Satellite vec should be loaded");
    assert_eq!(satellite_vec, expected_satellite_vec);

    let deactivated_satellite = get_satellite_by_id(deactivated_satellite.satellite_id, &db_pool).await.expect("Disabled satellite should be loaded");
    expected_satellite_vec.push(deactivated_satellite);
    let satellite_vec = get_satellite_vec_by_sphere_name(
        &sphere.sphere_name,
        true,
        &db_pool
    ).await.expect("Satellite vec should be loaded");
    assert_eq!(satellite_vec, expected_satellite_vec);

    Ok(())
}

#[tokio::test]
async fn test_get_satellite_sphere() -> Result<(), AppError> {
    let db_pool = get_db_pool().await;
    let mut user = create_test_user(&db_pool).await;

    let (expected_sphere, expected_satellite_vec) = create_sphere_with_satellite_vec(
        "sphere",
        1,
        &mut user,
        &db_pool,
    ).await.expect("Error creating sphere and satellites");
    
    let sphere = get_satellite_sphere(
        expected_satellite_vec.first().expect("Satellite should exist").satellite_id,
        &db_pool,
    ).await.expect("Error getting satellite");
    
    assert_eq!(sphere, expected_sphere);
    
    Ok(())
}

#[tokio::test]
async fn test_create_satellite() -> Result<(), AppError> {
    let db_pool = get_db_pool().await;
    let user = create_test_user(&db_pool).await;

    let sphere = create_sphere(
        "a",
        "a",
        false,
        &user,
        &db_pool,
    ).await.expect("Sphere should be created");

    let nsfw_sphere = create_sphere(
        "b",
        "b",
        true,
        &user,
        &db_pool,
    ).await.expect("Nsfw sphere should be created");


    assert_eq!(
        create_satellite("1", &sphere.sphere_name, "1", false, false, false, &user, &db_pool).await,
        Err(AppError::InsufficientPrivileges)
    );

    let user = User::get(user.user_id, &db_pool).await.expect("Should reload user");

    let satellite_1 = create_satellite(
        &sphere.sphere_name,
        "1",
        "1",
        true,
        false,
        true,
        &user,
        &db_pool
    ).await.expect("Satellite 1 should be created");

    assert_eq!(satellite_1.satellite_name, "1");
    assert_eq!(satellite_1.body, "<p class=\"mb-2.5\">1</p>");
    assert_eq!(satellite_1.markdown_body.as_deref(), Some("1"));
    assert_eq!(satellite_1.is_nsfw, false);
    assert_eq!(satellite_1.is_spoiler, true);
    assert_eq!(satellite_1.disable_timestamp, None);

    let satellite_2 = create_satellite(
        &sphere.sphere_name,
        "2",
        "2",
        false,
        true,
        false,
        &user,
        &db_pool
    ).await.expect("Satellite 2 should be created");

    assert_eq!(satellite_2.satellite_name, "2");
    assert_eq!(satellite_2.body, "2");
    assert_eq!(satellite_2.markdown_body, None);
    assert_eq!(satellite_2.is_nsfw, true);
    assert_eq!(satellite_2.is_spoiler, false);
    assert_eq!(satellite_2.disable_timestamp, None);

    let nsfw_satellite = create_satellite(
        &nsfw_sphere.sphere_name,
        "3",
        "3",
        false,
        false,
        false,
        &user,
        &db_pool
    ).await.expect("Nsfw satellite should be created");

    assert_eq!(nsfw_satellite.satellite_name, "3");
    assert_eq!(nsfw_satellite.body, "3");
    assert_eq!(nsfw_satellite.markdown_body, None);
    assert_eq!(nsfw_satellite.is_nsfw, true);
    assert_eq!(nsfw_satellite.is_spoiler, false);
    assert_eq!(nsfw_satellite.disable_timestamp, None);

    Ok(())
}

#[tokio::test]
async fn test_update_satellite() -> Result<(), AppError> {
    let db_pool = get_db_pool().await;
    let mut user = create_test_user(&db_pool).await;
    let base_user = create_user("a", &db_pool).await;

    let (_, satellite_1) = create_sphere_with_satellite(
        "1",
        "1",
        true,
        false,
        &mut user,
        &db_pool,
    ).await.expect("Sphere with satellite should be created");

    let nsfw_sphere = create_sphere(
        "2",
        "2",
        true,
        &mut user,
        &db_pool,
    ).await.expect("Nsfw sphere should be created");

    let user = User::get(user.user_id, &db_pool).await.expect("Should reload user");

    let nsfw_satellite = create_satellite(
        &nsfw_sphere.sphere_name,
        "2",
        "2",
        true,
        true,
        true,
        &user,
        &db_pool
    ).await.expect("Nsfw satellite should be created");

    assert_eq!(
        update_satellite(satellite_1.satellite_id, "a", "error", false, false, true, &base_user, &db_pool).await,
        Err(AppError::InsufficientPrivileges),
    );

    let updated_satellite_1 = update_satellite(
        satellite_1.satellite_id,
        "a",
        "a",
        true,
        false,
        true,
        &user,
        &db_pool
    ).await.expect("Satellite 1 should be updated");

    assert_eq!(updated_satellite_1.satellite_id, satellite_1.satellite_id);
    assert_eq!(updated_satellite_1.satellite_name, "a");
    assert_eq!(updated_satellite_1.body, "<p class=\"mb-2.5\">a</p>");
    assert_eq!(updated_satellite_1.markdown_body.as_deref(), Some("a"));
    assert_eq!(updated_satellite_1.is_nsfw, false);
    assert_eq!(updated_satellite_1.is_spoiler, true);
    assert_eq!(updated_satellite_1.disable_timestamp, None);

    let updated_nsfw_satellite = update_satellite(
        nsfw_satellite.satellite_id,
        "b",
        "b",
        false,
        false,
        false,
        &user,
        &db_pool
    ).await.expect("Nsfw satellite should be updated");

    assert_eq!(updated_nsfw_satellite.satellite_id, nsfw_satellite.satellite_id);
    assert_eq!(updated_nsfw_satellite.satellite_name, "b");
    assert_eq!(updated_nsfw_satellite.body, "b");
    assert_eq!(updated_nsfw_satellite.markdown_body, None);
    assert_eq!(updated_nsfw_satellite.is_nsfw, true);
    assert_eq!(updated_nsfw_satellite.is_spoiler, false);
    assert_eq!(updated_nsfw_satellite.disable_timestamp, None);

    Ok(())
}

#[tokio::test]
async fn test_activate_satellite() -> Result<(), AppError> {
    let db_pool = get_db_pool().await;
    let mut user = create_test_user(&db_pool).await;

    let (sphere, satellite) = create_sphere_with_satellite(
        "1",
        "1",
        false,
        false,
        &mut user,
        &db_pool,
    ).await.expect("Sphere with satellite should be created");

    let activated_satellite = activate_satellite(satellite.satellite_id, &user, &db_pool).await.expect("Should activate satellite without error");
    assert_eq!(satellite, activated_satellite);

    let deactivated_satellite = deactivate_satellite(satellite.satellite_id, &user, &db_pool).await.expect("Satellite should be disabled");

    assert_eq!(deactivated_satellite.satellite_id, satellite.satellite_id);
    assert_eq!(deactivated_satellite.sphere_id, sphere.sphere_id);
    assert_eq!(deactivated_satellite.satellite_name, "1");
    assert!(deactivated_satellite.disable_timestamp.is_some_and(|delete_timestamp| delete_timestamp > deactivated_satellite.timestamp));

    let reactivated_satellite = activate_satellite(satellite.satellite_id, &user, &db_pool).await.expect("Satellite should be reactivated");

    assert_eq!(reactivated_satellite.satellite_id, satellite.satellite_id);
    assert_eq!(reactivated_satellite.sphere_id, sphere.sphere_id);
    assert_eq!(reactivated_satellite.satellite_name, "1");
    assert!(reactivated_satellite.disable_timestamp.is_none());

    let satellite_vec = get_satellite_vec_by_sphere_name(
        &sphere.sphere_name,
        false,
        &db_pool,
    ).await.expect("Should get sphere satellite vec");

    assert_eq!(satellite_vec.len(), 1);
    assert_eq!(satellite_vec.first(), Some(&reactivated_satellite));

    Ok(())
}

#[tokio::test]
async fn test_deactivate_satellite() -> Result<(), AppError> {
    let db_pool = get_db_pool().await;
    let mut user = create_test_user(&db_pool).await;

    let (sphere, satellite) = create_sphere_with_satellite(
        "1",
        "1",
        false,
        false,
        &mut user,
        &db_pool,
    ).await.expect("Sphere with satellite should be created");

    let deactivated_satellite = deactivate_satellite(satellite.satellite_id, &user, &db_pool).await.expect("Satellite should be disabled");

    assert_eq!(deactivated_satellite.satellite_name, "1");
    assert_eq!(deactivated_satellite.body, "test");
    assert_eq!(deactivated_satellite.is_nsfw, false);
    assert_eq!(deactivated_satellite.is_spoiler, false);
    assert!(deactivated_satellite.disable_timestamp.is_some_and(|delete_timestamp| delete_timestamp > deactivated_satellite.timestamp));

    let satellite_vec = get_satellite_vec_by_sphere_name(
        &sphere.sphere_name,
        false,
        &db_pool,
    ).await.expect("Should get sphere satellite vec");

    assert!(satellite_vec.is_empty());

    Ok(())
}