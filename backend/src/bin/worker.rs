#[tokio::main]
async fn main() -> anyhow::Result<()> {
    oppor_backend::init_tracing();
    let config = oppor_backend::config::Config::from_env()?;
    let state = oppor_backend::state::AppState::connect(config).await?;
    oppor_backend::worker::run(state).await
}
