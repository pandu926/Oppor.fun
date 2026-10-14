use crate::{chain::Rpc, config::Config, error::Result, storage::Storage};
use sqlx::{PgPool, postgres::PgPoolOptions};
use std::{
    sync::{Arc, atomic::AtomicU64},
    time::Duration,
};

#[derive(Default)]
pub struct Metrics {
    pub requests: AtomicU64,
    pub failures: AtomicU64,
    pub latency_micros: AtomicU64,
}
#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub db: PgPool,
    pub redis: redis::aio::ConnectionManager,
    pub rpc: Rpc,
    pub storage: Storage,
    pub metrics: Arc<Metrics>,
    pub concurrent: Arc<tokio::sync::Semaphore>,
}
impl AppState {
    pub async fn connect(config: Config) -> anyhow::Result<Self> {
        // SQLx and HTTP clients can enable different rustls providers transitively.
        // Install one explicitly before constructing TLS clients, including Redis.
        let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
        let db = PgPoolOptions::new()
            .max_connections(config.db_max_connections)
            .min_connections(2)
            .acquire_timeout(Duration::from_secs(3))
            .idle_timeout(Duration::from_secs(300))
            .after_connect(|conn, _| {
                Box::pin(async move {
                    sqlx::query("SET statement_timeout='8s'")
                        .execute(&mut *conn)
                        .await?;
                    sqlx::query("SET lock_timeout='3s'")
                        .execute(&mut *conn)
                        .await?;
                    Ok(())
                })
            })
            .connect(&config.database_url)
            .await?;
        let redis = redis::Client::open(config.redis_url.as_str())?
            .get_connection_manager()
            .await?;
        let rpc = Rpc::new(&config)?;
        let storage = Storage::new(&config)?;
        Ok(Self {
            config: Arc::new(config),
            db,
            redis,
            rpc,
            storage,
            metrics: Arc::new(Metrics::default()),
            concurrent: Arc::new(tokio::sync::Semaphore::new(256)),
        })
    }
    pub async fn ensure_indexer(&self) -> Result<()> {
        let healthy:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM indexer_cursors WHERE chain_id=$1 AND factory_address=$2 AND halted_reason IS NULL AND observed_safe_head IS NOT NULL AND observed_safe_head-next_block<=100 AND updated_at>clock_timestamp()-make_interval(secs=>$3))")
            .bind(self.config.chain_id as i64).bind(self.config.factory.as_slice()).bind(self.config.max_indexer_lag as f64).fetch_one(&self.db).await?;
        if !healthy {
            return Err(crate::error::ApiError::unavailable());
        }
        Ok(())
    }
}
