use oppor_backend::{config::Config, state::AppState};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    oppor_backend::init_tracing();
    if std::env::args().any(|a| a == "migrate") {
        let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
        let database_url = oppor_backend::config::migration_database_url()?;
        let db = sqlx::postgres::PgPoolOptions::new()
            .max_connections(2)
            .connect(&database_url)
            .await?;
        oppor_backend::MIGRATOR.run(&db).await?;
        db.close().await;
        tracing::info!("Database migrations applied");
        return Ok(());
    }
    let config = Config::from_env()?;
    let bind = config.bind;
    let state = AppState::connect(config).await?;
    let listener = tokio::net::TcpListener::bind(bind).await?;
    tracing::info!(%bind,"Oppor API listening");
    axum::serve(
        listener,
        oppor_backend::http::router(state.clone())
            .into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown())
    .await?;
    state.db.close().await;
    Ok(())
}
async fn shutdown() {
    #[cfg(unix)]
    {
        let mut terminate =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                .expect("Install SIGTERM handler");
        tokio::select! {_ =tokio::signal::ctrl_c()=>{},_=terminate.recv()=>{}}
    }
    #[cfg(not(unix))]
    tokio::signal::ctrl_c()
        .await
        .expect("Install shutdown handler");
    tracing::info!("Shutdown requested");
}
