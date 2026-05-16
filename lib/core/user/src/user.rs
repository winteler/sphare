use std::cmp::max;
use std::collections::HashMap;
use std::default::Default;

use serde::{Deserialize, Serialize};

use sphare_core_common::errors::AppError;

use crate::role::{AdminRole, PermissionLevel};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub enum BanStatus {
    None,
    Until(chrono::DateTime<chrono::Utc>),
    Permanent,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct User {
    pub user_id: i64,
    pub oidc_id: String,
    pub username: String,
    pub email: String,
    pub is_nsfw: bool,
    pub admin_role: AdminRole,
    pub days_hide_spoiler: Option<i32>,
    pub show_nsfw: bool,
    pub permission_by_sphere_name_map: HashMap<String, PermissionLevel>,
    pub permission_by_sphere_id_map: HashMap<i64, PermissionLevel>,
    pub ban_status: BanStatus,
    pub ban_status_by_sphere_map: HashMap<String, BanStatus>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub delete_timestamp: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct UserHeader {
    pub username: String,
    pub is_nsfw: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct UserPostFilters {
    pub days_hide_spoiler: Option<i32>,
    pub show_nsfw: bool,
}

#[cfg_attr(feature = "ssr", derive(sqlx::FromRow))]
#[derive(Clone, Debug, PartialEq, Eq, Ord, PartialOrd, Serialize, Deserialize)]
pub struct UserBan {
    pub ban_id: i64,
    pub user_id: i64,
    pub username: String,
    pub sphere_id: Option<i64>,
    pub sphere_name: Option<String>,
    pub post_id: i64,
    pub comment_id: Option<i64>,
    pub infringed_rule_id: i64,
    pub moderator_id: i64,
    pub until_timestamp: Option<chrono::DateTime<chrono::Utc>>,
    pub create_timestamp: chrono::DateTime<chrono::Utc>,
    pub delete_timestamp: Option<chrono::DateTime<chrono::Utc>>,
}

impl BanStatus {
    pub fn is_permanent(&self) -> bool {
        *self == BanStatus::Permanent
    }
    pub fn is_active(&self) -> bool {
        match self {
            BanStatus::Until(until_timestamp) => *until_timestamp > chrono::offset::Utc::now(),
            _ => self.is_permanent(),
        }
    }
}

impl Default for User {
    fn default() -> Self {
        Self {
            user_id: -1,
            oidc_id: String::default(),
            username: String::default(),
            email: String::default(),
            is_nsfw: false,
            admin_role: AdminRole::None,
            show_nsfw: true,
            days_hide_spoiler: None,
            permission_by_sphere_name_map: HashMap::new(),
            permission_by_sphere_id_map: HashMap::new(),
            ban_status: BanStatus::None,
            ban_status_by_sphere_map: HashMap::new(),
            timestamp: chrono::DateTime::default(),
            delete_timestamp: None,
        }
    }
}

impl User {

    fn check_permissions(&self, user_permissions: Option<&PermissionLevel>, req_permission_level: PermissionLevel, use_admin_role: bool) -> Result<(), AppError> {
        let has_admin_permission = use_admin_role && self.admin_role.get_permission_level() >= req_permission_level;
        let has_sphere_permission = user_permissions.is_some_and(|permission_level| *permission_level >= req_permission_level);
        match has_admin_permission || has_sphere_permission {
            true => Ok(()),
            false => Err(AppError::InsufficientPrivileges)
        }
    }
    
    pub fn check_admin_role(&self, req_admin_role: AdminRole) -> Result<(), AppError> {
        match self.admin_role >= req_admin_role {
            true => Ok(()),
            false => Err(AppError::InsufficientPrivileges),
        }
    }

    pub fn check_sphere_permissions_by_name(&self, sphere_name: &str, req_permission_level: PermissionLevel) -> Result<(), AppError> {
        self.check_permissions(self.permission_by_sphere_name_map.get(sphere_name), req_permission_level, true)
    }

    pub fn check_sphere_permissions_by_id(&self, sphere_id: i64, req_permission_level: PermissionLevel) -> Result<(), AppError> {
        self.check_permissions(self.permission_by_sphere_id_map.get(&sphere_id), req_permission_level, true)
    }

    pub fn check_is_sphere_leader(&self, sphere_name: &str) -> Result<(), AppError> {
        self.check_permissions(self.permission_by_sphere_name_map.get(sphere_name), PermissionLevel::Lead, false)
    }

    pub fn get_sphere_permission_level(&self, sphere_name: &str) -> PermissionLevel {
        max(self.admin_role.get_permission_level(), self.permission_by_sphere_name_map.get(sphere_name).cloned().unwrap_or(PermissionLevel::None))
    }
    
    pub fn check_can_publish(&self) -> Result<(), AppError> {
        match self.ban_status.is_active() {
            true => match self.ban_status {
                BanStatus::Until(timestamp) => Err(AppError::GlobalBanUntil(timestamp)),
                BanStatus::Permanent => Err(AppError::PermanentGlobalBan),
                BanStatus::None => Err(AppError::InternalServerError(String::from("User with BanStatus::None despite ban_status.is_active == true"))), // should never happen
            },
            false => Ok(())
        }
    }

    pub fn check_can_publish_on_sphere(&self, sphere_name: &str) -> Result<(), AppError> {
        self.check_can_publish()?;
        match self.ban_status_by_sphere_map.get(sphere_name) {
            Some(ban_status) if ban_status.is_active() => match ban_status {
                BanStatus::Until(timestamp) => Err(AppError::SphereBanUntil(*timestamp)),
                BanStatus::Permanent => Err(AppError::PermanentSphereBan),
                BanStatus::None => Err(
                    AppError::InternalServerError(
                        String::from("User with sphere BanStatus::None despite ban_status.is_active == true")
                    )
                ), // should never happen
            },
            _ => Ok(())
        }
    }

    pub fn get_posts_filter(&self) -> UserPostFilters {
        UserPostFilters {
            days_hide_spoiler: self.days_hide_spoiler,
            show_nsfw: self.show_nsfw,
        }
    }
}

impl Default for UserPostFilters {
    fn default() -> Self {
        UserPostFilters {
            days_hide_spoiler: None,
            show_nsfw: true,
        }
    }
}

#[cfg(feature = "ssr")]
pub mod ssr {
    use std::cmp::min;
    use std::num::NonZeroUsize;
    use std::sync::Arc;

    use async_trait::async_trait;
    use axum_session_auth::Authentication;
    use lru::LruCache;
    use sqlx::PgPool;
    use tokio::sync::Mutex;

    use sphare_core_common::checks::check_username;
    use sphare_core_common::constants::USER_FETCH_LIMIT;
    use sphare_core_common::errors::AppError;

    use crate::role::ssr::get_user_sphere_role;
    use crate::role::UserSphereRole;

    use super::*;

    #[derive(sqlx::FromRow, Clone, Debug, PartialEq)]
    pub struct SqlUser {
        pub user_id: i64,
        pub oidc_id: String,
        pub username: String,
        pub email: String,
        pub is_nsfw: bool,
        pub admin_role: AdminRole,
        pub show_nsfw: bool,
        pub days_hide_spoiler: Option<i32>,
        pub timestamp: chrono::DateTime<chrono::Utc>,
        pub delete_timestamp: Option<chrono::DateTime<chrono::Utc>>,
    }

    impl SqlUser {
        pub async fn get_by_username(
            username: &str,
            db_pool: &PgPool,
        ) -> Result<SqlUser, AppError> {
            let sql_user = sqlx::query_as!(
            SqlUser,
            "SELECT * FROM users WHERE username = $1",
            username,
        )
                .fetch_one(db_pool)
                .await?;

            Ok(sql_user)
        }

        pub fn into_user(
            self,
            user_role_vec: Vec<UserSphereRole>,
            user_ban_vec: Vec<UserBan>,
        ) -> User {
            let mut permission_by_sphere_name_map: HashMap<String, PermissionLevel> = HashMap::new();
            let mut permission_by_sphere_id_map: HashMap<i64, PermissionLevel> = HashMap::new();
            for user_sphere_role in user_role_vec {
                permission_by_sphere_name_map.insert(
                    user_sphere_role.sphere_name.clone(),
                    user_sphere_role.permission_level,
                );
                permission_by_sphere_id_map.insert(
                    user_sphere_role.sphere_id,
                    user_sphere_role.permission_level,
                );
            }
            let mut global_ban_status = BanStatus::None;
            let mut ban_status_by_sphere_map: HashMap<String, BanStatus> = HashMap::new();
            let current_timestamp = chrono::offset::Utc::now();
            for user_ban in user_ban_vec {
                let (ban_status, is_valid) = match user_ban.until_timestamp {
                    Some(until_timestamp) => (
                        BanStatus::Until(until_timestamp),
                        until_timestamp > current_timestamp,
                    ),
                    None => (BanStatus::Permanent, true),
                };
                if is_valid {
                    match user_ban.sphere_name {
                        Some(sphere_name) => {
                            match ban_status_by_sphere_map.get_mut(&sphere_name) {
                                Some(current_ban_status) => {
                                    if ban_status > *current_ban_status {
                                        *current_ban_status = ban_status;
                                    }
                                },
                                None => _ = ban_status_by_sphere_map.insert(sphere_name, ban_status),
                            };
                        },
                        None => {
                            if ban_status > global_ban_status {
                                global_ban_status = ban_status;
                            }
                        }
                    };
                }
            }

            User {
                user_id: self.user_id,
                oidc_id: self.oidc_id,
                username: self.username,
                email: self.email,
                is_nsfw: self.is_nsfw,
                admin_role: self.admin_role,
                show_nsfw: self.show_nsfw,
                days_hide_spoiler: self.days_hide_spoiler,
                permission_by_sphere_name_map,
                permission_by_sphere_id_map,
                ban_status: global_ban_status,
                ban_status_by_sphere_map,
                timestamp: self.timestamp,
                delete_timestamp: self.delete_timestamp,
            }
        }
    }

    // Map of (user_id, lock) to guarantee thread-safety when performing some operations, such as refreshing tokens
    #[derive(Debug)]

    pub struct UserLockCache {
        lock_cache: Mutex<LruCache<i64, Arc<Mutex<()>>>>,
    }

    impl UserLockCache {
        pub fn new(max_size: NonZeroUsize) -> Self {

            Self {
                lock_cache: Mutex::new(LruCache::new(max_size)),
            }
        }

        // Get or insert a lock for the user, updating the LRU cache
        pub async fn get_user_lock(&self, user_id: i64) -> Arc<Mutex<()>> {
            let mut lock_cache = self.lock_cache.lock().await;

            // If the user ID exists, update access and return the lock
            if let Some(lock) = lock_cache.get(&user_id) {
                return Arc::clone(lock);
            }

            // If the user ID does not exist, insert a new entry
            let new_lock = Arc::new(Mutex::new(()));
            lock_cache.put(user_id, Arc::clone(&new_lock));
            new_lock
        }
    }

    impl User {
        pub async fn get(user_id: i64, db_pool: &PgPool) -> Option<Self> {
            match sqlx::query_as!(SqlUser, "SELECT * FROM users WHERE user_id = $1", user_id)
                .fetch_one(db_pool)
                .await
            {
                Ok(sql_user) => {
                    let user_sphere_role_vec = load_user_sphere_role_vec(sql_user.user_id, db_pool)
                        .await
                        .unwrap_or_default();
                    let user_ban_vec = load_user_ban_vec(sql_user.user_id, db_pool)
                        .await
                        .unwrap_or_default();
                    Some(sql_user.into_user(user_sphere_role_vec, user_ban_vec))
                }
                Err(select_error) => {
                    log::debug!("User not found with error: {}", select_error);
                    None
                }
            }
        }

        pub async fn check_can_set_user_sphere_role(
            &self,
            permission_level: PermissionLevel,
            user_id: i64,
            sphere_name: &str,
            db_pool: &PgPool,
        ) -> Result<(), AppError> {
            let user_role = get_user_sphere_role(user_id, sphere_name, db_pool).await;
            match (self.admin_role, self.permission_by_sphere_name_map.get(sphere_name), user_role) {
                (AdminRole::Admin, _, Ok(user_role)) |
                (_, Some(PermissionLevel::Lead), Ok(user_role)) if user_role.permission_level < PermissionLevel::Lead => Ok(()),
                (AdminRole::Admin, _, Err(AppError::NotFound)) |
                (_, Some(PermissionLevel::Lead), Err(AppError::NotFound)) => Ok(()),
                (_, Some(PermissionLevel::Manage), Ok(user_role))
                if permission_level < PermissionLevel::Manage && user_role.permission_level < PermissionLevel::Manage  => Ok(()),
                (_, Some(own_level), Err(AppError::NotFound)) if *own_level >= PermissionLevel::Manage && *own_level > permission_level => Ok(()),
                _ => Err(AppError::InsufficientPrivileges),
            }
        }
    }

    #[async_trait]
    impl Authentication<User, i64, PgPool> for User {
        async fn load_user(user_id: i64, pool: Option<&PgPool>) -> Result<User, anyhow::Error> {
            let pool = pool.ok_or(anyhow::anyhow!("Cannot get DB pool"))?;

            User::get(user_id, pool)
                .await
                .ok_or_else(|| anyhow::anyhow!("Cannot get user"))
        }

        fn is_authenticated(&self) -> bool {
            true
        }

        fn is_active(&self) -> bool {
            true
        }

        fn is_anonymous(&self) -> bool {
            false
        }
    }

    pub async fn create_or_update_user(
        oidc_id: &str,
        username: &str,
        email: &str,
        db_pool: &PgPool,
    ) -> Result<SqlUser, AppError> {
        log::debug!("Create or update user {username} with oidc id = {oidc_id}");
        let sql_user = sqlx::query_as!(
            SqlUser,
            "INSERT INTO users (oidc_id, username, email)
            VALUES ($1, $2, $3)
            ON CONFLICT (oidc_id) DO UPDATE
                SET username = EXCLUDED.username,
                    email = EXCLUDED.email
            RETURNING *",
            oidc_id,
            username,
            email,
        )
            .fetch_one(db_pool)
            .await?;

        Ok(sql_user)
    }

    async fn load_user_sphere_role_vec(
        user_id: i64,
        db_pool: &PgPool,
    ) -> Result<Vec<UserSphereRole>, AppError> {
        let user_sphere_role_vec = sqlx::query_as!(
            UserSphereRole,
            "SELECT r.*, u.username, s.sphere_name
            FROM user_sphere_roles r
            JOIN users u ON u.user_id = r.user_id
            JOIN spheres s ON s.sphere_id = r.sphere_id
            WHERE r.user_id = $1 AND r.delete_timestamp IS NULL",
            user_id
        )
            .fetch_all(db_pool)
            .await?;
        log::trace!("User roles: {:?}", user_sphere_role_vec);
        Ok(user_sphere_role_vec)
    }

    async fn load_user_ban_vec(user_id: i64, db_pool: &PgPool) -> Result<Vec<UserBan>, AppError> {
        let user_ban_vec = sqlx::query_as!(
            UserBan,
            "SELECT b.*, u.username, s.sphere_name FROM user_bans b
            JOIN users u ON u.user_id = b.user_id
            JOIN spheres s ON s.sphere_id = b.sphere_id
            WHERE
                b.user_id = $1 AND
                (b.until_timestamp > NOW() OR b.until_timestamp IS NULL) AND
                b.delete_timestamp IS NULL",
            user_id,
        )
            .fetch_all(db_pool)
            .await?;
        log::trace!("User bans: {:?}", user_ban_vec);
        Ok(user_ban_vec)
    }

    pub async fn get_matching_user_header_vec(
        username_prefix: &str,
        show_nsfw: bool,
        limit: i64,
        db_pool: &PgPool,
    ) -> Result<Vec<UserHeader>, AppError> {
        check_username(username_prefix, false)?;
        let user_header_vec = sqlx::query_as!(
            UserHeader,
            "SELECT username, is_nsfw
            FROM users
            WHERE
                username LIKE $1 AND
                ($2 OR NOT is_nsfw) AND
                delete_timestamp IS NULL
            ORDER BY username LIMIT $3",
            format!("{username_prefix}%"),
            show_nsfw,
            min(limit, USER_FETCH_LIMIT),
        )
            .fetch_all(db_pool)
            .await?;

        Ok(user_header_vec)
    }

    pub async fn set_user_settings(
        is_nsfw: bool,
        show_nsfw: bool,
        days_hide_spoilers: u32,
        user: &User,
        db_pool: &PgPool,
    ) -> Result<(), AppError> {
        let days_hide_spoilers = match days_hide_spoilers {
            x if x > 0 => Some(x as i32),
            _ => None,
        };
        sqlx::query!(
            "UPDATE users SET
                is_nsfw = $1,
                show_nsfw = $2,
                days_hide_spoiler = $3
            WHERE user_id = $4",
            is_nsfw,
            show_nsfw,
            days_hide_spoilers,
            user.user_id,
        )
            .execute(db_pool)
            .await?;
        Ok(())
    }

    pub async fn delete_user(
        user: &User,
        db_pool: &PgPool,
    ) -> Result<(), AppError> {
        log::info!("Delete user {} with id = {}", user.username, user.user_id);
        delete_user_posts(user, db_pool).await?;
        delete_user_comments(user, db_pool).await?;
        delete_user_roles(user, db_pool).await?;
        delete_user_bans(user, db_pool).await?;

        sqlx::query!(
            "UPDATE users SET
                 username = '',
                 email = '',
                 is_nsfw = false,
                 admin_role = 'None',
                 days_hide_spoiler = NULL,
                 show_nsfw = false,
                 timestamp = NOW(),
                 delete_timestamp = NOW()
            WHERE user_id = $1",
            user.user_id,
        )
            .execute(db_pool)
            .await?;

        Ok(())
    }

    async fn delete_user_posts(
        user: &User,
        db_pool: &PgPool,
    ) -> Result<(), AppError> {
        sqlx::query!(
            "UPDATE posts SET
                title = '',
                body = '',
                markdown_body = NULL,
                link_type = -1,
                link_url = NULL,
                link_embed = NULL,
                link_thumbnail_url = NULL,
                is_nsfw = false,
                is_spoiler = false,
                is_pinned = false,
                category_id = NULL,
                edit_timestamp = NOW(),
                delete_timestamp = NOW()
            WHERE creator_id = $1",
            user.user_id,
        )
            .execute(db_pool)
            .await?;

        Ok(())
    }

    async fn delete_user_comments(
        user: &User,
        db_pool: &PgPool,
    ) -> Result<(), AppError> {
        sqlx::query!(
            "UPDATE comments SET
                body = '',
                markdown_body = NULL,
                is_pinned = false,
                edit_timestamp = NOW(),
                delete_timestamp = NOW()
            WHERE creator_id = $1",
            user.user_id,
        )
            .execute(db_pool)
            .await?;

        Ok(())
    }

    async fn delete_user_roles(
        user: &User,
        db_pool: &PgPool,
    ) -> Result<(), AppError> {
        sqlx::query!(
            "UPDATE user_sphere_roles
            SET delete_timestamp = NOW()
            WHERE user_id = $1 AND delete_timestamp IS NULL",
            user.user_id,
        )
            .execute(db_pool)
            .await?;

        Ok(())
    }

    async fn delete_user_bans(
        user: &User,
        db_pool: &PgPool,
    ) -> Result<(), AppError> {
        sqlx::query!(
            "UPDATE user_bans
            SET delete_timestamp = NOW()
            WHERE user_id = $1 AND delete_timestamp IS NULL",
            user.user_id
        )
            .execute(db_pool)
            .await?;

        Ok(())
    }

    #[cfg(test)]
    mod tests {
        use std::ops::Add;

        use chrono::Days;

        use super::*;

        #[test]
        fn test_sql_user_into_user() {
            let past_timestamp = chrono::DateTime::from_timestamp_nanos(0);
            let future_timestamp = chrono::offset::Utc::now().add(Days::new(1));
            let sql_user = SqlUser {
                user_id: 0,
                oidc_id: String::from("a"),
                username: String::from("b"),
                email: String::from("c"),
                is_nsfw: false,
                admin_role: AdminRole::None,
                show_nsfw: true,
                days_hide_spoiler: None,
                timestamp: chrono::DateTime::from_timestamp_nanos(0),
                delete_timestamp: None,
            };
            let user_sphere_role_vec = vec![
                UserSphereRole {
                    role_id: 0,
                    user_id: 0,
                    username: String::from("b"),
                    sphere_id: 0,
                    sphere_name: String::from("0"),
                    permission_level: PermissionLevel::Moderate,
                    grantor_id: 0,
                    create_timestamp: past_timestamp,
                    delete_timestamp: None,
                },
                UserSphereRole {
                    role_id: 0,
                    user_id: 0,
                    username: String::from("b"),
                    sphere_id: 1,
                    sphere_name: String::from("1"),
                    permission_level: PermissionLevel::Lead,
                    grantor_id: 0,
                    create_timestamp: past_timestamp,
                    delete_timestamp: None,
                },
            ];
            let user_ban_vec = vec![
                UserBan {
                    ban_id: 0,
                    user_id: 0,
                    username: String::from("b"),
                    sphere_id: None,
                    sphere_name: None,
                    post_id: 0,
                    comment_id: None,
                    infringed_rule_id: 0,
                    moderator_id: 0,
                    until_timestamp: Some(past_timestamp),
                    create_timestamp: Default::default(),
                    delete_timestamp: None,
                },
                UserBan {
                    ban_id: 1,
                    user_id: 0,
                    username: String::from("b"),
                    sphere_id: Some(0),
                    sphere_name: Some(String::from("a")),
                    post_id: 0,
                    comment_id: None,
                    infringed_rule_id: 0,
                    moderator_id: 0,
                    until_timestamp: Some(past_timestamp),
                    create_timestamp: Default::default(),
                    delete_timestamp: None,
                },
                UserBan {
                    ban_id: 2,
                    user_id: 0,
                    username: String::from("b"),
                    sphere_id: Some(1),
                    sphere_name: Some(String::from("b")),
                    post_id: 0,
                    comment_id: None,
                    infringed_rule_id: 0,
                    moderator_id: 0,
                    until_timestamp: Some(future_timestamp),
                    create_timestamp: Default::default(),
                    delete_timestamp: None,
                },
                UserBan {
                    ban_id: 3,
                    user_id: 0,
                    username: String::from("b"),
                    sphere_id: Some(2),
                    sphere_name: Some(String::from("c")),
                    post_id: 0,
                    comment_id: None,
                    infringed_rule_id: 0,
                    moderator_id: 0,
                    until_timestamp: None,
                    create_timestamp: Default::default(),
                    delete_timestamp: None,
                },
            ];
            let user_1 = sql_user.clone().into_user(user_sphere_role_vec.clone(), user_ban_vec);
            assert_eq!(user_1.user_id, 0);
            assert_eq!(user_1.oidc_id, "a");
            assert_eq!(user_1.username, "b");
            assert_eq!(user_1.email, "c");
            assert_eq!(user_1.admin_role, AdminRole::None);
            assert_eq!(user_1.timestamp, chrono::DateTime::from_timestamp_nanos(0));
            assert_eq!(user_1.delete_timestamp, None);
            assert_eq!(user_1.permission_by_sphere_name_map[&String::from("0")], PermissionLevel::Moderate);
            assert_eq!(user_1.permission_by_sphere_name_map[&String::from("1")], PermissionLevel::Lead);
            assert_eq!(user_1.ban_status, BanStatus::None);
            assert_eq!(user_1.ban_status_by_sphere_map.get(&String::from("a")), None);
            assert_eq!(
                *user_1.ban_status_by_sphere_map.get(&String::from("b")).expect("User should have ban for sphere 'b'."),
                BanStatus::Until(future_timestamp)
            );
            assert_eq!(
                *user_1.ban_status_by_sphere_map.get(&String::from("c")).expect("User should have ban for sphere 'c'."),
                BanStatus::Permanent
            );

            let user_2_ban_vec = vec![UserBan {
                ban_id: 3,
                user_id: 0,
                username: String::from("b"),
                sphere_id: None,
                sphere_name: None,
                post_id: 0,
                comment_id: None,
                infringed_rule_id: 0,
                moderator_id: 0,
                until_timestamp: Some(future_timestamp),
                create_timestamp: Default::default(),
                delete_timestamp: None,
            }];
            let user_2 = sql_user.into_user(user_sphere_role_vec, user_2_ban_vec);
            assert_eq!(user_2.ban_status, BanStatus::Until(future_timestamp));
        }
    }
}

#[cfg(test)]
mod tests {
    use std::ops::Add;

    use chrono::Days;

    use super::*;

    fn get_user_sphere_permission_by_name_map() -> HashMap<String, PermissionLevel> {
        HashMap::from([
            (String::from("a"), PermissionLevel::None),
            (String::from("b"), PermissionLevel::Moderate),
            (String::from("c"), PermissionLevel::Ban),
            (String::from("d"), PermissionLevel::Manage),
            (String::from("e"), PermissionLevel::Lead),
        ])
    }

    fn get_user_sphere_permission_by_id_map() -> HashMap<i64, PermissionLevel> {
        HashMap::from([
            (1, PermissionLevel::None),
            (2, PermissionLevel::Moderate),
            (3, PermissionLevel::Ban),
            (4, PermissionLevel::Manage),
            (5, PermissionLevel::Lead),
        ])
    }

    #[test]
    fn test_ban_status_is_permanent() {
        let ban_status_none = BanStatus::None;
        let ban_status_until = BanStatus::Until(chrono::DateTime::from_timestamp_nanos(0));
        let ban_status_permanent = BanStatus::Permanent;
        assert_eq!(ban_status_none.is_permanent(), false);
        assert_eq!(ban_status_until.is_permanent(), false);
        assert_eq!(ban_status_permanent.is_permanent(), true);
    }

    #[test]
    fn test_ban_status_is_active() {
        let ban_status_none = BanStatus::None;
        let ban_status_until_past = BanStatus::Until(chrono::DateTime::from_timestamp_nanos(0));
        let ban_status_until_future = BanStatus::Until(chrono::offset::Utc::now().add(Days::new(1)));
        let ban_status_permanent = BanStatus::Permanent;
        assert_eq!(ban_status_none.is_active(), false);
        assert_eq!(ban_status_until_past.is_active(), false);
        assert_eq!(ban_status_until_future.is_active(), true);
        assert_eq!(ban_status_permanent.is_active(), true);
    }

    #[test]
    fn test_user_check_admin_role() {
        let mut user = User::default();
        assert_eq!(user.check_admin_role(AdminRole::None), Ok(()));
        assert_eq!(user.check_admin_role(AdminRole::Moderator), Err(AppError::InsufficientPrivileges));
        assert_eq!(user.check_admin_role(AdminRole::Admin), Err(AppError::InsufficientPrivileges));
        user.admin_role = AdminRole::Moderator;
        assert_eq!(user.check_admin_role(AdminRole::None), Ok(()));
        assert_eq!(user.check_admin_role(AdminRole::Moderator), Ok(()));
        assert_eq!(user.check_admin_role(AdminRole::Admin), Err(AppError::InsufficientPrivileges));
        user.admin_role = AdminRole::Admin;
        assert_eq!(user.check_admin_role(AdminRole::None), Ok(()));
        assert_eq!(user.check_admin_role(AdminRole::Moderator), Ok(()));
        assert_eq!(user.check_admin_role(AdminRole::Admin), Ok(()));
    }

    #[test]
    fn test_user_check_check_sphere_permission_by_name() {
        let mut user = User::default();
        user.permission_by_sphere_name_map = get_user_sphere_permission_by_name_map();
        assert_eq!(user.check_sphere_permissions_by_name("missing", PermissionLevel::None), Ok(()));
        assert_eq!(user.check_sphere_permissions_by_name("a",       PermissionLevel::None), Ok(()));
        assert_eq!(user.check_sphere_permissions_by_name("b",       PermissionLevel::None), Ok(()));
        assert_eq!(user.check_sphere_permissions_by_name("c",       PermissionLevel::None), Ok(()));
        assert_eq!(user.check_sphere_permissions_by_name("d",       PermissionLevel::None), Ok(()));
        assert_eq!(user.check_sphere_permissions_by_name("e",       PermissionLevel::None), Ok(()));

        assert_eq!(user.check_sphere_permissions_by_name("missing", PermissionLevel::Moderate), Err(AppError::InsufficientPrivileges));
        assert_eq!(user.check_sphere_permissions_by_name("a",       PermissionLevel::Moderate), Err(AppError::InsufficientPrivileges));
        assert_eq!(user.check_sphere_permissions_by_name("b",       PermissionLevel::Moderate), Ok(()));
        assert_eq!(user.check_sphere_permissions_by_name("c",       PermissionLevel::Moderate), Ok(()));
        assert_eq!(user.check_sphere_permissions_by_name("d",       PermissionLevel::Moderate), Ok(()));
        assert_eq!(user.check_sphere_permissions_by_name("e",       PermissionLevel::Moderate), Ok(()));

        assert_eq!(user.check_sphere_permissions_by_name("missing", PermissionLevel::Ban), Err(AppError::InsufficientPrivileges));
        assert_eq!(user.check_sphere_permissions_by_name("a",       PermissionLevel::Ban), Err(AppError::InsufficientPrivileges));
        assert_eq!(user.check_sphere_permissions_by_name("b",       PermissionLevel::Ban), Err(AppError::InsufficientPrivileges));
        assert_eq!(user.check_sphere_permissions_by_name("c",       PermissionLevel::Ban), Ok(()));
        assert_eq!(user.check_sphere_permissions_by_name("d",       PermissionLevel::Ban), Ok(()));
        assert_eq!(user.check_sphere_permissions_by_name("e",       PermissionLevel::Ban), Ok(()));

        assert_eq!(user.check_sphere_permissions_by_name("missing", PermissionLevel::Manage), Err(AppError::InsufficientPrivileges));
        assert_eq!(user.check_sphere_permissions_by_name("a",       PermissionLevel::Manage), Err(AppError::InsufficientPrivileges));
        assert_eq!(user.check_sphere_permissions_by_name("b",       PermissionLevel::Manage), Err(AppError::InsufficientPrivileges));
        assert_eq!(user.check_sphere_permissions_by_name("c",       PermissionLevel::Manage), Err(AppError::InsufficientPrivileges));
        assert_eq!(user.check_sphere_permissions_by_name("d",       PermissionLevel::Manage), Ok(()));
        assert_eq!(user.check_sphere_permissions_by_name("e",       PermissionLevel::Manage), Ok(()));

        assert_eq!(user.check_sphere_permissions_by_name("missing", PermissionLevel::Lead), Err(AppError::InsufficientPrivileges));
        assert_eq!(user.check_sphere_permissions_by_name("a",       PermissionLevel::Lead), Err(AppError::InsufficientPrivileges));
        assert_eq!(user.check_sphere_permissions_by_name("b",       PermissionLevel::Lead), Err(AppError::InsufficientPrivileges));
        assert_eq!(user.check_sphere_permissions_by_name("c",       PermissionLevel::Lead), Err(AppError::InsufficientPrivileges));
        assert_eq!(user.check_sphere_permissions_by_name("d",       PermissionLevel::Lead), Err(AppError::InsufficientPrivileges));
        assert_eq!(user.check_sphere_permissions_by_name("e",       PermissionLevel::Lead), Ok(()));

        user.admin_role = AdminRole::Moderator;

        assert_eq!(user.check_sphere_permissions_by_name("missing", PermissionLevel::None), Ok(()));
        assert_eq!(user.check_sphere_permissions_by_name("a",       PermissionLevel::None), Ok(()));
        assert_eq!(user.check_sphere_permissions_by_name("b",       PermissionLevel::None), Ok(()));
        assert_eq!(user.check_sphere_permissions_by_name("c",       PermissionLevel::None), Ok(()));
        assert_eq!(user.check_sphere_permissions_by_name("d",       PermissionLevel::None), Ok(()));
        assert_eq!(user.check_sphere_permissions_by_name("e",       PermissionLevel::None), Ok(()));

        assert_eq!(user.check_sphere_permissions_by_name("missing", PermissionLevel::Moderate), Ok(()));
        assert_eq!(user.check_sphere_permissions_by_name("a",       PermissionLevel::Moderate), Ok(()));
        assert_eq!(user.check_sphere_permissions_by_name("b",       PermissionLevel::Moderate), Ok(()));
        assert_eq!(user.check_sphere_permissions_by_name("c",       PermissionLevel::Moderate), Ok(()));
        assert_eq!(user.check_sphere_permissions_by_name("d",       PermissionLevel::Moderate), Ok(()));
        assert_eq!(user.check_sphere_permissions_by_name("e",       PermissionLevel::Moderate), Ok(()));

        assert_eq!(user.check_sphere_permissions_by_name("missing", PermissionLevel::Ban), Ok(()));
        assert_eq!(user.check_sphere_permissions_by_name("a",       PermissionLevel::Ban), Ok(()));
        assert_eq!(user.check_sphere_permissions_by_name("b",       PermissionLevel::Ban), Ok(()));
        assert_eq!(user.check_sphere_permissions_by_name("c",       PermissionLevel::Ban), Ok(()));
        assert_eq!(user.check_sphere_permissions_by_name("d",       PermissionLevel::Ban), Ok(()));
        assert_eq!(user.check_sphere_permissions_by_name("e",       PermissionLevel::Ban), Ok(()));

        assert_eq!(user.check_sphere_permissions_by_name("missing", PermissionLevel::Manage), Err(AppError::InsufficientPrivileges));
        assert_eq!(user.check_sphere_permissions_by_name("a",       PermissionLevel::Manage), Err(AppError::InsufficientPrivileges));
        assert_eq!(user.check_sphere_permissions_by_name("b",       PermissionLevel::Manage), Err(AppError::InsufficientPrivileges));
        assert_eq!(user.check_sphere_permissions_by_name("c",       PermissionLevel::Manage), Err(AppError::InsufficientPrivileges));
        assert_eq!(user.check_sphere_permissions_by_name("d",       PermissionLevel::Manage), Ok(()));
        assert_eq!(user.check_sphere_permissions_by_name("e",       PermissionLevel::Manage), Ok(()));

        assert_eq!(user.check_sphere_permissions_by_name("missing", PermissionLevel::Lead), Err(AppError::InsufficientPrivileges));
        assert_eq!(user.check_sphere_permissions_by_name("a",       PermissionLevel::Lead), Err(AppError::InsufficientPrivileges));
        assert_eq!(user.check_sphere_permissions_by_name("b",       PermissionLevel::Lead), Err(AppError::InsufficientPrivileges));
        assert_eq!(user.check_sphere_permissions_by_name("c",       PermissionLevel::Lead), Err(AppError::InsufficientPrivileges));
        assert_eq!(user.check_sphere_permissions_by_name("d",       PermissionLevel::Lead), Err(AppError::InsufficientPrivileges));
        assert_eq!(user.check_sphere_permissions_by_name("e",       PermissionLevel::Lead), Ok(()));

        user.admin_role = AdminRole::Admin;

        assert_eq!(user.check_sphere_permissions_by_name("missing", PermissionLevel::None), Ok(()));
        assert_eq!(user.check_sphere_permissions_by_name("a",       PermissionLevel::None), Ok(()));
        assert_eq!(user.check_sphere_permissions_by_name("b",       PermissionLevel::None), Ok(()));
        assert_eq!(user.check_sphere_permissions_by_name("c",       PermissionLevel::None), Ok(()));
        assert_eq!(user.check_sphere_permissions_by_name("d",       PermissionLevel::None), Ok(()));
        assert_eq!(user.check_sphere_permissions_by_name("e",       PermissionLevel::None), Ok(()));

        assert_eq!(user.check_sphere_permissions_by_name("missing", PermissionLevel::Moderate), Ok(()));
        assert_eq!(user.check_sphere_permissions_by_name("a",       PermissionLevel::Moderate), Ok(()));
        assert_eq!(user.check_sphere_permissions_by_name("b",       PermissionLevel::Moderate), Ok(()));
        assert_eq!(user.check_sphere_permissions_by_name("c",       PermissionLevel::Moderate), Ok(()));
        assert_eq!(user.check_sphere_permissions_by_name("d",       PermissionLevel::Moderate), Ok(()));
        assert_eq!(user.check_sphere_permissions_by_name("e",       PermissionLevel::Moderate), Ok(()));

        assert_eq!(user.check_sphere_permissions_by_name("missing", PermissionLevel::Ban), Ok(()));
        assert_eq!(user.check_sphere_permissions_by_name("a",       PermissionLevel::Ban), Ok(()));
        assert_eq!(user.check_sphere_permissions_by_name("b",       PermissionLevel::Ban), Ok(()));
        assert_eq!(user.check_sphere_permissions_by_name("c",       PermissionLevel::Ban), Ok(()));
        assert_eq!(user.check_sphere_permissions_by_name("d",       PermissionLevel::Ban), Ok(()));
        assert_eq!(user.check_sphere_permissions_by_name("e",       PermissionLevel::Ban), Ok(()));

        assert_eq!(user.check_sphere_permissions_by_name("missing", PermissionLevel::Manage), Ok(()));
        assert_eq!(user.check_sphere_permissions_by_name("a",       PermissionLevel::Manage), Ok(()));
        assert_eq!(user.check_sphere_permissions_by_name("b",       PermissionLevel::Manage), Ok(()));
        assert_eq!(user.check_sphere_permissions_by_name("c",       PermissionLevel::Manage), Ok(()));
        assert_eq!(user.check_sphere_permissions_by_name("d",       PermissionLevel::Manage), Ok(()));
        assert_eq!(user.check_sphere_permissions_by_name("e",       PermissionLevel::Manage), Ok(()));

        assert_eq!(user.check_sphere_permissions_by_name("missing", PermissionLevel::Lead), Ok(()));
        assert_eq!(user.check_sphere_permissions_by_name("a",       PermissionLevel::Lead), Ok(()));
        assert_eq!(user.check_sphere_permissions_by_name("b",       PermissionLevel::Lead), Ok(()));
        assert_eq!(user.check_sphere_permissions_by_name("c",       PermissionLevel::Lead), Ok(()));
        assert_eq!(user.check_sphere_permissions_by_name("d",       PermissionLevel::Lead), Ok(()));
        assert_eq!(user.check_sphere_permissions_by_name("e",       PermissionLevel::Lead), Ok(()));
    }

    #[test]
    fn test_user_check_check_sphere_permission_by_id() {
        let mut user = User::default();
        user.permission_by_sphere_id_map = get_user_sphere_permission_by_id_map();
        assert_eq!(user.check_sphere_permissions_by_id(0, PermissionLevel::None), Ok(()));
        assert_eq!(user.check_sphere_permissions_by_id(1, PermissionLevel::None), Ok(()));
        assert_eq!(user.check_sphere_permissions_by_id(2, PermissionLevel::None), Ok(()));
        assert_eq!(user.check_sphere_permissions_by_id(3, PermissionLevel::None), Ok(()));
        assert_eq!(user.check_sphere_permissions_by_id(4, PermissionLevel::None), Ok(()));
        assert_eq!(user.check_sphere_permissions_by_id(5, PermissionLevel::None), Ok(()));

        assert_eq!(user.check_sphere_permissions_by_id(0, PermissionLevel::Moderate), Err(AppError::InsufficientPrivileges));
        assert_eq!(user.check_sphere_permissions_by_id(1, PermissionLevel::Moderate), Err(AppError::InsufficientPrivileges));
        assert_eq!(user.check_sphere_permissions_by_id(2, PermissionLevel::Moderate), Ok(()));
        assert_eq!(user.check_sphere_permissions_by_id(3, PermissionLevel::Moderate), Ok(()));
        assert_eq!(user.check_sphere_permissions_by_id(4, PermissionLevel::Moderate), Ok(()));
        assert_eq!(user.check_sphere_permissions_by_id(5, PermissionLevel::Moderate), Ok(()));

        assert_eq!(user.check_sphere_permissions_by_id(0, PermissionLevel::Ban), Err(AppError::InsufficientPrivileges));
        assert_eq!(user.check_sphere_permissions_by_id(1, PermissionLevel::Ban), Err(AppError::InsufficientPrivileges));
        assert_eq!(user.check_sphere_permissions_by_id(2, PermissionLevel::Ban), Err(AppError::InsufficientPrivileges));
        assert_eq!(user.check_sphere_permissions_by_id(3, PermissionLevel::Ban), Ok(()));
        assert_eq!(user.check_sphere_permissions_by_id(4, PermissionLevel::Ban), Ok(()));
        assert_eq!(user.check_sphere_permissions_by_id(5, PermissionLevel::Ban), Ok(()));

        assert_eq!(user.check_sphere_permissions_by_id(0, PermissionLevel::Manage), Err(AppError::InsufficientPrivileges));
        assert_eq!(user.check_sphere_permissions_by_id(1, PermissionLevel::Manage), Err(AppError::InsufficientPrivileges));
        assert_eq!(user.check_sphere_permissions_by_id(2, PermissionLevel::Manage), Err(AppError::InsufficientPrivileges));
        assert_eq!(user.check_sphere_permissions_by_id(3, PermissionLevel::Manage), Err(AppError::InsufficientPrivileges));
        assert_eq!(user.check_sphere_permissions_by_id(4, PermissionLevel::Manage), Ok(()));
        assert_eq!(user.check_sphere_permissions_by_id(5, PermissionLevel::Manage), Ok(()));

        assert_eq!(user.check_sphere_permissions_by_id(0, PermissionLevel::Lead), Err(AppError::InsufficientPrivileges));
        assert_eq!(user.check_sphere_permissions_by_id(1, PermissionLevel::Lead), Err(AppError::InsufficientPrivileges));
        assert_eq!(user.check_sphere_permissions_by_id(2, PermissionLevel::Lead), Err(AppError::InsufficientPrivileges));
        assert_eq!(user.check_sphere_permissions_by_id(3, PermissionLevel::Lead), Err(AppError::InsufficientPrivileges));
        assert_eq!(user.check_sphere_permissions_by_id(4, PermissionLevel::Lead), Err(AppError::InsufficientPrivileges));
        assert_eq!(user.check_sphere_permissions_by_id(5, PermissionLevel::Lead), Ok(()));

        user.admin_role = AdminRole::Moderator;

        assert_eq!(user.check_sphere_permissions_by_id(0, PermissionLevel::None), Ok(()));
        assert_eq!(user.check_sphere_permissions_by_id(1, PermissionLevel::None), Ok(()));
        assert_eq!(user.check_sphere_permissions_by_id(2, PermissionLevel::None), Ok(()));
        assert_eq!(user.check_sphere_permissions_by_id(3, PermissionLevel::None), Ok(()));
        assert_eq!(user.check_sphere_permissions_by_id(4, PermissionLevel::None), Ok(()));
        assert_eq!(user.check_sphere_permissions_by_id(5, PermissionLevel::None), Ok(()));

        assert_eq!(user.check_sphere_permissions_by_id(0, PermissionLevel::Moderate), Ok(()));
        assert_eq!(user.check_sphere_permissions_by_id(1, PermissionLevel::Moderate), Ok(()));
        assert_eq!(user.check_sphere_permissions_by_id(2, PermissionLevel::Moderate), Ok(()));
        assert_eq!(user.check_sphere_permissions_by_id(3, PermissionLevel::Moderate), Ok(()));
        assert_eq!(user.check_sphere_permissions_by_id(4, PermissionLevel::Moderate), Ok(()));
        assert_eq!(user.check_sphere_permissions_by_id(5, PermissionLevel::Moderate), Ok(()));

        assert_eq!(user.check_sphere_permissions_by_id(0, PermissionLevel::Ban), Ok(()));
        assert_eq!(user.check_sphere_permissions_by_id(1, PermissionLevel::Ban), Ok(()));
        assert_eq!(user.check_sphere_permissions_by_id(2, PermissionLevel::Ban), Ok(()));
        assert_eq!(user.check_sphere_permissions_by_id(3, PermissionLevel::Ban), Ok(()));
        assert_eq!(user.check_sphere_permissions_by_id(4, PermissionLevel::Ban), Ok(()));
        assert_eq!(user.check_sphere_permissions_by_id(5, PermissionLevel::Ban), Ok(()));

        assert_eq!(user.check_sphere_permissions_by_id(0, PermissionLevel::Manage), Err(AppError::InsufficientPrivileges));
        assert_eq!(user.check_sphere_permissions_by_id(1, PermissionLevel::Manage), Err(AppError::InsufficientPrivileges));
        assert_eq!(user.check_sphere_permissions_by_id(2, PermissionLevel::Manage), Err(AppError::InsufficientPrivileges));
        assert_eq!(user.check_sphere_permissions_by_id(3, PermissionLevel::Manage), Err(AppError::InsufficientPrivileges));
        assert_eq!(user.check_sphere_permissions_by_id(4, PermissionLevel::Manage), Ok(()));
        assert_eq!(user.check_sphere_permissions_by_id(5, PermissionLevel::Manage), Ok(()));

        assert_eq!(user.check_sphere_permissions_by_id(0, PermissionLevel::Lead), Err(AppError::InsufficientPrivileges));
        assert_eq!(user.check_sphere_permissions_by_id(1, PermissionLevel::Lead), Err(AppError::InsufficientPrivileges));
        assert_eq!(user.check_sphere_permissions_by_id(2, PermissionLevel::Lead), Err(AppError::InsufficientPrivileges));
        assert_eq!(user.check_sphere_permissions_by_id(3, PermissionLevel::Lead), Err(AppError::InsufficientPrivileges));
        assert_eq!(user.check_sphere_permissions_by_id(4, PermissionLevel::Lead), Err(AppError::InsufficientPrivileges));
        assert_eq!(user.check_sphere_permissions_by_id(5, PermissionLevel::Lead), Ok(()));

        user.admin_role = AdminRole::Admin;

        assert_eq!(user.check_sphere_permissions_by_id(0, PermissionLevel::None), Ok(()));
        assert_eq!(user.check_sphere_permissions_by_id(1, PermissionLevel::None), Ok(()));
        assert_eq!(user.check_sphere_permissions_by_id(2, PermissionLevel::None), Ok(()));
        assert_eq!(user.check_sphere_permissions_by_id(3, PermissionLevel::None), Ok(()));
        assert_eq!(user.check_sphere_permissions_by_id(4, PermissionLevel::None), Ok(()));
        assert_eq!(user.check_sphere_permissions_by_id(5, PermissionLevel::None), Ok(()));

        assert_eq!(user.check_sphere_permissions_by_id(0, PermissionLevel::Moderate), Ok(()));
        assert_eq!(user.check_sphere_permissions_by_id(1, PermissionLevel::Moderate), Ok(()));
        assert_eq!(user.check_sphere_permissions_by_id(2, PermissionLevel::Moderate), Ok(()));
        assert_eq!(user.check_sphere_permissions_by_id(3, PermissionLevel::Moderate), Ok(()));
        assert_eq!(user.check_sphere_permissions_by_id(4, PermissionLevel::Moderate), Ok(()));
        assert_eq!(user.check_sphere_permissions_by_id(5, PermissionLevel::Moderate), Ok(()));

        assert_eq!(user.check_sphere_permissions_by_id(0, PermissionLevel::Ban), Ok(()));
        assert_eq!(user.check_sphere_permissions_by_id(1, PermissionLevel::Ban), Ok(()));
        assert_eq!(user.check_sphere_permissions_by_id(2, PermissionLevel::Ban), Ok(()));
        assert_eq!(user.check_sphere_permissions_by_id(3, PermissionLevel::Ban), Ok(()));
        assert_eq!(user.check_sphere_permissions_by_id(4, PermissionLevel::Ban), Ok(()));
        assert_eq!(user.check_sphere_permissions_by_id(5, PermissionLevel::Ban), Ok(()));

        assert_eq!(user.check_sphere_permissions_by_id(0, PermissionLevel::Manage), Ok(()));
        assert_eq!(user.check_sphere_permissions_by_id(1, PermissionLevel::Manage), Ok(()));
        assert_eq!(user.check_sphere_permissions_by_id(2, PermissionLevel::Manage), Ok(()));
        assert_eq!(user.check_sphere_permissions_by_id(3, PermissionLevel::Manage), Ok(()));
        assert_eq!(user.check_sphere_permissions_by_id(4, PermissionLevel::Manage), Ok(()));
        assert_eq!(user.check_sphere_permissions_by_id(5, PermissionLevel::Manage), Ok(()));

        assert_eq!(user.check_sphere_permissions_by_id(0, PermissionLevel::Lead), Ok(()));
        assert_eq!(user.check_sphere_permissions_by_id(1, PermissionLevel::Lead), Ok(()));
        assert_eq!(user.check_sphere_permissions_by_id(2, PermissionLevel::Lead), Ok(()));
        assert_eq!(user.check_sphere_permissions_by_id(3, PermissionLevel::Lead), Ok(()));
        assert_eq!(user.check_sphere_permissions_by_id(4, PermissionLevel::Lead), Ok(()));
        assert_eq!(user.check_sphere_permissions_by_id(5, PermissionLevel::Lead), Ok(()));
    }

    #[test]
    fn test_user_check_is_sphere_leader() {
        let mut user = User::default();
        user.permission_by_sphere_name_map = get_user_sphere_permission_by_name_map();
        assert_eq!(user.check_is_sphere_leader("missing"), Err(AppError::InsufficientPrivileges));
        assert_eq!(user.check_is_sphere_leader("a"), Err(AppError::InsufficientPrivileges));
        assert_eq!(user.check_is_sphere_leader("b"), Err(AppError::InsufficientPrivileges));
        assert_eq!(user.check_is_sphere_leader("c"), Err(AppError::InsufficientPrivileges));
        assert_eq!(user.check_is_sphere_leader("d"), Err(AppError::InsufficientPrivileges));
        assert_eq!(user.check_is_sphere_leader("e"), Ok(()));
        let mut admin = User::default();
        admin.admin_role = AdminRole::Moderator;
        assert_eq!(admin.check_is_sphere_leader("a"), Err(AppError::InsufficientPrivileges));
        admin.admin_role = AdminRole::Admin;
        assert_eq!(admin.check_is_sphere_leader("a"), Err(AppError::InsufficientPrivileges));
    }

    #[test]
    fn test_user_check_can_publish() {
        let past_timestamp = chrono::DateTime::from_timestamp_nanos(0);
        let future_timestamp = chrono::offset::Utc::now().add(Days::new(1));
        let mut user = User::default();
        assert_eq!(user.check_can_publish(), Ok(()));
        user.ban_status = BanStatus::Until(past_timestamp);
        assert_eq!(user.check_can_publish(), Ok(()));
        user.ban_status = BanStatus::Until(future_timestamp);
        assert_eq!(user.check_can_publish(), Err(AppError::GlobalBanUntil(future_timestamp)));
        user.ban_status = BanStatus::Permanent;
        assert_eq!(user.check_can_publish(), Err(AppError::PermanentGlobalBan));
    }

    #[test]
    fn test_user_check_can_publish_on_sphere() {
        let past_timestamp = chrono::DateTime::from_timestamp_nanos(0);
        let future_timestamp = chrono::offset::Utc::now().add(Days::new(1));
        let mut user = User {
            ban_status_by_sphere_map: HashMap::from([
                (String::from("a"), BanStatus::None),
                (String::from("b"), BanStatus::Until(past_timestamp)),
                (String::from("c"), BanStatus::Until(future_timestamp)),
                (String::from("d"), BanStatus::Permanent),
            ]),
            ..Default::default()
        };
        assert_eq!(user.check_can_publish_on_sphere("a"), Ok(()));
        assert_eq!(user.check_can_publish_on_sphere("b"), Ok(()));
        assert_eq!(user.check_can_publish_on_sphere("c"), Err(AppError::SphereBanUntil(future_timestamp)));
        assert_eq!(user.check_can_publish_on_sphere("d"), Err(AppError::PermanentSphereBan));
        user.ban_status = BanStatus::Until(past_timestamp);
        assert_eq!(user.check_can_publish_on_sphere("a"), Ok(()));
        assert_eq!(user.check_can_publish_on_sphere("b"), Ok(()));
        assert_eq!(user.check_can_publish_on_sphere("c"), Err(AppError::SphereBanUntil(future_timestamp)));
        assert_eq!(user.check_can_publish_on_sphere("d"), Err(AppError::PermanentSphereBan));
        user.ban_status = BanStatus::Until(future_timestamp);
        assert_eq!(user.check_can_publish_on_sphere("a"), Err(AppError::GlobalBanUntil(future_timestamp)));
        assert_eq!(user.check_can_publish_on_sphere("b"), Err(AppError::GlobalBanUntil(future_timestamp)));
        assert_eq!(user.check_can_publish_on_sphere("c"), Err(AppError::GlobalBanUntil(future_timestamp)));
        assert_eq!(user.check_can_publish_on_sphere("d"), Err(AppError::GlobalBanUntil(future_timestamp)));
        user.ban_status = BanStatus::Permanent;
        assert_eq!(user.check_can_publish_on_sphere("a"), Err(AppError::PermanentGlobalBan));
        assert_eq!(user.check_can_publish_on_sphere("b"), Err(AppError::PermanentGlobalBan));
        assert_eq!(user.check_can_publish_on_sphere("c"), Err(AppError::PermanentGlobalBan));
        assert_eq!(user.check_can_publish_on_sphere("d"), Err(AppError::PermanentGlobalBan));
    }

    #[test]
    fn test_user_get_posts_filter() {
        let mut user = User::default();
        let user_post_filters = user.get_posts_filter();
        assert_eq!(user_post_filters.show_nsfw, true);
        assert_eq!(user_post_filters.days_hide_spoiler, None);

        let days_hide_spoiler = Some(14);
        user.days_hide_spoiler = days_hide_spoiler;
        let user_post_filters = user.get_posts_filter();
        assert_eq!(user_post_filters.show_nsfw, true);
        assert_eq!(user_post_filters.days_hide_spoiler, days_hide_spoiler);
        
        user.show_nsfw = false;
        let user_post_filters = user.get_posts_filter();
        assert_eq!(user_post_filters.show_nsfw, false);
        assert_eq!(user_post_filters.days_hide_spoiler, days_hide_spoiler);

        user.days_hide_spoiler = None;
        user.show_nsfw = true;
        let user_post_filters = user.get_posts_filter();
        assert_eq!(user_post_filters.show_nsfw, true);
        assert_eq!(user_post_filters.days_hide_spoiler, None);
    }
    
    #[test]
    fn test_user_post_filters_default() {
        let default_post_filters = UserPostFilters::default();
        assert_eq!(default_post_filters.days_hide_spoiler, None);
        assert_eq!(default_post_filters.show_nsfw, true);
    }
}