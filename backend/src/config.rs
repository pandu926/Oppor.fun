use alloy_primitives::{Address, B256};
use anyhow::{Context, ensure};
use std::{env, net::SocketAddr, time::Duration};
use url::Url;
use zeroize::Zeroizing;

#[derive(Clone)]
pub struct Config {
    pub production: bool,
    pub bind: SocketAddr,
    pub database_url: String,
    pub redis_url: String,
    pub public_origin: String,
    pub origins: Vec<String>,
    pub trusted_proxies: Vec<ipnet::IpNet>,
    pub admin_wallets: Vec<Address>,
    pub domain: String,
    pub chain_id: u64,
    pub rpc_url: String,
    pub factory: Address,
    pub factory_code_hash: B256,
    pub escrow_code_hash: B256,
    pub deployment_block: u64,
    pub confirmations: u64,
    pub session_ttl: i64,
    pub seed_key: Zeroizing<[u8; 32]>,
    pub rate_key: Zeroizing<[u8; 32]>,
    pub db_max_connections: u32,
    pub http_timeout: Duration,
    pub max_indexer_lag: i64,
    pub storage_endpoint: String,
    pub storage_bucket: String,
    pub storage_region: String,
    pub storage_access_key: String,
    pub storage_secret_key: Zeroizing<String>,
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        let production = env::var("APP_ENV").unwrap_or_else(|_| "production".into()) != "local";
        let origin = Url::parse(&required("PUBLIC_ORIGIN")?)?;
        ensure!(
            origin.host_str().is_some()
                && origin.username().is_empty()
                && origin.password().is_none()
                && origin.query().is_none()
                && origin.fragment().is_none()
                && origin.path() == "/",
            "PUBLIC_ORIGIN must be an origin without credentials, path, query or fragment"
        );
        ensure!(
            origin.scheme() == "https" || (!production && origin.scheme() == "http"),
            "Production requires HTTPS"
        );
        let public_origin = origin.origin().ascii_serialization();
        let origins: Vec<String> = env::var("ALLOWED_ORIGINS")
            .unwrap_or_else(|_| public_origin.clone())
            .split(',')
            .map(str::trim)
            .map(str::to_owned)
            .collect();
        for value in &origins {
            let u = Url::parse(value)?;
            ensure!(
                u.origin().ascii_serialization() == *value
                    && (u.scheme() == "https" || !production),
                "ALLOWED_ORIGINS must contain exact trusted origins"
            );
        }
        ensure!(
            origins.contains(&public_origin),
            "PUBLIC_ORIGIN must be allowed"
        );
        let trusted_proxies = env::var("TRUSTED_PROXY_CIDRS")
            .unwrap_or_default()
            .split(',')
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .map(str::parse::<ipnet::IpNet>)
            .collect::<std::result::Result<Vec<_>, _>>()?;
        let admin_wallets = env::var("ADMIN_WALLETS")
            .unwrap_or_default()
            .split(',')
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .map(str::parse::<Address>)
            .collect::<std::result::Result<Vec<_>, _>>()?;
        ensure!(
            admin_wallets.len() <= 20 && admin_wallets.iter().all(|wallet| !wallet.is_zero()),
            "ADMIN_WALLETS must contain at most 20 nonzero wallet addresses"
        );
        ensure!(
            admin_wallets
                .iter()
                .collect::<std::collections::HashSet<_>>()
                .len()
                == admin_wallets.len(),
            "ADMIN_WALLETS cannot contain duplicates"
        );
        ensure!(
            trusted_proxies
                .iter()
                .all(|network| network.prefix_len() > 0),
            "Trusting all proxy IPs is not allowed"
        );
        let database_url = required("DATABASE_URL")?;
        let database = Url::parse(&database_url)?;
        ensure!(
            matches!(database.scheme(), "postgres" | "postgresql"),
            "DATABASE_URL must use PostgreSQL"
        );
        let redis_url = required("REDIS_URL")?;
        let redis = Url::parse(&redis_url)?;
        ensure!(
            matches!(redis.scheme(), "redis" | "rediss"),
            "REDIS_URL must use Redis"
        );
        if production {
            ensure!(
                verified_database_tls(&database),
                "Production PostgreSQL requires sslmode=verify-full"
            );
            ensure!(
                verified_redis_tls(&redis),
                "Production Redis requires TLS (rediss://)"
            );
        }
        let rpc_url = required("ARC_RPC_URL")?;
        let rpc = Url::parse(&rpc_url)?;
        ensure!(
            rpc.scheme() == "https" || (!production && rpc.scheme() == "http"),
            "RPC requires HTTPS in production"
        );
        let storage_endpoint = required("OBJECT_STORAGE_ENDPOINT")?;
        let storage = Url::parse(&storage_endpoint)?;
        ensure!(
            storage.scheme() == "https" || (!production && storage.scheme() == "http"),
            "Object storage requires HTTPS in production"
        );
        let chain_id: u64 = number("ARC_CHAIN_ID", 0)?;
        ensure!(
            chain_id > 0 && chain_id <= i64::MAX as u64,
            "Invalid chain ID"
        );
        let factory: Address = required("FACTORY_ADDRESS")?.parse()?;
        ensure!(!factory.is_zero(), "Factory address cannot be zero");
        let factory_code_hash: B256 = required("FACTORY_CODE_HASH")?.parse()?;
        let escrow_code_hash: B256 = required("ESCROW_CODE_HASH")?.parse()?;
        ensure!(
            !factory_code_hash.is_zero() && !escrow_code_hash.is_zero(),
            "Bytecode hashes cannot be zero"
        );
        let session_ttl = number("SESSION_TTL_SECONDS", 604800)?;
        ensure!(
            (300..=604800).contains(&session_ttl),
            "Session TTL must be between five minutes and seven days"
        );
        let db_max_connections = number("DATABASE_MAX_CONNECTIONS", 24)?;
        ensure!(
            (2..=256).contains(&db_max_connections),
            "Invalid database pool size"
        );
        let domain = match origin.port() {
            Some(port) => format!("{}:{port}", origin.host_str().context("Missing host")?),
            None => origin.host_str().context("Missing host")?.to_owned(),
        };
        let deployment_block: u64 = number("FACTORY_DEPLOYMENT_BLOCK", 0)?;
        let confirmations: u64 = number("CHAIN_CONFIRMATIONS", 2)?;
        let max_indexer_lag: i64 = number("INDEXER_MAX_LAG_SECONDS", 90)?;
        ensure!(
            deployment_block <= i64::MAX as u64,
            "Invalid deployment block"
        );
        ensure!(confirmations <= 10_000, "Invalid confirmation depth");
        ensure!(
            (5..=600).contains(&max_indexer_lag),
            "Indexer freshness must be between 5 and 600 seconds"
        );
        let seed_key = key("RAFFLE_SEED_ENCRYPTION_KEY")?;
        let rate_key = key("RATE_LIMIT_KEY")?;
        ensure!(
            seed_key != rate_key,
            "Seed and rate-limit keys must be independent"
        );
        Ok(Self {
            production,
            bind: env::var("HTTP_BIND")
                .unwrap_or_else(|_| "127.0.0.1:8080".into())
                .parse()?,
            database_url,
            redis_url,
            public_origin,
            origins,
            trusted_proxies,
            admin_wallets,
            domain,
            chain_id,
            rpc_url,
            factory,
            factory_code_hash,
            escrow_code_hash,
            deployment_block,
            confirmations,
            session_ttl,
            seed_key: Zeroizing::new(seed_key),
            rate_key: Zeroizing::new(rate_key),
            db_max_connections,
            http_timeout: Duration::from_secs(15),
            max_indexer_lag,
            storage_endpoint,
            storage_bucket: required("OBJECT_STORAGE_BUCKET")?,
            storage_region: env::var("OBJECT_STORAGE_REGION")
                .unwrap_or_else(|_| "us-east-1".into()),
            storage_access_key: required("OBJECT_STORAGE_ACCESS_KEY")?,
            storage_secret_key: Zeroizing::new(required("OBJECT_STORAGE_SECRET_KEY")?),
        })
    }

    pub fn cookie_name(&self) -> &'static str {
        if self.production {
            "__Host-oppor_session"
        } else {
            "oppor_session"
        }
    }
}

fn verified_database_tls(database: &Url) -> bool {
    let modes: Vec<_> = database
        .query_pairs()
        .filter(|(key, _)| key == "sslmode" || key == "ssl-mode")
        .collect();
    modes.len() == 1 && modes[0].1 == "verify-full"
}
fn verified_redis_tls(redis: &Url) -> bool {
    redis.scheme() == "rediss" && redis.fragment().is_none()
}

fn required(name: &str) -> anyhow::Result<String> {
    let value = env::var(name).with_context(|| format!("Missing configuration: {name}"))?;
    ensure!(!value.trim().is_empty(), "Empty configuration: {name}");
    Ok(value)
}
fn number<T: std::str::FromStr>(name: &str, default: T) -> anyhow::Result<T>
where
    T::Err: std::error::Error + Send + Sync + 'static,
{
    match env::var(name) {
        Ok(v) => v
            .parse()
            .with_context(|| format!("Invalid configuration: {name}")),
        Err(_) => Ok(default),
    }
}
fn key(name: &str) -> anyhow::Result<[u8; 32]> {
    let bytes = hex::decode(required(name)?)
        .with_context(|| format!("{name} must be 64 hex characters"))?;
    let key: [u8; 32] = bytes
        .try_into()
        .map_err(|_| anyhow::anyhow!("{name} must be 32 bytes"))?;
    ensure!(key != [0; 32], "{name} cannot be all zero");
    Ok(key)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn postgres_tls_cannot_be_overridden_with_duplicate_parameters() {
        for query in [
            "",
            "sslmode=require",
            "sslmode=verify-full&sslmode=disable",
            "sslmode=disable&sslmode=verify-full",
            "sslmode=verify-full&ssl-mode=disable",
            "ssl-mode=disable&sslmode=verify-full",
        ] {
            assert!(!verified_database_tls(
                &Url::parse(&format!("postgres://localhost/oppor?{query}")).unwrap()
            ));
        }
        assert!(verified_database_tls(
            &Url::parse("postgres://localhost/oppor?sslmode=verify-full").unwrap()
        ));
    }
    #[test]
    fn redis_certificate_verification_cannot_be_disabled() {
        assert!(!verified_redis_tls(
            &Url::parse("rediss://localhost/0#insecure").unwrap()
        ));
        assert!(!verified_redis_tls(
            &Url::parse("redis://localhost/0").unwrap()
        ));
        assert!(verified_redis_tls(
            &Url::parse("rediss://localhost/0").unwrap()
        ));
    }
}
