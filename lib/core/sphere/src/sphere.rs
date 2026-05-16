use serde::{Deserialize, Serialize};

use sphare_core_common::common::SphereHeader;

#[cfg_attr(feature = "ssr", derive(sqlx::FromRow))]
#[derive(Clone, Debug, PartialEq, Eq, Ord, PartialOrd, Serialize, Deserialize)]
pub struct Sphere {
    pub sphere_id: i64,
    pub sphere_name: String,
    pub normalized_sphere_name: String,
    pub description: String,
    pub is_nsfw: bool,
    pub is_banned: bool,
    pub icon_url: Option<String>,
    pub banner_url: Option<String>,
    pub num_members: i32,
    pub creator_id: i64,
    pub create_timestamp: chrono::DateTime<chrono::Utc>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[cfg_attr(feature = "ssr", derive(sqlx::FromRow))]
#[derive(Clone, Debug, PartialEq, Eq, Ord, PartialOrd, Serialize, Deserialize)]
pub struct SphereSubscription {
    pub subscription_id: i64,
    pub user_id: i64,
    pub sphere_id: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[cfg_attr(feature = "ssr", derive(sqlx::FromRow))]
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct SphereWithUserInfo {
    #[cfg_attr(feature = "ssr", sqlx(flatten))]
    pub sphere: Sphere,
    pub subscription_id: Option<i64>,
}

impl From<&Sphere> for SphereHeader {
    fn from(sphere: &Sphere) -> Self {
        Self::new(sphere.sphere_name.clone(), sphere.icon_url.clone(), sphere.is_nsfw)
    }
}

#[cfg(feature = "ssr")]
pub mod ssr {
    use sqlx::PgPool;

    use sphare_core_common::checks::{check_sphere_name, check_string_length};
    use sphare_core_common::constants::MAX_SPHERE_DESCRIPTION_LENGTH;
    use sphare_core_common::errors::AppError;
    use sphare_core_common::errors::AppError::InternalServerError;
    use sphare_core_common::routes::get_sphere_path;
    use sphare_core_user::role::ssr::init_sphere_leader;
    use sphare_core_user::role::PermissionLevel;
    use sphare_core_user::user::User;

    use crate::sphere::{Sphere, SphereHeader, SphereWithUserInfo};

    pub async fn get_sphere_by_name(sphere_name: &str, db_pool: &PgPool) -> Result<Sphere, AppError> {
        check_sphere_name(sphere_name)?;
        let sphere = sqlx::query_as::<_, Sphere>(
            "SELECT * FROM spheres WHERE sphere_name = $1"
        )
            .bind(sphere_name)
            .fetch_one(db_pool)
            .await?;

        Ok(sphere)
    }

    pub async fn get_sphere_with_user_info(
        sphere_name: &str,
        user_id: Option<i64>,
        db_pool: &PgPool,
    ) -> Result<SphereWithUserInfo, AppError> {
        check_sphere_name(sphere_name)?;
        let sphere = sqlx::query_as::<_, SphereWithUserInfo>(
            "SELECT s.*, sub.subscription_id
            FROM spheres s
            LEFT JOIN sphere_subscriptions sub ON
                sub.sphere_id = s.sphere_id AND
                sub.user_id = $1
            WHERE s.sphere_name = $2",
        )
            .bind(user_id)
            .bind(sphere_name)
            .fetch_one(db_pool)
            .await?;

        Ok(sphere)
    }

    pub async fn get_post_sphere(
        post_id: i64,
        db_pool: &PgPool,
    ) -> Result<Sphere, AppError> {
        let sphere = sqlx::query_as::<_, Sphere>(
            "SELECT s.*
            FROM spheres s
            JOIN posts p on p.sphere_id = s.sphere_id
            WHERE p.post_id = $1"
        )
            .bind(post_id)
            .fetch_one(db_pool)
            .await?;

        Ok(sphere)
    }

    pub async fn is_sphere_available(sphere_name: &str, db_pool: &PgPool) -> Result<bool, AppError> {
        check_sphere_name(sphere_name)?;
        let sphere_exist = sqlx::query!(
            "SELECT sphere_id FROM spheres WHERE normalized_sphere_name = normalize_sphere_name($1)",
            sphere_name,
        )
            .fetch_one(db_pool)
            .await;

        match sphere_exist {
            Ok(_) => Ok(false),
            Err(sqlx::error::Error::RowNotFound) => Ok(true),
            Err(e) => Err(e.into()),
        }
    }

    pub async fn get_popular_sphere_headers(
        limit: i64,
        db_pool: &PgPool,
    ) -> Result<Vec<SphereHeader>, AppError> {
        let sphere_header_vec = sqlx::query_as!(
            SphereHeader,
            "SELECT sphere_name, icon_url, is_nsfw
            FROM spheres
            where NOT is_nsfw
            ORDER BY num_members DESC, sphere_name LIMIT $1",
            limit
        )
            .fetch_all(db_pool)
            .await?;

        Ok(sphere_header_vec)
    }

    pub async fn get_subscribed_sphere_headers(
        user_id: i64,
        db_pool: &PgPool,
    ) -> Result<Vec<SphereHeader>, AppError> {
        let sphere_header_vec = sqlx::query_as!(
            SphereHeader,
            "SELECT s.sphere_name, s.icon_url, s.is_nsfw
            FROM spheres s
            JOIN sphere_subscriptions sub ON
                s.sphere_id = sub.sphere_id AND
                sub.user_id = $1
            ORDER BY sphere_name",
            user_id,
        )
            .fetch_all(db_pool)
            .await?;

        Ok(sphere_header_vec)
    }

    /// creates a sphere, subscribe to it and return the path to the new sphere
    pub async fn create_sphere_and_subscribe(
        sphere_name: &str,
        description: &str,
        is_nsfw: bool,
        user: &User,
        db_pool: &PgPool,
    ) -> Result<(Sphere, String), AppError> {
        check_sphere_name(sphere_name)?;
        check_string_length(description, "Sphere description", MAX_SPHERE_DESCRIPTION_LENGTH, false)?;
        log::trace!("Create Sphere '{sphere_name}', {description}, {is_nsfw}");

        let new_sphere_path = get_sphere_path(sphere_name);

        let mut sphere = create_sphere(
            sphere_name,
            description,
            is_nsfw,
            user,
            db_pool,
        ).await?;

        subscribe(sphere.sphere_id, user.user_id, db_pool).await?;

        sphere.num_members = 1;

        Ok((sphere, new_sphere_path))
    }

    pub async fn create_sphere(
        name: &str,
        description: &str,
        is_nsfw: bool,
        user: &User,
        db_pool: &PgPool,
    ) -> Result<Sphere, AppError> {
        user.check_can_publish()?;
        check_sphere_name(name)?;

        let sphere = sqlx::query_as::<_, Sphere>(
            "INSERT INTO spheres (sphere_name, description, is_nsfw, creator_id) VALUES ($1, $2, $3, $4) RETURNING *"
        )
            .bind(name)
            .bind(description)
            .bind(is_nsfw)
            .bind(user.user_id)
            .fetch_one(db_pool)
            .await?;

        init_sphere_leader(user.user_id, &sphere.sphere_name, &db_pool).await?;

        Ok(sphere)
    }

    pub async fn update_sphere_description(
        sphere_name: &str,
        description: &str,
        user: &User,
        db_pool: &PgPool,
    ) -> Result<Sphere, AppError> {
        check_sphere_name(sphere_name)?;
        check_string_length(description, "Sphere description", MAX_SPHERE_DESCRIPTION_LENGTH, false)?;
        user.check_sphere_permissions_by_name(sphere_name, PermissionLevel::Manage)?;

        let sphere = sqlx::query_as::<_, Sphere>(
            "UPDATE spheres SET description = $1, timestamp = NOW() WHERE sphere_name = $2 RETURNING *"
        )
            .bind(description)
            .bind(sphere_name)
            .fetch_one(db_pool)
            .await?;

        Ok(sphere)
    }

    pub async fn subscribe(sphere_id: i64, user_id: i64, db_pool: &PgPool) -> Result<(), AppError> {
        sqlx::query!(
            "INSERT INTO sphere_subscriptions (user_id, sphere_id) VALUES ($1, $2)",
            user_id,
            sphere_id
        )
            .execute(db_pool)
            .await?;

        sqlx::query!(
            "UPDATE spheres SET num_members = num_members + 1 WHERE sphere_id = $1",
            sphere_id
        )
            .execute(db_pool)
            .await?;

        Ok(())
    }

    pub async fn unsubscribe(sphere_id: i64, user_id: i64, db_pool: &PgPool) -> Result<(), AppError> {
        let deleted_rows = sqlx::query!(
            "DELETE FROM sphere_subscriptions WHERE user_id = $1 AND sphere_id = $2",
            user_id,
            sphere_id,
        )
            .execute(db_pool)
            .await?
            .rows_affected();

        if deleted_rows != 1 {
            return Err(InternalServerError(format!("Expected one subscription deleted, got {deleted_rows} instead.")))
        }

        sqlx::query!(
            "UPDATE spheres SET num_members = num_members - 1 WHERE sphere_id = $1",
            sphere_id
        )
            .execute(db_pool)
            .await?;

        Ok(())
    }
}