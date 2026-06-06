#[cfg(feature = "ssr")]
pub mod ssr {
    use std::sync::LazyLock;
    use leptos::config::Env;

    pub const LEPTOS_ENV_KEY: &str = "LEPTOS_ENV";

    pub static LEPTOS_ENV: LazyLock<Env> = LazyLock::new(|| {
        let leptos_env = std::env::var(LEPTOS_ENV_KEY).unwrap().to_lowercase();
        match leptos_env.as_ref() {
            "dev" | "development" => Env::DEV,
            "prod" | "production" => Env::PROD,
            _ => panic!("Unsupported LEPTOS_ENV environment variable. Use either `dev` or `prod`."),
        }
    });
}