use crate::{
    crypto::json_hash,
    domain::Campaign,
    error::{ApiError, Result},
};
use axum::http::HeaderMap;
use serde_json::{Value, json};
use sqlx::{Postgres, Row, Transaction};
use uuid::Uuid;

pub struct Mutation {
    tx: Transaction<'static, Postgres>,
    actor: Uuid,
    route: String,
    key: String,
    cached: Option<Value>,
}
impl Mutation {
    pub async fn begin(
        pool: &sqlx::PgPool,
        actor: Uuid,
        route: String,
        headers: &HeaderMap,
        body: &Value,
    ) -> Result<Self> {
        let key = headers
            .get("idempotency-key")
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| {
                ApiError::bad(
                    "IDEMPOTENCY_KEY_REQUIRED",
                    "An Idempotency-Key header is required.",
                )
            })?;
        if !(8..=128).contains(&key.len())
            || !key
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
        {
            return Err(ApiError::bad(
                "INVALID_IDEMPOTENCY_KEY",
                "Invalid Idempotency-Key header.",
            ));
        }
        let hash = json_hash(body)?;
        let mut tx = pool.begin().await?;
        let suspended: bool =
            sqlx::query_scalar("SELECT suspended FROM users WHERE id=$1 FOR SHARE")
                .bind(actor)
                .fetch_one(&mut *tx)
                .await?;
        if suspended {
            return Err(ApiError::forbidden());
        }
        sqlx::query("INSERT INTO idempotency_keys(actor_id,route,key,request_hash) VALUES($1,$2,$3,$4) ON CONFLICT DO NOTHING")
            .bind(actor).bind(&route).bind(key).bind(hash.as_slice()).execute(&mut *tx).await?;
        let row=sqlx::query("SELECT request_hash,response_json FROM idempotency_keys WHERE actor_id=$1 AND route=$2 AND key=$3 FOR UPDATE")
            .bind(actor).bind(&route).bind(key).fetch_one(&mut *tx).await?;
        let previous: Vec<u8> = row.try_get("request_hash")?;
        if previous != hash.as_slice() {
            return Err(ApiError::conflict(
                "IDEMPOTENCY_CONFLICT",
                "This idempotency key was used with a different request.",
            ));
        }
        Ok(Self {
            tx,
            actor,
            route,
            key: key.to_owned(),
            cached: row.try_get("response_json")?,
        })
    }
    pub fn cached(&self) -> Option<Value> {
        self.cached.clone()
    }
    pub fn tx(&mut self) -> &mut Transaction<'static, Postgres> {
        &mut self.tx
    }
    pub async fn audit(
        &mut self,
        campaign: Option<Uuid>,
        action: &str,
        metadata: Value,
    ) -> Result<()> {
        sqlx::query("INSERT INTO audit_logs(id,actor_id,campaign_id,action,metadata) VALUES($1,$2,$3,$4,$5)")
            .bind(Uuid::new_v4()).bind(self.actor).bind(campaign).bind(action).bind(metadata).execute(&mut *self.tx).await?;
        Ok(())
    }
    pub async fn finish(mut self, response: Value) -> Result<Value> {
        sqlx::query("UPDATE idempotency_keys SET response_json=$4 WHERE actor_id=$1 AND route=$2 AND key=$3")
            .bind(self.actor).bind(&self.route).bind(&self.key).bind(&response).execute(&mut *self.tx).await?;
        self.tx.commit().await?;
        Ok(response)
    }
}
pub async fn lock_campaign(tx: &mut Transaction<'_, Postgres>, id: Uuid) -> Result<Campaign> {
    Ok(
        sqlx::query_as("SELECT * FROM campaigns WHERE id=$1 FOR UPDATE")
            .bind(id)
            .fetch_one(&mut **tx)
            .await?,
    )
}
pub async fn db_now(tx: &mut Transaction<'_, Postgres>) -> Result<chrono::DateTime<chrono::Utc>> {
    Ok(sqlx::query_scalar("SELECT clock_timestamp()")
        .fetch_one(&mut **tx)
        .await?)
}
pub fn mutation_metadata(version: i64) -> Value {
    json!({"version":version})
}
