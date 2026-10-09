use crate::{
    auth::{Auth, MaybeAuth},
    crypto,
    domain::{Campaign, CampaignInput, DistributionMode, TaskInput},
    error::{ApiError, Result},
    http::ApiJson,
    mutations::{Mutation, db_now, lock_campaign},
    state::AppState,
};
use alloy_primitives::{B256, keccak256};
use axum::{
    Json,
    extract::{Path, Query, State},
    http::HeaderMap,
};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sqlx::{Postgres, Row, Transaction};
use uuid::Uuid;

pub async fn load(state: &AppState, id: Uuid) -> Result<Campaign> {
    Ok(sqlx::query_as("SELECT * FROM campaigns WHERE id=$1")
        .bind(id)
        .fetch_one(&state.db)
        .await?)
}
pub fn public_view(c: &Campaign) -> Result<Value> {
    let spec = c.spec()?;
    Ok(
        json!({"id":c.id,"campaign_key":c.key(),"creator_wallet":c.creator(),"chain_id":c.chain_id.to_string(),"escrow_address":crypto::hash_string(c.escrow_address.as_deref()),"title":c.title,"description":c.description,"status":c.display_status(Utc::now()),"reward":spec.reward,"distribution":spec.distribution,"start_at":c.start_at,"cutoff_at":c.cutoff_at,"review_deadline":c.review_deadline,"claim_deadline":c.claim_deadline,"registered_count":c.registered_count,"registration_limit":c.registration_limit,"config_hash":crypto::hash_string(c.config_hash.as_deref()),"rules_hash":crypto::hash_string(c.rules_hash.as_deref()),"rules":c.rules_json,"version":c.version,"verification_mode":"MANUAL","created_at":c.created_at}),
    )
}
pub async fn detail(
    State(state): State<AppState>,
    auth: MaybeAuth,
    Path(id): Path<Uuid>,
) -> Result<Json<Value>> {
    let c = load(&state, id).await?;
    if !c.listed && !auth.0.as_ref().is_some_and(|a| a.user_id == c.creator_id) {
        return Err(ApiError::missing());
    }
    if c.moderation_hidden && !auth.0.as_ref().is_some_and(|a| a.user_id == c.creator_id) {
        let participant = if let Some(a) = auth.0.as_ref() {
            sqlx::query_scalar::<_, bool>(
                "SELECT EXISTS(SELECT 1 FROM entries WHERE campaign_id=$1 AND user_id=$2)",
            )
            .bind(id)
            .bind(a.user_id)
            .fetch_one(&state.db)
            .await?
        } else {
            false
        };
        if !participant {
            return Err(ApiError::missing());
        }
    }
    let mut view = public_view(&c)?;
    view["tasks"] = tasks(&state, id).await?;
    Ok(Json(view))
}
pub async fn tasks(state: &AppState, id: Uuid) -> Result<Value> {
    let rows=sqlx::query("SELECT id,position,task_type,target_url,instructions,required FROM tasks WHERE campaign_id=$1 ORDER BY position").bind(id).fetch_all(&state.db).await?;
    Ok(Value::Array(rows.into_iter().map(|r|Ok(json!({"id":r.try_get::<Uuid,_>("id")?,"position":r.try_get::<i32,_>("position")?,"task_type":r.try_get::<String,_>("task_type")?,"target_url":r.try_get::<String,_>("target_url")?,"instructions":r.try_get::<String,_>("instructions")?,"required":r.try_get::<bool,_>("required")?}))).collect::<Result<Vec<_>>>()?))
}

#[derive(Debug, Deserialize, Serialize, Default)]
#[serde(deny_unknown_fields)]
pub struct ListQuery {
    pub limit: Option<u32>,
    pub cursor: Option<String>,
    pub mode: Option<String>,
    pub asset_kind: Option<String>,
    pub status: Option<String>,
    pub sort: Option<String>,
}
#[derive(Serialize, Deserialize)]
struct Cursor {
    at: DateTime<Utc>,
    id: Uuid,
}
pub async fn list(
    State(state): State<AppState>,
    Query(query): Query<ListQuery>,
) -> Result<Json<Value>> {
    let revision: i64 =
        sqlx::query_scalar("SELECT listing_revision FROM platform_state WHERE singleton")
            .fetch_one(&state.db)
            .await?;
    let cache_key = format!(
        "oppor:{}:{}:campaign-list:{revision}:{}",
        state.config.chain_id,
        state.config.factory,
        crypto::json_hash(&query)?
    );
    let cached: std::result::Result<String, _> = tokio::time::timeout(
        std::time::Duration::from_millis(100),
        redis::cmd("GET")
            .arg(&cache_key)
            .query_async(&mut state.redis.clone()),
    )
    .await
    .unwrap_or_else(|_| {
        Err(redis::RedisError::from((
            redis::ErrorKind::IoError,
            "Cache timeout",
        )))
    });
    if let Ok(value) = cached
        && let Ok(parsed) = serde_json::from_str::<Value>(&value)
    {
        return Ok(Json(parsed));
    }
    let value = page(&state, &query, None).await?;
    let bytes = serde_json::to_string(&value).map_err(|_| ApiError::internal())?;
    let _: std::result::Result<std::result::Result<(), redis::RedisError>, _> =
        tokio::time::timeout(
            std::time::Duration::from_millis(100),
            redis::cmd("SET")
                .arg(&cache_key)
                .arg(bytes)
                .arg("EX")
                .arg(15)
                .query_async(&mut state.redis.clone()),
        )
        .await;
    Ok(Json(value))
}
pub async fn mine(
    State(state): State<AppState>,
    auth: Auth,
    Query(query): Query<ListQuery>,
) -> Result<Json<Value>> {
    page(&state, &query, Some(auth.user_id)).await.map(Json)
}
async fn page(state: &AppState, q: &ListQuery, creator: Option<Uuid>) -> Result<Value> {
    let limit = q.limit.unwrap_or(25);
    if limit == 0 || limit > 100 {
        return Err(ApiError::invalid(
            "The page size must be between one and 100.",
        ));
    }
    let ending = match q.sort.as_deref().unwrap_or("newest") {
        "newest" => false,
        "ending" => true,
        _ => return Err(ApiError::invalid("Unsupported sort order.")),
    };
    if q.mode
        .as_ref()
        .is_some_and(|s| !matches!(s.as_str(), "ALL_ELIGIBLE" | "RAFFLE"))
    {
        return Err(ApiError::invalid("Unsupported distribution mode."));
    }
    let kind = match q.asset_kind.as_deref() {
        None => None,
        Some("ERC20") => Some(0i16),
        Some("ERC721") => Some(1),
        Some("ERC1155") => Some(2),
        _ => return Err(ApiError::invalid("Unsupported reward asset kind.")),
    };
    if q.status.as_ref().is_some_and(|s| {
        !matches!(
            s.as_str(),
            "DRAFT"
                | "CONFIG_LOCKED"
                | "FUNDING"
                | "SCHEDULED"
                | "ACTIVE"
                | "REVIEWING"
                | "ELIGIBILITY_LOCKED"
                | "ALLOCATION_READY"
                | "CLAIM_OPEN"
                | "COMPLETED"
                | "CANCELLED"
                | "EXPIRED"
                | "CLOSED"
                | "INTEGRITY_ERROR"
        )
    }) {
        return Err(ApiError::invalid("Unsupported campaign status."));
    }
    let cursor = q
        .cursor
        .as_ref()
        .map(|v| {
            if v.len() > 300 {
                return Err(ApiError::invalid("Invalid pagination cursor."));
            }
            let b = URL_SAFE_NO_PAD
                .decode(v)
                .map_err(|_| ApiError::invalid("Invalid pagination cursor."))?;
            serde_json::from_slice::<Cursor>(&b)
                .map_err(|_| ApiError::invalid("Invalid pagination cursor."))
        })
        .transpose()?;
    let mut builder = sqlx::QueryBuilder::<Postgres>::new(
        "SELECT c.* FROM campaigns c JOIN campaign_rewards r ON r.campaign_id=c.id WHERE ",
    );
    if let Some(id) = creator {
        builder.push("c.creator_id=").push_bind(id);
    } else {
        builder.push("c.listed=true AND NOT c.moderation_hidden");
    }
    if let Some(m) = &q.mode {
        builder.push(" AND c.mode=").push_bind(m);
    }
    if let Some(k) = kind {
        builder.push(" AND r.asset_kind=").push_bind(k);
    }
    if let Some(s) = &q.status {
        builder.push(" AND (CASE WHEN c.status IN ('INTEGRITY_ERROR','CANCELLED','CLOSED') THEN c.status WHEN c.chain_state=2 AND clock_timestamp()>=c.claim_deadline THEN 'COMPLETED' WHEN c.chain_state=1 AND clock_timestamp()>=c.review_deadline THEN 'EXPIRED' WHEN c.status IN ('SCHEDULED','ACTIVE','REVIEWING') AND c.activated THEN CASE WHEN clock_timestamp()>=c.cutoff_at THEN 'REVIEWING' WHEN clock_timestamp()>=c.start_at THEN 'ACTIVE' ELSE 'SCHEDULED' END ELSE c.status END)=").push_bind(s);
    }
    if let Some(c) = cursor {
        builder
            .push(if ending {
                " AND (c.cutoff_at,c.id) > ("
            } else {
                " AND (c.created_at,c.id) < ("
            })
            .push_bind(c.at)
            .push(",")
            .push_bind(c.id)
            .push(")");
    }
    builder
        .push(if ending {
            " ORDER BY c.cutoff_at ASC,c.id ASC"
        } else {
            " ORDER BY c.created_at DESC,c.id DESC"
        })
        .push(" LIMIT ")
        .push_bind(limit as i64 + 1);
    let mut rows = builder
        .build_query_as::<Campaign>()
        .fetch_all(&state.db)
        .await?;
    let has_more = rows.len() > limit as usize;
    rows.truncate(limit as usize);
    let next = if has_more {
        rows.last().map(|c| {
            URL_SAFE_NO_PAD.encode(
                serde_json::to_vec(&Cursor {
                    at: if ending { c.cutoff_at } else { c.created_at },
                    id: c.id,
                })
                .expect("Serializable cursor"),
            )
        })
    } else {
        None
    };
    Ok(
        json!({"items":rows.iter().map(public_view).collect::<Result<Vec<_>>>()?,"next_cursor":next}),
    )
}

pub async fn create(
    State(state): State<AppState>,
    auth: Auth,
    headers: HeaderMap,
    ApiJson(input): ApiJson<CampaignInput>,
) -> Result<Json<Value>> {
    let body = serde_json::to_value(&input).map_err(|_| ApiError::internal())?;
    let mut op = Mutation::begin(
        &state.db,
        auth.user_id,
        "create-campaign".into(),
        &headers,
        &body,
    )
    .await?;
    if let Some(v) = op.cached() {
        return Ok(Json(v));
    }
    input.validate(state.config.chain_id, db_now(op.tx()).await?)?;
    let id = Uuid::new_v4();
    let key = keccak256(crypto::random32());
    sqlx::query("INSERT INTO campaigns(id,campaign_key,creator_id,creator_wallet,chain_id,title,description,mode,start_at,cutoff_at,review_deadline,claim_deadline,capacity,registration_limit,winner_count,spec_json) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16)")
        .bind(id).bind(key.as_slice()).bind(auth.user_id).bind(auth.wallet.as_slice()).bind(state.config.chain_id as i64).bind(&input.title).bind(&input.description).bind(input.distribution.mode.as_str())
        .bind(input.start_at).bind(input.cutoff_at).bind(input.review_deadline).bind(input.claim_deadline).bind(input.distribution.capacity as i32).bind(input.effective_limit()).bind(input.distribution.winner_count as i32).bind(body).execute(&mut **op.tx()).await?;
    persist_reward(op.tx(), id, &input).await?;
    insert_tasks(op.tx(), id, &input.tasks).await?;
    op.audit(Some(id), "CAMPAIGN_CREATED", json!({})).await?;
    let c = lock_campaign(op.tx(), id).await?;
    let view = public_view(&c)?;
    op.finish(view).await.map(Json)
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct UpdateInput {
    pub expected_version: i64,
    pub campaign: CampaignInput,
}
pub async fn update(
    State(state): State<AppState>,
    auth: Auth,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
    ApiJson(input): ApiJson<UpdateInput>,
) -> Result<Json<Value>> {
    let mut op = Mutation::begin(
        &state.db,
        auth.user_id,
        format!("update-campaign:{id}"),
        &headers,
        &serde_json::to_value(&input).map_err(|_| ApiError::internal())?,
    )
    .await?;
    if let Some(v) = op.cached() {
        return Ok(Json(v));
    }
    let c = lock_campaign(op.tx(), id).await?;
    check_draft(&c, auth.user_id, input.expected_version)?;
    let s = &input.campaign;
    s.validate(state.config.chain_id, db_now(op.tx()).await?)?;
    sqlx::query("UPDATE campaigns SET title=$2,description=$3,mode=$4,start_at=$5,cutoff_at=$6,review_deadline=$7,claim_deadline=$8,capacity=$9,registration_limit=$10,winner_count=$11,spec_json=$12,version=version+1,updated_at=clock_timestamp() WHERE id=$1")
        .bind(id).bind(&s.title).bind(&s.description).bind(s.distribution.mode.as_str()).bind(s.start_at).bind(s.cutoff_at).bind(s.review_deadline).bind(s.claim_deadline).bind(s.distribution.capacity as i32).bind(s.effective_limit()).bind(s.distribution.winner_count as i32).bind(serde_json::to_value(s).map_err(|_|ApiError::internal())?).execute(&mut **op.tx()).await?;
    persist_reward(op.tx(), id, s).await?;
    sqlx::query("DELETE FROM tasks WHERE campaign_id=$1")
        .bind(id)
        .execute(&mut **op.tx())
        .await?;
    insert_tasks(op.tx(), id, &s.tasks).await?;
    op.audit(Some(id), "CAMPAIGN_UPDATED", json!({"version":c.version+1}))
        .await?;
    let c = lock_campaign(op.tx(), id).await?;
    op.finish(public_view(&c)?).await.map(Json)
}
pub fn check_draft(c: &Campaign, user: Uuid, version: i64) -> Result<()> {
    c.ensure_creator(user)?;
    if c.status != "DRAFT" {
        return Err(ApiError::conflict(
            "CONFIG_LOCKED",
            "Campaign configuration is already locked.",
        ));
    }
    if c.version != version {
        return Err(ApiError::conflict(
            "STALE_VERSION",
            "Refresh the campaign before editing.",
        ));
    }
    Ok(())
}

async fn persist_reward(
    tx: &mut Transaction<'_, Postgres>,
    id: Uuid,
    s: &CampaignInput,
) -> Result<()> {
    sqlx::query("INSERT INTO campaign_rewards(campaign_id,asset_kind,token_address,token_id,target_amount) VALUES($1,$2,$3,$4::text::numeric,$5::text::numeric) ON CONFLICT(campaign_id) DO UPDATE SET asset_kind=EXCLUDED.asset_kind,token_address=EXCLUDED.token_address,token_id=EXCLUDED.token_id,target_amount=EXCLUDED.target_amount")
        .bind(id).bind(s.reward.asset_kind.id() as i16).bind(s.reward.token_address.as_slice()).bind(&s.reward.token_id).bind(&s.reward.amount_base_units).execute(&mut **tx).await?;
    sqlx::query("DELETE FROM reward_nft_inventory WHERE campaign_id=$1")
        .bind(id)
        .execute(&mut **tx)
        .await?;
    for token in &s.reward.nft_inventory {
        sqlx::query(
            "INSERT INTO reward_nft_inventory(campaign_id,token_id) VALUES($1,$2::text::numeric)",
        )
        .bind(id)
        .bind(token)
        .execute(&mut **tx)
        .await?;
    }
    Ok(())
}
async fn insert_tasks(
    tx: &mut Transaction<'_, Postgres>,
    id: Uuid,
    tasks: &[TaskInput],
) -> Result<()> {
    for (i, t) in tasks.iter().enumerate() {
        insert_task(tx, id, i, t).await?;
    }
    Ok(())
}
async fn insert_task(
    tx: &mut Transaction<'_, Postgres>,
    id: Uuid,
    pos: usize,
    t: &TaskInput,
) -> Result<Uuid> {
    let task = Uuid::new_v4();
    sqlx::query("INSERT INTO tasks(id,campaign_id,position,task_type,target_url,instructions,required) VALUES($1,$2,$3,$4,$5,$6,$7)").bind(task).bind(id).bind(pos as i32).bind(&t.task_type).bind(&t.target_url).bind(&t.instructions).bind(t.required).execute(&mut **tx).await?;
    Ok(task)
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TaskEdit {
    pub expected_version: i64,
    pub task: TaskInput,
}
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct VersionInput {
    pub expected_version: i64,
}
pub async fn add_task(
    State(state): State<AppState>,
    auth: Auth,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
    ApiJson(input): ApiJson<TaskEdit>,
) -> Result<Json<Value>> {
    let mut op = Mutation::begin(
        &state.db,
        auth.user_id,
        format!("add-task:{id}"),
        &headers,
        &serde_json::to_value(&input).map_err(|_| ApiError::internal())?,
    )
    .await?;
    if let Some(v) = op.cached() {
        return Ok(Json(v));
    }
    let c = lock_campaign(op.tx(), id).await?;
    check_draft(&c, auth.user_id, input.expected_version)?;
    let mut s = c.spec()?;
    s.tasks.push(input.task.clone());
    s.validate(state.config.chain_id, db_now(op.tx()).await?)?;
    let task = insert_task(op.tx(), id, s.tasks.len() - 1, &input.task).await?;
    save_task_spec(&mut op, id, &s).await?;
    op.finish(json!({"task_id":task,"version":c.version+1}))
        .await
        .map(Json)
}
pub async fn update_task(
    State(state): State<AppState>,
    auth: Auth,
    Path((id, task_id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    ApiJson(input): ApiJson<TaskEdit>,
) -> Result<Json<Value>> {
    let mut op = Mutation::begin(
        &state.db,
        auth.user_id,
        format!("edit-task:{id}:{task_id}"),
        &headers,
        &serde_json::to_value(&input).map_err(|_| ApiError::internal())?,
    )
    .await?;
    if let Some(v) = op.cached() {
        return Ok(Json(v));
    }
    let c = lock_campaign(op.tx(), id).await?;
    check_draft(&c, auth.user_id, input.expected_version)?;
    let mut s = c.spec()?;
    let position: i32 =
        sqlx::query_scalar("SELECT position FROM tasks WHERE id=$1 AND campaign_id=$2")
            .bind(task_id)
            .bind(id)
            .fetch_one(&mut **op.tx())
            .await?;
    s.tasks[position as usize] = input.task.clone();
    s.validate(state.config.chain_id, db_now(op.tx()).await?)?;
    let t = &input.task;
    sqlx::query("UPDATE tasks SET task_type=$3,target_url=$4,instructions=$5,required=$6 WHERE id=$1 AND campaign_id=$2").bind(task_id).bind(id).bind(&t.task_type).bind(&t.target_url).bind(&t.instructions).bind(t.required).execute(&mut **op.tx()).await?;
    save_task_spec(&mut op, id, &s).await?;
    op.finish(json!({"task_id":task_id,"version":c.version+1}))
        .await
        .map(Json)
}
pub async fn delete_task(
    State(state): State<AppState>,
    auth: Auth,
    Path((id, task_id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    ApiJson(input): ApiJson<VersionInput>,
) -> Result<Json<Value>> {
    let mut op = Mutation::begin(
        &state.db,
        auth.user_id,
        format!("delete-task:{id}:{task_id}"),
        &headers,
        &serde_json::to_value(&input).map_err(|_| ApiError::internal())?,
    )
    .await?;
    if let Some(v) = op.cached() {
        return Ok(Json(v));
    }
    let c = lock_campaign(op.tx(), id).await?;
    check_draft(&c, auth.user_id, input.expected_version)?;
    let mut s = c.spec()?;
    let pos: i32 = sqlx::query_scalar("SELECT position FROM tasks WHERE id=$1 AND campaign_id=$2")
        .bind(task_id)
        .bind(id)
        .fetch_one(&mut **op.tx())
        .await?;
    s.tasks.remove(pos as usize);
    s.validate(state.config.chain_id, db_now(op.tx()).await?)?;
    sqlx::query("DELETE FROM tasks WHERE id=$1 AND campaign_id=$2")
        .bind(task_id)
        .bind(id)
        .execute(&mut **op.tx())
        .await?;
    // Updating in ascending order avoids colliding with the next unique position.
    let remaining = sqlx::query(
        "SELECT id,position FROM tasks WHERE campaign_id=$1 AND position>$2 ORDER BY position",
    )
    .bind(id)
    .bind(pos)
    .fetch_all(&mut **op.tx())
    .await?;
    for row in remaining {
        sqlx::query("UPDATE tasks SET position=position-1 WHERE id=$1")
            .bind(row.try_get::<Uuid, _>("id")?)
            .execute(&mut **op.tx())
            .await?;
    }
    save_task_spec(&mut op, id, &s).await?;
    op.finish(json!({"deleted":true,"version":c.version+1}))
        .await
        .map(Json)
}
async fn save_task_spec(op: &mut Mutation, id: Uuid, s: &CampaignInput) -> Result<()> {
    sqlx::query("UPDATE campaigns SET spec_json=$2,version=version+1,updated_at=clock_timestamp() WHERE id=$1").bind(id).bind(serde_json::to_value(s).map_err(|_|ApiError::internal())?).execute(&mut **op.tx()).await?;
    op.audit(Some(id), "TASKS_UPDATED", json!({})).await
}

pub async fn lock_config(
    State(state): State<AppState>,
    auth: Auth,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
    ApiJson(input): ApiJson<VersionInput>,
) -> Result<Json<Value>> {
    let mut op = Mutation::begin(
        &state.db,
        auth.user_id,
        format!("lock-config:{id}"),
        &headers,
        &serde_json::to_value(&input).map_err(|_| ApiError::internal())?,
    )
    .await?;
    if let Some(v) = op.cached() {
        return Ok(Json(v));
    }
    let mut c = lock_campaign(op.tx(), id).await?;
    check_draft(&c, auth.user_id, input.expected_version)?;
    let s = c.spec()?;
    s.validate(state.config.chain_id, db_now(op.tx()).await?)?;
    let commitment = if s.distribution.mode == DistributionMode::Raffle {
        let seed = zeroize::Zeroizing::new(crypto::random32());
        let commitment = crypto::seed_commitment(c.key(), B256::new(*seed));
        let encrypted = crypto::encrypt_seed(&state.config.seed_key, c.key(), &seed)?;
        sqlx::query(
            "INSERT INTO raffles(campaign_id,seed_commitment,encrypted_seed) VALUES($1,$2,$3)",
        )
        .bind(id)
        .bind(commitment.as_slice())
        .bind(encrypted)
        .execute(&mut **op.tx())
        .await?;
        commitment
    } else {
        B256::ZERO
    };
    let task_rows =
        sqlx::query("SELECT id,position FROM tasks WHERE campaign_id=$1 ORDER BY position")
            .bind(id)
            .fetch_all(&mut **op.tx())
            .await?;
    let task_ids = task_rows
        .iter()
        .map(|r| r.try_get::<Uuid, _>("id"))
        .collect::<std::result::Result<Vec<_>, _>>()?;
    let rules = json!({"schema_version":1,"campaign_key":c.key(),"chain_id":s.chain_id,"creator":c.creator(),"refund_recipient":s.refund_recipient.unwrap_or(c.creator()),"reward":s.reward,"distribution":s.distribution,"task_ids":task_ids,"tasks":s.tasks,"starts_at":s.start_at.timestamp().to_string(),"cutoff_at":s.cutoff_at.timestamp().to_string(),"review_deadline":s.review_deadline.timestamp().to_string(),"claim_deadline":s.claim_deadline.timestamp().to_string(),"verification_mode":"MANUAL","raffle_algorithm":"raffle-v1","raffle_seed_commitment":commitment,"claim_policy":"RECIPIENT_SELF_CLAIM","refund_policy":"IMMUTABLE_RECIPIENT_AFTER_DEADLINE"});
    let hash = crypto::json_hash(&rules)?;
    c.rules_hash = Some(hash.to_vec());
    let config_hash =
        crate::chain::configuration_hash(&crate::chain::configuration(&c, commitment)?);
    sqlx::query("UPDATE campaigns SET status='CONFIG_LOCKED',rules_json=$2,rules_hash=$3,config_hash=$4,version=version+1,updated_at=clock_timestamp() WHERE id=$1").bind(id).bind(&rules).bind(hash.as_slice()).bind(config_hash.as_slice()).execute(&mut **op.tx()).await?;
    op.audit(
        Some(id),
        "CONFIG_LOCKED",
        json!({"config_hash":config_hash}),
    )
    .await?;
    op.finish(json!({"campaign_id":id,"config_hash":config_hash,"rules_hash":hash,"rules":rules,"version":c.version+1})).await.map(Json)
}
