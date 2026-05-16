use std::collections::BTreeSet;

use sphare_core_common::common::SphereHeader;
use sphare_core_common::errors::AppError;
use sphare_core_content::comment::ssr::create_comment;
use sphare_core_content::comment::CommentWithContext;
use sphare_core_content::embed::Link;
use sphare_core_content::post::ssr::create_post;
use sphare_core_content::post::{PostTags, PostWithSphereInfo};
use sphare_core_content::search::ssr::{get_matching_sphere_header_vec, search_comments, search_posts, search_spheres};
use sphare_core_sphere::sphere::ssr::create_sphere;
use sphare_core_sphere::sphere_management::ssr::set_sphere_icon_url;
use sphare_core_user::user::ssr::{get_matching_user_header_vec, set_user_settings};
use sphare_core_user::user::{User, UserHeader};

use crate::common::{create_test_user, create_user, get_db_pool};
use crate::data_factory::{create_simple_post, set_sphere_num_members};

mod common;
mod data_factory;

#[tokio::test]
async fn test_get_matching_user_header_vec() {
    let db_pool = get_db_pool().await;

    let num_users = 10usize;
    let mut expected_username_set = BTreeSet::<String>::new();
    for i in 0..num_users {
        expected_username_set.insert(
            create_user(
                i.to_string().as_str(),
                &db_pool,
            ).await.username
        );
    }

    let user_header_vec = get_matching_user_header_vec("1", false, num_users as i64, &db_pool).await.expect("Should get user header vec");

    let mut previous_username = None;
    for user_header in user_header_vec {
        assert_eq!(user_header.username.chars().next().unwrap(), '1');
        assert_eq!(user_header.is_nsfw, false);
        if let Some(previous_username) = previous_username {
            assert!(previous_username < user_header.username)
        }
        previous_username = Some(user_header.username);
    }

    for i in num_users..2 * num_users {
        expected_username_set.insert(
            create_user(
                i.to_string().as_str(),
                &db_pool,
            ).await.username
        );
    }

    get_matching_user_header_vec("", false, num_users as i64, &db_pool).await.expect_err("Should get error for empty username prefix");

    let nsfw_user = create_user("nsfw", &db_pool).await;
    set_user_settings(true, false, 0, &nsfw_user, &db_pool).await.expect("Should set user settings");
    let nsfw_header_vec = UserHeader {
        username: nsfw_user.username,
        is_nsfw: true,
    };

    let user_header_vec = get_matching_user_header_vec("nsfw", false, num_users as i64, &db_pool).await.expect("Should get empty user header vec");
    assert!(user_header_vec.is_empty());
    let user_header_vec = get_matching_user_header_vec("nsfw", true, num_users as i64, &db_pool).await.expect("Should get user header vec");
    assert_eq!(user_header_vec.len(), 1);
    assert_eq!(user_header_vec.first(), Some(&nsfw_header_vec));
}

#[tokio::test]
async fn test_get_matching_sphere_header_vec() -> Result<(), AppError> {
    let db_pool = get_db_pool().await;
    let user = create_test_user(&db_pool).await;

    let num_spheres = 20usize;
    let mut expected_sphere_name_vec = Vec::new();
    for i in 0..num_spheres {
        expected_sphere_name_vec.push(
            create_sphere(
                i.to_string().as_str(),
                "sphere",
                i % 2 == 1,
                &user,
                &db_pool,
            ).await?.sphere_name,
        );
    }

    let user = User::get(user.user_id, &db_pool).await.expect("User should be reloaded.");

    let first_sphere_icon_url = Some("a");
    set_sphere_icon_url(expected_sphere_name_vec.first().unwrap(), first_sphere_icon_url, &user, &db_pool).await.expect("Sphere icon should be set.");

    let sphere_header_vec = get_matching_sphere_header_vec("1", num_spheres as i64, &db_pool).await?;

    let mut previous_sphere_name = None;
    for sphere_header in sphere_header_vec {
        assert_eq!(sphere_header.icon_url, None);
        assert_eq!(sphere_header.sphere_name.chars().next().unwrap(), '1');
        if let Some(previous_sphere_name) = previous_sphere_name {
            assert!(previous_sphere_name < sphere_header.sphere_name)
        }
        previous_sphere_name = Some(sphere_header.sphere_name.clone());
    }

    for i in num_spheres..2 * num_spheres {
        expected_sphere_name_vec.push(
            create_sphere(
                i.to_string().as_str(),
                "sphere",
                i % 2 == 0,
                &user,
                &db_pool,
            )
                .await?
                .sphere_name,
        );
    }

    let sphere_header_vec = get_matching_sphere_header_vec("", num_spheres as i64, &db_pool).await?;

    assert_eq!(sphere_header_vec.len(), num_spheres);
    assert_eq!(sphere_header_vec.first().unwrap().icon_url.as_deref(), first_sphere_icon_url);

    Ok(())
}

#[tokio::test]
async fn test_search_spheres() {
    let db_pool = get_db_pool().await;
    let user = create_test_user(&db_pool).await;

    let sphere_1 = create_sphere(
        "music",
        "The place to share your favorite music!",
        false,
        &user,
        &db_pool
    ).await.expect("Sphere 1 should be created.");
    let sphere_2 = create_sphere(
        "classical_music",
        "The place to share classical music!",
        false,
        &user,
        &db_pool
    ).await.expect("Sphere 2 should be created.");
    let sphere_3 = create_sphere(
        "classicalMusic",
        "The real place to share classical music!",
        true,
        &user,
        &db_pool
    ).await.expect("Sphere 3 should be created.");
    let sphere_4 = create_sphere(
        "gastronomie",
        "Pour partager les meilleures saveurs.",
        false,
        &user,
        &db_pool
    ).await.expect("Sphere 4 should be created.");

    let sphere_1 = set_sphere_num_members(sphere_1.sphere_id, 3, &db_pool).await.expect("Sphere 1 num_members should be set.");
    let sphere_2 = set_sphere_num_members(sphere_2.sphere_id, 2, &db_pool).await.expect("Sphere 2 num_members should be set.");
    let sphere_3 = set_sphere_num_members(sphere_3.sphere_id, 1, &db_pool).await.expect("Sphere 3 num_members should be set.");

    let sphere_1_header = SphereHeader::from(&sphere_1);
    let sphere_2_header = SphereHeader::from(&sphere_2);
    let sphere_3_header = SphereHeader::from(&sphere_3);
    let sphere_4_header = SphereHeader::from(&sphere_4);

    let no_match_sphere_vec = search_spheres("no match", true, 10, 0, &db_pool).await.expect("No match search should run");
    assert!(no_match_sphere_vec.is_empty());

    let music_sphere_vec = search_spheres("music", true, 10, 0, &db_pool).await.expect("Music search should run");
    assert_eq!(music_sphere_vec.len(), 3);
    assert_eq!(music_sphere_vec.first(), Some(&sphere_1_header));
    assert_eq!(music_sphere_vec.get(1), Some(&sphere_2_header));
    assert_eq!(music_sphere_vec.get(2), Some(&sphere_3_header));

    let music_sphere_vec = search_spheres("music", true, 1, 1, &db_pool).await.expect("Music search should run");
    assert_eq!(music_sphere_vec.len(), 1);
    assert_eq!(music_sphere_vec.first(), Some(&sphere_2_header));

    let music_sphere_vec = search_spheres("music", false, 10, 0, &db_pool).await.expect("Music search should run");
    assert_eq!(music_sphere_vec.len(), 2);
    assert_eq!(music_sphere_vec.first(), Some(&sphere_1_header));
    assert_eq!(music_sphere_vec.get(1), Some(&sphere_2_header));

    let music_sphere_vec = search_spheres("music", true, 10, 1, &db_pool).await.expect("Music search should run");
    assert_eq!(music_sphere_vec.len(), 2);
    assert_eq!(music_sphere_vec.first(), Some(&sphere_2_header));
    assert_eq!(music_sphere_vec.get(1), Some(&sphere_3_header));

    let saveurs_sphere_vec = search_spheres("saveurs", true, 10, 0, &db_pool).await.expect("Saveurs search should run");
    assert_eq!(saveurs_sphere_vec.len(), 1);
    assert_eq!(saveurs_sphere_vec.first(), Some(&sphere_4_header));
}

#[tokio::test]
async fn test_search_posts() {
    let db_pool = get_db_pool().await;
    let user = create_test_user(&db_pool).await;

    let sphere_1 = create_sphere("1", "1", false, &user, &db_pool).await.expect("Sphere 1 should be created.");
    let sphere_2 = create_sphere("2", "2", false, &user, &db_pool).await.expect("Sphere 2 should be created.");

    let post_1 = create_simple_post(&sphere_1.sphere_name, None, "One apple a day", "keeps the doctor away.", None, &user, &db_pool).await;
    let post_2 = create_simple_post(&sphere_1.sphere_name, None, "Bonjour", "Adieu.", None, &user, &db_pool).await;
    let post_3 = create_simple_post(&sphere_1.sphere_name, None, "Et re-bonjour", "À la prochaine.", None, &user, &db_pool).await;
    let post_4 = PostWithSphereInfo::from_post(
        create_post(
            &sphere_2.sphere_name,
            None,
            "Salutations",
            "ça veut dire bonjour.",
            None,
            Link::default(),
            PostTags::new(true, false, false, None),
            &user,
            &db_pool
        ).await.expect("Sphere 4 should be created."),
        sphere_2.sphere_name.clone(),
        None,
        None,
    );
    let post_5 = PostWithSphereInfo::from_post(
        create_post(
            &sphere_2.sphere_name, None,
            "Qu'entendez-vous par là?",
            "Me souhaitez vous le bonjour ou constatez vous que c’est une bonne journée, que je le veuille ou non, ou encore que c’est une journée où il faut être bon ?",
            None,
            Link::default(),
            PostTags::new(false, true, false, None),
            &user,
            &db_pool
        ).await.expect("Sphere 5 should be created."),
        sphere_2.sphere_name.clone(),
        None,
        None,
    );
    let post_6 = create_simple_post(&sphere_2.sphere_name, None, "Guten morgen", "xml_body", Some("# Wie geht's?"), &user, &db_pool).await;

    let no_match_post_vec = search_posts("no match", None,true, true, 10, 0, &db_pool).await.expect("No match search should run");
    assert!(no_match_post_vec.is_empty());

    let apple_post_vec = search_posts("apple", None, true, true, 10, 0, &db_pool).await.expect("Apple search should run");
    assert_eq!(apple_post_vec.len(), 1);
    assert_eq!(apple_post_vec.first(), Some(&post_1));

    let bonjour_post_vec = search_posts("bonjour", None, true, true, 10, 0, &db_pool).await.expect("Bonjour search should run");
    assert_eq!(bonjour_post_vec.len(), 4);
    assert_eq!(bonjour_post_vec.first(), Some(&post_2));
    assert_eq!(bonjour_post_vec.get(1), Some(&post_3));
    assert_eq!(bonjour_post_vec.get(2), Some(&post_4));
    assert_eq!(bonjour_post_vec.get(3), Some(&post_5));

    let bonjour_post_vec = search_posts("bonjour", Some(&sphere_1.sphere_name), true, true, 10, 0, &db_pool).await.expect("Bonjour search should run");
    assert_eq!(bonjour_post_vec.len(), 2);
    assert_eq!(bonjour_post_vec.first(), Some(&post_2));
    assert_eq!(bonjour_post_vec.get(1), Some(&post_3));

    let bonjour_post_vec = search_posts("bonjour", Some(&sphere_2.sphere_name), true, true, 10, 0, &db_pool).await.expect("Bonjour search should run");
    assert_eq!(bonjour_post_vec.len(), 2);
    assert_eq!(bonjour_post_vec.first(), Some(&post_4));
    assert_eq!(bonjour_post_vec.get(1), Some(&post_5));

    let bonjour_post_vec = search_posts("bonjour", None, true, true, 2, 1, &db_pool).await.expect("Bonjour search should run");
    assert_eq!(bonjour_post_vec.len(), 2);
    assert_eq!(bonjour_post_vec.first(), Some(&post_3));
    assert_eq!(bonjour_post_vec.get(1), Some(&post_4));

    let bonjour_post_vec = search_posts("bonjour", None, false, true, 10, 0, &db_pool).await.expect("Bonjour search should run");
    assert_eq!(bonjour_post_vec.len(), 3);
    assert_eq!(bonjour_post_vec.first(), Some(&post_2));
    assert_eq!(bonjour_post_vec.get(1), Some(&post_3));
    assert_eq!(bonjour_post_vec.get(2), Some(&post_5));

    let bonjour_post_vec = search_posts("bonjour", None, true, false, 10, 0, &db_pool).await.expect("Bonjour search should run");
    assert_eq!(bonjour_post_vec.len(), 3);
    assert_eq!(bonjour_post_vec.first(), Some(&post_2));
    assert_eq!(bonjour_post_vec.get(1), Some(&post_3));
    assert_eq!(bonjour_post_vec.get(2), Some(&post_4));

    let bonjour_post_vec = search_posts("bonjour", None, false, false, 10, 0, &db_pool).await.expect("Bonjour search should run");
    assert_eq!(bonjour_post_vec.len(), 2);
    assert_eq!(bonjour_post_vec.first(), Some(&post_2));
    assert_eq!(bonjour_post_vec.get(1), Some(&post_3));

    let geht_post_vec = search_posts("geht", None, true, true, 10, 0, &db_pool).await.expect("Geht search should run");
    assert_eq!(geht_post_vec.len(), 1);
    assert_eq!(geht_post_vec.first(), Some(&post_6));
}

#[tokio::test]
async fn test_search_comments() {
    let db_pool = get_db_pool().await;
    let user = create_test_user(&db_pool).await;

    let sphere_1 = create_sphere("1", "1", false, &user, &db_pool).await.expect("Sphere 1 should be created.");
    let sphere_2 = create_sphere("2", "2", false, &user, &db_pool).await.expect("Sphere 2 should be created.");
    
    let sphere_1_header = SphereHeader::new(sphere_1.sphere_name.clone(), sphere_1.icon_url, sphere_1.is_nsfw);
    let sphere_2_header = SphereHeader::new(sphere_2.sphere_name.clone(), sphere_2.icon_url, sphere_2.is_nsfw);

    let post_1 = create_simple_post(&sphere_1.sphere_name, None, "1", "1", None, &user, &db_pool).await;
    let post_2 = create_simple_post(&sphere_2.sphere_name, None, "2", "2", None, &user, &db_pool).await;
    
    let comment_1 = create_comment(
        post_1.post.post_id, 
        None, 
        "Hello there", 
        None, 
        false, 
        &user, 
        &db_pool
    ).await.expect("hello comment should be created.");
    let comment_2 = create_comment(
        post_1.post.post_id, 
        Some(comment_1.comment_id), 
        "Général Kenobi !", 
        None, 
        false, 
        &user, &db_pool
    ).await.expect("Général comment should be created.");
    let comment_3 = create_comment(
        post_2.post.post_id,
        None,
        "En général, on dit un pain au chocolat.",
        None,
        false,
        &user, &db_pool
    ).await.expect("Général comment 2 should be created.");
    let comment_4 = create_comment(
        post_2.post.post_id,
        Some(comment_3.comment_id),
        "Non en général, on dit une chocolatine.",
        None,
        false,
        &user, &db_pool
    ).await.expect("Général comment 3 should be created.");
    let comment_5 = create_comment(
        post_2.post.post_id, None, 
        "xml_body", 
        Some(" **Es ist eine Falle!**"), 
        false, 
        &user, 
        &db_pool
    ).await.expect("Falle comment should be created.");
    
    let comment_1 = CommentWithContext::from_comment(comment_1, sphere_1_header.clone(), &post_1.post);
    let comment_2 = CommentWithContext::from_comment(comment_2, sphere_1_header, &post_1.post);
    let comment_3 = CommentWithContext::from_comment(comment_3, sphere_2_header.clone(), &post_2.post);
    let comment_4 = CommentWithContext::from_comment(comment_4, sphere_2_header.clone(), &post_2.post);
    let comment_5 = CommentWithContext::from_comment(comment_5, sphere_2_header, &post_2.post);

    let no_match_comment_vec = search_comments("no match", None, 10, 0, &db_pool).await.expect("No match search should run");
    assert!(no_match_comment_vec.is_empty());

    let hello_comment_vec = search_comments("hello", None, 10, 0, &db_pool).await.expect("Hello search should run");
    assert_eq!(hello_comment_vec.len(), 1);
    assert_eq!(hello_comment_vec.first(), Some(&comment_1));

    let general_comment_vec = search_comments("général", None, 10, 0, &db_pool).await.expect("General search should run");
    assert_eq!(general_comment_vec.len(), 3);
    assert_eq!(general_comment_vec.first(), Some(&comment_2));
    assert_eq!(general_comment_vec.get(1), Some(&comment_3));
    assert_eq!(general_comment_vec.get(2), Some(&comment_4));

    let general_comment_vec = search_comments("général", Some(&sphere_1.sphere_name), 10, 0, &db_pool).await.expect("General search should run");
    assert_eq!(general_comment_vec.len(), 1);
    assert_eq!(general_comment_vec.first(), Some(&comment_2));

    let general_comment_vec = search_comments("général", Some(&sphere_2.sphere_name), 10, 0, &db_pool).await.expect("General search should run");
    assert_eq!(general_comment_vec.len(), 2);
    assert_eq!(general_comment_vec.first(), Some(&comment_3));
    assert_eq!(general_comment_vec.get(1), Some(&comment_4));

    let general_comment_vec = search_comments("général", None, 1, 1, &db_pool).await.expect("General search should run");
    assert_eq!(general_comment_vec.len(), 1);
    assert_eq!(general_comment_vec.first(), Some(&comment_3));

    let falle_comment_vec = search_comments("Falle", None, 10, 0, &db_pool).await.expect("Falle search should run");
    assert_eq!(falle_comment_vec.len(), 1);
    assert_eq!(falle_comment_vec.first(), Some(&comment_5));
}