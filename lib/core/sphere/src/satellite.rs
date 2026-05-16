use serde::{Deserialize, Serialize};

#[cfg_attr(feature = "ssr", derive(sqlx::FromRow))]
#[derive(Clone, Debug, PartialEq, Eq, Ord, PartialOrd, Serialize, Deserialize)]
pub struct Satellite {
    pub satellite_id: i64,
    pub satellite_name: String,
    pub sphere_id: i64,
    pub body: String,
    pub markdown_body: Option<String>,
    pub is_nsfw: bool,
    pub is_spoiler: bool,
    pub num_posts: i32,
    pub creator_id: i64,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub disable_timestamp: Option<chrono::DateTime<chrono::Utc>>,
}

#[cfg(feature = "ssr")]
pub mod ssr {
    use sqlx::PgPool;
    use sphare_core_common::checks::{check_satellite_name, check_sphere_name, check_string_length};
    use sphare_core_common::constants::MAX_CONTENT_LENGTH;
    use sphare_core_common::editor::ssr::get_html_and_markdown_strings;
    use sphare_core_common::errors::AppError;
    use sphare_core_user::role::PermissionLevel;
    use sphare_core_user::user::User;

    use crate::satellite::Satellite;
    use crate::sphere::Sphere;

    pub async fn get_satellite_by_id(satellite_id: i64, db_pool: &PgPool) -> Result<Satellite, AppError> {
        let satellite = sqlx::query_as!(
            Satellite,
            "SELECT * FROM satellites
            WHERE satellite_id = $1",
            satellite_id
        )
            .fetch_one(db_pool)
            .await?;

        Ok(satellite)
    }

    pub async fn get_satellite_vec_by_sphere_name(
        sphere_name: &str,
        include_inactive: bool,
        db_pool: &PgPool
    ) -> Result<Vec<Satellite>, AppError> {
        let satellite_vec = sqlx::query_as!(
            Satellite,
            "SELECT sa.* FROM satellites sa
            JOIN spheres s ON s.sphere_id = sa.sphere_id
            WHERE
                s.sphere_name = $1 AND
                (
                    $2 OR sa.disable_timestamp IS NULL
                )
            ORDER BY sa.disable_timestamp DESC NULLS FIRST, sa.satellite_name",
            sphere_name,
            include_inactive,
        )
            .fetch_all(db_pool)
            .await?;

        Ok(satellite_vec)
    }

    pub async fn get_satellite_sphere(satellite_id: i64, db_pool: &PgPool) -> Result<Sphere, AppError> {
        let sphere = sqlx::query_as::<_, Sphere>(
            "SELECT s.* FROM spheres s
            JOIN satellites sa ON sa.sphere_id = s.sphere_id
            WHERE sa.satellite_id = $1"
        )
            .bind(satellite_id)
            .fetch_one(db_pool)
            .await?;

        Ok(sphere)
    }

    pub async fn create_satellite(
        sphere_name: &str,
        satellite_name: &str,
        body: &str,
        is_markdown: bool,
        is_nsfw: bool,
        is_spoiler: bool,
        user: &User,
        db_pool: &PgPool
    ) -> Result<Satellite, AppError> {
        check_sphere_name(sphere_name)?;
        check_satellite_name(satellite_name)?;
        check_string_length(body, "Satellite body", MAX_CONTENT_LENGTH as usize, false)?;
        user.check_sphere_permissions_by_name(sphere_name, PermissionLevel::Manage)?;

        let (body, markdown_body) = get_html_and_markdown_strings(body, is_markdown)?;

        let satellite = sqlx::query_as!(
            Satellite,
            "INSERT INTO satellites
            (satellite_name, sphere_id, body, markdown_body, is_nsfw, is_spoiler, creator_id)
            VALUES (
                $1,
                (SELECT sphere_id FROM spheres WHERE sphere_name = $2),
                $3, $4,
                (
                    CASE
                        WHEN $5 THEN TRUE
                        ELSE (SELECT is_nsfw FROM spheres WHERE sphere_name = $2)
                    END
                ),
                $6, $7
            )
            RETURNING *",
            satellite_name,
            sphere_name,
            body,
            markdown_body,
            is_nsfw,
            is_spoiler,
            user.user_id,
        )
            .fetch_one(db_pool)
            .await?;

        Ok(satellite)
    }

    pub async fn update_satellite(
        satellite_id: i64,
        satellite_name: &str,
        body: &str,
        is_markdown: bool,
        is_nsfw: bool,
        is_spoiler: bool,
        user: &User,
        db_pool: &PgPool
    ) -> Result<Satellite, AppError> {
        check_satellite_name(satellite_name)?;
        check_string_length(body, "Satellite body", MAX_CONTENT_LENGTH as usize, false)?;

        let sphere = get_satellite_sphere(satellite_id, db_pool).await?;
        user.check_sphere_permissions_by_name(&sphere.sphere_name, PermissionLevel::Manage)?;

        let (body, markdown_body) = get_html_and_markdown_strings(body, is_markdown)?;

        let satellite = sqlx::query_as!(
            Satellite,
            "UPDATE satellites
            SET
                satellite_name = $1,
                body = $2,
                markdown_body = $3,
                is_nsfw = $4,
                is_spoiler = $5
            WHERE satellite_id = $6
            RETURNING *",
            satellite_name,
            body,
            markdown_body,
            is_nsfw || sphere.is_nsfw,
            is_spoiler,
            satellite_id,
        )
            .fetch_one(db_pool)
            .await?;

        Ok(satellite)
    }

    pub async fn activate_satellite(
        satellite_id: i64,
        user: &User,
        db_pool: &PgPool
    ) -> Result<Satellite, AppError> {
        let sphere = get_satellite_sphere(satellite_id, db_pool).await?;
        user.check_sphere_permissions_by_name(&sphere.sphere_name, PermissionLevel::Manage)?;

        let satellite = sqlx::query_as!(
            Satellite,
            "UPDATE satellites
            SET disable_timestamp = NULL
            WHERE satellite_id = $1
            RETURNING *",
            satellite_id,
        )
            .fetch_one(db_pool)
            .await?;

        Ok(satellite)
    }

    pub async fn deactivate_satellite(
        satellite_id: i64,
        user: &User,
        db_pool: &PgPool
    ) -> Result<Satellite, AppError> {
        let sphere = get_satellite_sphere(satellite_id, db_pool).await?;
        user.check_sphere_permissions_by_name(&sphere.sphere_name, PermissionLevel::Manage)?;

        let satellite = sqlx::query_as!(
            Satellite,
            "UPDATE satellites
            SET disable_timestamp = NOW()
            WHERE satellite_id = $1
            RETURNING *",
            satellite_id,
        )
            .fetch_one(db_pool)
            .await?;

        Ok(satellite)
    }
}