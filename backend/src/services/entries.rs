use crate::{
    auth::Auth,
    domain::{Entry, validate_url},
    error::{ApiError, Result},
    http::ApiJson,
    mutations::{Mutation, db_now, lock_campaign},
    state::AppState,
};
use axum::{
    Json,
    extract::{Path, Query, State},
    http::HeaderMap,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sqlx::Row;
use uuid::Uuid;

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RegisterInput {
    pub x_username: Option<String>,
    pub discord_username: Option<String>,
}
pub fn view(e: &Entry) -> Value {
    json!({"id":e.id,"campaign_id":e.campaign_id,"payout_wallet":alloy_primitives::Address::from_slice(&e.payout_wallet),"slot_number":e.slot_number,"status":e.status,"version":e.version,"submitted_at":e.submitted_at})
}
pub async fn register(
    State(state): State<AppState>,
    auth: Auth,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
    ApiJson(input): ApiJson<RegisterInput>,
) -> Result<Json<Value>> {
    if input.x_username.as_ref().is_some_and(|s| s.len() > 64)
        || input
            .discord_username
            .as_ref()
            .is_some_and(|s| s.chars().count() > 100)
    {
        return Err(ApiError::invalid(
            "The declared social username is too long.",
        ));
    }
    let mut op = Mutation::begin(
        &state.db,
        auth.user_id,
        format!("register:{id}"),
        &headers,
        &serde_json::to_value(&input).map_err(|_| ApiError::internal())?,
    )
    .await?;
    if let Some(v) = op.cached() {
        return Ok(Json(v));
    }
    let c = lock_campaign(op.tx(), id).await?;
    c.ensure_open(db_now(op.tx()).await?)?;
    if !c.listed || c.moderation_hidden {
        return Err(ApiError::missing());
    }
    if c.registered_count >= c.registration_limit {
        return Err(ApiError::conflict(
            "CAPACITY_REACHED",
            "The campaign participant limit has been reached.",
        ));
    }
    if c.creator_id == auth.user_id {
        return Err(ApiError::invalid(
            "Creators cannot participate in their own campaign.",
        ));
    }
    let entry = Uuid::new_v4();
    sqlx::query("INSERT INTO entries(id,campaign_id,user_id,payout_wallet,slot_number,x_username_declared,discord_username_declared) VALUES($1,$2,$3,$4,$5,$6,$7)")
        .bind(entry).bind(id).bind(auth.user_id).bind(auth.wallet.as_slice()).bind(c.registered_count+1).bind(input.x_username).bind(input.discord_username).execute(&mut **op.tx()).await?;
    sqlx::query("UPDATE campaigns SET registered_count=registered_count+1 WHERE id=$1")
        .bind(id)
        .execute(&mut **op.tx())
        .await?;
    op.audit(Some(id), "ENTRY_REGISTERED", json!({"entry_id":entry}))
        .await?;
    let e: Entry = sqlx::query_as("SELECT * FROM entries WHERE id=$1")
        .bind(entry)
        .fetch_one(&mut **op.tx())
        .await?;
    op.finish(view(&e)).await.map(Json)
}
pub async fn mine(
    State(state): State<AppState>,
    auth: Auth,
    Path(id): Path<Uuid>,
) -> Result<Json<Value>> {
    let e: Entry = sqlx::query_as("SELECT * FROM entries WHERE campaign_id=$1 AND user_id=$2")
        .bind(id)
        .bind(auth.user_id)
        .fetch_one(&state.db)
        .await?;
    let mut value = view(&e);
    let decision=sqlx::query("SELECT decision,reason FROM entry_reviews WHERE entry_id=$1 ORDER BY created_at DESC,id DESC LIMIT 1").bind(e.id).fetch_optional(&state.db).await?;
    if let Some(row) = decision {
        value["review"] = json!({"decision":row.try_get::<String,_>("decision")?,"reason":row.try_get::<String,_>("reason")?});
    }
    Ok(Json(value))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EntryQuery {
    pub limit: Option<u32>,
    pub after: Option<Uuid>,
    pub status: Option<String>,
}
pub async fn list(
    State(state): State<AppState>,
    auth: Auth,
    Path(id): Path<Uuid>,
    Query(q): Query<EntryQuery>,
) -> Result<Json<Value>> {
    super::campaigns::load(&state, id)
        .await?
        .ensure_creator(auth.user_id)?;
    let limit = q.limit.unwrap_or(25);
    if limit == 0 || limit > 100 {
        return Err(ApiError::invalid(
            "The page size must be between one and 100.",
        ));
    }
    if q.status.as_ref().is_some_and(|s| {
        !matches!(
            s.as_str(),
            "REGISTERED" | "SUBMITTED" | "ELIGIBLE" | "DISQUALIFIED" | "NOT_SUBMITTED"
        )
    }) {
        return Err(ApiError::invalid("Invalid entry status."));
    }
    let mut rows:Vec<Entry>=sqlx::query_as("SELECT * FROM entries WHERE campaign_id=$1 AND ($2::uuid IS NULL OR id>$2) AND ($3::text IS NULL OR status=$3) ORDER BY id LIMIT $4")
        .bind(id).bind(q.after).bind(q.status).bind(limit as i64+1).fetch_all(&state.db).await?;
    let more = rows.len() > limit as usize;
    rows.truncate(limit as usize);
    Ok(Json(
        json!({"items":rows.iter().map(view).collect::<Vec<_>>(),"next_cursor":if more{rows.last().map(|e|e.id)}else{None}}),
    ))
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EvidenceInput {
    pub expected_version: i64,
    pub text: Option<String>,
    pub url: Option<String>,
    pub upload_id: Option<Uuid>,
}
pub async fn evidence(
    State(state): State<AppState>,
    auth: Auth,
    Path((id, task_id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    ApiJson(input): ApiJson<EvidenceInput>,
) -> Result<Json<Value>> {
    if input
        .text
        .as_ref()
        .is_some_and(|s| s.chars().count() > 2000)
        || (input.text.as_ref().is_none_or(|s| s.trim().is_empty())
            && input.url.is_none()
            && input.upload_id.is_none())
    {
        return Err(ApiError::invalid(
            "Provide evidence text, a URL, or a completed image upload.",
        ));
    }
    if let Some(url) = &input.url {
        validate_url(url)?;
    }
    let mut op = Mutation::begin(
        &state.db,
        auth.user_id,
        format!("evidence:{id}:{task_id}"),
        &headers,
        &serde_json::to_value(&input).map_err(|_| ApiError::internal())?,
    )
    .await?;
    if let Some(v) = op.cached() {
        return Ok(Json(v));
    }
    let c = lock_campaign(op.tx(), id).await?;
    c.ensure_open(db_now(op.tx()).await?)?;
    let e: Entry =
        sqlx::query_as("SELECT * FROM entries WHERE campaign_id=$1 AND user_id=$2 FOR UPDATE")
            .bind(id)
            .bind(auth.user_id)
            .fetch_one(&mut **op.tx())
            .await?;
    if e.version != input.expected_version {
        return Err(ApiError::conflict(
            "STALE_VERSION",
            "Refresh the entry before updating evidence.",
        ));
    }
    let exists: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM tasks WHERE campaign_id=$1 AND id=$2)")
            .bind(id)
            .bind(task_id)
            .fetch_one(&mut **op.tx())
            .await?;
    if !exists {
        return Err(ApiError::missing());
    }
    if let Some(upload) = input.upload_id {
        let ready:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM uploads WHERE id=$1 AND entry_id=$2 AND user_id=$3 AND status='COMPLETE')").bind(upload).bind(e.id).bind(auth.user_id).fetch_one(&mut **op.tx()).await?;
        if !ready {
            return Err(ApiError::invalid(
                "The evidence upload is unavailable or belongs to a different entry.",
            ));
        }
    }
    let revision: i32 = sqlx::query_scalar(
        "SELECT COALESCE(MAX(revision),0)+1 FROM submissions WHERE entry_id=$1 AND task_id=$2",
    )
    .bind(e.id)
    .bind(task_id)
    .fetch_one(&mut **op.tx())
    .await?;
    let value = json!({"text":input.text,"url":input.url,"upload_id":input.upload_id});
    sqlx::query("INSERT INTO submissions(id,campaign_id,entry_id,task_id,revision,evidence_json,upload_id) VALUES($1,$2,$3,$4,$5,$6,$7)")
        .bind(Uuid::new_v4()).bind(id).bind(e.id).bind(task_id).bind(revision).bind(value).bind(input.upload_id).execute(&mut **op.tx()).await?;
    // Editing a submitted entry requires an explicit resubmission of the complete latest evidence.
    sqlx::query("UPDATE entries SET status='REGISTERED',submitted_at=NULL,version=version+1,updated_at=clock_timestamp() WHERE id=$1").bind(e.id).execute(&mut **op.tx()).await?;
    op.audit(
        Some(id),
        "EVIDENCE_UPDATED",
        json!({"entry_id":e.id,"task_id":task_id,"revision":revision}),
    )
    .await?;
    op.finish(json!({"entry_id":e.id,"task_id":task_id,"revision":revision,"version":e.version+1,"status":"REGISTERED"})).await.map(Json)
}

pub async fn submit(
    State(state): State<AppState>,
    auth: Auth,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
    ApiJson(input): ApiJson<super::campaigns::VersionInput>,
) -> Result<Json<Value>> {
    let mut op = Mutation::begin(
        &state.db,
        auth.user_id,
        format!("submit:{id}"),
        &headers,
        &serde_json::to_value(&input).map_err(|_| ApiError::internal())?,
    )
    .await?;
    if let Some(v) = op.cached() {
        return Ok(Json(v));
    }
    let c = lock_campaign(op.tx(), id).await?;
    c.ensure_open(db_now(op.tx()).await?)?;
    let e: Entry =
        sqlx::query_as("SELECT * FROM entries WHERE campaign_id=$1 AND user_id=$2 FOR UPDATE")
            .bind(id)
            .bind(auth.user_id)
            .fetch_one(&mut **op.tx())
            .await?;
    if e.version != input.expected_version {
        return Err(ApiError::conflict(
            "STALE_VERSION",
            "Refresh the entry before submitting.",
        ));
    }
    let missing:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM tasks t WHERE t.campaign_id=$1 AND t.required AND NOT EXISTS(SELECT 1 FROM submissions s WHERE s.entry_id=$2 AND s.task_id=t.id))")
        .bind(id).bind(e.id).fetch_one(&mut **op.tx()).await?;
    if missing {
        return Err(ApiError::invalid(
            "Evidence is required for every required task.",
        ));
    }
    sqlx::query("UPDATE entries SET status='SUBMITTED',submitted_at=clock_timestamp(),version=version+1,updated_at=clock_timestamp() WHERE id=$1").bind(e.id).execute(&mut **op.tx()).await?;
    op.audit(Some(id), "ENTRY_SUBMITTED", json!({"entry_id":e.id}))
        .await?;
    op.finish(json!({"entry_id":e.id,"status":"SUBMITTED","version":e.version+1}))
        .await
        .map(Json)
}

pub async fn read_evidence(
    State(state): State<AppState>,
    auth: Auth,
    Path((id, entry_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<Value>> {
    let c = super::campaigns::load(&state, id).await?;
    let e: Entry = sqlx::query_as("SELECT * FROM entries WHERE campaign_id=$1 AND id=$2")
        .bind(id)
        .bind(entry_id)
        .fetch_one(&state.db)
        .await?;
    if c.creator_id != auth.user_id && e.user_id != auth.user_id {
        return Err(ApiError::forbidden());
    }
    let rows=sqlx::query("SELECT DISTINCT ON(s.task_id) s.task_id,s.revision,s.evidence_json,u.object_key FROM submissions s LEFT JOIN uploads u ON u.id=s.upload_id WHERE s.entry_id=$1 ORDER BY s.task_id,s.revision DESC").bind(entry_id).fetch_all(&state.db).await?;
    let mut evidence = Vec::new();
    for row in rows {
        let key: Option<String> = row.try_get("object_key")?;
        let signed = match key {
            Some(k) => Some(state.storage.signed_get(&k).await?),
            None => None,
        };
        evidence.push(json!({"task_id":row.try_get::<Uuid,_>("task_id")?,"revision":row.try_get::<i32,_>("revision")?,"evidence":row.try_get::<Value,_>("evidence_json")?,"image_url":signed}));
    }
    Ok(Json(
        json!({"entry":view(&e),"x_username_declared":e.x_username_declared,"discord_username_declared":e.discord_username_declared,"evidence":evidence}),
    ))
}
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ReviewInput {
    pub expected_version: i64,
    pub decision: String,
    pub reason: String,
}
pub async fn review(
    State(state): State<AppState>,
    auth: Auth,
    Path((id, entry_id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    ApiJson(input): ApiJson<ReviewInput>,
) -> Result<Json<Value>> {
    if !matches!(input.decision.as_str(), "ELIGIBLE" | "DISQUALIFIED")
        || input.reason.trim().is_empty()
        || input.reason.chars().count() > 2000
    {
        return Err(ApiError::invalid(
            "A valid decision and a review reason are required.",
        ));
    }
    let mut op = Mutation::begin(
        &state.db,
        auth.user_id,
        format!("review:{id}:{entry_id}"),
        &headers,
        &serde_json::to_value(&input).map_err(|_| ApiError::internal())?,
    )
    .await?;
    if let Some(v) = op.cached() {
        return Ok(Json(v));
    }
    let c = lock_campaign(op.tx(), id).await?;
    c.ensure_creator(auth.user_id)?;
    c.ensure_review(db_now(op.tx()).await?)?;
    let e: Entry =
        sqlx::query_as("SELECT * FROM entries WHERE campaign_id=$1 AND id=$2 FOR UPDATE")
            .bind(id)
            .bind(entry_id)
            .fetch_one(&mut **op.tx())
            .await?;
    if e.version != input.expected_version {
        return Err(ApiError::conflict(
            "STALE_VERSION",
            "Refresh the entry before reviewing.",
        ));
    }
    if !matches!(e.status.as_str(), "SUBMITTED" | "ELIGIBLE" | "DISQUALIFIED") {
        return Err(ApiError::conflict(
            "NOT_SUBMITTED",
            "Only entries submitted before cutoff can be reviewed.",
        ));
    }
    sqlx::query("INSERT INTO entry_reviews(id,entry_id,reviewer_id,decision,reason,entry_version) VALUES($1,$2,$3,$4,$5,$6)").bind(Uuid::new_v4()).bind(entry_id).bind(auth.user_id).bind(&input.decision).bind(input.reason).bind(e.version).execute(&mut **op.tx()).await?;
    sqlx::query(
        "UPDATE entries SET status=$2,version=version+1,updated_at=clock_timestamp() WHERE id=$1",
    )
    .bind(entry_id)
    .bind(&input.decision)
    .execute(&mut **op.tx())
    .await?;
    op.audit(
        Some(id),
        "ENTRY_REVIEWED",
        json!({"entry_id":entry_id,"decision":input.decision,"version":e.version+1}),
    )
    .await?;
    op.finish(json!({"entry_id":entry_id,"status":input.decision,"version":e.version+1}))
        .await
        .map(Json)
}
