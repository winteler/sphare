use anyhow::Context;
    use leptos::prelude::use_context;
    use sqlx::{postgres::PgPoolOptions, PgPool};

    use crate::errors::AppError;

pub const DB_URL_ENV: &str = "DATABASE_URL";
pub const DB_POOL_SIZE_ENV: &str = "DATABASE_POOL_SIZE";
pub const DB_POOL_SIZE_DEFAULT: u32 = 6;
pub fn get_db_pool() -> Result<PgPool, AppError> {
    use_context::<PgPool>().ok_or_else(|| AppError::new("DB pool missing."))
}

pub async fn create_db_pool() -> anyhow::Result<sqlx::Pool<sqlx::Postgres>> {
    let db_pool_size = std::env::var(DB_POOL_SIZE_ENV)
        .ok()
        .and_then(|v| v.parse::<u32>().ok())
        .unwrap_or(DB_POOL_SIZE_DEFAULT);

    PgPoolOptions::new()
        .max_connections(db_pool_size)
        .connect(&std::env::var(DB_URL_ENV)?)
        .await
        .with_context(|| "Failed to connect to DB")
}