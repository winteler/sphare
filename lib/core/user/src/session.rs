#[cfg(feature = "ssr")]
pub mod ssr {
    use std::sync::{Arc};

    use axum_session_sqlx::SessionPgPool;
    use leptos::prelude::use_context;
    use sqlx::PgPool;

    use sphare_core_common::errors::AppError;

    use crate::user::ssr::UserLockCache;
    use crate::user::User;

    pub const DB_URL_ENV: &str = "DATABASE_URL";

    pub type AuthSession = axum_session_auth::AuthSession<User, i64, SessionPgPool, PgPool>;

    pub fn get_session() -> Result<AuthSession, AppError> {
        use_context::<AuthSession>().ok_or_else(|| AppError::new("Auth session missing."))
    }

    pub fn get_federation() -> Result<AuthSession, AppError> {
        use_context::<AuthSession>().ok_or_else(|| AppError::new("Auth session missing."))
    }

    pub fn get_user_lock_cache() -> Result<Arc<UserLockCache>, AppError> {
        use_context::<Arc<UserLockCache>>().ok_or_else(|| AppError::new("User lock cache missing."))
    }
}