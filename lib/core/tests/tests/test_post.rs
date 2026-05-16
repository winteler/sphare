use std::collections::{HashMap, HashSet};
use std::time::Duration;

use rand::RngExt;
use sqlx::PgPool;

use sphare_core_common::colors::Color;
use sphare_core_common::editor::get_styled_html_from_markdown;
use sphare_core_common::editor::ssr::get_html_and_markdown_strings;
use sphare_core_common::errors::AppError;
use sphare_core_common::routes::get_post_path;
use sphare_core_content::comment::ssr::create_comment;
use sphare_core_content::embed::{Link, LinkType};
use sphare_core_content::filter::{CategorySetFilter, SphereCategoryFilter};
use sphare_core_content::moderation::ssr::moderate_post;
use sphare_core_content::post::ssr::{create_post, create_post_and_vote, delete_post, edit_post, get_homepage_post_vec, get_post_by_id, get_post_inherited_attributes, get_post_vec_by_satellite_id, get_post_vec_by_sphere_name, get_post_with_info_by_id, get_sorted_post_vec, get_subscribed_post_vec, update_post, update_post_scores};
use sphare_core_content::post::{PostDataInputs, PostLocation, PostTags, PostWithSphereInfo};
use sphare_core_content::ranking::ssr::vote_on_content;
use sphare_core_content::ranking::{PostSortType, SortType, VoteValue};
use sphare_core_sphere::rule::ssr::add_rule;
use sphare_core_sphere::satellite::ssr::create_satellite;
use sphare_core_sphere::satellite::Satellite;
use sphare_core_sphere::sphere::ssr::{create_sphere, get_post_sphere, subscribe};
use sphare_core_sphere::sphere::Sphere;
use sphare_core_sphere::sphere_category::ssr::set_sphere_category;
use sphare_core_user::user::User;

use crate::common::*;
use crate::data_factory::*;
use crate::utils::{get_user_post_vote, sort_post_vec, test_post_score, POST_SORT_TYPE_ARRAY};

mod common;
mod data_factory;
mod utils;

async fn create_sphere_with_filter_posts(
    sphere_name: &str,
    num_post: usize,
    in_satellite: bool,
    user: &mut User,
    db_pool: &PgPool
) -> (Sphere, Satellite, Vec<PostWithSphereInfo>, PostWithSphereInfo, PostWithSphereInfo, PostWithSphereInfo) {
    let (sphere, satellite) = create_sphere_with_satellite(
        sphere_name,
        "satellite",
        false,
        false,
        user,
        db_pool,
    ).await.expect("sphere with satellite should be created.");

    let post_satellite_id = match in_satellite {
        true => Some(satellite.satellite_id),
        false => None,
    };

    let base_post_vec = create_posts(
        &sphere,
        post_satellite_id,
        num_post,
        Some((0..num_post).map(|i| (i + 3) as i32).collect()),
        None,
        Vec::new(),
        user,
        db_pool,
    ).await.expect("posts should be created.");

    let new_spoiler_post = create_post(
        &sphere.sphere_name,
        post_satellite_id,
        "new_spoiler",
        "a",
        None,
        Link::default(),
        PostTags::new(true, false, false, None),
        user,
        db_pool,
    ).await.expect("new spoiler post should be created.");

    let day_old_spoiler_post = create_post(
        &sphere.sphere_name,
        post_satellite_id,
        "old_spoiler",
        "a",
        None,
        Link::default(),
        PostTags::new(true, false, false, None),
        user,
        db_pool,
    ).await.expect("old spoiler post should be created.");
    let day_old_spoiler_post = set_post_timestamp(day_old_spoiler_post.post_id, -2, db_pool).await.expect("old spoiler timestamp should be set.");
    let day_old_spoiler_post = set_post_score(day_old_spoiler_post.post_id, 1, db_pool).await.expect("old spoiler score should be set.");

    let nsfw_post = create_post(
        &sphere.sphere_name,
        post_satellite_id,
        "nsfw",
        "a",
        None,
        Link::default(),
        PostTags::new(false, true, false, None),
        user,
        db_pool,
    ).await.expect("nsfw_post should be created.");
    let nsfw_post = set_post_score(nsfw_post.post_id, 2, db_pool).await.expect("nsfw_post score should be set.");

    (
        sphere.clone(),
        satellite,
        base_post_vec,
        PostWithSphereInfo::from_post(new_spoiler_post, sphere.sphere_name.clone(), None, None),
        PostWithSphereInfo::from_post(day_old_spoiler_post, sphere.sphere_name.clone(), None, None),
        PostWithSphereInfo::from_post(nsfw_post, sphere.sphere_name.clone(), None, None),
    )
}

#[tokio::test]
async fn test_get_post_by_id() -> Result<(), AppError> {
    let db_pool = get_db_pool().await;
    let user = create_test_user(&db_pool).await;

    let sphere = create_sphere("a", "sphere", false, &user, &db_pool).await?;

    let post_1_title = "1";
    let post_1_body = "test";
    let expected_post_1 = create_post(
        &sphere.sphere_name, None, post_1_title, post_1_body, None, Link::default(),PostTags::default(), &user, &db_pool
    ).await.expect("Should be able to create post 1.");

    let post_2_title = "1";
    let post_2_body = "test";
    let post_2_markdown_body = "test";
    let expected_post_2 = create_post(
        &sphere.sphere_name, None, post_2_title, post_2_body, Some(post_2_markdown_body), Link::default(),PostTags::default(), &user, &db_pool
    ).await.expect("Should be able to create post 2.");

    let post_1 = get_post_by_id(expected_post_1.post_id, &db_pool).await.expect("Should be able to load post 1.");
    assert_eq!(post_1, expected_post_1);
    let post_2 = get_post_by_id(expected_post_2.post_id, &db_pool).await.expect("Should be able to load post 1.");
    assert_eq!(post_2, expected_post_2);

    Ok(())
}

#[tokio::test]
async fn test_get_post_with_info_by_id() -> Result<(), AppError> {
    let db_pool = get_db_pool().await;
    let user = create_test_user(&db_pool).await;

    let sphere = create_sphere("a", "sphere", false, &user, &db_pool).await?;

    let user = User::get(user.user_id, &db_pool).await.expect("Should reload user.");
    let sphere_category = set_sphere_category(
        &sphere.sphere_name,
        "b",
        Color::Orange,
        "test",
        true,
        &user,
        &db_pool
    ).await.expect("Should be able to set sphere category.");

    let post_1_title = "1";
    let post_1_body = "test";
    let post_1 = create_post(
        &sphere.sphere_name,
        None,
        post_1_title,
        post_1_body,
        None,
        Link::default(),
        PostTags::new(false, false, false, Some(sphere_category.category_id)),
        &user,
        &db_pool
    ).await.expect("Should be able to create post 1.");

    let post_2_title = "1";
    let post_2_body = "test";
    let post_2_markdown_body = "test";
    let post_2 = create_post(
        &sphere.sphere_name,
        None,
        post_2_title,
        post_2_body,
        Some(post_2_markdown_body),
        Link::default(),
        PostTags::default(),
        &user,
        &db_pool
    ).await.expect("Should be able to create post 2.");

    let post_1_without_vote = get_post_with_info_by_id(post_1.post_id, None, &db_pool).await.expect("Should be able to load post 1.");
    assert_eq!(post_1_without_vote.post, post_1);
    assert_eq!(post_1_without_vote.sphere_category.expect("Should have category"), sphere_category.clone().into());
    assert_eq!(post_1_without_vote.vote, None);

    let post_1_without_vote = get_post_with_info_by_id(post_1.post_id, Some(&user), &db_pool).await.expect("Should be able to load post 1.");
    assert_eq!(post_1_without_vote.post, post_1);
    assert_eq!(post_1_without_vote.sphere_category.expect("Should have category"), sphere_category.into());
    assert_eq!(post_1_without_vote.vote, None);

    let post_2_without_vote = get_post_with_info_by_id(post_2.post_id, None, &db_pool).await.expect("Should be able to load post 2.");
    assert_eq!(post_2_without_vote.post, post_2);
    assert_eq!(post_2_without_vote.sphere_category, None);
    assert_eq!(post_2_without_vote.vote, None);

    let post_2_without_vote = get_post_with_info_by_id(post_2.post_id, Some(&user), &db_pool).await.expect("Should be able to load post 2.");
    assert_eq!(post_2_without_vote.post, post_2);
    assert_eq!(post_2_without_vote.sphere_category, None);
    assert_eq!(post_2_without_vote.vote, None);

    let post_1_vote = vote_on_content(VoteValue::Up, post_1.post_id, None, None, &user, &db_pool).await.expect("Should be possible to vote on post_1.");
    let post_2_vote = vote_on_content(VoteValue::Down, post_2.post_id, None, None, &user, &db_pool).await.expect("Should be possible to vote on post_2.");

    let post_1_with_vote = get_post_with_info_by_id(post_1.post_id, Some(&user), &db_pool).await.expect("Should be able to load post 1.");
    assert_eq!(post_1_with_vote.post.post_id, post_1.post_id);
    assert_eq!(post_1_with_vote.post.creator_id, user.user_id);
    assert_eq!(post_1_with_vote.post.creator_name, user.username);
    assert_eq!(post_1_with_vote.post.title, post_1_title);
    assert_eq!(post_1_with_vote.post.body, post_1_body);
    assert_eq!(post_1_with_vote.post.markdown_body, None);
    assert_eq!(post_1_with_vote.post.score, 1);
    assert_eq!(post_1_with_vote.vote, post_1_vote);

    let post_2_with_vote = get_post_with_info_by_id(post_2.post_id, Some(&user), &db_pool).await.expect("Should be able to load post 2.");
    assert_eq!(post_2_with_vote.post.post_id, post_2.post_id);
    assert_eq!(post_2_with_vote.post.creator_id, user.user_id);
    assert_eq!(post_2_with_vote.post.creator_name, user.username);
    assert_eq!(post_2_with_vote.post.title, post_2_title);
    assert_eq!(post_2_with_vote.post.body, post_2_body);
    assert_eq!(post_2_with_vote.post.markdown_body, Some(String::from(post_2_markdown_body)));
    assert_eq!(post_2_with_vote.post.score, -1);
    assert_eq!(post_2_with_vote.vote, post_2_vote);

    Ok(())
}

#[tokio::test]
async fn test_get_post_inherited_attributes() -> Result<(), AppError> {
    let db_pool = get_db_pool().await;
    let mut user = create_test_user(&db_pool).await;

    let (sphere_1, satellite_1) = create_sphere_with_satellite(
        "1",
        "1",
        false,
        true,
        &mut user,
        &db_pool,
    ).await.expect("Should be able to create sphere with satellite.");

    let (sphere_2, satellite_2) = create_sphere_with_satellite(
        "2",
        "2",
        true,
        false,
        &mut user,
        &db_pool,
    ).await.expect("Should be able to create sphere with satellite.");

    let nsfw_sphere = create_sphere(
        "nsfw",
        "nsfw",
        true,
        &user,
        &db_pool
    ).await.expect("Should be able to create nsfw sphere.");

    let sphere_post = create_post(
        &sphere_1.sphere_name,
        None,
        "1",
        "1",
        None,
        Link::default(),
        PostTags::new(true, true, false, None),
        &user,
        &db_pool
    ).await.expect("Should be able to create sphere 1 post.");

    let sphere_post_inherited_attr = get_post_inherited_attributes(
        sphere_post.post_id,
        &db_pool
    ).await.expect("Should be able to get inherited post attr.");
    assert_eq!(sphere_post_inherited_attr.is_nsfw, false);
    assert_eq!(sphere_post_inherited_attr.is_spoiler, false);

    let nsfw_sphere_post = create_post(
        &nsfw_sphere.sphere_name,
        None,
        "2",
        "2",
        None,
        Link::default(),
        PostTags::default(),
        &user,
        &db_pool
    ).await.expect("Should be able to create nsfw post.");

    let nsfw_sphere_post_inherited_attr = get_post_inherited_attributes(
        nsfw_sphere_post.post_id,
        &db_pool
    ).await.expect("Should be able to get inherited post attr.");
    assert_eq!(nsfw_sphere_post_inherited_attr.is_nsfw, true);
    assert_eq!(nsfw_sphere_post_inherited_attr.is_spoiler, false);

    let satellite_1_post = create_post(
        &sphere_1.sphere_name,
        Some(satellite_1.satellite_id),
        "3",
        "3",
        None,
        Link::default(),
        PostTags::new(false, true, false, None),
        &user,
        &db_pool
    ).await.expect("Should be able to create satellite 1 post.");

    let satellite_1_post_inherited_attr = get_post_inherited_attributes(
        satellite_1_post.post_id,
        &db_pool,
    ).await.expect("Should be able to get inherited post attr.");
    assert_eq!(satellite_1_post_inherited_attr.is_nsfw, false);
    assert_eq!(satellite_1_post_inherited_attr.is_spoiler, true);

    let satellite_2_post = create_post(
        &sphere_2.sphere_name,
        Some(satellite_2.satellite_id),
        "4",
        "4",
        None,
        Link::default(),
        PostTags::new(true, false, false, None),
        &user,
        &db_pool
    ).await.expect("Should be able to create satellite 2 post.");

    let satellite_2_post_inherited_attr = get_post_inherited_attributes(
        satellite_2_post.post_id,
        &db_pool,
    ).await.expect("Should be able to get inherited post attr.");
    assert_eq!(satellite_2_post_inherited_attr.is_nsfw, true);
    assert_eq!(satellite_2_post_inherited_attr.is_spoiler, false);

    Ok(())
}

#[tokio::test]
async fn test_get_post_sphere() -> Result<(), AppError> {
    let db_pool = get_db_pool().await;
    let user = create_test_user(&db_pool).await;

    let sphere = create_sphere("a", "sphere", false, &user, &db_pool).await?;
    let post = create_post(
        &sphere.sphere_name, None, "1", "test", None, Link::default(),PostTags::default(), &user, &db_pool
    ).await.expect("Should be able to create post.");

    let result_sphere = get_post_sphere(post.post_id, &db_pool).await.expect("Post sphere should be available.");
    assert_eq!(result_sphere, sphere);

    Ok(())
}

#[tokio::test]
async fn test_get_subscribed_post_vec() -> Result<(), AppError> {
    let db_pool = get_db_pool().await;
    let mut user = create_test_user(&db_pool).await;

    let sphere1_name = "1";
    let sphere2_name = "2";
    let num_post = 10usize;

    let (sphere1, _, mut sphere1_post_vec) = create_sphere_with_posts(
        sphere1_name,
        Some("url"),
        num_post,
        Some((0..num_post).map(|i| i as i32).collect()),
        (0..num_post).map(|i| (i % 2) == 0).collect(),
        &mut user,
        &db_pool,
    ).await.expect("Should create sphere 1 with posts.");

    // create post in satellite to make sure it doesn't get included in the results
    let satellite = create_satellite(
        &sphere1.sphere_name,
        "a",
        "satellite",
        false,
        false,
        false,
        &user,
        &db_pool,
    ).await.expect("Should be able to insert satellite.");

    create_post(
        &sphere1.sphere_name,
        Some(satellite.satellite_id),
        "satellite",
        "satellite",
        None,
        Link::default(),
        PostTags::default(),
        &user,
        &db_pool,
    ).await.expect("Should create satellite post.");


    let (_, _, mut sphere2_post_vec) = create_sphere_with_posts(
        sphere2_name,
        None,
        num_post,
        Some((0..num_post).map(|i| 100 + i as i32).collect()),
        (0..num_post).map(|i| (i % 2) == 0).collect(),
        &mut user,
        &db_pool,
    ).await.expect("Should create sphere 2 with posts.");

    let mut full_post_vec = sphere1_post_vec.clone();
    full_post_vec.append(&mut sphere2_post_vec.clone());

    // When no subscription, simply return all sorted posts
    for sort_type in POST_SORT_TYPE_ARRAY {
        let post_vec = get_subscribed_post_vec(
            SortType::Post(sort_type),
            num_post as i64,
            0,
            &user,
            &db_pool,
        ).await?;
        sort_post_vec(&mut full_post_vec, sort_type, false);
        assert_eq!(post_vec, full_post_vec[0..num_post]);
    }

    // When subscribed, the subscribed posts should come first, followed by unsubscribed posts
    subscribe(sphere1.sphere_id, user.user_id, &db_pool).await?;
    for sort_type in POST_SORT_TYPE_ARRAY {
        sort_post_vec(&mut sphere1_post_vec, sort_type, false);
        sort_post_vec(&mut sphere2_post_vec, sort_type, false);
        let mut expected_vec = sphere1_post_vec.clone();
        expected_vec.append(&mut sphere2_post_vec.clone());

        let post_vec = get_subscribed_post_vec(
            SortType::Post(sort_type),
            (num_post + 3) as i64,
            0,
            &user,
            &db_pool,
        ).await?;
        assert_eq!(post_vec, expected_vec[0..num_post+3]);

        let post_vec = get_subscribed_post_vec(
            SortType::Post(sort_type),
            num_post as i64,
            num_post as i64,
            &user,
            &db_pool,
        ).await?;
        assert_eq!(post_vec, sphere2_post_vec);

        let post_vec = get_subscribed_post_vec(
            SortType::Post(sort_type),
            2*num_post as i64,
            0,
            &user,
            &db_pool,
        ).await?;
        assert_eq!(post_vec, expected_vec);
    }

    // Check that moderated and deleted posts are not returned
    let (moderated_post, deleted_post) = get_moderated_and_deleted_posts(sphere1_name, &user, &db_pool).await;

    let post_vec = get_subscribed_post_vec(
        SortType::Post(PostSortType::Recent),
        num_post as i64,
        0,
        &user,
        &db_pool,
    ).await?;

    assert!(!post_vec.contains(&moderated_post));
    assert!(!post_vec.contains(&deleted_post));

    Ok(())
}

#[tokio::test]
async fn test_get_subscribed_post_vec_with_filters() {
    let db_pool = get_db_pool().await;
    let mut user = create_test_user(&db_pool).await;

    let num_post = 10;
    let (sphere, _, mut post_vec, new_spoiler_post, old_spoiler_post, nsfw_post) = create_sphere_with_filter_posts(
        "sphere",
        num_post,
        false,
        &mut user,
        &db_pool,
    ).await;
    subscribe(sphere.sphere_id, user.user_id, &db_pool).await.expect("Should subscribe to sphere 1");

    let (sphere_2, sphere_2_post) = create_sphere_with_post(
        "sphere_2",
        &mut user,
        &db_pool,
    ).await;
    let sphere_2_post = set_post_score(sphere_2_post.post_id, 50, &db_pool).await.expect("Should set post score");
    subscribe(sphere_2.sphere_id, user.user_id, &db_pool).await.expect("Should subscribe to sphere 1");
    post_vec.push(PostWithSphereInfo::from_post(sphere_2_post, sphere_2.sphere_name.clone(), None, None));

    let (_, _, mut sphere3_posts) = create_sphere_with_posts(
        "sphere_3",
        None,
        num_post,
        Some((0..num_post).map(|i| 100 + i as i32).collect()),
        Vec::new(),
        &mut user,
        &db_pool,
    ).await.expect("Sphere 3 with posts should be created");

    let num_load_posts = 2*num_post;

    let mut default_filter_expected_post = post_vec.clone();
    default_filter_expected_post.push(new_spoiler_post.clone());
    default_filter_expected_post.push(old_spoiler_post.clone());

    for sort_type in POST_SORT_TYPE_ARRAY {
        let post_vec: Vec<PostWithSphereInfo> = get_subscribed_post_vec(
            SortType::Post(sort_type),
            (2*num_post) as i64,
            0,
            &user,
            &db_pool,
        ).await.expect("Should load subscribed posts.");

        sort_post_vec(&mut default_filter_expected_post, sort_type, false);
        sort_post_vec(&mut sphere3_posts, sort_type, false);
        let mut expected_post_vec = default_filter_expected_post.clone();
        expected_post_vec.append(&mut sphere3_posts.clone());
        assert_eq!(post_vec, expected_post_vec[0..num_load_posts]);
    }

    user.days_hide_spoiler = Some(1);
    user.show_nsfw = false;
    let mut one_day_spoiler_filter_expected_post = post_vec.clone();
    one_day_spoiler_filter_expected_post.push(old_spoiler_post.clone());

    for sort_type in POST_SORT_TYPE_ARRAY {
        let post_vec: Vec<PostWithSphereInfo> = get_subscribed_post_vec(
            SortType::Post(sort_type),
            (2*num_post) as i64,
            0,
            &user,
            &db_pool,
        )
            .await
            .expect("Should load subscribed posts.");
        sort_post_vec(&mut one_day_spoiler_filter_expected_post, sort_type, false);
        sort_post_vec(&mut sphere3_posts, sort_type, false);
        let mut expected_post_vec = one_day_spoiler_filter_expected_post.clone();
        expected_post_vec.append(&mut sphere3_posts.clone());
        assert_eq!(post_vec, expected_post_vec[0..num_load_posts]);
    }

    user.days_hide_spoiler = Some(3);
    user.show_nsfw = true;
    let mut three_day_spoiler_filter_expected_post = post_vec.clone();
    three_day_spoiler_filter_expected_post.push(nsfw_post.clone());

    for sort_type in POST_SORT_TYPE_ARRAY {
        let post_vec: Vec<PostWithSphereInfo> = get_subscribed_post_vec(
            SortType::Post(sort_type),
            (2*num_post) as i64,
            0,
            &user,
            &db_pool,
        )
            .await
            .expect("Should load subscribed posts.");
        sort_post_vec(&mut three_day_spoiler_filter_expected_post, sort_type, false);
        sort_post_vec(&mut sphere3_posts, sort_type, false);
        let mut expected_post_vec = three_day_spoiler_filter_expected_post.clone();
        expected_post_vec.append(&mut sphere3_posts.clone());
        assert_eq!(post_vec, expected_post_vec[0..num_load_posts]);
    }
}

#[tokio::test]
async fn test_get_sorted_post_vec() -> Result<(), AppError> {
    let db_pool = get_db_pool().await;
    let mut user = create_test_user(&db_pool).await;

    let sphere1_name = "1";
    let sphere2_name = "2";
    let num_post = 10;
    let mut expected_post_vec = Vec::<PostWithSphereInfo>::new();

    let (_, _, mut expected_sphere1_post_vec) = create_sphere_with_posts(
        sphere1_name,
        Some("url"),
        num_post,
        Some((0..num_post).map(|i| i as i32).collect()),
        (0..num_post).map(|i| (i % 2) == 0).collect(),
        &mut user,
        &db_pool,
    ).await?;
    expected_post_vec.append(&mut expected_sphere1_post_vec);

    let (_, _, mut expected_sphere2_post_vec) = create_sphere_with_posts(
        sphere2_name,
        None,
        num_post,
        Some((0..num_post).map(|i| (i + num_post) as i32).collect()),
        (0..num_post).map(|i| (i % 2) == 0).collect(),
        &mut user,
        &db_pool,
    ).await?;
    expected_post_vec.append(&mut expected_sphere2_post_vec);

    // create nsfw post to check it's filtered from result
    create_post(
        sphere1_name,
        None,
        "nsfw",
        "nsfw",
        None,
        Link::default(),
        PostTags::new(false, true, false, None),
        &user,
        &db_pool,
    ).await.expect("nsfw_post should be created.");

    // create post in satellite to make sure it doesn't get included in the results
    let satellite = create_satellite(
        sphere1_name,
        "a",
        "satellite",
        false,
        false,
        false,
        &user,
        &db_pool,
    ).await.expect("Should be able to insert satellite.");

    create_post(
        sphere1_name,
        Some(satellite.satellite_id),
        "satellite",
        "satellite",
        None,
        Link::default(),
        PostTags::default(),
        &user,
        &db_pool,
    ).await.expect("Should create satellite post.");

    for sort_type in POST_SORT_TYPE_ARRAY {
        let post_vec = get_sorted_post_vec(
            SortType::Post(sort_type),
            num_post as i64,
            0,
            None,
            &db_pool
        ).await.expect("First post vec should be loaded");
        let second_post_vec = get_sorted_post_vec(
            SortType::Post(sort_type),
            num_post as i64,
            num_post as i64,
            None,
            &db_pool
        ).await.expect("Second post vec should be loaded");
        sort_post_vec(&mut expected_post_vec, sort_type, true);
        assert_eq!(post_vec, expected_post_vec[..num_post]);
        assert_eq!(second_post_vec, expected_post_vec[num_post..2*num_post]);
    }

    // Check that moderated and deleted posts are not returned
    let (moderated_post, deleted_post) = get_moderated_and_deleted_posts(sphere1_name, &user, &db_pool).await;

    let post_vec = get_sorted_post_vec(SortType::Post(PostSortType::Recent), num_post as i64, 0, None, &db_pool).await?;

    assert!(!post_vec.contains(&moderated_post));
    assert!(!post_vec.contains(&deleted_post));

    Ok(())
}

#[tokio::test]
async fn test_get_sorted_post_vec_with_filters() {
    let db_pool = get_db_pool().await;
    let mut user = create_test_user(&db_pool).await;

    let num_post = 10;
    let (_, _, post_vec, new_spoiler_post, old_spoiler_post, nsfw_post) = create_sphere_with_filter_posts(
        "sphere",
        num_post,
        false,
        &mut user,
        &db_pool,
    ).await;

    let mut default_filter_expected_post = post_vec.clone();
    default_filter_expected_post.push(new_spoiler_post.clone());
    default_filter_expected_post.push(old_spoiler_post.clone());

    for sort_type in POST_SORT_TYPE_ARRAY {
        let post_vec = get_sorted_post_vec(
            SortType::Post(sort_type),
            (2*num_post) as i64,
            0,
            Some(&user),
            &db_pool
        ).await.expect("Post vec should be loaded");
        sort_post_vec(&mut default_filter_expected_post, sort_type, true);
        assert_eq!(post_vec, default_filter_expected_post);
    }

    user.days_hide_spoiler = Some(1);
    user.show_nsfw = false;
    let mut one_day_spoiler_filter_expected_post = post_vec.clone();
    one_day_spoiler_filter_expected_post.push(old_spoiler_post.clone());

    for sort_type in POST_SORT_TYPE_ARRAY {
        let post_vec = get_sorted_post_vec(
            SortType::Post(sort_type),
            (2*num_post) as i64,
            0,
            Some(&user),
            &db_pool
        ).await.expect("Post vec should be loaded");
        sort_post_vec(&mut one_day_spoiler_filter_expected_post, sort_type, true);
        assert_eq!(post_vec, one_day_spoiler_filter_expected_post);
    }

    user.days_hide_spoiler = Some(3);
    user.show_nsfw = true;
    let mut three_day_spoiler_filter_expected_post = post_vec.clone();
    three_day_spoiler_filter_expected_post.push(nsfw_post.clone());

    for sort_type in POST_SORT_TYPE_ARRAY {
        let post_vec = get_sorted_post_vec(
            SortType::Post(sort_type),
            (2*num_post) as i64,
            0,
            Some(&user),
            &db_pool
        ).await.expect("Post vec should be loaded");
        sort_post_vec(&mut three_day_spoiler_filter_expected_post, sort_type, true);
        assert_eq!(post_vec, three_day_spoiler_filter_expected_post);
    }
}

#[tokio::test]
async fn test_get_post_vec_by_sphere_name() -> Result<(), AppError> {
    let db_pool = get_db_pool().await;
    let mut user = create_test_user(&db_pool).await;

    let sphere_name = "sphere";
    let num_posts = 20usize;

    let (sphere, sphere_category_1, mut expected_post_vec) = create_sphere_with_posts(
        sphere_name,
        None,
        num_posts,
        Some((0..num_posts).map(|i| i as i32).collect()),
        (0..num_posts).map(|i| (i % 2) == 0).collect(),
        &mut user,
        &db_pool,
    ).await.expect("Should create sphere with posts");

    let sphere_category_2 = set_sphere_category(
        sphere_name,
        "a",
        Color::Red,
        "a",
        true,
        &user,
        &db_pool
    ).await.expect("Sphere category should be set");

    let category_map = HashMap::from([
        (sphere_category_1.category_id, sphere_category_1.clone()),
        (sphere_category_2.category_id, sphere_category_2.clone()),
    ]);

    let category_post_1 = create_post(
        sphere_name,
        None,
        "1",
        "1",
        None,
        Link::default(),
        PostTags::new(true, true, false, Some(sphere_category_2.category_id)),
        &user,
        &db_pool
    ).await.expect("Post 1 with category should be created.");

    let category_post_1 = set_post_score(category_post_1.post_id, -50, &db_pool).await.expect("Post score should be set.");

    expected_post_vec.push(PostWithSphereInfo::from_post(
        category_post_1,
        sphere_name.to_string(),
        Some(sphere_category_2.clone().into()),
        sphere.icon_url.clone())
    );

    // create post in satellite to make sure it doesn't get included in the results
    let satellite = create_satellite(
        sphere_name,
        "a",
        "satellite",
        false,
        false,
        false,
        &user,
        &db_pool,
    ).await.expect("Should be able to insert satellite.");

    create_post(
        sphere_name,
        Some(satellite.satellite_id),
        "satellite",
        "satellite",
        None,
        Link::default(),
        PostTags::default(),
        &user,
        &db_pool,
    ).await.expect("Should create satellite post.");

    let load_count = 15;
    for sort_type in POST_SORT_TYPE_ARRAY {
        sort_post_vec(&mut expected_post_vec, sort_type, true);
        let post_vec = get_post_vec_by_sphere_name(
            sphere_name,
            SphereCategoryFilter::All,
            SortType::Post(sort_type),
            load_count as i64,
            0,
            None,
            &db_pool,
        ).await.expect("First post vec should be loaded");


        let post_vec: Vec<PostWithSphereInfo> = post_vec.into_iter().map(|post| {
            let sphere_category = post.category_id.map(|category_id| {
                category_map.get(&category_id).expect("Category should be in map").clone().into()
            });
            PostWithSphereInfo::from_post(post, sphere_name.to_string(), sphere_category, sphere.icon_url.clone())
        }).collect();
        assert_eq!(post_vec, expected_post_vec[..load_count]);

        let second_post_vec = get_post_vec_by_sphere_name(
            sphere_name,
            SphereCategoryFilter::All,
            SortType::Post(sort_type),
            load_count as i64,
            load_count as i64,
            None,
            &db_pool,
        ).await?;

        let second_post_vec: Vec<PostWithSphereInfo> = second_post_vec.into_iter().map(|post| {
            let sphere_category = post.category_id.map(|category_id| {
                category_map.get(&category_id).expect("Category should be in map").clone().into()
            });
            PostWithSphereInfo::from_post(post, sphere_name.to_string(), sphere_category, sphere.icon_url.clone())
        }).collect();
        assert_eq!(second_post_vec, expected_post_vec[load_count..(num_posts + 1)]);
    }

    // Check that moderated and deleted posts are not returned
    let (moderated_post, deleted_post) = get_moderated_and_deleted_posts(sphere_name, &user, &db_pool).await;

    let post_vec = get_post_vec_by_sphere_name(
        sphere_name,
        SphereCategoryFilter::All,
        SortType::Post(PostSortType::Hot),
        num_posts as i64,
        0,
        None,
        &db_pool,
    ).await?;

    assert!(!post_vec.contains(&moderated_post.post));
    assert!(!post_vec.contains(&deleted_post.post));

    Ok(())
}

#[tokio::test]
async fn test_get_post_vec_by_sphere_name_with_pinned_post() -> Result<(), AppError> {
    let db_pool = get_db_pool().await;
    let mut user = create_test_user(&db_pool).await;

    let sphere_name = "sphere";
    let num_posts = 20usize;

    create_sphere_with_posts(
        sphere_name,
        Some("url"),
        num_posts,
        Some((0..num_posts).map(|i| i as i32).collect()),
        (0..num_posts).map(|i| (i % 2) == 0).collect(),
        &mut user,
        &db_pool,
    ).await.expect("Sphere with post should be created");
    let partial_load_num_post = num_posts / 2;

    let pinned_post = create_post(
        sphere_name,
        None,
        "pinned",
        "a",
        None,
        Link::default(),
        PostTags::new(false, false, true, None),
        &user,
        &db_pool
    ).await.expect("Pinned post should be created");

    for sort_type in POST_SORT_TYPE_ARRAY {
        let post_vec = get_post_vec_by_sphere_name(
            sphere_name,
            SphereCategoryFilter::All,
            SortType::Post(sort_type),
            partial_load_num_post as i64,
            0,
            None,
            &db_pool,
        ).await?;

        assert_eq!(post_vec.len(), partial_load_num_post);
        assert_eq!(post_vec.first(), Some(&pinned_post));
    }

    Ok(())
}

#[tokio::test]
async fn test_get_post_vec_by_sphere_name_with_category() -> Result<(), AppError> {
    let db_pool = get_db_pool().await;
    let mut user = create_test_user(&db_pool).await;

    let sphere_name = "sphere";
    let num_posts = 10usize;

    let (sphere, _, init_post_vec) = create_sphere_with_posts(
        sphere_name,
        None,
        num_posts,
        Some((0..num_posts).map(|i| i as i32).collect()),
        (0..num_posts).map(|i| (i % 2) == 0).collect(),
        &mut user,
        &db_pool,
    ).await.expect("Sphere with post should be created");
    let mut no_category_post_vec: Vec<PostWithSphereInfo> = init_post_vec.into_iter().filter(|post| {
        post.sphere_category.is_none()
    }).collect();

    let sphere_category = set_sphere_category(
        sphere_name,
        "a",
        Color::Green,
        "a",
        true,
        &user,
        &db_pool
    ).await.expect("Sphere category should be set.");

    let category_post_1 = create_post(
        sphere_name,
        None,
        "1",
        "1",
        None,
        Link::default(),
        PostTags::new(false, false, false, Some(sphere_category.category_id)),
        &user,
        &db_pool
    ).await.expect("Post 1 with category should be created.");

    let category_post_2 = create_post(
        sphere_name,
        None,
        "2",
        "2",
        None,
        Link::default(),
        PostTags::new(false, false, false, Some(sphere_category.category_id)),
        &user,
        &db_pool
    ).await.expect("Post 2 with category should be created.");
    let category_post_2 = set_post_score(
        category_post_2.post_id,
        50,
        &db_pool,
    ).await.expect("Post 2 score should be set");

    let mut expected_post_vec = vec![
        PostWithSphereInfo::from_post(category_post_1, sphere_name.to_string(), Some(sphere_category.clone().into()), sphere.icon_url.clone()),
        PostWithSphereInfo::from_post(category_post_2, sphere_name.to_string(), Some(sphere_category.clone().into()), sphere.icon_url.clone()),
    ];

    for sort_type in POST_SORT_TYPE_ARRAY {
        println!("Sort type: {:?}", sort_type);
        let category_post_vec = get_post_vec_by_sphere_name(
            sphere_name,
            SphereCategoryFilter::CategorySet(CategorySetFilter::new(sphere_category.category_id)),
            SortType::Post(sort_type),
            num_posts as i64,
            0,
            None,
            &db_pool,
        ).await?;
        let category_post_vec: Vec<PostWithSphereInfo> = category_post_vec.into_iter().map(|post| {
            let sphere_category = post.category_id.map(|_| sphere_category.clone().into());
            PostWithSphereInfo::from_post(post, sphere_name.to_string(), sphere_category, sphere.icon_url.clone())
        }).collect();
        sort_post_vec(&mut expected_post_vec, sort_type, true);
        assert_eq!(category_post_vec, expected_post_vec);
    }

    //
    for sort_type in POST_SORT_TYPE_ARRAY {
        println!("Sort type: {:?}", sort_type);
        let category_post_vec = get_post_vec_by_sphere_name(
            sphere_name,
            SphereCategoryFilter::CategorySet(CategorySetFilter {
                filters: HashSet::new(),
                only_category: false,
            }),
            SortType::Post(sort_type),
            num_posts as i64,
            0,
            None,
            &db_pool,
        ).await?;
        let result_post_vec: Vec<PostWithSphereInfo> = category_post_vec.into_iter().map(|post| {
            let sphere_category = post.category_id.map(|_| sphere_category.clone().into());
            PostWithSphereInfo::from_post(post, sphere_name.to_string(), sphere_category, sphere.icon_url.clone())
        }).collect();
        sort_post_vec(&mut no_category_post_vec, sort_type, true);
        assert_eq!(result_post_vec, no_category_post_vec);
    }

    let mut all_post_vec = no_category_post_vec.clone();
    all_post_vec.append(&mut expected_post_vec.clone());

    for sort_type in POST_SORT_TYPE_ARRAY {
        println!("Sort type: {:?}", sort_type);
        let category_post_vec = get_post_vec_by_sphere_name(
            sphere_name,
            SphereCategoryFilter::CategorySet(CategorySetFilter {
                filters: std::iter::once(sphere_category.category_id).collect(),
                only_category: false,
            }),
            SortType::Post(sort_type),
            num_posts as i64,
            0,
            None,
            &db_pool,
        ).await?;
        let category_post_vec: Vec<PostWithSphereInfo> = category_post_vec.into_iter().map(|post| {
            let sphere_category = post.category_id.map(|_| sphere_category.clone().into());
            PostWithSphereInfo::from_post(post, sphere_name.to_string(), sphere_category, sphere.icon_url.clone())
        }).collect();
        sort_post_vec(&mut all_post_vec, sort_type, true);
        assert_eq!(category_post_vec, all_post_vec);
    }

    Ok(())
}

#[tokio::test]
async fn test_get_post_vec_by_sphere_name_with_filters() {
    let db_pool = get_db_pool().await;
    let mut user = create_test_user(&db_pool).await;

    let num_post = 10;
    let (sphere, _, post_vec, new_spoiler_post, old_spoiler_post, nsfw_post) = create_sphere_with_filter_posts(
        "sphere",
        num_post,
        false,
        &mut user,
        &db_pool,
    ).await;

    let _ = create_sphere_with_posts(
        "sphere_2",
        None,
        num_post,
        None,
        Vec::new(),
        &mut user,
        &db_pool,
    ).await.expect("Sphere 2 with posts should be created");

    let mut default_filter_expected_post = post_vec.clone();
    default_filter_expected_post.push(new_spoiler_post.clone());
    default_filter_expected_post.push(old_spoiler_post.clone());

    for sort_type in POST_SORT_TYPE_ARRAY {
        let post_vec: Vec<PostWithSphereInfo> = get_post_vec_by_sphere_name(
            &sphere.sphere_name,
            SphereCategoryFilter::All,
            SortType::Post(sort_type),
            (2*num_post) as i64,
            0,
            Some(&user),
            &db_pool,
        )
            .await
            .expect("Should load posts by sphere name")
            .into_iter()
            .map(|post| PostWithSphereInfo::from_post(post, sphere.sphere_name.clone(), None, None)).collect();

        sort_post_vec(&mut default_filter_expected_post, sort_type, true);
        assert_eq!(post_vec, default_filter_expected_post);
    }

    user.days_hide_spoiler = Some(1);
    user.show_nsfw = false;
    let mut one_day_spoiler_filter_expected_post = post_vec.clone();
    one_day_spoiler_filter_expected_post.push(old_spoiler_post.clone());

    for sort_type in POST_SORT_TYPE_ARRAY {
        let post_vec: Vec<PostWithSphereInfo> = get_post_vec_by_sphere_name(
            &sphere.sphere_name,
            SphereCategoryFilter::All,
            SortType::Post(sort_type),
            (2*num_post) as i64,
            0,
            Some(&user),
            &db_pool,
        )
            .await
            .expect("Should load posts by sphere name")
            .into_iter()
            .map(|post| PostWithSphereInfo::from_post(post, sphere.sphere_name.clone(), None, None)).collect();
        sort_post_vec(&mut one_day_spoiler_filter_expected_post, sort_type, true);
        assert_eq!(post_vec, one_day_spoiler_filter_expected_post);
    }

    user.days_hide_spoiler = Some(3);
    user.show_nsfw = true;
    let mut three_day_spoiler_filter_expected_post = post_vec.clone();
    three_day_spoiler_filter_expected_post.push(nsfw_post.clone());

    for sort_type in POST_SORT_TYPE_ARRAY {
        let post_vec: Vec<PostWithSphereInfo> = get_post_vec_by_sphere_name(
            &sphere.sphere_name,
            SphereCategoryFilter::All,
            SortType::Post(sort_type),
            (2*num_post) as i64,
            0,
            Some(&user),
            &db_pool,
        )
            .await
            .expect("Should load posts by sphere name")
            .into_iter()
            .map(|post| PostWithSphereInfo::from_post(post, sphere.sphere_name.clone(), None, None)).collect();
        sort_post_vec(&mut three_day_spoiler_filter_expected_post, sort_type, true);
        assert_eq!(post_vec, three_day_spoiler_filter_expected_post);
    }
}

#[tokio::test]
async fn test_get_post_vec_by_satellite_id() -> Result<(), AppError> {
    let db_pool = get_db_pool().await;
    let mut user = create_test_user(&db_pool).await;

    let sphere_name = "sphere";
    let num_posts = 20usize;

    let (sphere, satellite_vec) = create_sphere_with_satellite_vec(
        sphere_name,
        2,
        &mut user,
        &db_pool,
    ).await.expect("Sphere with satellites should be created");

    let satellite_1 = satellite_vec.first().expect("Should have 1st satellite");
    let satellite_2 = satellite_vec.get(1).expect("Should have 2nd satellite");

    let sphere_category = set_sphere_category(
        sphere_name,
        "a",
        Color::Green,
        "a",
        true,
        &user,
        &db_pool
    ).await.expect("Sphere category should be set.");

    let mut expected_post_vec = create_posts(
        &sphere,
        Some(satellite_1.satellite_id),
        num_posts,
        Some((0..num_posts).map(|i| i as i32).collect()),
        Some(&sphere_category),
        (0..num_posts).map(|i| (i % 2) == 0).collect(),
        &user,
        &db_pool,
    ).await.expect("Should create satellite 1 posts");

    // Create other posts in sphere and other satellite to make sure we only get posts from satellite 1
    create_post(
        sphere_name,
        None,
        "1",
        "1",
        None,
        Link::default(),
        PostTags::default(),
        &user,
        &db_pool
    ).await.expect("Sphere post should be created.");

    create_posts(
        &sphere,
        Some(satellite_2.satellite_id),
        num_posts,
        Some((0..num_posts).map(|i| i as i32).collect()),
        None,
        (0..num_posts).map(|_| false).collect(),
        &user,
        &db_pool,
    ).await.expect("Should create satellite 2 posts");

    let load_count = 15;
    for sort_type in POST_SORT_TYPE_ARRAY {
        sort_post_vec(&mut expected_post_vec, sort_type, true);
        let post_vec = get_post_vec_by_satellite_id(
            satellite_1.satellite_id,
            None,
            SortType::Post(sort_type),
            load_count as i64,
            0,
            None,
            &db_pool,
        ).await.expect("First post vec should be loaded");

        let post_vec: Vec<PostWithSphereInfo> = post_vec.into_iter().map(|post| {
            let sphere_category = post.category_id.map(|_| {
                sphere_category.clone().into()
            });
            PostWithSphereInfo::from_post(post, sphere.sphere_name.clone(), sphere_category, sphere.icon_url.clone())
        }).collect();

        assert_eq!(post_vec, expected_post_vec[..load_count]);

        let second_post_vec = get_post_vec_by_satellite_id(
            satellite_1.satellite_id,
            None,
            SortType::Post(sort_type),
            load_count as i64,
            load_count as i64,
            None,
            &db_pool,
        ).await?;

        let second_post_vec: Vec<PostWithSphereInfo> = second_post_vec.into_iter().map(|post| {
            let sphere_category = post.category_id.map(|_| {
                sphere_category.clone().into()
            });
            PostWithSphereInfo::from_post(post, sphere.sphere_name.clone(), sphere_category, sphere.icon_url.clone())
        }).collect();
        assert_eq!(second_post_vec, expected_post_vec[load_count..num_posts]);
    }
    // Check that moderated and deleted posts are not returned
    let (moderated_post, deleted_post) = get_moderated_and_deleted_posts(sphere_name, &user, &db_pool).await;

    let post_vec = get_post_vec_by_satellite_id(
        satellite_1.satellite_id,
        None,
        SortType::Post(PostSortType::Hot),
        num_posts as i64,
        0,
        None,
        &db_pool,
    ).await?;

    assert!(!post_vec.contains(&moderated_post.post));
    assert!(!post_vec.contains(&deleted_post.post));

    Ok(())
}

#[tokio::test]
async fn test_get_post_vec_by_satellite_id_with_pinned_post() -> Result<(), AppError> {
    let db_pool = get_db_pool().await;
    let mut user = create_test_user(&db_pool).await;

    let sphere_name = "sphere";
    let num_posts = 20usize;

    let (sphere, satellite) = create_sphere_with_satellite(
        sphere_name,
        "test",
        false,
        false,
        &mut user,
        &db_pool,
    ).await.expect("Sphere with satellites should be created");

    let pinned_post = create_post(
        sphere_name,
        Some(satellite.satellite_id),
        "pinned",
        "a",
        None,
        Link::default(),
        PostTags::new(false, false, true, None),
        &user,
        &db_pool
    ).await.expect("Pinned post should be created");

    create_posts(
        &sphere,
        Some(satellite.satellite_id),
        num_posts,
        Some((0..num_posts).map(|i| i as i32).collect()),
        None,
        (0..num_posts).map(|_| false).collect(),
        &user,
        &db_pool,
    ).await.expect("Should create satellite posts");

    let load_count = 15;
    for sort_type in POST_SORT_TYPE_ARRAY {
        let post_vec = get_post_vec_by_satellite_id(
            satellite.satellite_id,
            None,
            SortType::Post(sort_type),
            load_count as i64,
            0,
            None,
            &db_pool,
        ).await.expect("First post vec should be loaded");

        assert_eq!(post_vec.len(), load_count);
        assert_eq!(post_vec.first(), Some(&pinned_post));
    }

    Ok(())
}

#[tokio::test]
async fn test_get_post_vec_by_satellite_id_with_category() -> Result<(), AppError> {
    let db_pool = get_db_pool().await;
    let mut user = create_test_user(&db_pool).await;

    let sphere_name = "sphere";
    let num_posts = 20usize;

    let (sphere, satellite) = create_sphere_with_satellite(
        sphere_name,
        "satellite",
        false,
        false,
        &mut user,
        &db_pool,
    ).await.expect("Sphere with satellite should be created");

    let sphere_category = set_sphere_category(
        sphere_name,
        "a",
        Color::Green,
        "a",
        true,
        &user,
        &db_pool
    ).await.expect("Sphere category should be set.");

    let other_sphere_category = set_sphere_category(
        sphere_name,
        "b",
        Color::Blue,
        "b",
        true,
        &user,
        &db_pool
    ).await.expect("Other sphere category should be set.");

    let category_post_1 = create_post(
        sphere_name,
        Some(satellite.satellite_id),
        "1",
        "1",
        None,
        Link::default(),
        PostTags::new(false, false, false, Some(sphere_category.category_id)),
        &user,
        &db_pool
    ).await.expect("Post 1 with category should be created.");

    let category_post_2 = create_post(
        sphere_name,
        Some(satellite.satellite_id),
        "2",
        "2",
        None,
        Link::default(),
        PostTags::new(false, false, false, Some(sphere_category.category_id)),
        &user,
        &db_pool
    ).await.expect("Post 2 with category should be created.");

    let mut expected_post_vec = vec![
        PostWithSphereInfo::from_post(category_post_1, sphere.sphere_name.clone(), Some(sphere_category.clone().into()), sphere.icon_url.clone()),
        PostWithSphereInfo::from_post(category_post_2, sphere.sphere_name.clone(), Some(sphere_category.clone().into()), sphere.icon_url.clone()),
    ];

    create_posts(
        &sphere,
        Some(satellite.satellite_id),
        num_posts,
        Some((0..num_posts).map(|i| i as i32).collect()),
        Some(&other_sphere_category),
        (0..num_posts).map(|i| (i % 2) == 0).collect(),
        &user,
        &db_pool,
    ).await.expect("Should create satellite 1 posts");

    for sort_type in POST_SORT_TYPE_ARRAY {
        let category_post_vec = get_post_vec_by_satellite_id(
            satellite.satellite_id,
            Some(sphere_category.category_id),
            SortType::Post(sort_type),
            num_posts as i64,
            0,
            None,
            &db_pool,
        ).await?;
        let category_post_vec: Vec<PostWithSphereInfo> = category_post_vec.into_iter().map(|post| {
            let sphere_category = post.category_id.map(|_| sphere_category.clone().into());
            PostWithSphereInfo::from_post(post, sphere.sphere_name.clone(), sphere_category, sphere.icon_url.clone())
        }).collect();
        sort_post_vec(&mut expected_post_vec, sort_type, true);
        assert_eq!(category_post_vec, expected_post_vec);
    }

    Ok(())
}

#[tokio::test]
async fn test_get_post_vec_by_satellite_id_with_filters() {
    let db_pool = get_db_pool().await;
    let mut user = create_test_user(&db_pool).await;

    let num_post = 10;
    let (
        sphere,
        satellite,
        post_vec,
        new_spoiler_post,
        old_spoiler_post,
        nsfw_post
    ) = create_sphere_with_filter_posts(
        "sphere",
        num_post,
        true,
        &mut user,
        &db_pool,
    ).await;

    create_posts(
        &sphere,
        None,
        num_post,
        None,
        None,
        Vec::new(),
        &user,
        &db_pool,
    ).await.expect("posts should be created.");

    let other_satellite = create_satellite(
        &sphere.sphere_name,
        "other_satellite",
        "satellite_body",
        false,
        false,
        false,
        &user,
        &db_pool,
    ).await.expect("Other satellite should be created");

    create_posts(
        &sphere,
        Some(other_satellite.satellite_id),
        num_post,
        None,
        None,
        Vec::new(),
        &user,
        &db_pool,
    ).await.expect("Other satellite posts should be created.");

    // create other posts that should not appear in result
    let _ = create_sphere_with_posts(
        "sphere_2",
        None,
        num_post,
        None,
        Vec::new(),
        &mut user,
        &db_pool,
    ).await.expect("Sphere 2 with posts should be created");

    let mut default_filter_expected_post = post_vec.clone();
    default_filter_expected_post.push(new_spoiler_post.clone());
    default_filter_expected_post.push(old_spoiler_post.clone());

    for sort_type in POST_SORT_TYPE_ARRAY {
        let post_vec: Vec<PostWithSphereInfo> = get_post_vec_by_satellite_id(
            satellite.satellite_id,
            None,
            SortType::Post(sort_type),
            (2*num_post) as i64,
            0,
            Some(&user),
            &db_pool,
        )
            .await
            .expect("Should load posts by satellite id")
            .into_iter()
            .map(|post| PostWithSphereInfo::from_post(post, sphere.sphere_name.clone(), None, None)).collect();

        sort_post_vec(&mut default_filter_expected_post, sort_type, true);
        assert_eq!(post_vec, default_filter_expected_post);
    }

    user.days_hide_spoiler = Some(1);
    user.show_nsfw = false;
    let mut one_day_spoiler_filter_expected_post = post_vec.clone();
    one_day_spoiler_filter_expected_post.push(old_spoiler_post.clone());

    for sort_type in POST_SORT_TYPE_ARRAY {
        let post_vec: Vec<PostWithSphereInfo> = get_post_vec_by_satellite_id(
            satellite.satellite_id,
            None,
            SortType::Post(sort_type),
            (2*num_post) as i64,
            0,
            Some(&user),
            &db_pool,
        )
            .await
            .expect("Should load posts by satellite id")
            .into_iter()
            .map(|post| PostWithSphereInfo::from_post(post, sphere.sphere_name.clone(), None, None)).collect();
        sort_post_vec(&mut one_day_spoiler_filter_expected_post, sort_type, true);
        assert_eq!(post_vec, one_day_spoiler_filter_expected_post);
    }

    user.days_hide_spoiler = Some(3);
    user.show_nsfw = true;
    let mut three_day_spoiler_filter_expected_post = post_vec.clone();
    three_day_spoiler_filter_expected_post.push(nsfw_post.clone());

    for sort_type in POST_SORT_TYPE_ARRAY {
        let post_vec: Vec<PostWithSphereInfo> = get_post_vec_by_satellite_id(
            satellite.satellite_id,
            None,
            SortType::Post(sort_type),
            (2*num_post) as i64,
            0,
            Some(&user),
            &db_pool,
        )
            .await
            .expect("Should load posts by satellite id")
            .into_iter()
            .map(|post| PostWithSphereInfo::from_post(post, sphere.sphere_name.clone(), None, None)).collect();
        sort_post_vec(&mut three_day_spoiler_filter_expected_post, sort_type, true);
        assert_eq!(post_vec, three_day_spoiler_filter_expected_post);
    }
}

#[tokio::test]
async fn test_get_homepage_post_vec() {
    let db_pool = get_db_pool().await;
    let mut user = create_test_user(&db_pool).await;
    let mut other_user = create_user("other", &db_pool).await;

    let num_posts = 10;

    let (sphere_1, _, mut sphere_1_post_vec) = create_sphere_with_posts(
        "1",
        Some("url"),
        num_posts,
        Some((0..num_posts).map(|i| i as i32).collect()),
        (0..num_posts).map(|i| (i % 2) == 0).collect(),
        &mut user,
        &db_pool,
    ).await.expect("Sphere 1 with posts should be created");

    let (_, _, mut sphere_2_post_vec) = create_sphere_with_posts(
        "2",
        None,
        num_posts,
        Some((0..num_posts).map(|i| 20 + i as i32).collect()),
        (0..num_posts).map(|i| (i % 2) == 0).collect(),
        &mut other_user,
        &db_pool,
    ).await.expect("Sphere 2 with posts should be created");

    let mut combined_post_vec = sphere_1_post_vec.clone();
    combined_post_vec.append(&mut sphere_2_post_vec.clone());

    subscribe(sphere_1.sphere_id, user.user_id, &db_pool).await.expect("User should subscribe to sphere 1");

    for sort_type in POST_SORT_TYPE_ARRAY {
        let subscribed_post_vec = get_homepage_post_vec(
            SortType::Post(sort_type),
            0,
            Some(&user),
            &db_pool,
        )
            .await
            .expect("Should load subscribed posts");
        sort_post_vec(&mut sphere_1_post_vec, sort_type, false);
        sort_post_vec(&mut sphere_2_post_vec, sort_type, false);
        assert_eq!(subscribed_post_vec[0..num_posts], sphere_1_post_vec);
        assert_eq!(subscribed_post_vec[num_posts..], sphere_2_post_vec);

        let anonymous_post_vec = get_homepage_post_vec(
            SortType::Post(sort_type),
            0,
            None,
            &db_pool,
        )
            .await
            .expect("Should load anonymous posts");
        sort_post_vec(&mut combined_post_vec, sort_type, false);
        assert_eq!(anonymous_post_vec, combined_post_vec);
    }
}

#[tokio::test]
async fn test_create_post_and_vote() {
    let db_pool = get_db_pool().await;
    let mut user = create_test_user(&db_pool).await;

    let (sphere, satellite) = create_sphere_with_satellite("a", "satellite", false, false, &mut user, &db_pool).await.expect("Should create sphere");
    let post_1_title = "1";
    let post_1_body = "test";
    let post_1_location = PostLocation {
        sphere: sphere.sphere_name.clone(),
        satellite_id: None,
    };
    let post_1_inputs = PostDataInputs {
        title: post_1_title.to_string(),
        body: post_1_body.to_string(),
        is_markdown: false,
        embed_type: Default::default(),
        link: None,
        post_tags: Default::default(),
    };

    let (post_1, vote_1, post_1_path) = create_post_and_vote(
        post_1_location,
        post_1_inputs,
        &user,
        &db_pool,
    ).await.expect("Should create post 1 and vote");

    let vote_1 = vote_1.expect("Vote 1 should be some");

    let expected_post_1 = get_post_by_id(post_1.post_id, &db_pool).await.expect("Should load post by id");
    let expected_vote_1 = get_user_post_vote(post_1.post_id, user.user_id, &db_pool).await.expect("Should load vote by id");

    assert_eq!(post_1.title, expected_post_1.title);
    assert_eq!(post_1.body, expected_post_1.body);
    assert_eq!(post_1.sphere_id, expected_post_1.sphere_id);
    assert_eq!(post_1.satellite_id, expected_post_1.satellite_id);
    assert_eq!(expected_post_1.score, 1);

    assert_eq!(post_1.title, post_1_title);
    assert_eq!(post_1.body, post_1_body);
    assert_eq!(post_1.markdown_body, None);
    assert_eq!(post_1.sphere_id, sphere.sphere_id);
    assert_eq!(post_1.satellite_id, None);

    assert_eq!(vote_1, expected_vote_1);
    assert_eq!(vote_1.post_id, post_1.post_id);
    assert_eq!(vote_1.comment_id, None);
    assert_eq!(vote_1.user_id, user.user_id);
    assert_eq!(vote_1.value, VoteValue::Up);

    assert_eq!(post_1_path, get_post_path(&sphere.sphere_name, None, post_1.post_id));

    let post_2_title = "2";
    let post_2_body = "test_2";
    let (post_2_html_body, post_2_markdown_body) = get_html_and_markdown_strings(post_2_body, true).expect("Should get html body");
    let post_2_location = PostLocation {
        sphere: sphere.sphere_name.clone(),
        satellite_id: Some(satellite.satellite_id),
    };
    let post_2_inputs = PostDataInputs {
        title: post_2_title.to_string(),
        body: post_2_body.to_string(),
        is_markdown: true,
        embed_type: Default::default(),
        link: None,
        post_tags: Default::default(),
    };

    let (post_2, vote_2, post_path) = create_post_and_vote(
        post_2_location,
        post_2_inputs,
        &user,
        &db_pool,
    ).await.expect("Should create post 2 and vote");

    let vote_2 = vote_2.expect("Vote 2 should be some");

    let expected_post_2 = get_post_by_id(post_2.post_id, &db_pool).await.expect("Should load post by id");
    let expected_vote_2 = get_user_post_vote(post_2.post_id, user.user_id, &db_pool).await.expect("Should load vote by id");

    assert_eq!(post_2.title, expected_post_2.title);
    assert_eq!(post_2.body, expected_post_2.body);
    assert_eq!(post_2.sphere_id, expected_post_2.sphere_id);
    assert_eq!(post_2.satellite_id, expected_post_2.satellite_id);
    assert_eq!(expected_post_2.score, 1);

    assert_eq!(post_2.title, post_2_title);
    assert_eq!(post_2.body, post_2_html_body);
    assert_eq!(post_2.markdown_body.as_deref(), post_2_markdown_body);
    assert_eq!(post_2.sphere_id, sphere.sphere_id);
    assert_eq!(post_2.satellite_id, Some(satellite.satellite_id));

    assert_eq!(vote_2, expected_vote_2);
    assert_eq!(vote_2.post_id, post_2.post_id);
    assert_eq!(vote_2.comment_id, None);
    assert_eq!(vote_2.user_id, user.user_id);
    assert_eq!(vote_2.value, VoteValue::Up);

    assert_eq!(post_path, get_post_path(&sphere.sphere_name, Some(satellite.satellite_id), post_2.post_id));
}

#[tokio::test]
async fn test_create_post() -> Result<(), AppError> {
    let db_pool = get_db_pool().await;
    let user = create_test_user(&db_pool).await;

    let sphere_1 = create_sphere("a", "sphere", false, &user, &db_pool).await?;
    let sphere_2 = create_sphere("b", "sphere", true, &user, &db_pool).await?;

    let post_1_title = "1";
    let post_1_body = "test";
    let post_1 = create_post(
        &sphere_1.sphere_name,
        None,
        post_1_title,
        post_1_body,
        None,
        Link::default(),
        PostTags::default(),
        &user,
        &db_pool
    ).await.expect("Should be able to create post 1.");

    assert_eq!(post_1.title, post_1_title);
    assert_eq!(post_1.body, post_1_body);
    assert_eq!(post_1.markdown_body, None);
    assert_eq!(post_1.link, Link::default());
    assert_eq!(post_1.is_nsfw, false);
    assert_eq!(post_1.is_spoiler, false);
    assert_eq!(post_1.category_id, None);
    assert_eq!(post_1.is_edited, false);
    assert_eq!(post_1.sphere_id, sphere_1.sphere_id);
    assert_eq!(post_1.satellite_id, None);
    assert_eq!(post_1.creator_id, user.user_id);
    assert_eq!(post_1.creator_name, user.username);
    assert_eq!(post_1.is_creator_moderator, false); // user not refreshed yet
    assert_eq!(post_1.moderator_message, None);
    assert_eq!(post_1.infringed_rule_id, None);
    assert_eq!(post_1.infringed_rule_title, None);
    assert_eq!(post_1.moderator_id, None);
    assert_eq!(post_1.moderator_name, None);
    assert_eq!(post_1.num_comments, 0);
    assert_eq!(post_1.is_pinned, false);
    assert_eq!(post_1.score, 0);
    assert_eq!(post_1.delete_timestamp, None);

    // cannot create pinned comment without moderator permissions (need to reload user to actualize them)
    assert_eq!(
        create_post(&sphere_1.sphere_name, None, post_1_title, post_1_body, None, Link::default(), PostTags::new(false, false, true, None), &user, &db_pool).await,
        Err(AppError::InsufficientPrivileges),
    );

    let user = User::get(user.user_id, &db_pool).await.expect("User should be reloaded.");
    let post_2_title = "1";
    let post_2_body = "test";
    let post_2_markdown_body = "test";
    let post_2_link = Link::new(
        LinkType::Link,
        Some(String::from("link")),
        None,
        Some(String::from("thumbnail"))
    );
    let post_2 = create_post(
        &sphere_1.sphere_name,
        None,
        post_2_title,
        post_2_body,
        Some(post_2_markdown_body),
        post_2_link.clone(),
        PostTags::new(true, true, true, None),
        &user,
        &db_pool
    ).await.expect("Should be able to create post 2.");

    assert_eq!(post_2.title, post_2_title);
    assert_eq!(post_2.body, post_2_body);
    assert_eq!(post_2.markdown_body, Some(String::from(post_2_markdown_body)));
    assert_eq!(post_2.link, post_2_link);
    assert_eq!(post_2.markdown_body, Some(String::from(post_2_markdown_body)));
    assert_eq!(post_2.is_nsfw, true);
    assert_eq!(post_2.is_spoiler, true);
    assert_eq!(post_2.category_id, None);
    assert_eq!(post_2.is_edited, false);
    assert_eq!(post_2.sphere_id, sphere_1.sphere_id);
    assert_eq!(post_2.satellite_id, None);
    assert_eq!(post_2.creator_id, user.user_id);
    assert_eq!(post_2.creator_name, user.username);
    assert_eq!(post_2.is_creator_moderator, true);
    assert_eq!(post_2.moderator_message, None);
    assert_eq!(post_2.infringed_rule_id, None);
    assert_eq!(post_2.infringed_rule_title, None);
    assert_eq!(post_2.moderator_id, None);
    assert_eq!(post_2.moderator_name, None);
    assert_eq!(post_2.num_comments, 0);
    assert_eq!(post_2.is_pinned, true);
    assert_eq!(post_2.score, 0);
    assert_eq!(post_2.delete_timestamp, None);

    let nsfw_post_title = "1";
    let nsfw_post_body = "test";
    let nsfw_link = Link::new(
        LinkType::Image,
        Some(String::from("image")),
        Some(String::from("embed")),
        Some(String::from("thumbnail")),
    );
    let nsfw_post = create_post(
        &sphere_2.sphere_name, 
        None, 
        nsfw_post_title, 
        nsfw_post_body, 
        None,
        nsfw_link.clone(),
        PostTags::default(),
        &user, 
        &db_pool
    ).await.expect("Should be able to create nsfw post.");

    assert_eq!(nsfw_post.title, nsfw_post_title);
    assert_eq!(nsfw_post.body, nsfw_post_body);
    assert_eq!(nsfw_post.markdown_body, None);
    assert_eq!(nsfw_post.link, nsfw_link);
    assert_eq!(nsfw_post.is_nsfw, true);
    assert_eq!(nsfw_post.is_spoiler, false);
    assert_eq!(nsfw_post.category_id, None);
    assert_eq!(nsfw_post.is_edited, false);
    assert_eq!(nsfw_post.sphere_id, sphere_2.sphere_id);
    assert_eq!(nsfw_post.satellite_id, None);
    assert_eq!(nsfw_post.creator_id, user.user_id);
    assert_eq!(nsfw_post.creator_name, user.username);
    assert_eq!(nsfw_post.is_creator_moderator, true);
    assert_eq!(nsfw_post.moderator_message, None);
    assert_eq!(nsfw_post.infringed_rule_id, None);
    assert_eq!(nsfw_post.infringed_rule_title, None);
    assert_eq!(nsfw_post.moderator_id, None);
    assert_eq!(nsfw_post.moderator_name, None);
    assert_eq!(nsfw_post.num_comments, 0);
    assert_eq!(nsfw_post.is_pinned, false);
    assert_eq!(nsfw_post.score, 0);
    assert_eq!(nsfw_post.delete_timestamp, None);

    let post_1_with_info = get_post_with_info_by_id(post_1.post_id, None, &db_pool).await.expect("Should be able to load post 1.");

    assert_eq!(post_1_with_info.post, post_1);
    assert_eq!(post_1_with_info.vote, None);

    let post_2_with_info = get_post_with_info_by_id(post_2.post_id, None, &db_pool).await.expect("Should be able to load post 2.");

    assert_eq!(post_2_with_info.post, post_2);
    assert_eq!(post_2_with_info.vote, None);

    let nsfw_post_with_info = get_post_with_info_by_id(nsfw_post.post_id, None, &db_pool).await.expect("Should be able to load post 2.");

    assert_eq!(nsfw_post_with_info.post, nsfw_post);
    assert_eq!(nsfw_post_with_info.vote, None);

    Ok(())
}

#[tokio::test]
async fn test_create_post_in_satellite() -> Result<(), AppError> {
    let db_pool = get_db_pool().await;
    let mut user = create_test_user(&db_pool).await;

    let (sphere_1, satellite_1) = create_sphere_with_satellite(
        "1",
        "1",
        false,
        false,
        &mut user,
        &db_pool
    ).await.expect("Should be able to create sphere.");

    let post = create_post(
        &sphere_1.sphere_name,
        Some(satellite_1.satellite_id),
        "1",
        "1",
        None,
        Link::default(),
        PostTags::default(),
        &user,
        &db_pool
    ).await.expect("Should be able to create post in satellite 1.");

    assert_eq!(post.title, "1");
    assert_eq!(post.body, "1");
    assert_eq!(post.markdown_body, None);
    assert_eq!(post.link, Link::default());
    assert_eq!(post.is_nsfw, false);
    assert_eq!(post.is_spoiler, false);
    assert_eq!(post.category_id, None);
    assert_eq!(post.is_edited, false);
    assert_eq!(post.sphere_id, sphere_1.sphere_id);
    assert_eq!(post.satellite_id, Some(satellite_1.satellite_id));
    assert_eq!(post.creator_id, user.user_id);
    assert_eq!(post.creator_name, user.username);
    assert_eq!(post.is_creator_moderator, true);
    assert_eq!(post.moderator_message, None);
    assert_eq!(post.infringed_rule_id, None);
    assert_eq!(post.infringed_rule_title, None);
    assert_eq!(post.moderator_id, None);
    assert_eq!(post.moderator_name, None);
    assert_eq!(post.num_comments, 0);
    assert_eq!(post.is_pinned, false);
    assert_eq!(post.score, 0);
    assert_eq!(post.delete_timestamp, None);

    let (sphere_2, satellite_2) = create_sphere_with_satellite(
        "2",
        "2",
        true,
        true,
        &mut user,
        &db_pool
    ).await.expect("Should be able to create sphere.");

    let link = Link::new(
        LinkType::Video,
        Some(String::from("link")),
        Some(String::from("embed")),
        None,
    );

    let post = create_post(
        &sphere_2.sphere_name,
        Some(satellite_2.satellite_id),
        "2",
        "2",
        None,
        link.clone(),
        PostTags::new(false, false, true, None),
        &user,
        &db_pool
    ).await.expect("Should be able to create post in satellite 2.");

    assert_eq!(post.title, "2");
    assert_eq!(post.body, "2");
    assert_eq!(post.markdown_body, None);
    assert_eq!(post.link, link);
    assert_eq!(post.is_nsfw, true);
    assert_eq!(post.is_spoiler, true);
    assert_eq!(post.category_id, None);
    assert_eq!(post.is_edited, false);
    assert_eq!(post.sphere_id, sphere_2.sphere_id);
    assert_eq!(post.satellite_id, Some(satellite_2.satellite_id));
    assert_eq!(post.creator_id, user.user_id);
    assert_eq!(post.creator_name, user.username);
    assert_eq!(post.is_creator_moderator, true);
    assert_eq!(post.moderator_message, None);
    assert_eq!(post.infringed_rule_id, None);
    assert_eq!(post.infringed_rule_title, None);
    assert_eq!(post.moderator_id, None);
    assert_eq!(post.moderator_name, None);
    assert_eq!(post.num_comments, 0);
    assert_eq!(post.is_pinned, true);
    assert_eq!(post.score, 0);
    assert_eq!(post.delete_timestamp, None);

    // cannot create post for non-existent satellite
    assert!(
        matches!(
            create_post(
                &sphere_1.sphere_name,
                Some(-1),
                "a",
                "b",
                None,
                Link::default(),
                PostTags::default(),
                &user,
                &db_pool
            ).await,
            Err(AppError::DatabaseError(_))
        )
    );

    Ok(())
}

#[tokio::test]
async fn test_edit_post() {
    let db_pool = get_db_pool().await;
    let mut user = create_test_user(&db_pool).await;

    let (sphere, post) = create_sphere_with_post("a", &mut user, &db_pool).await;

    let post_inputs_vec = vec![
        PostDataInputs {
            title: String::from("1"),
            body: String::from("test"),
            is_markdown: false,
            embed_type: Default::default(),
            link: None,
            post_tags: Default::default(),
        },
        PostDataInputs {
            title: String::from("2"),
            body: String::from("markdown_body"),
            is_markdown: true,
            embed_type: Default::default(),
            link: None,
            post_tags: PostTags {
                is_spoiler: true,
                is_nsfw: true,
                is_pinned: true,
                category_id: None,
            },
        },
    ];

    for post_inputs in post_inputs_vec {
        let post = edit_post(
            post.post_id,
            post_inputs.clone(),
            &user,
            &db_pool,
        ).await.expect("Should create post 1 and vote");

        let expected_post = get_post_by_id(post.post_id, &db_pool).await.expect("Should load post by id");

        let (expected_body, expected_markdown_body) = get_html_and_markdown_strings(&post_inputs.body, post_inputs.is_markdown).expect("Should get expected body");

        assert_eq!(post, expected_post);
        assert_eq!(post.title, post_inputs.title);
        assert_eq!(post.body, expected_body);
        assert_eq!(post.markdown_body.as_deref(), expected_markdown_body);
        assert_eq!(post.link, Link::default());
        assert_eq!(post.is_pinned, post_inputs.post_tags.is_pinned);
        assert_eq!(post.is_spoiler, post_inputs.post_tags.is_spoiler);
        assert_eq!(post.is_nsfw, post_inputs.post_tags.is_nsfw);
        assert_eq!(post.sphere_id, sphere.sphere_id);
    }
}

#[tokio::test]
async fn test_update_post() -> Result<(), AppError> {
    let db_pool = get_db_pool().await;
    let mut user = create_test_user(&db_pool).await;

    let (sphere, post) = create_sphere_with_post(
        "sphere",
        &mut user,
        &db_pool,
    ).await;
    
    let nsfw_sphere = create_sphere(
        "nsfw",
        "nsfw",
        true,
        &user,
        &db_pool,
    ).await?;

    let updated_title = "updated post";
    let updated_markdown_body = "# Here is a post with markdown";
    let updated_html_body = get_styled_html_from_markdown(updated_markdown_body).expect("Should get html from markdown.");
    let updated_link = Link::new(
        LinkType::Rich,
        Some(String::from("updated_link")),
        Some(String::from("embed")),
        Some(String::from("thumbnail")),
    );
    let updated_post = update_post(
        post.post_id,
        updated_title,
        &updated_html_body,
        Some(updated_markdown_body),
        updated_link.clone(),
        PostTags::default(),
        &user,
        &db_pool
    ).await?;

    assert_eq!(updated_post.title, updated_title);
    assert_eq!(updated_post.body, updated_html_body);
    assert_eq!(updated_post.markdown_body, Some(String::from(updated_markdown_body)));
    assert_eq!(updated_post.link, updated_link);
    assert!(
        updated_post.edit_timestamp.is_some() &&
        updated_post.edit_timestamp.unwrap() > updated_post.create_timestamp &&
        updated_post.create_timestamp == post.create_timestamp
    );
    assert_eq!(updated_post.delete_timestamp, None);

    let nsfw_sphere_post = create_post(
        &nsfw_sphere.sphere_name,
        None,
        "post",
        "body",
        None,
        Link::new(
            LinkType::Image,
            Some(String::from("link")),
            Some(String::from("embed")),
            Some(String::from("thumbnail")),
        ),
        PostTags::new(false, true, false, None),
        &user,
        &db_pool,
    ).await?;

    let updated_nsfw_post = update_post(
        nsfw_sphere_post.post_id,
        updated_title,
        &updated_html_body,
        Some(updated_markdown_body),
        Link::default(),
        PostTags::default(),
        &user,
        &db_pool
    ).await?;

    assert_eq!(updated_nsfw_post.title, updated_title);
    assert_eq!(updated_nsfw_post.body, updated_html_body);
    assert_eq!(updated_nsfw_post.markdown_body, Some(String::from(updated_markdown_body)));
    assert_eq!(updated_nsfw_post.link, Link::default());
    // a post in a nsfw sphere is always nsfw, input of the update is ignored
    assert_eq!(updated_nsfw_post.is_nsfw, true);
    assert!(
        updated_nsfw_post.edit_timestamp.is_some() &&
            updated_nsfw_post.edit_timestamp.unwrap() > updated_nsfw_post.create_timestamp &&
            updated_nsfw_post.create_timestamp == nsfw_sphere_post.create_timestamp
    );
    assert_eq!(updated_nsfw_post.delete_timestamp, None);

    // Cannot update moderator post
    let rule = add_rule(&sphere.sphere_name, 0, "1", "2", false, &user, &db_pool).await.expect("Should add rule");
    moderate_post(post.post_id, rule.rule_id, "reason", &user, &db_pool).await.expect("Should moderate post.");
    assert_eq!(
        update_post(
            post.post_id,
            updated_title,
            &updated_html_body,
            Some(updated_markdown_body),
            Link::default(),
            PostTags::default(),
            &user,
            &db_pool
        ).await,
        Err(AppError::NotFound),
    );

    // Cannot update deleted post
    delete_post(updated_nsfw_post.post_id, &user, &db_pool).await.expect("Post should be deleted.");
    assert_eq!(
        update_post(
            nsfw_sphere_post.post_id,
            updated_title,
            &updated_html_body,
            Some(updated_markdown_body),
            Link::default(),
            PostTags::default(),
            &user,
            &db_pool
        ).await,
        Err(AppError::NotFound),
    );

    Ok(())
}

#[tokio::test]
async fn test_update_post_in_satellite() -> Result<(), AppError> {
    let db_pool = get_db_pool().await;
    let mut user = create_test_user(&db_pool).await;

    let (sphere_1, satellite_1) = create_sphere_with_satellite(
        "1",
        "1",
        false,
        false,
        &mut user,
        &db_pool
    ).await.expect("Should be able to create sphere");

    let post = create_post(
        &sphere_1.sphere_name,
        Some(satellite_1.satellite_id),
        "1",
        "1",
        None,
        Link::default(),
        PostTags::new(true, true, false, None),
        &user,
        &db_pool
    ).await.expect("Should be able to create post in");

    let updated_title = "updated post";
    let updated_markdown_body = "# Here is a post with markdown";
    let updated_html_body = get_styled_html_from_markdown(updated_markdown_body).expect("Should get html from markdown");
    let updated_link = Link::new(
        LinkType::Video,
        Some(String::from("updated_link")),
        None,
        None,
    );
    
    let updated_post = update_post(
        post.post_id,
        updated_title,
        &updated_html_body,
        Some(updated_markdown_body),
        updated_link.clone(),
        PostTags::new(false, false, true, None),
        &user,
        &db_pool
    ).await.expect("Should be able to update post");

    assert_eq!(updated_post.title, updated_title);
    assert_eq!(updated_post.body, updated_html_body);
    assert_eq!(updated_post.markdown_body, Some(String::from(updated_markdown_body)));
    assert_eq!(updated_post.link, updated_link);
    assert_eq!(updated_post.satellite_id, Some(satellite_1.satellite_id));
    assert_eq!(updated_post.is_spoiler, false);
    assert_eq!(updated_post.is_nsfw, false);
    assert_eq!(updated_post.is_pinned, true);
    assert!(
        updated_post.edit_timestamp.is_some() &&
            updated_post.edit_timestamp.unwrap() > updated_post.create_timestamp &&
            updated_post.create_timestamp == post.create_timestamp
    );
    assert_eq!(updated_post.delete_timestamp, None);

    let (sphere_2, satellite_2) = create_sphere_with_satellite(
        "2",
        "2",
        true,
        true,
        &mut user,
        &db_pool
    ).await.expect("Should be able to create sphere");

    let post = create_post(
        &sphere_2.sphere_name,
        Some(satellite_2.satellite_id),
        "2",
        "2",
        None,
        Link::default(),
        PostTags::new(true, true, false, None),
        &user,
        &db_pool
    ).await.expect("Should be able to create post in");

    let updated_post = update_post(
        post.post_id,
        updated_title,
        &updated_html_body,
        Some(updated_markdown_body),
        Link::default(),
        PostTags::default(),
        &user,
        &db_pool
    ).await.expect("Should be able to update post");

    assert_eq!(updated_post.title, updated_title);
    assert_eq!(updated_post.body, updated_html_body);
    assert_eq!(updated_post.markdown_body, Some(String::from(updated_markdown_body)));
    assert_eq!(updated_post.link, Link::default());
    assert_eq!(updated_post.satellite_id, Some(satellite_2.satellite_id));
    assert_eq!(updated_post.is_spoiler, true);
    assert_eq!(updated_post.is_nsfw, true);
    assert_eq!(updated_post.is_pinned, false);
    assert!(
        updated_post.edit_timestamp.is_some() &&
            updated_post.edit_timestamp.unwrap() > updated_post.create_timestamp &&
            updated_post.create_timestamp == post.create_timestamp
    );
    assert_eq!(updated_post.delete_timestamp, None);

    Ok(())
}

#[tokio::test]
async fn test_delete_post() {
    let db_pool = get_db_pool().await;
    let mut user = create_test_user(&db_pool).await;

    let sphere_name = "sphere";
    create_sphere_with_satellite(
        sphere_name,
        "satellite",
        false,
        true,
        &mut user,
        &db_pool,
    ).await.expect("Should create sphere");

    let post = create_post(
        sphere_name,
        None,
        "post",
        "body",
        Some("markdown_body"),
        Link::new(
            LinkType::Rich,
            Some(String::from("link")),
            Some(String::from("embed")),
            Some(String::from("thumbnail")),
        ),
        PostTags::new(true, true, true, None),
        &user,
        &db_pool,
    ).await.expect("Should create post");

    let deleted_post = delete_post(
        post.post_id,
        &user,
        &db_pool
    ).await.expect("Should delete post");

    assert_eq!(deleted_post.post_id, deleted_post.post_id);
    assert_eq!(deleted_post.satellite_id, deleted_post.satellite_id);
    assert_eq!(deleted_post.body, "");
    assert_eq!(deleted_post.markdown_body, None);
    assert_eq!(deleted_post.link, Link::default());
    assert_eq!(deleted_post.creator_id, user.user_id);
    assert_eq!(deleted_post.creator_name, "");
    assert_eq!(deleted_post.is_spoiler, false);
    assert_eq!(deleted_post.is_nsfw, false);
    assert_eq!(deleted_post.is_pinned, false);
    assert!(
        deleted_post.edit_timestamp.is_some() &&
            deleted_post.edit_timestamp.unwrap() > deleted_post.create_timestamp &&
            deleted_post.create_timestamp == post.create_timestamp
    );
    assert!(
        deleted_post.delete_timestamp.is_some() &&
            deleted_post.delete_timestamp.unwrap() > deleted_post.create_timestamp
    );

    let rule = add_rule(sphere_name, 0, "1", "2", false, &user, &db_pool).await.expect("Should add rule");
    let post = create_simple_post(sphere_name, None, "a", "b", None, &user, &db_pool).await;
    let post = moderate_post(post.post.post_id, rule.rule_id, "reason", &user, &db_pool).await.expect("Should moderate post.");
    assert_eq!(
        delete_post(
            post.post_id,
            &user,
            &db_pool
        ).await,
        Err(AppError::NotFound),
    );
}

#[tokio::test]
async fn increment_post_comment_count() -> Result<(), AppError> {
    let db_pool = get_db_pool().await;
    let mut user = create_test_user(&db_pool).await;

    let (_, post) = create_sphere_with_post("sphere", &mut user, &db_pool).await;
    assert_eq!(post.num_comments, 0);

    let _comment = create_comment(post.post_id, None, "a", None, false, &user, &db_pool).await.expect("Should create comment.");

    let post = get_post_by_id(post.post_id, &db_pool).await.expect("Should get post.");
    assert_eq!(post.num_comments, 1);

    Ok(())
}

#[tokio::test]
async fn test_update_post_scores() -> Result<(), AppError> {
    let db_pool = get_db_pool().await;
    let mut user = create_test_user(&db_pool).await;

    let (_, post) = create_sphere_with_post("sphere", &mut user, &db_pool).await;
    let post = set_post_score(post.post_id, 10, &db_pool).await.expect("Post score should be set.");

    // wait to have a meaningful difference in scores after update
    tokio::time::sleep(Duration::from_secs(2)).await;

    update_post_scores(&db_pool).await.expect("Post scores should be updatable.");

    let updated_post = get_post_with_info_by_id(post.post_id, None, &db_pool).await.expect("Should be able to get updated post.");

    test_post_score(&post);
    test_post_score(&updated_post.post);
    assert_eq!(post.score, updated_post.post.score);
    assert_eq!(post.create_timestamp, updated_post.post.create_timestamp);
    assert!(post.scoring_timestamp < updated_post.post.scoring_timestamp);
    assert!(post.recommended_score > updated_post.post.recommended_score);
    assert!(post.trending_score > updated_post.post.trending_score);

    Ok(())
}

#[tokio::test]
async fn test_post_scores() -> Result<(), AppError> {
    let db_pool = get_db_pool().await;
    let mut user = create_test_user(&db_pool).await;

    let (_, post) = create_sphere_with_post("sphere", &mut user, &db_pool).await;

    let mut rng = rand::rng();

    // wait to have a meaningful impact of elapsed time on the score
    tokio::time::sleep(Duration::from_secs(2)).await;

    set_post_score(post.post_id, rng.random_range(-100..101), &db_pool).await?;

    let post_with_vote = get_post_with_info_by_id(post.post_id, Some(&user), &db_pool).await?;

    test_post_score(&post_with_vote.post);
    Ok(())
}
