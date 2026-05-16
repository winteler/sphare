#[cfg(feature = "ssr")]
pub mod ssr {
    use std::sync::{Arc, LazyLock};

    use axum_session_sqlx::SessionPgPool;
    use leptos::config::Env;
    use leptos::prelude::use_context;
    use sqlx::PgPool;

    use sphare_core_common::errors::AppError;

    use crate::user::ssr::UserLockCache;
    use crate::user::User;

    pub const DB_URL_ENV: &str = "DATABASE_URL";
    pub const LEPTOS_ENV_KEY: &str = "LEPTOS_ENV";

    pub static LEPTOS_ENV: LazyLock<Env> = LazyLock::new(|| {
        let leptos_env = std::env::var(LEPTOS_ENV_KEY).unwrap().to_lowercase();
        match leptos_env.as_ref() {
            "dev" | "development" => Env::DEV,
            "prod" | "production" => Env::PROD,
            _ => panic!("Unsupported LEPTOS_ENV environment variable. Use either `dev` or `prod`."),
        }
    });

    pub type AuthSession = axum_session_auth::AuthSession<User, i64, SessionPgPool, PgPool>;

    pub fn get_session() -> Result<AuthSession, AppError> {
        use_context::<AuthSession>().ok_or_else(|| AppError::new("Auth session missing."))
    }

    pub fn get_user_lock_cache() -> Result<Arc<UserLockCache>, AppError> {
        use_context::<Arc<UserLockCache>>().ok_or_else(|| AppError::new("User lock cache missing."))
    }
}