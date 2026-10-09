use oppor_backend::{config::Config, state::AppState};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    oppor_backend::init_tracing();
    let config = Config::from_env()?;
    let bind = config.bind;
    let state = AppState::connect(config).await?;
    if std::env::args().any(|a| a == "migrate") {
        oppor_backend::MIGRATOR.run(&state.db).await?;
        tracing::info!("Database migrations applied");
        return Ok(());
    }
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
