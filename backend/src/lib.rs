pub mod auth;
pub mod chain;
pub mod config;
pub mod crypto;
pub mod domain;
pub mod error;
pub mod http;
pub mod mutations;
pub mod services;
pub mod state;
pub mod storage;
pub mod worker;

pub static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations");

pub fn init_tracing() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "oppor_backend=info,tower_http=info".into()),
        )
        .json()
        .with_target(true)
        .init();
}
