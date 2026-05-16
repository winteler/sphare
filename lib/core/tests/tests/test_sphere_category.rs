use sphare_core_common::colors::Color;
use sphare_core_common::errors::AppError;
use sphare_core_content::embed::Link;
use sphare_core_content::post::ssr::create_post;
use sphare_core_content::post::PostTags;
use sphare_core_sphere::sphere::ssr::create_sphere;
use sphare_core_sphere::sphere_category::ssr::get_sphere_category_vec;
use sphare_core_sphere::sphere_category::ssr::{delete_sphere_category, set_sphere_category, CATEGORY_NOT_DELETED_STR};
use sphare_core_user::user::User;

use crate::common::{create_user, get_db_pool};

mod common;
mod data_factory;

#[tokio::test]
async fn test_get_sphere_category_vec() -> Result<(), AppError> {
    let db_pool = get_db_pool().await;
    let user = create_user("test", &db_pool).await;
    let sphere_1 = create_sphere("1", "1", false, &user, &db_pool).await?;
    let sphere_2 = create_sphere("2", "2", false, &user, &db_pool).await?;
    let user = User::get(user.user_id, &db_pool).await.expect("User should be loaded after sphere creation");

    let sphere_1_category_1 = set_sphere_category(
        &sphere_1.sphere_name,
        "1",
        Color::Black,
        "1",
        false,
        &user,
        &db_pool
    ).await.expect("Category 1 should be added.");

    let sphere_1_category_1_updated = set_sphere_category(
        &sphere_1.sphere_name,
        &sphere_1_category_1.category_name,
        Color::Black,
        "updated",
        true,
        &user,
        &db_pool
    ).await.expect("Category 1 should be added.");

    let sphere_1_category_2 = set_sphere_category(
        &sphere_1.sphere_name,
        "2",
        Color::Black,
        "2",
        true,
        &user,
        &db_pool
    ).await.expect("Category 2 should be added.");

    let sphere_1_category_off = set_sphere_category(
        &sphere_1.sphere_name,
        "0",
        Color::Black,
        "0",
        false,
        &user,
        &db_pool
    ).await.expect("Category off should be added.");

    let sphere_2_category_1 = set_sphere_category(
        &sphere_2.sphere_name,
        "1",
        Color::Black,
        "1",
        true,
        &user,
        &db_pool
    ).await.expect("Category 1 should be added.");

    let sphere_1_category_vec = get_sphere_category_vec(
        &sphere_1.sphere_name,
        &db_pool
    ).await.expect("Should load sphere categories");
    let sphere_2_category_vec = get_sphere_category_vec(
        &sphere_2.sphere_name,
        &db_pool
    ).await?;

    assert_eq!(sphere_1_category_vec.len(), 3);
    assert_eq!(sphere_1_category_vec.first(), Some(&sphere_1_category_1_updated));
    assert_eq!(sphere_1_category_vec.get(1), Some(&sphere_1_category_2));
    assert_eq!(sphere_1_category_vec.get(2), Some(&sphere_1_category_off));
    assert_eq!(sphere_2_category_vec.len(), 1);
    assert_eq!(sphere_2_category_vec.first(), Some(&sphere_2_category_1));

    Ok(())
}

#[tokio::test]
async fn test_set_sphere_category() -> Result<(), AppError> {
    let db_pool = get_db_pool().await;
    let user = create_user("test", &db_pool).await;
    let sphere = create_sphere("sphere", "a", false, &user, &db_pool).await?;
    let sphere_2 = create_sphere("sphere_2", "a", false, &user, &db_pool).await?;

    let category_name = "a";
    let description = "b";

    // Cannot create two categories with the same name in one sphere
    assert_eq!(
        set_sphere_category(
            &sphere.sphere_name,
            category_name,
            Color::Black,
            description,
            true,
            &user,
            &db_pool
        ).await,
        Err(AppError::InsufficientPrivileges)
    );

    let user = User::get(user.user_id, &db_pool).await.expect("User should be loaded after sphere creation");

    let sphere_category = set_sphere_category(
        &sphere.sphere_name,
        category_name,
        Color::Black,
        description,
        true,
        &user,
        &db_pool
    ).await.expect("Category should be added.");

    assert_eq!(sphere_category.sphere_id, sphere.sphere_id);
    assert_eq!(sphere_category.category_name, category_name);
    assert_eq!(sphere_category.description, description);
    assert_eq!(sphere_category.creator_id, user.user_id);
    assert!(sphere_category.is_active);
    assert_eq!(sphere_category.delete_timestamp, None);

    let updated_description = "c";
    let updated_category = set_sphere_category(
        &sphere.sphere_name,
        category_name,
        Color::Black,
        updated_description,
        false,
        &user,
        &db_pool
    ).await.expect("Category should be updated.");

    assert_eq!(updated_category.sphere_id, sphere.sphere_id);
    assert_eq!(updated_category.category_name, category_name);
    assert_eq!(updated_category.description, updated_description);
    assert_eq!(updated_category.creator_id, user.user_id);
    assert!(!updated_category.is_active);
    assert_eq!(updated_category.delete_timestamp, None);

    // Can create a category with the same name for a different sphere
    let sphere_2_category = set_sphere_category(
        &sphere_2.sphere_name,
        category_name,
        Color::Black,
        description,
        false,
        &user,
        &db_pool
    ).await.expect("Category should be added.");

    assert_eq!(sphere_2_category.sphere_id, sphere_2.sphere_id);
    assert_eq!(sphere_2_category.category_name, category_name);
    assert_eq!(sphere_2_category.description, description);
    assert_eq!(sphere_2_category.creator_id, user.user_id);
    assert!(!sphere_2_category.is_active);
    assert_eq!(sphere_2_category.delete_timestamp, None);

    Ok(())
}

#[tokio::test]
async fn test_delete_sphere_category() -> Result<(), AppError> {
    let db_pool = get_db_pool().await;
    let user = create_user("test", &db_pool).await;
    let sphere = create_sphere("sphere", "a", false, &user, &db_pool).await?;
    let user = User::get(user.user_id, &db_pool).await.expect("User should be loaded after sphere creation");

    let category_name = "a";
    let sphere_category = set_sphere_category(
        &sphere.sphere_name,
        category_name,
        Color::Purple,
        "b",
        true,
        &user,
        &db_pool
    ).await.expect("Category should be added.");

    delete_sphere_category(&sphere.sphere_name, &sphere_category.category_name, &user, &db_pool).await.expect("Sphere category should be deleted.");

    assert!(get_sphere_category_vec(&sphere.sphere_name, &db_pool).await.expect("Sphere category should be deleted.").is_empty());

    let sphere_category = set_sphere_category(
        &sphere.sphere_name,
        category_name,
        Color::Cyan,
        "b",
        true,
        &user,
        &db_pool
    ).await.expect("Category should be added.");

    create_post(
        &sphere.sphere_name,
        None,
        "a",
        "b",
        None,
        Link::default(),
        PostTags::new(false, false, false, Some(sphere_category.category_id)),
        &user,
        &db_pool
    ).await.expect("Post should be created.");

    assert_eq!(
        delete_sphere_category(&sphere.sphere_name, &sphere_category.category_name, &user, &db_pool).await,
        Err(AppError::InternalServerError(String::from(CATEGORY_NOT_DELETED_STR))),
    );

    Ok(())
}