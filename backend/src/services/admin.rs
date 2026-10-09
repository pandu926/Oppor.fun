use crate::{
    auth::Auth,
    error::{ApiError, Result},
    http::ApiJson,
    mutations::{Mutation, lock_campaign},
    state::AppState,
};
use alloy_primitives::Address;
use axum::{
    Json,
    extract::{FromRequestParts, Path, Query, State},
    http::{HeaderMap, request::Parts},
};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sqlx::{FromRow, Postgres, Row, Transaction};
use uuid::Uuid;

pub struct Admin {
    auth: Auth,
    reauthenticate_at: DateTime<Utc>,
}
impl FromRequestParts<AppState> for Admin {
    type Rejection = ApiError;
    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self> {
        let auth = Auth::from_request_parts(parts, state).await?;
        if !state.config.admin_wallets.contains(&auth.wallet) {
            return Err(ApiError::forbidden());
        }
        let row=sqlx::query("SELECT created_at,created_at>clock_timestamp()-interval '15 minutes' AS fresh FROM sessions WHERE id=$1 AND revoked_at IS NULL AND expires_at>clock_timestamp()")
            .bind(auth.session_id).fetch_optional(&state.db).await?.ok_or_else(ApiError::unauthorized)?;
        if !row.try_get::<bool, _>("fresh")? {
            return Err(reauth_required());
        }
        Ok(Self {
            auth,
            reauthenticate_at: row.try_get::<DateTime<Utc>, _>("created_at")?
                + Duration::minutes(15),
        })
    }
}
fn reauth_required() -> ApiError {
    ApiError {
        status: axum::http::StatusCode::FORBIDDEN,
        code: "ADMIN_REAUTH_REQUIRED",
        message: "Sign in again to perform administrative operations.",
    }
}
impl Admin {
    async fn begin(
        &self,
        state: &AppState,
        headers: &HeaderMap,
        scope: String,
        body: &Value,
    ) -> Result<Mutation> {
        let mut op = Mutation::begin(&state.db, self.auth.user_id, scope, headers, body).await?;
        // Fence administrative actions against concurrent session revocation, including cached retries.
        let fresh:bool=sqlx::query_scalar("SELECT created_at>clock_timestamp()-interval '15 minutes' FROM sessions WHERE id=$1 AND revoked_at IS NULL AND expires_at>clock_timestamp() FOR SHARE")
            .bind(self.auth.session_id).fetch_optional(&mut **op.tx()).await?.ok_or_else(ApiError::unauthorized)?;
        if !fresh {
            return Err(reauth_required());
        }
        Ok(op)
    }
}
pub async fn me(admin: Admin) -> Json<Value> {
    Json(
        json!({"user_id":admin.auth.user_id,"wallet":admin.auth.wallet,"role":"ADMIN","reauthenticate_at":admin.reauthenticate_at,"permissions":["READ_PLATFORM_STATISTICS","MODERATE_CAMPAIGNS","SUSPEND_USERS","REVOKE_USER_SESSIONS","READ_AUDIT_LOGS","READ_JOBS"]}),
    )
}

pub async fn stats(State(state): State<AppState>, _admin: Admin) -> Result<Json<Value>> {
    let counts:Value=sqlx::query_scalar("SELECT jsonb_build_object('users',(SELECT count(*) FROM users),'suspended_users',(SELECT count(*) FROM users WHERE suspended),'campaigns',(SELECT count(*) FROM campaigns),'hidden_campaigns',(SELECT count(*) FROM campaigns WHERE moderation_hidden),'public_campaigns',(SELECT count(*) FROM campaigns WHERE listed AND NOT moderation_hidden),'entries',(SELECT count(*) FROM entries),'claimed_allocations',(SELECT count(*) FROM allocations WHERE claimed_at IS NOT NULL),'jobs',(SELECT coalesce(jsonb_object_agg(state,n),'{}'::jsonb) FROM (SELECT state,count(*) n FROM jobs GROUP BY state) j))").fetch_one(&state.db).await?;
    let indexer:Option<Value>=sqlx::query_scalar("SELECT jsonb_build_object('next_block',next_block::text,'observed_safe_head',observed_safe_head::text,'updated_at',updated_at,'halted_reason',halted_reason,'healthy',halted_reason IS NULL AND observed_safe_head IS NOT NULL AND observed_safe_head-next_block<=100 AND updated_at>clock_timestamp()-make_interval(secs=>$3)) FROM indexer_cursors WHERE chain_id=$1 AND factory_address=$2")
        .bind(state.config.chain_id as i64).bind(state.config.factory.as_slice()).bind(state.config.max_indexer_lag as f64).fetch_optional(&state.db).await?;
    Ok(Json(
        json!({"counts":counts,"indexer":indexer,"generated_at":Utc::now()}),
    ))
}

#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ListQuery {
    pub limit: Option<u32>,
    pub cursor: Option<String>,
    pub status: Option<String>,
    pub wallet: Option<Address>,
    pub campaign_id: Option<Uuid>,
    pub actor_id: Option<Uuid>,
    pub hidden: Option<bool>,
    pub suspended: Option<bool>,
}
#[derive(Deserialize, Serialize)]
struct Cursor {
    at: DateTime<Utc>,
    id: Uuid,
}
enum Listing {
    Campaigns,
    Users,
    Audit,
    Jobs,
}
pub async fn campaigns(
    State(state): State<AppState>,
    _admin: Admin,
    Query(query): Query<ListQuery>,
) -> Result<Json<Value>> {
    list(&state, Listing::Campaigns, query).await.map(Json)
}
pub async fn users(
    State(state): State<AppState>,
    _admin: Admin,
    Query(query): Query<ListQuery>,
) -> Result<Json<Value>> {
    list(&state, Listing::Users, query).await.map(Json)
}
pub async fn audit_logs(
    State(state): State<AppState>,
    _admin: Admin,
    Query(query): Query<ListQuery>,
) -> Result<Json<Value>> {
    list(&state, Listing::Audit, query).await.map(Json)
}
pub async fn jobs(
    State(state): State<AppState>,
    _admin: Admin,
    Query(query): Query<ListQuery>,
) -> Result<Json<Value>> {
    list(&state, Listing::Jobs, query).await.map(Json)
}
async fn list(state: &AppState, kind: Listing, q: ListQuery) -> Result<Value> {
    let limit = q.limit.unwrap_or(25);
    if !(1..=100).contains(&limit) {
        return Err(ApiError::invalid(
            "The page size must be between one and 100.",
        ));
    }
    let permitted = match kind {
        Listing::Campaigns => {
            q.wallet.is_none()
                && q.campaign_id.is_none()
                && q.actor_id.is_none()
                && q.suspended.is_none()
                && q.status.is_none()
        }
        Listing::Users => {
            q.campaign_id.is_none()
                && q.actor_id.is_none()
                && q.hidden.is_none()
                && q.status.is_none()
        }
        Listing::Audit => {
            q.wallet.is_none() && q.hidden.is_none() && q.suspended.is_none() && q.status.is_none()
        }
        Listing::Jobs => {
            q.wallet.is_none()
                && q.actor_id.is_none()
                && q.hidden.is_none()
                && q.suspended.is_none()
        }
    };
    if !permitted {
        return Err(ApiError::invalid(
            "A filter is not supported by this administrative listing.",
        ));
    }
    if q.status
        .as_deref()
        .is_some_and(|s| !matches!(s, "PENDING" | "RUNNING" | "FAILED" | "SUCCEEDED"))
    {
        return Err(ApiError::invalid("Invalid job status."));
    }
    let cursor = q
        .cursor
        .as_ref()
        .map(|raw| {
            if raw.len() > 300 {
                return Err(ApiError::invalid("Invalid pagination cursor."));
            }
            let bytes = URL_SAFE_NO_PAD
                .decode(raw)
                .map_err(|_| ApiError::invalid("Invalid pagination cursor."))?;
            serde_json::from_slice::<Cursor>(&bytes)
                .map_err(|_| ApiError::invalid("Invalid pagination cursor."))
        })
        .transpose()?;
    let select = match kind {
        Listing::Campaigns => "SELECT j.* FROM campaigns j WHERE true",
        Listing::Users => {
            "SELECT j.id,j.created_at,jsonb_build_object('id',j.id,'wallet','0x'||encode(j.primary_wallet,'hex'),'created_at',j.created_at,'suspended',j.suspended,'version',j.admin_version,'suspension_reason',j.suspension_reason,'suspension_updated_at',j.suspension_updated_at) AS data FROM users j WHERE true"
        }
        Listing::Audit => {
            "SELECT j.id,j.created_at,jsonb_build_object('id',j.id,'actor_id',j.actor_id,'campaign_id',j.campaign_id,'action',j.action,'metadata',j.metadata,'created_at',j.created_at) AS data FROM audit_logs j WHERE true"
        }
        Listing::Jobs => {
            "SELECT j.id,j.created_at,jsonb_build_object('id',j.id,'campaign_id',j.campaign_id,'kind',j.kind,'status',j.state,'attempts',j.attempts,'available_at',j.available_at,'locked_until',j.locked_until,'last_error',j.last_error,'created_at',j.created_at) AS data FROM jobs j WHERE true"
        }
    };
    let mut sql = sqlx::QueryBuilder::<Postgres>::new(select);
    if let Some(hidden) = q.hidden {
        sql.push(" AND j.moderation_hidden=").push_bind(hidden);
    }
    if let Some(suspended) = q.suspended {
        sql.push(" AND j.suspended=").push_bind(suspended);
    }
    if let Some(wallet) = q.wallet {
        sql.push(" AND j.primary_wallet=")
            .push_bind(wallet.to_vec());
    }
    if let Some(campaign) = q.campaign_id {
        sql.push(" AND j.campaign_id=").push_bind(campaign);
    }
    if let Some(actor) = q.actor_id {
        sql.push(" AND j.actor_id=").push_bind(actor);
    }
    if let Some(status) = q.status {
        sql.push(" AND j.state=").push_bind(status);
    }
    if let Some(cursor) = cursor {
        sql.push(" AND (j.created_at,j.id)<(")
            .push_bind(cursor.at)
            .push(",")
            .push_bind(cursor.id)
            .push(")");
    }
    sql.push(" ORDER BY j.created_at DESC,j.id DESC LIMIT ")
        .push_bind(limit as i64 + 1);
    let mut rows = sql.build().fetch_all(&state.db).await?;
    let more = rows.len() > limit as usize;
    rows.truncate(limit as usize);
    let next = if more {
        rows.last()
            .map(|row| -> Result<String> {
                Ok(URL_SAFE_NO_PAD.encode(
                    serde_json::to_vec(&Cursor {
                        at: row.try_get("created_at")?,
                        id: row.try_get("id")?,
                    })
                    .map_err(|_| ApiError::internal())?,
                ))
            })
            .transpose()?
    } else {
        None
    };
    let items=rows.iter().map(|row|->Result<Value>{
        if matches!(kind,Listing::Campaigns){
            let c=crate::domain::Campaign::from_row(row)?;
            Ok(json!({"campaign":super::campaigns::public_view(&c)?,"moderation":{"hidden":c.moderation_hidden,"version":c.moderation_version,"reason":c.moderation_reason,"updated_at":c.moderation_updated_at}}))
        }else{Ok(row.try_get::<Value,_>("data")?)}
    }).collect::<Result<Vec<_>>>()?;
    Ok(json!({"items":items,"next_cursor":next}))
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModerationInput {
    pub expected_version: i64,
    pub hidden: bool,
    pub reason: String,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SuspensionInput {
    pub expected_version: i64,
    pub suspended: bool,
    pub reason: String,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RevokeInput {
    pub expected_version: i64,
    pub reason: String,
}
fn validate_reason(reason: &str, version: i64) -> Result<()> {
    if version < 0 || reason.trim().is_empty() || reason.chars().count() > 2000 {
        return Err(ApiError::invalid(
            "A nonnegative version and a reason of 1–2,000 characters are required.",
        ));
    }
    Ok(())
}
pub async fn moderate(
    State(state): State<AppState>,
    admin: Admin,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
    ApiJson(input): ApiJson<ModerationInput>,
) -> Result<Json<Value>> {
    validate_reason(&input.reason, input.expected_version)?;
    let mut op = admin
        .begin(
            &state,
            &headers,
            format!("admin-moderate:{id}"),
            &serde_json::to_value(&input).map_err(|_| ApiError::internal())?,
        )
        .await?;
    if let Some(cached) = op.cached() {
        return Ok(Json(cached));
    }
    let c = lock_campaign(op.tx(), id).await?;
    if c.moderation_version != input.expected_version {
        return Err(ApiError::conflict(
            "STALE_VERSION",
            "Refresh the campaign moderation state.",
        ));
    }
    sqlx::query("UPDATE campaigns SET moderation_hidden=$2,moderation_version=moderation_version+1,moderation_reason=$3,moderation_updated_at=clock_timestamp() WHERE id=$1")
        .bind(id).bind(input.hidden).bind(input.reason.trim()).execute(&mut **op.tx()).await?;
    op.audit(Some(id),"ADMIN_CAMPAIGN_MODERATED",json!({"hidden":input.hidden,"reason":input.reason.trim(),"version":c.moderation_version+1})).await?;
    op.finish(json!({"campaign_id":id,"hidden":input.hidden,"version":c.moderation_version+1}))
        .await
        .map(Json)
}
async fn target_user(
    tx: &mut Transaction<'_, Postgres>,
    state: &AppState,
    id: Uuid,
    version: i64,
) -> Result<()> {
    let row = sqlx::query("SELECT primary_wallet,admin_version FROM users WHERE id=$1 FOR UPDATE")
        .bind(id)
        .fetch_one(&mut **tx)
        .await?;
    let wallet = Address::from_slice(&row.try_get::<Vec<u8>, _>("primary_wallet")?);
    if state.config.admin_wallets.contains(&wallet) {
        return Err(ApiError::conflict(
            "PROTECTED_ADMIN",
            "Administrative accounts must be managed through the deployment configuration.",
        ));
    }
    if row.try_get::<i64, _>("admin_version")? != version {
        return Err(ApiError::conflict(
            "STALE_VERSION",
            "Refresh the user administration state.",
        ));
    }
    Ok(())
}
pub async fn suspend(
    State(state): State<AppState>,
    admin: Admin,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
    ApiJson(input): ApiJson<SuspensionInput>,
) -> Result<Json<Value>> {
    validate_reason(&input.reason, input.expected_version)?;
    let mut op = admin
        .begin(
            &state,
            &headers,
            format!("admin-suspend:{id}"),
            &serde_json::to_value(&input).map_err(|_| ApiError::internal())?,
        )
        .await?;
    if let Some(cached) = op.cached() {
        return Ok(Json(cached));
    }
    target_user(op.tx(), &state, id, input.expected_version).await?;
    sqlx::query("UPDATE users SET suspended=$2,admin_version=admin_version+1,suspension_reason=$3,suspension_updated_at=clock_timestamp() WHERE id=$1").bind(id).bind(input.suspended).bind(input.reason.trim()).execute(&mut **op.tx()).await?;
    let revoked = if input.suspended {
        sqlx::query("UPDATE sessions SET revoked_at=clock_timestamp() WHERE user_id=$1 AND revoked_at IS NULL AND expires_at>clock_timestamp()").bind(id).execute(&mut **op.tx()).await?.rows_affected()
    } else {
        0
    };
    op.audit(None,"ADMIN_USER_SUSPENSION_CHANGED",json!({"user_id":id,"suspended":input.suspended,"reason":input.reason.trim(),"revoked_sessions":revoked,"version":input.expected_version+1})).await?;
    op.finish(json!({"user_id":id,"suspended":input.suspended,"version":input.expected_version+1,"revoked_sessions":revoked})).await.map(Json)
}
pub async fn revoke_sessions(
    State(state): State<AppState>,
    admin: Admin,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
    ApiJson(input): ApiJson<RevokeInput>,
) -> Result<Json<Value>> {
    validate_reason(&input.reason, input.expected_version)?;
    let mut op = admin
        .begin(
            &state,
            &headers,
            format!("admin-revoke-sessions:{id}"),
            &serde_json::to_value(&input).map_err(|_| ApiError::internal())?,
        )
        .await?;
    if let Some(cached) = op.cached() {
        return Ok(Json(cached));
    }
    target_user(op.tx(), &state, id, input.expected_version).await?;
    let revoked=sqlx::query("UPDATE sessions SET revoked_at=clock_timestamp() WHERE user_id=$1 AND revoked_at IS NULL AND expires_at>clock_timestamp()").bind(id).execute(&mut **op.tx()).await?.rows_affected();
    sqlx::query("UPDATE users SET admin_version=admin_version+1 WHERE id=$1")
        .bind(id)
        .execute(&mut **op.tx())
        .await?;
    op.audit(None,"ADMIN_USER_SESSIONS_REVOKED",json!({"user_id":id,"reason":input.reason.trim(),"revoked_sessions":revoked,"version":input.expected_version+1})).await?;
    op.finish(json!({"user_id":id,"version":input.expected_version+1,"revoked_sessions":revoked}))
        .await
        .map(Json)
}
