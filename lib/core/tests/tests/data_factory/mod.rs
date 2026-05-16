#![allow(dead_code)]
use std::collections::HashMap;

use sqlx::PgPool;

use sphare_core_common::colors::Color;
use sphare_core_common::common::Rule;
use sphare_core_common::errors::AppError;
use sphare_core_content::comment::ssr::{create_comment, delete_comment};
use sphare_core_content::comment::Comment;
use sphare_core_content::comment::CommentWithChildren;
use sphare_core_content::embed::Link;
use sphare_core_content::moderation::ssr::moderate_comment;
use sphare_core_content::moderation::ssr::moderate_post;
use sphare_core_content::post::ssr::{create_post, delete_post};
use sphare_core_content::post::Post;
use sphare_core_content::post::{PostTags, PostWithSphereInfo};
use sphare_core_content::ranking::ssr::vote_on_content;
use sphare_core_content::ranking::{Vote, VoteValue};
use sphare_core_sphere::rule::ssr::add_rule;
use sphare_core_sphere::satellite::ssr::create_satellite;
use sphare_core_sphere::satellite::Satellite;
use sphare_core_sphere::sphere::ssr::create_sphere;
use sphare_core_sphere::sphere::Sphere;
use sphare_core_sphere::sphere_category::ssr::set_sphere_category;
use sphare_core_sphere::sphere_category::SphereCategory;
use sphare_core_sphere::sphere_management::ssr::set_sphere_icon_url;
use sphare_core_user::role::AdminRole;
use sphare_core_user::user::User;

pub async fn create_sphere_with_post(
    sphere_name: &str,
    user: &mut User,
    db_pool: &PgPool,
) -> (Sphere, Post) {
    let sphere = create_sphere(
        sphere_name,
        "sphere",
        false,
        user,
        db_pool,
    ).await.expect("Should be able to create sphere.");

    *user = User::get(user.user_id, db_pool).await.expect("Should reload user.");

    let post = create_post(
        sphere_name,
        None,
        "post",
        "body",
        None,
        Link::default(),
        PostTags::default(),
        user,
        db_pool,
    ).await.expect("Should be able to create post.");

    (sphere, post)
}

pub async fn create_sphere_with_post_and_comment(
    sphere_name: &str,
    user: &mut User,
    db_pool: &PgPool,
) -> (Sphere, Post, Comment) {
    let (sphere, post) = create_sphere_with_post(sphere_name, user, db_pool).await;

    let comment = create_comment(post.post_id, None, "comment", None, false, user, db_pool).await.expect("Comment should be created.");

    (sphere, post, comment)
}

pub async fn create_sphere_with_posts(
    sphere_name: &str,
    sphere_icon_url: Option<&str>,
    num_posts: usize,
    score_vec: Option<Vec<i32>>,
    category_vec: Vec<bool>,
    user: &mut User,
    db_pool: &PgPool,
) -> Result<(Sphere, SphereCategory, Vec<PostWithSphereInfo>), AppError> {
    let mut sphere = create_sphere(
        sphere_name,
        "sphere",
        false,
        user,
        db_pool,
    ).await?;

    *user = User::get(user.user_id, db_pool).await.expect("Should reload user.");

    set_sphere_icon_url(sphere_name, sphere_icon_url, user, db_pool).await.expect("Should set icon url.");
    sphere.icon_url = sphere_icon_url.map(|x| x.to_string());

    let sphere_category = set_sphere_category(
        sphere_name,
        "create_posts",
        Color::Blue,
        "test",
        true,
        user,
        db_pool,
    ).await.expect("Sphere category should be created.");

    let expected_post_vec = create_posts(
        &sphere,
        None,
        num_posts,
        score_vec,
        Some(&sphere_category),
        category_vec,
        user,
        db_pool,
    ).await?;

    Ok((sphere, sphere_category, expected_post_vec))
}

pub async fn create_posts(
    sphere: &Sphere,
    satellite_id: Option<i64>,
    num_posts: usize,
    score_vec: Option<Vec<i32>>,
    sphere_category: Option<&SphereCategory>,
    category_vec: Vec<bool>,
    user: &User,
    db_pool: &PgPool,
) -> Result<Vec<PostWithSphereInfo>, AppError> {

    let mut expected_post_vec = Vec::<PostWithSphereInfo>::with_capacity(num_posts);
    for i in 0..num_posts {
        let category_id = match (category_vec.get(i), sphere_category) {
            (Some(has_category), Some(sphere_category)) if *has_category => Some(sphere_category.category_id),
            _ => None,
        };
        let mut post = create_post(
            &sphere.sphere_name,
            satellite_id,
            i.to_string().as_str(),
            "body",
            None,
            Link::default(),
            PostTags::new(false, false, false, category_id),
            &user,
            db_pool,
        ).await?;

        if let Some(score_vec) = &score_vec {
            if i < score_vec.len() {
                post = set_post_score(post.post_id, score_vec[i], db_pool).await?;
            }
        }

        let sphere_category_header = match (category_id, sphere_category.cloned()) {
            (Some(_), Some(sphere_category)) => Some(sphere_category.into()),
            _ => None,
        };
        expected_post_vec.push(PostWithSphereInfo::from_post(post, sphere.sphere_name.clone(), sphere_category_header, sphere.icon_url.clone()));
    }

    Ok(expected_post_vec)
}

pub async fn create_sphere_with_satellite(
    sphere_name: &str,
    satellite_name: &str,
    is_nsfw_satellite: bool,
    is_spoiler_satellite: bool,
    user: &mut User,
    db_pool: &PgPool,
) -> Result<(Sphere, Satellite), AppError> {
    let sphere = create_sphere(
        sphere_name,
        "sphere",
        false,
        user,
        db_pool,
    ).await?;

    *user = User::get(user.user_id, db_pool).await.expect("Should reload user.");

    let satellite = create_satellite(
        &sphere.sphere_name,
        satellite_name,
        "test",
        false,
        is_nsfw_satellite,
        is_spoiler_satellite,
        user,
        db_pool,
    ).await.expect("Satellite should be inserted");

    Ok((sphere, satellite))
}

pub async fn create_sphere_with_satellite_vec(
    sphere_name: &str,
    num_satellites: usize,
    user: &mut User,
    db_pool: &PgPool,
) -> Result<(Sphere, Vec<Satellite>), AppError> {
    let sphere = create_sphere(
        sphere_name,
        "sphere",
        false,
        user,
        db_pool,
    ).await?;
    
    *user = User::get(user.user_id, db_pool).await.expect("Should reload user.");
    
    let mut satellite_vec = Vec::new();
    for i in 0..num_satellites {
        let satellite = create_satellite(
            &sphere.sphere_name,
            i.to_string().as_str(),
            "test",
            false,
            false,
            false,
            user,
            db_pool,
        ).await.expect("Satellite 1 should be inserted");
        
        satellite_vec.push(satellite);
    }
    
    Ok((sphere, satellite_vec))
}

pub async fn create_simple_post(
    sphere_name: &str,
    satellite_id: Option<i64>,
    post_title: &str,
    post_body: &str,
    post_markdown_body: Option<&str>,
    user: &User,
    db_pool: &PgPool,
) -> PostWithSphereInfo {
    let post = create_post(
        sphere_name,
        satellite_id,
        post_title,
        post_body,
        post_markdown_body,
        Link::default(),
        PostTags::default(),
        user,
        db_pool,
    ).await.expect("Post should be created");
    
    PostWithSphereInfo::from_post(post, String::from(sphere_name), None, None)
}

pub async fn create_post_with_comments(
    sphere_name: &str,
    post_title: &str,
    num_comments: usize,
    parent_index_vec: Vec<Option<usize>>,
    score_vec: Vec<i32>,
    vote_value_vec: Vec<Option<VoteValue>>,
    user: &User,
    db_pool: &PgPool,
) -> (Post, Vec<Comment>, Vec<Option<Vote>>) {
    let post = create_post(
        sphere_name,
        None,
        post_title,
        "body",
        None,
        Link::default(),
        PostTags::default(),
        user,
        db_pool,
    ).await.expect("Post should be created");

    let mut comment_vec = Vec::new();
    let mut vote_vec = Vec::new();

    for i in 0..num_comments {
        let parent_index = parent_index_vec.get(i).cloned().unwrap_or(None);
        let parent_id = parent_index.map(|parent_index| {
            comment_vec.get(parent_index)
                .map(|parent: &Comment| parent.comment_id)
                .expect("Should retrieve parent comment id")
        });

        let mut comment = create_comment(
            post.post_id,
            parent_id,
            i.to_string().as_str(),
            None,
            false,
            user,
            db_pool,
        ).await.expect("Comment should be created");

        if let Some(Some(vote_value)) = vote_value_vec.get(i) {
            let vote = vote_on_content(
                *vote_value,
                post.post_id,
                Some(comment.comment_id),
                None,
                user,
                db_pool,
            ).await.expect("Vote should be set");

            comment.score = match &vote {
                Some(vote) if vote.value == VoteValue::Up => 1,
                Some(vote) if vote.value == VoteValue::Down => -1,
                _ => 0,
            };
            vote_vec.push(vote);
        } else {
            vote_vec.push(None);
        }

        if let Some(score) = score_vec.get(i) {
            comment = set_comment_score(comment.comment_id, *score, db_pool).await.expect("Comment score should be set");
        }

        comment_vec.push(comment);
    }

    (post, comment_vec, vote_vec)
}

pub async fn create_post_with_comment_tree(
    sphere_name: &str,
    post_title: &str,
    num_comments: usize,
    parent_index_vec: Vec<Option<usize>>,
    score_vec: Vec<i32>,
    vote_value_vec: Vec<Option<VoteValue>>,
    user: &User,
    db_pool: &PgPool,
) -> (Post, Vec<CommentWithChildren>) {
    let (post, comment_vec, vote_vec) = create_post_with_comments(sphere_name, post_title, num_comments, parent_index_vec, score_vec, vote_value_vec, user, db_pool).await;
    let mut comment_map: HashMap<i64, CommentWithChildren> = comment_vec.iter().enumerate().map(|(i, comment)| (comment.comment_id, CommentWithChildren {
        comment: comment.clone(),
        vote: vote_vec.get(i).cloned().unwrap_or(None),
        child_comments: Vec::new()
    })).collect();

    let mut comment_tree = Vec::new();

    for comment in comment_vec.iter().rev() {
        let comment_with_children = comment_map.get(&comment.comment_id).expect("Comment should be in map").clone();
        if let Some(parent_id) = comment_with_children.comment.parent_id {
            let parent_comment = comment_map.get_mut(&parent_id).expect("Parent should be in map");
            parent_comment.child_comments.push(comment_with_children);
        } else {
            comment_tree.push(comment_with_children);
        }
    };

    (post, comment_tree)
}

/// creates, moderates and returns a post. Expects `user` to have management rights on `sphere_name`
pub async fn get_moderated_post(sphere_name: &str, user: &User, db_pool: &PgPool) -> PostWithSphereInfo {
    let rule = add_rule(sphere_name, 0, "1", "2", false, &user, &db_pool).await.expect("Should add rule");
    let post = create_simple_post(sphere_name, None, "a", "b", None, &user, &db_pool).await;
    let post = moderate_post(post.post.post_id, rule.rule_id, "reason", &user, &db_pool).await.expect("Should moderate post.");
    PostWithSphereInfo::from_post(post, sphere_name.to_string(), None, None)
}

/// creates, deletes and returns a post.
pub async fn get_deleted_post(sphere_name: &str, user: &User, db_pool: &PgPool) -> PostWithSphereInfo {
    let post = create_simple_post(sphere_name, None, "a", "b", None, &user, &db_pool).await;
    let post = delete_post(post.post.post_id, &user, &db_pool).await.expect("Post should be deleted.");
    PostWithSphereInfo::from_post(post, sphere_name.to_string(), None, None)
}

/// creates, moderates/deletes and returns two posts. Expects `user` to have management rights on `sphere_name`
pub async fn get_moderated_and_deleted_posts(sphere_name: &str, user: &User, db_pool: &PgPool) -> (PostWithSphereInfo, PostWithSphereInfo) {
    let moderated_post = get_moderated_post(sphere_name, user, db_pool).await;
    let deleted_post = get_deleted_post(sphere_name, user, db_pool).await;
    (moderated_post, deleted_post)
}

/// creates, moderates and returns a comment. Expects `user` to have management rights on `sphere_name`
pub async fn get_moderated_comment(post: &Post, sphere_name: &str, user: &User, db_pool: &PgPool) -> Comment {
    let rule = add_rule(sphere_name, 0, "1", "2", false, &user, &db_pool).await.expect("Should add rule");
    let comment = create_comment(
        post.post_id,
        None,
        "a",
        None,
        false,
        &user,
        &db_pool
    ).await.expect("Comment should be created.");
    let comment = moderate_comment(comment.comment_id, rule.rule_id, "reason", &user, &db_pool).await.expect("Should moderate comment.");
    comment
}

/// creates, deletes and returns a comment.
pub async fn get_deleted_comment(post: &Post, user: &User, db_pool: &PgPool) -> Comment {
    let comment = create_comment(
        post.post_id,
        None,
        "a",
        None,
        false,
        &user,
        &db_pool
    ).await.expect("Comment should be created.");
    let comment = delete_comment(comment.comment_id, &user, &db_pool).await.expect("Comment should be deleted.");
    comment
}

pub async fn get_moderated_and_deleted_comments(post: &Post, sphere_name: &str, user: &User, db_pool: &PgPool) -> (Comment, Comment) {
    let moderated_comment = get_moderated_comment(post, sphere_name, user, db_pool).await;
    let deleted_comment = get_deleted_comment(post, user, db_pool).await;
    (moderated_comment, deleted_comment)
}

pub async fn set_sphere_num_members(
    sphere_id: i64,
    num_members: i32,
    db_pool: &PgPool,
) -> Result<Sphere, AppError> {
    let sphere = sqlx::query_as::<_, Sphere>(
        "UPDATE spheres
        SET num_members = $1, timestamp = NOW()
        WHERE sphere_id = $2
        RETURNING *",
    )
        .bind(num_members)
        .bind(sphere_id)
        .fetch_one(db_pool)
        .await?;

    Ok(sphere)
}

pub async fn set_post_score(
    post_id: i64,
    score: i32,
    db_pool: &PgPool,
) -> Result<Post, AppError> {
    let post = sqlx::query_as::<_, Post>(
        "WITH updated_post AS (
            UPDATE posts SET score = $1, scoring_timestamp = NOW()
            WHERE post_id = $2
            RETURNING *
        )
        SELECT p.*, u.username as creator_name, NULL as moderator_name
        FROM updated_post p
        JOIN users u ON u.user_id = p.creator_id",
    )
        .bind(score)
        .bind(post_id)
        .fetch_one(db_pool)
        .await?;

    Ok(post)
}

pub async fn set_post_timestamp(
    post_id: i64,
    day_offset: i64,
    db_pool: &PgPool,
) -> Result<Post, AppError> {
    let post = sqlx::query_as::<_, Post>(
        "WITH updated_post AS (
            UPDATE posts
            SET create_timestamp = create_timestamp + (INTERVAL '1 day' * $1),
            scoring_timestamp = NOW()
            WHERE post_id = $2
            RETURNING *
        )
        SELECT p.*, u.username as creator_name, NULL as moderator_name
        FROM updated_post p
        JOIN users u ON u.user_id = p.creator_id",
    )
        .bind(day_offset)
        .bind(post_id)
        .fetch_one(db_pool)
        .await?;

    Ok(post)
}

pub async fn set_comment_score(
    comment_id: i64,
    score: i32,
    db_pool: &PgPool,
) -> Result<Comment, AppError> {
    let comment = sqlx::query_as::<_, Comment>(
        "WITH updated_comment AS (
            UPDATE comments SET score = $1
            WHERE comment_id = $2
            RETURNING *
        )
        SELECT c.*, u.username as creator_name, NULL as moderator_name
        FROM updated_comment c
        JOIN users u ON u.user_id = c.creator_id",
    )
        .bind(score)
        .bind(comment_id)
        .fetch_one(db_pool)
        .await
        .expect("Should set comment score");

    Ok(comment)
}

pub async fn add_base_rule(
    priority: i16,
    title: &str,
    description: &str,
    markdown_description: Option<&str>,
    user: &User,
    db_pool: &PgPool,
) -> Result<Rule, AppError> {
    user.check_admin_role(AdminRole::Admin)?;
    sqlx::query!(
        "UPDATE rules
         SET priority = priority + 1
         WHERE sphere_id IS NULL AND priority >= $1 AND delete_timestamp IS NULL",
        priority,
    ).execute(db_pool).await?;

    let rule = sqlx::query_as!(
        Rule,
        "INSERT INTO rules
        (sphere_id, priority, title, description, markdown_description, user_id)
        VALUES (
            NULL, $1, $2, $3, $4, $5
        ) RETURNING *",
        priority,
        title,
        description,
        markdown_description,
        user.user_id,
    ).fetch_one(db_pool).await?;
    Ok(rule)
}

pub async fn update_base_rule(
    current_priority: i16,
    priority: i16,
    title: &str,
    description: &str,
    markdown_description: Option<&str>,
    user: &User,
    db_pool: &PgPool,
) -> Result<Rule, AppError> {
    user.check_admin_role(AdminRole::Admin)?;

    let current_rule = sqlx::query_as!(
        Rule,
        "UPDATE rules
         SET delete_timestamp = NOW()
         WHERE sphere_id IS NULL AND priority = $1 AND delete_timestamp IS NULL
         RETURNING *",
        current_priority,
    ).fetch_one(db_pool).await?;

    if priority > current_priority {
        sqlx::query!(
            "UPDATE rules
            SET priority = priority - 1
            WHERE sphere_id IS NULL AND priority BETWEEN $1 AND $2 AND delete_timestamp IS NULL",
            current_priority,
            priority,
        ).execute(db_pool).await?;
    } else if priority < current_priority {
        sqlx::query!(
            "UPDATE rules
            SET priority = priority + 1
            WHERE sphere_id is NULL AND priority BETWEEN $2 AND $1 AND delete_timestamp IS NULL",
            current_priority,
            priority,
        ).execute(db_pool).await?;
    }

    let new_rule = sqlx::query_as!(
        Rule,
        "INSERT INTO rules
        (rule_key, sphere_id, priority, title, description, markdown_description, user_id)
        VALUES (
            $1, NULL, $2, $3, $4, $5, $6
        ) RETURNING *",
        current_rule.rule_key,
        priority,
        title,
        description,
        markdown_description,
        user.user_id,
    ).fetch_one(db_pool).await?;

    Ok(new_rule)
}

pub async fn remove_base_rule(
    priority: i16,
    user: &User,
    db_pool: &PgPool,
) -> Result<(), AppError> {
    user.check_admin_role(AdminRole::Admin)?;

    sqlx::query!(
        "UPDATE rules
         SET delete_timestamp = NOW()
         WHERE sphere_id IS NULL AND priority = $1 AND delete_timestamp IS NULL",
        priority,
    ).execute(db_pool).await?;

    sqlx::query!(
        "UPDATE rules
         SET priority = priority - 1
         WHERE sphere_id IS NULL AND priority > $1 AND delete_timestamp IS NULL",
        priority,
    ).execute(db_pool).await?;

    Ok(())
}