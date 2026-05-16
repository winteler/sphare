use sphare_core_common::errors::AppError;
use sphare_core_content::comment::ssr::get_comment_by_id;
use sphare_core_content::moderation::ssr::ban_user_from_sphere;
use sphare_core_content::ranking::VoteValue;
use sphare_core_content::{post, ranking};
use sphare_core_sphere::rule::ssr::add_rule;
use sphare_core_user::role::AdminRole;
use sphare_core_user::user::{BanStatus, User};

use crate::common::*;
use crate::data_factory::{create_sphere_with_post, create_sphere_with_post_and_comment};
use crate::utils::get_user_comment_vote;

mod common;
mod data_factory;
mod utils;

#[tokio::test]
async fn test_vote_on_content_post() -> Result<(), AppError> {
    let db_pool = get_db_pool().await;
    let mut user = create_test_user(&db_pool).await;

    let (_, post) = create_sphere_with_post("sphere", &mut user, &db_pool).await;

    let post_with_vote = post::ssr::get_post_with_info_by_id(post.post_id, Some(&user), &db_pool).await?;
    assert!(post_with_vote.vote.is_none());

    let vote_value = VoteValue::Up;
    ranking::ssr::vote_on_content(
        vote_value,
        post.post_id,
        None,
        None,
        &user,
        &db_pool,
    ).await.expect("Upvote should be created.");

    let post_with_vote = post::ssr::get_post_with_info_by_id(post.post_id, Some(&user), &db_pool).await?;
    let vote = post_with_vote.vote.expect("Vote should be Some");
    assert_eq!(vote.value, vote_value);
    assert_eq!(vote.user_id, user.user_id);
    assert_eq!(vote.post_id, post.post_id);
    assert_eq!(vote.comment_id, None);
    assert_eq!(post.score + 1, post_with_vote.post.score);

    // repeating vote just returns same result
    let repeat_vote = ranking::ssr::vote_on_content(
        VoteValue::Up,
        post.post_id,
        None,
        Some(vote.vote_id),
        &user,
        &db_pool,
    ).await.expect("Repeat vote should be ok.").expect("Vote should be Some.");
    assert_eq!(repeat_vote, vote);

    // assert error when existing vote is not referenced
    assert!(ranking::ssr::vote_on_content(
        VoteValue::Up,
        post.post_id,
        None,
        None,
        &user,
        &db_pool,
    )
        .await
        .is_err());

    ranking::ssr::vote_on_content(
        VoteValue::Down,
        post.post_id,
        None,
        Some(vote.vote_id),
        &user,
        &db_pool,
    ).await.expect("Downvote should be created.");

    let post_with_vote = post::ssr::get_post_with_info_by_id(post.post_id, Some(&user), &db_pool).await?;
    let vote = post_with_vote.vote.expect("Post should have vote");
    assert_eq!(vote.value, VoteValue::Down);
    assert_eq!(vote.user_id, user.user_id);
    assert_eq!(vote.post_id, post.post_id);
    assert_eq!(vote.comment_id, None);
    assert_eq!(post.score - 1, post_with_vote.post.score);

    ranking::ssr::vote_on_content(
        VoteValue::None,
        post.post_id,
        None,
        Some(vote.vote_id),
        &user,
        &db_pool,
    ).await.expect("Vote should be deleted.");

    let post_with_vote = post::ssr::get_post_with_info_by_id(post.post_id, Some(&user), &db_pool).await?;
    assert_eq!(post_with_vote.vote, None);
    assert_eq!(post.score, post_with_vote.post.score);

    Ok(())
}

#[tokio::test]
async fn test_vote_on_content_comment() {
    let db_pool = get_db_pool().await;
    let mut user = create_test_user(&db_pool).await;

    let (_, _, init_comment) = create_sphere_with_post_and_comment("sphere", &mut user, &db_pool).await;

    let comment = get_comment_by_id(init_comment.comment_id, &db_pool).await.expect("Should get comment");

    let vote_value = VoteValue::Up;
    ranking::ssr::vote_on_content(
        vote_value,
        comment.post_id,
        Some(comment.comment_id),
        None,
        &user,
        &db_pool,
    ).await.expect("Upvote should be created.");

    let comment = get_comment_by_id(comment.comment_id, &db_pool).await.expect("Should get comment aftervote");
    let vote = get_user_comment_vote(&comment, user.user_id, &db_pool).await.expect("Should get user comment vote");
    assert_eq!(vote.value, vote_value);
    assert_eq!(vote.user_id, user.user_id);
    assert_eq!(vote.post_id, comment.post_id);
    assert_eq!(vote.comment_id, Some(comment.comment_id));
    assert_eq!(init_comment.score + 1, comment.score);

    // repeating vote just returns same result
    let repeat_vote = ranking::ssr::vote_on_content(
        VoteValue::Up,
        comment.post_id,
        Some(comment.comment_id),
        Some(vote.vote_id),
        &user,
        &db_pool,
    ).await.expect("Repeat vote should be ok.").expect("Vote should be Some.");
    assert_eq!(repeat_vote, vote);

    // assert error when existing vote is not referenced
    assert!(ranking::ssr::vote_on_content(
        VoteValue::Up,
        comment.post_id,
        Some(comment.comment_id),
        None,
        &user,
        &db_pool,
    )
        .await
        .is_err());

    ranking::ssr::vote_on_content(
        VoteValue::Down,
        comment.post_id,
        Some(comment.comment_id),
        Some(vote.vote_id),
        &user,
        &db_pool,
    ).await.expect("Downvote should be created.");

    let comment = get_comment_by_id(comment.comment_id, &db_pool).await.expect("Should get comment after 2nd vote");
    let vote = get_user_comment_vote(&comment, user.user_id, &db_pool).await.expect("Should get user comment vote");
    assert_eq!(vote.value, VoteValue::Down);
    assert_eq!(vote.user_id, user.user_id);
    assert_eq!(vote.post_id, comment.post_id);
    assert_eq!(vote.comment_id, Some(comment.comment_id));
    assert_eq!(init_comment.score - 1, comment.score);

    ranking::ssr::vote_on_content(
        VoteValue::None,
        comment.post_id,
        Some(comment.comment_id),
        Some(vote.vote_id),
        &user,
        &db_pool,
    ).await.expect("Vote should be deleted.");

    let comment = get_comment_by_id(comment.comment_id, &db_pool).await.expect("Should get comment after third vote");
    assert_eq!(get_user_comment_vote(&comment, user.user_id, &db_pool).await, Err(AppError::NotFound));
    assert_eq!(init_comment.score, comment.score);
}

#[tokio::test]
async fn test_vote_on_content_with_ban() {
    let db_pool = get_db_pool().await;
    let mut user = create_test_user(&db_pool).await;

    let (sphere, _, comment) = create_sphere_with_post_and_comment("sphere", &mut user, &db_pool).await;
    let mut user = User::get(user.user_id, &db_pool).await.expect("User should be found.");
    user.admin_role = AdminRole::Admin;

    let (_, _, comment_2) = create_sphere_with_post_and_comment("sphere_2", &mut user, &db_pool).await;
    let sphere_rule = add_rule(
        &sphere.sphere_name, 0, "test", "test", false, &user, &db_pool
    ).await.expect("Shere rule should be added.");

    let user_1 = create_user("1", &db_pool).await;
    ban_user_from_sphere(
        user_1.user_id, sphere.sphere_id, comment.post_id, Some(comment.comment_id), sphere_rule.rule_id, None, &user, &db_pool
    ).await.expect("User should be banned.");

    ranking::ssr::vote_on_content(
        VoteValue::Up,
        comment.post_id,
        None,
        None,
        &user_1,
        &db_pool,
    ).await.expect_err("User 1 cannot vote in banned sphere.");

    ranking::ssr::vote_on_content(
        VoteValue::Up,
        comment.post_id,
        Some(comment.comment_id),
        None,
        &user_1,
        &db_pool,
    ).await.expect_err("User 1 cannot vote in banned sphere.");

    ranking::ssr::vote_on_content(
        VoteValue::Up,
        comment_2.post_id,
        None,
        None,
        &user_1,
        &db_pool,
    ).await.expect("User 1 can still vote in other spheres.");

    ranking::ssr::vote_on_content(
        VoteValue::Up,
        comment_2.post_id,
        Some(comment_2.comment_id),
        None,
        &user_1,
        &db_pool,
    ).await.expect("User 1 can still vote in other spheres.");

    let mut user_2 = create_user("2", &db_pool).await;
    user_2.ban_status = BanStatus::Permanent;

    ranking::ssr::vote_on_content(
        VoteValue::Up,
        comment.post_id,
        None,
        None,
        &user_2,
        &db_pool,
    ).await.expect_err("User 2 cannot vote anywhere.");

    ranking::ssr::vote_on_content(
        VoteValue::Up,
        comment.post_id,
        Some(comment.comment_id),
        None,
        &user_2,
        &db_pool,
    ).await.expect_err("User 2 cannot vote anywhere.");

    ranking::ssr::vote_on_content(
        VoteValue::Up,
        comment_2.post_id,
        None,
        None,
        &user_2,
        &db_pool,
    ).await.expect_err("User 2 cannot vote anywhere.");

    ranking::ssr::vote_on_content(
        VoteValue::Up,
        comment_2.post_id,
        Some(comment_2.comment_id),
        None,
        &user_2,
        &db_pool,
    ).await.expect_err("User 2 cannot vote anywhere.");
}