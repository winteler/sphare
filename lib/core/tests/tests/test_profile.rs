use sphare_core_common::errors::AppError;
use sphare_core_content::comment::ssr::create_comment;
use sphare_core_content::comment::CommentWithContext;
use sphare_core_content::embed::Link;
use sphare_core_content::post::ssr::create_post;
use sphare_core_content::post::{PostTags, PostWithSphereInfo};
use sphare_core_content::profile::ssr::{get_user_comment_vec, get_user_post_vec};
use sphare_core_content::ranking::{CommentSortType, PostSortType, SortType, VoteValue};
use sphare_core_sphere::satellite::ssr::create_satellite;

use crate::common::{create_user, get_db_pool};
use crate::data_factory::{create_post_with_comments, create_sphere_with_post_and_comment, create_sphere_with_posts, get_moderated_and_deleted_comments, get_moderated_and_deleted_posts, set_comment_score, set_post_score};
use crate::utils::{sort_comment_vec, sort_post_vec, COMMENT_SORT_TYPE_ARRAY, POST_SORT_TYPE_ARRAY};

mod common;
mod data_factory;
mod utils;

#[tokio::test]
async fn test_get_user_post_vec() -> Result<(), AppError> {
    let db_pool = get_db_pool().await;
    let mut user_1 = create_user("1", &db_pool).await;
    let mut user_2 = create_user("2", &db_pool).await;

    let sphere1_name = "1";
    let sphere2_name = "2";
    let num_post = 10usize;

    let (_, _, mut user_1_expected_post_vec) = create_sphere_with_posts(
        sphere1_name,
        None,
        num_post,
        Some((0..num_post).map(|i| i as i32).collect()),
        (0..num_post).map(|i| (i % 2) == 0).collect(),
        &mut user_1,
        &db_pool,
    ).await.expect("Should create sphere 1 with posts.");
    
    let satellite = create_satellite(sphere1_name, "1", "test", false, false, false, &user_1, &db_pool).await.expect("Should create satellite 1");
    let satellite_post = create_post(
        sphere1_name,
        Some(satellite.satellite_id),
        "satellite",
        "satellite",
        None,
        Link::default(),
        PostTags::default(),
        &user_1,
        &db_pool,
    ).await.expect("Should create satellite post");
    let satellite_post = set_post_score(satellite_post.post_id, -1, &db_pool).await.expect("Should set post score");
    user_1_expected_post_vec.push(PostWithSphereInfo::from_post(satellite_post, sphere1_name.to_string(), None, None));

    let (_, _, mut user_2_expected_post_vec) = create_sphere_with_posts(
        sphere2_name,
        None,
        num_post,
        Some((0..num_post).map(|i| i as i32).collect()),
        (0..num_post).map(|i| (i % 2) == 0).collect(),
        &mut user_2,
        &db_pool,
    ).await.expect("Should create sphere 2 with posts.");

    let sphere_2_post = create_post(
        sphere2_name,
        None,
        "sphere_2",
        "sphere_2",
        None,
        Link::default(),
        PostTags::default(),
        &user_1,
        &db_pool,
    ).await.expect("Should create satellite post");
    let sphere_2_post = set_post_score(sphere_2_post.post_id, -2, &db_pool).await.expect("Should set post score");
    user_1_expected_post_vec.push(PostWithSphereInfo::from_post(sphere_2_post, sphere2_name.to_string(), None, None));
    
    for sort_type in POST_SORT_TYPE_ARRAY {
        let user_1_post_vec = get_user_post_vec(
            &user_1.username,
            SortType::Post(sort_type),
            (num_post + 2) as i64,
            0,
            &db_pool,
        ).await?;
        sort_post_vec(&mut user_1_expected_post_vec, sort_type, false);
        assert_eq!(user_1_post_vec, user_1_expected_post_vec);

        let user_2_post_vec = get_user_post_vec(
            &user_2.username,
            SortType::Post(sort_type),
            num_post as i64,
            0,
            &db_pool,
        ).await?;
        sort_post_vec(&mut user_2_expected_post_vec, sort_type, false);
        assert_eq!(user_2_post_vec, user_2_expected_post_vec);
    }

    let (moderated_post, deleted_post) = get_moderated_and_deleted_posts(sphere1_name, &user_1, &db_pool).await;
    let post_vec = get_user_post_vec(
        &user_1.username,
        SortType::Post(PostSortType::Recent),
        num_post as i64,
        0,
        &db_pool,
    ).await.expect("Should get user_post vec");
    assert!(!post_vec.contains(&moderated_post));
    assert!(!post_vec.contains(&deleted_post));
    
    Ok(())
}

#[tokio::test]
async fn test_get_user_comment_vec() {
    let db_pool = get_db_pool().await;
    let mut user_1 = create_user("1", &db_pool).await;
    let mut user_2 = create_user("2", &db_pool).await;

    let sphere1_name = "1";
    let sphere2_name = "2";
    let num_comments = 10usize;

    let mut user_1_expected_comment_vec = Vec::new();
    let (sphere_1, user_1_post, user_1_comment) = create_sphere_with_post_and_comment(sphere1_name, &mut user_1, &db_pool).await;
    let (sphere_2, user_2_post, user_2_comment) = create_sphere_with_post_and_comment(sphere2_name, &mut user_2, &db_pool).await;

    let user_1_comment = set_comment_score(user_1_comment.comment_id, 50, &db_pool).await.expect("Should set comment score");
    user_1_expected_comment_vec.push(CommentWithContext::from_comment(
        user_1_comment,
        (&sphere_1).into(),
        &user_1_post,
    ));
    let user_1_comment_2 = create_comment(
        user_2_post.post_id,
        None,
        "user_1_comment",
        None,
        false,
        &user_1,
        &db_pool
    ).await.expect("Should create comment in user_2_post");
    let user_1_comment_2 = set_comment_score(user_1_comment_2.comment_id, -50, &db_pool).await.expect("Should set comment score");
    user_1_expected_comment_vec.push(
        CommentWithContext::from_comment(
            user_1_comment_2,
            (&sphere_2).into(),
            &user_2_post,
        )
    );
    let (user_1_post_2, user_1_post_comment_vec, _) = create_post_with_comments(
        sphere2_name,
        "user_1_post",
        num_comments,
        (0..num_comments).map(|i| match i {
            i if i > 1 && (i % 2 == 0) => Some(i%2),
            _ => None,
        }).collect(),
        (0..(num_comments as i32)).collect(),
        (0..num_comments).map(|i| match i {
            i if i > 2 && (i % 2 == 0) => Some(VoteValue::Up),
            _ => None,
        }).collect(),
        &user_1,
        &db_pool
    ).await;
    user_1_expected_comment_vec.append(&mut user_1_post_comment_vec.into_iter().map(|comment|
        CommentWithContext::from_comment(
            comment,
            (&sphere_2).into(),
            &user_1_post_2,
        )
    ).collect());

    for sort_type in COMMENT_SORT_TYPE_ARRAY {
        println!("Sort comments by: {sort_type:?}");
        let user_1_comment_vec_1 = get_user_comment_vec(
            &user_1.username,
            SortType::Comment(sort_type),
            num_comments as i64,
            0,
            &db_pool
        ).await.expect("First comment vec should be loaded");
        let user_1_comment_vec_2 = get_user_comment_vec(
            &user_1.username,
            SortType::Comment(sort_type),
            num_comments as i64,
            num_comments as i64,
            &db_pool
        ).await.expect("Second post vec should be loaded");
        sort_comment_vec(&mut user_1_expected_comment_vec, sort_type, false);
        assert_eq!(user_1_comment_vec_1, user_1_expected_comment_vec[..num_comments]);
        assert_eq!(user_1_comment_vec_2, user_1_expected_comment_vec[num_comments..user_1_expected_comment_vec.len()]);
    }

    let user_2_comment_vec = get_user_comment_vec(
        &user_2.username,
        SortType::Comment(CommentSortType::Best),
        num_comments as i64,
        0,
        &db_pool,
    ).await.expect("Should get user 2 comments");

    assert_eq!(user_2_comment_vec.len(), 1);
    assert_eq!(
        user_2_comment_vec.first(),
        Some(&CommentWithContext::from_comment(
            user_2_comment,
            (&sphere_2).into(),
            &user_2_post,
        ))
    );

    let (moderated_comment, deleted_comment) = get_moderated_and_deleted_comments(&user_1_post, sphere1_name, &user_1, &db_pool).await;
    let comment_vec = get_user_comment_vec(
        &user_1.username,
        SortType::Comment(CommentSortType::Recent),
        num_comments as i64,
        0,
        &db_pool,
    ).await.expect("Should get user 1 comments");
    assert!(!comment_vec.contains(&CommentWithContext::from_comment(
        moderated_comment,
        (&sphere_1).into(),
        &user_1_post,
    )));
    assert!(!comment_vec.contains(&CommentWithContext::from_comment(
        deleted_comment,
        (&sphere_1).into(),
        &user_1_post,
    )));
}