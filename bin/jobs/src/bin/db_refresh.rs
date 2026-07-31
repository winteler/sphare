use sphare_core_common::db_utils::create_db_pool;
use sphare_core_content::post::ssr::update_post_scores;


#[tokio::main]
async fn main() {
    simple_logger::init_with_level(log::Level::Info).expect("Should be able to initialize logging.");

    let subscriber = tracing_subscriber::fmt().with_max_level(tracing::Level::ERROR).finish();
    tracing::subscriber::set_global_default(subscriber).expect("setting tracing default failed");

    let pool = create_db_pool().await.expect("Failed to create db pool");

    sqlx::migrate!("../../migrations/")
        .run(&pool)
        .await
        .expect("Should be able to run SQLx migrations.");

    update_post_scores(&pool).await.expect("Should update post scores");
}
