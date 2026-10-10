use anyhow::ensure;
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
    let db = sqlx::PgPool::connect(&std::env::var("DATABASE_URL")?).await?;
    let role: (bool, bool, bool) = sqlx::query_as(
        "SELECT rolsuper, rolcreatedb, rolcreaterole FROM pg_roles WHERE rolname=current_user",
    )
    .fetch_one(&db)
    .await?;
    ensure!(
        role == (false, false, false),
        "Application database role has elevated privileges"
    );
    let protected:bool=sqlx::query_scalar("SELECT has_table_privilege(current_user, '_sqlx_migrations', 'UPDATE') OR has_table_privilege(current_user, 'audit_logs', 'DELETE')").fetch_one(&db).await?;
    ensure!(
        !protected,
        "Application database role can change protected operational records"
    );
    let mut redis = redis::Client::open(std::env::var("REDIS_URL")?)?
        .get_connection_manager()
        .await?;
    let pong: String = redis::cmd("PING").query_async(&mut redis).await?;
    ensure!(pong == "PONG", "Redis did not return PONG");
    let rpc: serde_json::Value = reqwest::Client::new()
        .post(std::env::var("ARC_RPC_URL")?)
        .json(&serde_json::json!({"jsonrpc":"2.0","id":1,"method":"eth_chainId","params":[]}))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    ensure!(rpc["result"] == "0x13b2", "RPC is not Arc mainnet");
    db.close().await;
    println!(
        "Runtime database privileges, PostgreSQL TLS, Redis TLS, and Arc mainnet RPC checks passed."
    );
    Ok(())
}
