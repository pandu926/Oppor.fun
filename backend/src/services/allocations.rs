use crate::{
    auth::Auth,
    crypto::{self, Allocation},
    domain::{AllocationPolicy, AssetKind, Campaign, DistributionMode},
    error::{ApiError, Result},
    http::ApiJson,
    mutations::{Mutation, db_now, lock_campaign},
    state::AppState,
};
use alloy_primitives::{Address, B256, U256};
use axum::{
    Json,
    extract::{Path, State},
    http::HeaderMap,
};
use serde_json::{Value, json};
use sqlx::{Postgres, Row};
use uuid::Uuid;

pub async fn lock(
    State(state): State<AppState>,
    auth: Auth,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
    ApiJson(input): ApiJson<super::campaigns::VersionInput>,
) -> Result<Json<Value>> {
    let mut op = Mutation::begin(
        &state.db,
        auth.user_id,
        format!("lock-eligibility:{id}"),
        &headers,
        &serde_json::to_value(&input).map_err(|_| ApiError::internal())?,
    )
    .await?;
    if let Some(v) = op.cached() {
        return Ok(Json(v));
    }
    let c = lock_campaign(op.tx(), id).await?;
    c.ensure_creator(auth.user_id)?;
    let now = db_now(op.tx()).await?;
    c.ensure_review(now)?;
    if c.version != input.expected_version {
        return Err(ApiError::conflict(
            "STALE_VERSION",
            "Refresh the campaign before locking eligibility.",
        ));
    }
    if (c.review_deadline - now).num_seconds() < 120 {
        return Err(ApiError::conflict(
            "REVIEW_DEADLINE_TOO_CLOSE",
            "At least two minutes are required to prepare the final distribution.",
        ));
    }
    let pending: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM entries WHERE campaign_id=$1 AND status='SUBMITTED')",
    )
    .bind(id)
    .fetch_one(&mut **op.tx())
    .await?;
    if pending {
        return Err(ApiError::conflict(
            "PENDING_REVIEWS",
            "Review every submitted entry before locking eligibility.",
        ));
    }
    sqlx::query("UPDATE entries SET status='NOT_SUBMITTED',version=version+1 WHERE campaign_id=$1 AND status='REGISTERED'").bind(id).execute(&mut **op.tx()).await?;
    let rows=sqlx::query("SELECT id,payout_wallet FROM entries WHERE campaign_id=$1 AND status='ELIGIBLE' ORDER BY payout_wallet").bind(id).fetch_all(&mut **op.tx()).await?;
    let wallets: Vec<Address> = rows
        .iter()
        .map(|r| {
            r.try_get::<Vec<u8>, _>("payout_wallet")
                .map(|w| Address::from_slice(&w))
        })
        .collect::<std::result::Result<_, _>>()?;
    let snapshot_id = Uuid::new_v4();
    let artifact = json!({"schema_version":1,"campaign_key":c.key(),"chain_id":c.chain_id.to_string(),"wallets":wallets,"eligible_count":rows.len(),"cutoff_at":c.cutoff_at.timestamp().to_string(),"locked_at":now.timestamp().to_string()});
    let hash = crypto::json_hash(&artifact)?;
    sqlx::query("INSERT INTO eligibility_snapshots(id,campaign_id,snapshot_hash,artifact,eligible_count,locked_at) VALUES($1,$2,$3,$4,$5,$6)")
        .bind(snapshot_id).bind(id).bind(hash.as_slice()).bind(&artifact).bind(rows.len() as i32).bind(now).execute(&mut **op.tx()).await?;
    for (chunk_index, chunk) in rows.chunks(1000).enumerate() {
        let mut q = sqlx::QueryBuilder::<Postgres>::new(
            "INSERT INTO snapshot_entries(snapshot_id,ordinal,entry_id,payout_wallet) ",
        );
        let base = chunk_index * 1000;
        q.push_values(chunk.iter().enumerate(), |mut b, (i, r)| {
            b.push_bind(snapshot_id)
                .push_bind((base + i) as i32)
                .push_bind(r.get::<Uuid, _>("id"))
                .push_bind(r.get::<Vec<u8>, _>("payout_wallet"));
        });
        q.build().execute(&mut **op.tx()).await?;
    }
    sqlx::query(
        "INSERT INTO jobs(id,kind,dedupe_key,campaign_id) VALUES($1,'BUILD_ALLOCATIONS',$2,$3)",
    )
    .bind(Uuid::new_v4())
    .bind(format!("allocations:{id}"))
    .bind(id)
    .execute(&mut **op.tx())
    .await?;
    sqlx::query("UPDATE campaigns SET status='ELIGIBILITY_LOCKED',version=version+1,updated_at=clock_timestamp() WHERE id=$1").bind(id).execute(&mut **op.tx()).await?;
    op.audit(
        Some(id),
        "ELIGIBILITY_LOCKED",
        json!({"snapshot_hash":hash,"eligible_count":rows.len()}),
    )
    .await?;
    op.finish(json!({"campaign_id":id,"status":"ELIGIBILITY_LOCKED","snapshot_hash":hash,"eligible_count":rows.len(),"version":c.version+1})).await.map(Json)
}

pub async fn preview(
    State(state): State<AppState>,
    auth: Auth,
    Path(id): Path<Uuid>,
) -> Result<Json<Value>> {
    let c = super::campaigns::load(&state, id).await?;
    c.ensure_creator(auth.user_id)?;
    let row=sqlx::query("SELECT root,manifest_hash,leaf_count,allocated_total::text AS total,object_key FROM allocation_manifests WHERE campaign_id=$1").bind(id).fetch_one(&state.db).await?;
    Ok(Json(
        json!({"root":crypto::hash_string(Some(&row.try_get::<Vec<u8>,_>("root")?)),"manifest_hash":crypto::hash_string(Some(&row.try_get::<Vec<u8>,_>("manifest_hash")?)),"leaf_count":row.try_get::<i32,_>("leaf_count")?,"allocated_quantity":row.try_get::<String,_>("total")?,"manifest_url":state.storage.signed_get(&row.try_get::<String,_>("object_key")?).await?,"trust_model":"Creator-reviewed eligibility; reproducible server raffle. Not trustless randomness."}),
    ))
}
async fn ensure_public(state: &AppState, id: Uuid) -> Result<Campaign> {
    let c = super::campaigns::load(state, id).await?;
    if !c.listed {
        return Err(ApiError::missing());
    }
    if c.status == "INTEGRITY_ERROR" {
        return Err(ApiError::conflict(
            "INTEGRITY_ERROR",
            "The campaign's onchain distribution does not match its published artifacts.",
        ));
    }
    if c.chain_state != 2 && c.chain_state != 4 {
        return Err(ApiError::conflict(
            "RESULTS_NOT_FINAL",
            "The creator has not finalized the distribution.",
        ));
    }
    state.ensure_indexer().await?;
    state.rpc.verify_escrow(&c, &state.config).await?;
    let root = state
        .rpc
        .call(
            c.escrow()?,
            crate::chain::Escrow::distributionRootCall {}.abi_encode(),
        )
        .await?;
    let expected: Vec<u8> =
        sqlx::query_scalar("SELECT root FROM allocation_manifests WHERE campaign_id=$1")
            .bind(id)
            .fetch_one(&state.db)
            .await?;
    if root != expected {
        return Err(ApiError::conflict(
            "ROOT_MISMATCH",
            "The onchain root does not match the published manifest.",
        ));
    }
    Ok(c)
}
use alloy_sol_types::SolCall;
pub async fn results(State(state): State<AppState>, Path(id): Path<Uuid>) -> Result<Json<Value>> {
    ensure_public(&state, id).await?;
    let snapshot: Vec<u8> =
        sqlx::query_scalar("SELECT snapshot_hash FROM eligibility_snapshots WHERE campaign_id=$1")
            .bind(id)
            .fetch_one(&state.db)
            .await?;
    let snapshot_hash = B256::from_slice(&snapshot);
    let has_raffle: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM raffles WHERE campaign_id=$1 AND executed_at IS NOT NULL)",
    )
    .bind(id)
    .fetch_one(&state.db)
    .await?;
    let raffle_url = if has_raffle {
        Some(
            state
                .storage
                .signed_get(&format!("raffles/{id}.json"))
                .await?,
        )
    } else {
        None
    };
    let row=sqlx::query("SELECT root,manifest_hash,leaf_count,object_key FROM allocation_manifests WHERE campaign_id=$1").bind(id).fetch_one(&state.db).await?;
    Ok(Json(
        json!({"eligibility_hash":snapshot_hash,"eligible_snapshot_url":state.storage.signed_get(&format!("snapshots/{snapshot_hash}.json")).await?,"raffle_transcript_url":raffle_url,"root":crypto::hash_string(Some(&row.try_get::<Vec<u8>,_>("root")?)),"manifest_hash":crypto::hash_string(Some(&row.try_get::<Vec<u8>,_>("manifest_hash")?)),"leaf_count":row.try_get::<i32,_>("leaf_count")?,"manifest_url":state.storage.signed_get(&row.try_get::<String,_>("object_key")?).await?}),
    ))
}
pub async fn manifest(State(state): State<AppState>, Path(id): Path<Uuid>) -> Result<Json<Value>> {
    ensure_public(&state, id).await?;
    let row = sqlx::query(
        "SELECT manifest_hash,object_key FROM allocation_manifests WHERE campaign_id=$1",
    )
    .bind(id)
    .fetch_one(&state.db)
    .await?;
    Ok(Json(
        json!({"manifest_hash":crypto::hash_string(Some(&row.try_get::<Vec<u8>,_>("manifest_hash")?)),"url":state.storage.signed_get(&row.try_get::<String,_>("object_key")?).await?,"expires_in":120}),
    ))
}
pub async fn proofs(
    State(state): State<AppState>,
    Path((id, wallet)): Path<(Uuid, Address)>,
) -> Result<Json<Value>> {
    let c = ensure_public(&state, id).await?;
    let rows=sqlx::query("SELECT claim_index,token_id::text AS token_id,quantity::text AS quantity,proof_json,claim_tx_hash,claimed_at FROM allocations WHERE campaign_id=$1 AND recipient=$2 ORDER BY claim_index").bind(id).bind(wallet.as_slice()).fetch_all(&state.db).await?;
    let items=rows.into_iter().map(|r|Ok(json!({"index":r.try_get::<i32,_>("claim_index")?.to_string(),"recipient":wallet,"token_id":r.try_get::<String,_>("token_id")?,"quantity":r.try_get::<String,_>("quantity")?,"proof":r.try_get::<Value,_>("proof_json")?,"claim_tx_hash":crypto::hash_string(r.try_get::<Option<Vec<u8>>,_>("claim_tx_hash")?.as_deref()),"claimed_at":r.try_get::<Option<chrono::DateTime<chrono::Utc>>,_>("claimed_at")?}))).collect::<Result<Vec<_>>>()?;
    Ok(Json(
        json!({"campaign_id":id,"escrow":c.escrow()?,"claim_deadline":c.claim_deadline,"allocations":items}),
    ))
}

pub struct DistributionResult {
    pub allocations: Vec<Allocation>,
    pub leaves: Vec<B256>,
    pub root: B256,
    pub total: U256,
    pub transcript: Option<Value>,
}
pub fn build(
    c: &Campaign,
    snapshot_hash: B256,
    entries: &[(Uuid, Address)],
    seed: Option<B256>,
) -> Result<DistributionResult> {
    if entries.len() > 100_000 || entries.windows(2).any(|pair| pair[0].1 >= pair[1].1) {
        return Err(ApiError::invalid(
            "The eligibility snapshot must contain at most 100,000 unique wallets in ascending order.",
        ));
    }
    let s = c.spec()?;
    let escrow = c.escrow()?;
    let (selected, transcript) = if s.distribution.mode == DistributionMode::Raffle {
        let seed = seed.ok_or_else(ApiError::internal)?;
        let k = (s.distribution.winner_count as usize).min(entries.len());
        let draw = crypto::draw_seed(c.key(), snapshot_hash, seed);
        let (winners, counter) = crypto::raffle(draw, entries.len(), k);
        let transcript = json!({"algorithm_version":"raffle-v1","campaign_key":c.key(),"eligibility_hash":snapshot_hash,"seed":seed,"commitment":crypto::seed_commitment(c.key(),seed),"draw_seed":draw,"counter_final":counter.to_string(),"winner_ordinals":winners,"winners":winners.iter().map(|i|entries[*i].1).collect::<Vec<_>>(),"trust_model":"REPRODUCIBLE_SERVER_DRAW"});
        (winners, Some(transcript))
    } else {
        ((0..entries.len()).collect(), None)
    };
    let k = selected.len();
    let pool = crate::domain::positive(&s.reward.amount_base_units)?;
    let mut allocations = Vec::with_capacity(k);
    if k > 0 {
        let quantity = if s.reward.asset_kind == AssetKind::Erc721 {
            U256::from(1)
        } else if s.distribution.allocation_policy == AllocationPolicy::FixedReward {
            crate::domain::positive(
                s.distribution
                    .reward_per_recipient
                    .as_deref()
                    .ok_or_else(ApiError::internal)?,
            )?
        } else {
            pool / U256::from(k)
        };
        if quantity == U256::ZERO {
            return Err(ApiError::invalid(
                "The pool does not provide at least one base unit per recipient.",
            ));
        }
        let mut nft_ids = s
            .reward
            .nft_inventory
            .iter()
            .map(|v| crate::domain::amount(v))
            .collect::<Result<Vec<_>>>()?;
        nft_ids.sort();
        for (pos, index) in selected.iter().enumerate() {
            let token_id = if s.reward.asset_kind == AssetKind::Erc721 {
                nft_ids
                    .get(pos)
                    .copied()
                    .ok_or_else(|| ApiError::invalid("The NFT inventory is insufficient."))?
                    .to_string()
            } else {
                s.reward.token_id.clone()
            };
            allocations.push(Allocation {
                index: "0".into(),
                recipient: entries[*index].1,
                token_id,
                quantity: quantity.to_string(),
                proof: vec![],
            });
        }
    }
    allocations.sort_by_key(|a| a.recipient);
    let mut total = U256::ZERO;
    for (i, a) in allocations.iter_mut().enumerate() {
        a.index = i.to_string();
        total = total
            .checked_add(crate::domain::positive(&a.quantity)?)
            .ok_or_else(ApiError::internal)?;
    }
    if total > pool || allocations.len() > 100_000 {
        return Err(ApiError::invalid(
            "The allocation exceeds the funded pool or leaf limit.",
        ));
    }
    let leaves = allocations
        .iter()
        .map(|a| {
            crypto::leaf(
                c.chain_id as u64,
                escrow,
                c.key(),
                s.reward.asset_kind.id(),
                s.reward.token_address,
                a,
            )
        })
        .collect::<Result<Vec<_>>>()?;
    let (root, proofs) = crypto::merkle(&leaves);
    for (a, p) in allocations.iter_mut().zip(proofs) {
        a.proof = p;
    }
    for (a, h) in allocations.iter().zip(&leaves) {
        if !crypto::verify_proof(root, *h, &a.proof) {
            return Err(ApiError::internal());
        }
    }
    Ok(DistributionResult {
        allocations,
        leaves,
        root,
        total,
        transcript,
    })
}
