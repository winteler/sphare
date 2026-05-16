#![allow(dead_code)]
use std::sync::LazyLock;
use std::sync::Mutex;

use fluent_templates::{static_loader, StaticLoader};
use leptos::prelude::*;
use leptos::server_fn::const_format::formatcp;
use leptos_fluent::{I18n, Language};

use sphare_core_user::user::ssr::create_or_update_user;
use sphare_core_user::user::User;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;

pub const TEST_DB_NAME_ENV: &str = "TEST_DATABASE_NAME";
pub const TEST_DB_URL_ENV: &str = "TEST_DATABASE_URL";
static DB_NUM: Mutex<i32> = Mutex::new(0);

const EN_LANG: Language = Language {
    id: "en",
    name: "English",
    dir: &leptos_fluent::WritingDirection::Ltr,
    flag: None,
    script: None,
};
const FR_LANG: Language = Language {
    id: "fr",
    name: "Français",
    dir: &leptos_fluent::WritingDirection::Ltr,
    flag: None,
    script: None,
};
const LANGUAGES: &'static [&Language] = &[
    &EN_LANG,
    &FR_LANG,
];

async fn get_main_db_pool() -> PgPool {
    let main_db = std::env::var(TEST_DB_NAME_ENV).expect(&format!("Test DB name should be in env variable {TEST_DB_NAME_ENV}."));
    let main_db_url = std::env::var(TEST_DB_URL_ENV).expect(&format!("Test DB address should be in env variable {TEST_DB_URL_ENV}.")) + &main_db;
    PgPoolOptions::new()
        .max_connections(5)
        .connect(&main_db_url)
        .await
        .expect("Should be able to connect to Test DB.")
}

async fn clear_base_rules(db_pool: &PgPool) {
    sqlx::query!("DELETE FROM rules WHERE sphere_id IS NULL").execute(db_pool).await.expect("Should delete base rules");
}

pub async fn get_db_pool() -> PgPool {
    let db_name = {
        let mut db_num = DB_NUM.lock().unwrap();
        *db_num += 1;
        format!("test{db_num}")
    };

    let main_db_pool = get_main_db_pool().await;
    println!("Setup database: {db_name}");

    sqlx::query(format!("DROP DATABASE IF EXISTS {db_name} WITH (FORCE)").as_str())
        .execute(&main_db_pool)
        .await
        .unwrap_or_else(|_| panic!("Should be able to delete database: {db_name}"));

    sqlx::query(format!("CREATE DATABASE {db_name}").as_str())
        .execute(&main_db_pool)
        .await
        .unwrap_or_else(|_| panic!("Should be able to create database {db_name}"));

    let test_db_url =
        std::env::var(TEST_DB_URL_ENV).expect(formatcp!("Test DB address should be available in env variable {TEST_DB_URL_ENV}.")) + db_name.as_str();

    let db_pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&test_db_url)
        .await
        .expect("Should be able to connect test DB");

    sqlx::migrate!("../../../migrations/")
        .run(&db_pool)
        .await
        .expect("SQLx migrations should be executed.");

    // Clear base rules that are created in migrations
    clear_base_rules(&db_pool).await;

    db_pool
}

pub async fn create_test_user(db_pool: &PgPool) -> User {
    create_user("test", db_pool).await
}

pub async fn create_user(
    test_id: &str,
    db_pool: &PgPool
) -> User {
    let sql_user = create_or_update_user(test_id, test_id, test_id, db_pool)
        .await
        .expect("Should be possible to create user.");
    User::get(sql_user.user_id, db_pool).await.expect("New user should be available in DB.")
}

pub fn get_i18n() -> I18n {
    static_loader! {
            static TRANSLATIONS = {
                locales: "../../../locales",
                fallback_language: "en",
            };
        }
    let compound: Vec<&LazyLock<StaticLoader>> = vec![&TRANSLATIONS];
    I18n::new(
        RwSignal::new(LANGUAGES[0]),
        LANGUAGES,
        Signal::derive(move || compound.clone())
    )
}