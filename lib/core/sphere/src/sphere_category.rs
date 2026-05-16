use serde::{Deserialize, Serialize};

use sphare_core_common::colors::Color;
use sphare_core_common::common::SphereCategoryHeader;

#[cfg_attr(feature = "ssr", derive(sqlx::FromRow))]
#[derive(Clone, Debug, PartialEq, Eq, Ord, PartialOrd, Serialize, Deserialize)]
pub struct SphereCategory {
    pub category_id: i64,
    pub sphere_id: i64,
    pub category_name: String,
    pub category_color: Color,
    pub description: String,
    pub is_active: bool,
    pub creator_id: i64,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub delete_timestamp: Option<chrono::DateTime<chrono::Utc>>,
}

impl From<SphereCategory> for SphereCategoryHeader {
    fn from(sphere_category: SphereCategory) -> Self {
        SphereCategoryHeader {
            category_name: sphere_category.category_name,
            category_color: sphere_category.category_color,
        }
    }
}

impl From<&SphereCategory> for SphereCategoryHeader {
    fn from(sphere_category: &SphereCategory) -> Self {
        SphereCategoryHeader {
            category_name: sphere_category.category_name.clone(),
            category_color: sphere_category.category_color,
        }
    }
}

#[cfg(feature = "ssr")]
pub mod ssr {
    use sqlx::PgPool;
    use sphare_core_common::checks::{check_sphere_name, check_string_length};
    use sphare_core_common::colors::Color;
    use sphare_core_common::constants::{MAX_CATEGORY_DESCRIPTION_LENGTH, MAX_CATEGORY_NAME_LENGTH};
    use sphare_core_common::errors::AppError;
    use sphare_core_user::role::PermissionLevel;
    use sphare_core_user::user::User;

    use crate::sphere_category::SphereCategory;

    pub const CATEGORY_NOT_DELETED_STR: &str = "Category was not deleted, it either doesn't exist or is used.";

    pub async fn get_sphere_category_vec(
        sphere_name: &str,
        db_pool: &PgPool,
    ) -> Result<Vec<SphereCategory>, AppError> {
        check_sphere_name(sphere_name)?;
        let sphere_category_vec = sqlx::query_as!(
            SphereCategory,
            "SELECT sc.* FROM sphere_categories sc
            JOIN spheres s ON s.sphere_id = sc.sphere_id
            WHERE s.sphere_name = $1
            ORDER BY sc.is_active DESC, sc.category_name",
            sphere_name
        )
            .fetch_all(db_pool)
            .await?;

        Ok(sphere_category_vec)
    }

    pub async fn set_sphere_category(
        sphere_name: &str,
        category_name: &str,
        category_color: Color,
        description: &str,
        is_active: bool,
        user: &User,
        db_pool: &PgPool,
    ) -> Result<SphereCategory, AppError> {
        check_sphere_name(sphere_name)?;
        check_string_length(category_name, "Category name", MAX_CATEGORY_NAME_LENGTH, false)?;
        check_string_length(description, "Category description", MAX_CATEGORY_DESCRIPTION_LENGTH, false)?;
        user.check_sphere_permissions_by_name(sphere_name, PermissionLevel::Manage)?;

        let category = sqlx::query_as!(
            SphereCategory,
            "INSERT INTO sphere_categories
            (sphere_id, category_name, category_color, description, is_active, creator_id)
            VALUES (
                (SELECT sphere_id FROM spheres WHERE sphere_name = $1),
                $2, $3, $4, $5, $6
            ) ON CONFLICT (sphere_id, category_name) DO UPDATE
                SET description = EXCLUDED.description,
                    category_color = EXCLUDED.category_color,
                    is_active = EXCLUDED.is_active,
                    timestamp = NOW()
            RETURNING *",
            sphere_name,
            category_name,
            category_color as i32,
            description,
            is_active,
            user.user_id,
        )
            .fetch_one(db_pool)
            .await?;

        Ok(category)
    }

    pub async fn delete_sphere_category(
        sphere_name: &str,
        category_name: &str,
        user: &User,
        db_pool: &PgPool,
    ) -> Result<(), AppError> {
        check_sphere_name(sphere_name)?;
        check_string_length(category_name, "Category name", MAX_CATEGORY_NAME_LENGTH, false)?;
        user.check_sphere_permissions_by_name(sphere_name, PermissionLevel::Manage)?;

        let result = sqlx::query!(
            "DELETE FROM sphere_categories c
             WHERE sphere_id = (
                    SELECT sphere_id FROM spheres WHERE sphere_name = $1
                ) AND category_name = $2 AND NOT EXISTS (
                SELECT 1 FROM posts p WHERE p.category_id = c.category_id
             )",
            sphere_name,
            category_name,
        )
            .execute(db_pool)
            .await?;

        match result.rows_affected() {
            0 => Err(AppError::InternalServerError(String::from(CATEGORY_NOT_DELETED_STR))),
            1 => Ok(()),
            count => Err(AppError::InternalServerError(format!("Expected 1 category to be deleted, got {count} instead"))),
        }
    }
}