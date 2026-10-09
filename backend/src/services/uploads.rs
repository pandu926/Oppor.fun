use crate::{
    auth::Auth,
    error::{ApiError, Result},
    http::ApiJson,
    mutations::{Mutation, db_now, lock_campaign},
    state::AppState,
};
use axum::{
    Json,
    extract::{Path, State},
    http::HeaderMap,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sqlx::Row;
use uuid::Uuid;

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct UploadInput {
    pub campaign_id: Uuid,
    pub content_type: String,
    pub size_bytes: u64,
}
pub async fn presign(
    State(state): State<AppState>,
    auth: Auth,
    headers: HeaderMap,
    ApiJson(input): ApiJson<UploadInput>,
) -> Result<Json<Value>> {
    if input.size_bytes == 0
        || input.size_bytes > 5 * 1024 * 1024
        || !matches!(
            input.content_type.as_str(),
            "image/png" | "image/jpeg" | "image/webp"
        )
    {
        return Err(ApiError::invalid(
            "Uploads must be PNG, JPEG or WebP images of at most five MiB.",
        ));
    }
    let mut op = Mutation::begin(
        &state.db,
        auth.user_id,
        "presign-upload".into(),
        &headers,
        &serde_json::to_value(&input).map_err(|_| ApiError::internal())?,
    )
    .await?;
    if let Some(v) = op.cached() {
        return Ok(Json(v));
    }
    let c = lock_campaign(op.tx(), input.campaign_id).await?;
    c.ensure_open(db_now(op.tx()).await?)?;
    let entry: Uuid =
        sqlx::query_scalar("SELECT id FROM entries WHERE campaign_id=$1 AND user_id=$2 FOR UPDATE")
            .bind(c.id)
            .bind(auth.user_id)
            .fetch_one(&mut **op.tx())
            .await?;
    let count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM uploads WHERE entry_id=$1 AND status!='DELETED'")
            .bind(entry)
            .fetch_one(&mut **op.tx())
            .await?;
    if count >= 40 {
        return Err(ApiError::conflict(
            "UPLOAD_LIMIT_REACHED",
            "This entry has reached its evidence upload limit.",
        ));
    }
    let upload = Uuid::new_v4();
    let key = format!("staging/{}/{upload}", auth.user_id);
    let signed =
        crate::storage::presigned_post(&state.config, &key, &input.content_type, input.size_bytes)?;
    sqlx::query("INSERT INTO uploads(id,entry_id,user_id,staging_key,content_type,expected_size,expires_at) VALUES($1,$2,$3,$4,$5,$6,clock_timestamp()+interval '5 minutes')")
        .bind(upload).bind(entry).bind(auth.user_id).bind(&key).bind(input.content_type).bind(input.size_bytes as i64).execute(&mut **op.tx()).await?;
    op.finish(json!({"upload_id":upload,"upload":signed}))
        .await
        .map(Json)
}

pub async fn complete(
    State(state): State<AppState>,
    auth: Auth,
    Path(upload_id): Path<Uuid>,
    headers: HeaderMap,
) -> Result<Json<Value>> {
    // Read and copy outside the database transaction; a later lock rechecks ownership, state and cutoff.
    let row=sqlx::query("SELECT u.*,e.campaign_id FROM uploads u JOIN entries e ON e.id=u.entry_id WHERE u.id=$1 AND u.user_id=$2")
        .bind(upload_id).bind(auth.user_id).fetch_one(&state.db).await?;
    let campaign_id: Uuid = row.try_get("campaign_id")?;
    let status: String = row.try_get("status")?;
    let mut op = Mutation::begin(
        &state.db,
        auth.user_id,
        format!("complete-upload:{upload_id}"),
        &headers,
        &json!({"upload_id":upload_id}),
    )
    .await?;
    if let Some(v) = op.cached() {
        return Ok(Json(v));
    }
    if status == "COMPLETE" {
        return op
            .finish(json!({"upload_id":upload_id,"status":"COMPLETE"}))
            .await
            .map(Json);
    }
    // Release the transaction before object storage I/O. The idempotency row is rolled back and recreated below.
    drop(op);
    if status != "PENDING"
        || row.try_get::<chrono::DateTime<chrono::Utc>, _>("expires_at")? <= chrono::Utc::now()
    {
        return Err(ApiError::conflict(
            "UPLOAD_EXPIRED",
            "The upload has expired.",
        ));
    }
    super::campaigns::load(&state, campaign_id)
        .await?
        .ensure_open(chrono::Utc::now())?;
    let staging: String = row.try_get("staging_key")?;
    let bytes = state
        .storage
        .read_evidence(
            &staging,
            row.try_get::<i64, _>("expected_size")? as u64,
            &row.try_get::<String, _>("content_type")?,
        )
        .await?;
    let hash = alloy_primitives::keccak256(&bytes);
    let key = format!("evidence/{}/{upload_id}/{hash}", auth.user_id);
    state.storage.put_immutable(&key, bytes).await?;
    op = Mutation::begin(
        &state.db,
        auth.user_id,
        format!("complete-upload:{upload_id}"),
        &headers,
        &json!({"upload_id":upload_id}),
    )
    .await?;
    if let Some(v) = op.cached() {
        return Ok(Json(v));
    }
    let c = lock_campaign(op.tx(), campaign_id).await?;
    c.ensure_open(db_now(op.tx()).await?)?;
    let current = sqlx::query(
        "SELECT status,object_key,expires_at FROM uploads WHERE id=$1 AND user_id=$2 FOR UPDATE",
    )
    .bind(upload_id)
    .bind(auth.user_id)
    .fetch_one(&mut **op.tx())
    .await?;
    let current_status: String = current.try_get("status")?;
    if current_status == "COMPLETE" {
        return op
            .finish(json!({"upload_id":upload_id,"status":"COMPLETE"}))
            .await
            .map(Json);
    }
    if current_status != "PENDING"
        || current.try_get::<chrono::DateTime<chrono::Utc>, _>("expires_at")?
            <= db_now(op.tx()).await?
    {
        return Err(ApiError::conflict(
            "UPLOAD_EXPIRED",
            "The upload is no longer available.",
        ));
    }
    sqlx::query("UPDATE uploads SET status='COMPLETE',object_key=$2,content_hash=$3,completed_at=clock_timestamp() WHERE id=$1").bind(upload_id).bind(&key).bind(hash.as_slice()).execute(&mut **op.tx()).await?;
    op.audit(
        Some(campaign_id),
        "UPLOAD_COMPLETED",
        json!({"upload_id":upload_id,"content_hash":hash}),
    )
    .await?;
    op.finish(json!({"upload_id":upload_id,"status":"COMPLETE"}))
        .await
        .map(Json)
}
