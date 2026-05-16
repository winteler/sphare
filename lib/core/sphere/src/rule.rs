use std::str::FromStr;

use leptos::prelude::*;
use leptos_fluent::move_tr;
use strum_macros::{Display, EnumString, IntoStaticStr};

#[derive(Clone, Copy, Debug, Default, Display, EnumString, Eq, IntoStaticStr, PartialEq)]
pub enum BaseRule {
    #[default]
    BeRespectful,
    RespectRules,
    NoIllegalContent,
    PlatformIntegrity,
}

impl BaseRule {
    pub fn get_localized_title(self) -> Signal<String> {
        match self {
            BaseRule::BeRespectful => move_tr!("rule-respectful-title"),
            BaseRule::RespectRules => move_tr!("rule-respect-rules-title"),
            BaseRule::NoIllegalContent => move_tr!("rule-no-illegal-content-title"),
            BaseRule::PlatformIntegrity => move_tr!("rule-platform-integrity-title"),
        }
    }

    pub fn get_localized_description(self) -> Signal<String> {
        match self {
            BaseRule::BeRespectful => move_tr!("rule-respectful-description"),
            BaseRule::RespectRules => move_tr!("rule-respect-rules-description"),
            BaseRule::NoIllegalContent => move_tr!("rule-no-illegal-content-description"),
            BaseRule::PlatformIntegrity => move_tr!("rule-platform-integrity-description"),
        }
    }
}

#[cfg(feature = "ssr")]
pub mod ssr {
    use sqlx::PgPool;
    use sphare_core_common::checks::{check_sphere_name, check_string_length};
    use sphare_core_common::common::Rule;
    use sphare_core_common::constants::{MAX_MOD_MESSAGE_LENGTH, MAX_TITLE_LENGTH};
    use sphare_core_common::editor::ssr::get_html_and_markdown_strings;
    use sphare_core_common::errors::AppError;
    use sphare_core_user::role::PermissionLevel;
    use sphare_core_user::user::User;

    pub async fn load_rule_by_id(
        rule_id: i64,
        db_pool: &PgPool,
    ) -> Result<Rule, AppError> {
        let rule = sqlx::query_as!(
            Rule,
            "SELECT * FROM rules
            WHERE rule_id = $1",
            rule_id
        )
            .fetch_one(db_pool)
            .await?;

        Ok(rule)
    }

    pub async fn get_rule_vec(
        sphere_name: Option<&str>,
        db_pool: &PgPool,
    ) -> Result<Vec<Rule>, AppError> {
        if let Some(sphere_name) = sphere_name {
            check_sphere_name(sphere_name)?;
        }
        let sphere_rule_vec = sqlx::query_as!(
            Rule,
            "SELECT r.* FROM rules r
            LEFT JOIN spheres s ON s.sphere_id = r.sphere_id
            WHERE
                COALESCE(s.sphere_name, $1) IS NOT DISTINCT FROM $1 AND
                r.delete_timestamp IS NULL
            ORDER BY s.sphere_name NULLS FIRST, r.priority, r.create_timestamp",
            sphere_name
        )
            .fetch_all(db_pool)
            .await?;

        Ok(sphere_rule_vec)
    }

    pub async fn add_rule(
        sphere_name: &str,
        priority: i16,
        title: &str,
        description: &str,
        is_markdown: bool,
        user: &User,
        db_pool: &PgPool,
    ) -> Result<Rule, AppError> {
        check_sphere_name(sphere_name)?;
        check_string_length(title, "Title", MAX_TITLE_LENGTH as usize, false)?;
        check_string_length(description, "Description", MAX_MOD_MESSAGE_LENGTH, true)?;
        user.check_sphere_permissions_by_name(sphere_name, PermissionLevel::Manage)?;
        let (description, markdown_description) = get_html_and_markdown_strings(description, is_markdown)?;

        sqlx::query!(
            "UPDATE rules
             SET priority = priority + 1
             WHERE sphere_id = (
                    SELECT sphere_id FROM spheres WHERE sphere_name = $1
                 ) AND priority >= $2 AND delete_timestamp IS NULL",
            sphere_name,
            priority,
        )
            .execute(db_pool)
            .await?;

        let rule = sqlx::query_as!(
            Rule,
            "INSERT INTO rules
            (sphere_id, priority, title, description, markdown_description, user_id)
            VALUES (
                (SELECT sphere_id FROM spheres WHERE sphere_name = $1),
                $2, $3, $4, $5, $6
            ) RETURNING *",
            sphere_name,
            priority,
            title,
            description,
            markdown_description,
            user.user_id,
        )
            .fetch_one(db_pool)
            .await?;

        Ok(rule)
    }

    pub async fn update_rule(
        sphere_name: &str,
        current_priority: i16,
        priority: i16,
        title: &str,
        description: &str,
        is_markdown: bool,
        user: &User,
        db_pool: &PgPool,
    ) -> Result<Rule, AppError> {
        check_sphere_name(sphere_name)?;
        check_string_length(title, "Title", MAX_TITLE_LENGTH as usize, false)?;
        check_string_length(description, "Description", MAX_MOD_MESSAGE_LENGTH, true)?;
        user.check_sphere_permissions_by_name(sphere_name, PermissionLevel::Manage)?;

        let (description, markdown_description) = get_html_and_markdown_strings(description, is_markdown)?;

        let current_rule = sqlx::query_as!(
            Rule,
            "UPDATE rules
             SET delete_timestamp = NOW()
             WHERE sphere_id = (
                    SELECT sphere_id FROM spheres where sphere_name = $1
                ) AND priority = $2 AND delete_timestamp IS NULL
             RETURNING *",
            sphere_name,
            current_priority,
        ).fetch_one(db_pool).await?;

        if priority > current_priority {
            sqlx::query!(
                "UPDATE rules
                SET priority = priority - 1
                WHERE sphere_id = (
                        SELECT sphere_id FROM spheres WHERE sphere_name = $1
                    ) AND priority BETWEEN $2 AND $3 AND delete_timestamp IS NULL",
                sphere_name,
                current_priority,
                priority,
            ).execute(db_pool).await?;
        } else if priority < current_priority {
            sqlx::query!(
                "UPDATE rules
                SET priority = priority + 1
                WHERE sphere_id = (
                        SELECT sphere_id FROM spheres WHERE sphere_name = $1
                    ) AND priority BETWEEN $3 AND $2 AND delete_timestamp IS NULL",
                sphere_name,
                current_priority,
                priority,
            ).execute(db_pool).await?;
        }

        let new_rule = sqlx::query_as!(
            Rule,
            "INSERT INTO rules
            (rule_key, sphere_id, priority, title, description, markdown_description, user_id)
            VALUES (
                $1,
                (SELECT sphere_id FROM spheres WHERE sphere_name = $2),
                $3, $4, $5, $6, $7
            ) RETURNING *",
            current_rule.rule_key,
            sphere_name,
            priority,
            title,
            description,
            markdown_description,
            user.user_id,
        ).fetch_one(db_pool).await?;

        Ok(new_rule)
    }

    pub async fn remove_rule(
        sphere_name: &str,
        priority: i16,
        user: &User,
        db_pool: &PgPool,
    ) -> Result<(), AppError> {
        check_sphere_name(sphere_name)?;
        user.check_sphere_permissions_by_name(sphere_name, PermissionLevel::Manage)?;

        sqlx::query!(
            "UPDATE rules
             SET delete_timestamp = NOW()
             WHERE sphere_id = (
                    SELECT sphere_id FROM spheres WHERE sphere_name = $1
                ) AND priority = $2 AND delete_timestamp IS NULL",
            sphere_name,
            priority,
        )
            .execute(db_pool)
            .await?;

        sqlx::query!(
            "UPDATE rules
             SET priority = priority - 1
             WHERE sphere_id = (
                    SELECT sphere_id FROM spheres WHERE sphere_name = $1
                ) AND priority > $2 AND delete_timestamp IS NULL",
            sphere_name,
            priority,
        )
            .execute(db_pool)
            .await?;

        Ok(())
    }
}

pub fn get_rule_title(rule_title: &str, is_sphere_rule: bool) -> Signal<String> {
    match is_sphere_rule {
        true => rule_title.into(),
        false => BaseRule::from_str(rule_title).unwrap_or_default().get_localized_title(),
    }
}

pub fn get_rule_description(
    rule_title: &str,
    rule_description: &str,
    is_sphere_rule: bool,
) -> Signal<String> {
    match is_sphere_rule {
        true => rule_description.into(),
        false => BaseRule::from_str(rule_title).unwrap_or_default().get_localized_description(),
    }
}