use crate::{
    chain::{self, Escrow, Factory},
    domain::Campaign,
    error::{ApiError, Result},
    state::AppState,
};
use alloy_primitives::{Address, B256, U256};
use alloy_sol_types::SolEvent;
use futures_util::{StreamExt, TryStreamExt, stream};
use serde_json::{Value, json};
use sqlx::{Postgres, Row, Transaction};
use uuid::Uuid;

pub async fn tick(state: &AppState) -> Result<()> {
    let cfg = &state.config;
    state.rpc.verify_factory(cfg).await?;
    sqlx::query("INSERT INTO indexer_cursors(chain_id,factory_address,next_block) VALUES($1,$2,$3) ON CONFLICT DO NOTHING")
        .bind(cfg.chain_id as i64).bind(cfg.factory.as_slice()).bind(cfg.deployment_block as i64).execute(&state.db).await?;
    let cursor=sqlx::query("SELECT next_block,previous_block_hash,halted_reason FROM indexer_cursors WHERE chain_id=$1 AND factory_address=$2")
        .bind(cfg.chain_id as i64).bind(cfg.factory.as_slice()).fetch_one(&state.db).await?;
    if cursor
        .try_get::<Option<String>, _>("halted_reason")?
        .is_some()
    {
        return Err(ApiError::unavailable());
    }
    let from = cursor.try_get::<i64, _>("next_block")? as u64;
    if let Some(previous) = cursor.try_get::<Option<Vec<u8>>, _>("previous_block_hash")? {
        if from == 0 {
            return Err(ApiError::unavailable());
        }
        let block = state.rpc.block(from - 1).await?;
        let actual: B256 =
            serde_json::from_value(block["hash"].clone()).map_err(|_| ApiError::unavailable())?;
        if actual.as_slice() != previous {
            halt(state, "BLOCK_HASH_MISMATCH").await?;
            return Err(ApiError::unavailable());
        }
    }
    let Some(safe_head) = state.rpc.head().await?.checked_sub(cfg.confirmations) else {
        return Ok(());
    };
    if safe_head < from {
        if from > safe_head.saturating_add(1) {
            return Err(ApiError::unavailable());
        }
        sqlx::query("UPDATE indexer_cursors SET updated_at=clock_timestamp(),observed_safe_head=$4 WHERE chain_id=$1 AND factory_address=$2 AND next_block=$3 AND halted_reason IS NULL")
            .bind(cfg.chain_id as i64).bind(cfg.factory.as_slice()).bind(from as i64).bind(safe_head as i64).execute(&state.db).await?;
        return Ok(());
    }
    let mut to = safe_head.min(from + 49);
    let mut factory_logs = loop {
        match state.rpc.logs(from, to, &[cfg.factory]).await {
            Ok(logs) => break logs,
            Err(_) if to > from => to = from + (to - from) / 2,
            Err(e) => return Err(e),
        }
    };
    factory_logs.sort_by_key(log_order);
    let mut discovered = Vec::new();
    for log in &factory_logs {
        if topic(log)? == Factory::CampaignCreated::SIGNATURE_HASH {
            let event = decode::<Factory::CampaignCreated>(log)?;
            let known:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM campaigns WHERE chain_id=$1 AND campaign_key=$2 AND creator_wallet=$3)").bind(cfg.chain_id as i64).bind(event.campaignKey.as_slice()).bind(event.creator.as_slice()).fetch_one(&state.db).await?;
            if known {
                if alloy_primitives::keccak256(state.rpc.code(event.escrow).await?)
                    != cfg.escrow_code_hash
                {
                    return Err(ApiError::unavailable());
                }
                discovered.push(event.escrow);
            }
        }
    }
    let existing: Vec<Vec<u8>> = sqlx::query_scalar(
        "SELECT escrow_address FROM campaigns WHERE chain_id=$1 AND escrow_address IS NOT NULL",
    )
    .bind(cfg.chain_id as i64)
    .fetch_all(&state.db)
    .await?;
    discovered.extend(existing.iter().map(|b| Address::from_slice(b)));
    discovered.sort();
    discovered.dedup();
    let mut logs = factory_logs;
    for addresses in discovered.chunks(200) {
        logs.extend(state.rpc.logs(from, to, addresses).await?);
    }
    logs.sort_by_key(log_order);
    // Bound RPC concurrency and fetch each distinct receipt/block once, outside DB transactions.
    let mut hashes = std::collections::BTreeSet::new();
    let mut blocks = std::collections::BTreeSet::new();
    for log in &logs {
        if recognized(topic(log)?) {
            hashes.insert(
                serde_json::from_value::<B256>(log["transactionHash"].clone())
                    .map_err(|_| ApiError::unavailable())?,
            );
            blocks.insert(chain::quantity(&log["blockNumber"])?);
        }
    }
    let receipts: std::collections::HashMap<B256, Value> = stream::iter(hashes)
        .map(|hash| async move { Ok::<_, ApiError>((hash, state.rpc.receipt(hash).await?)) })
        .buffer_unordered(8)
        .try_collect()
        .await?;
    let block_hashes: std::collections::HashMap<u64, B256> = stream::iter(blocks)
        .map(|number| async move {
            let block = state.rpc.block(number).await?;
            let hash = serde_json::from_value(block["hash"].clone())
                .map_err(|_| ApiError::unavailable())?;
            Ok::<_, ApiError>((number, hash))
        })
        .buffer_unordered(8)
        .try_collect()
        .await?;
    let mut verified = Vec::new();
    for log in logs {
        if !recognized(topic(&log)?) {
            continue;
        }
        if log.get("removed").is_some_and(|v| v == &json!(true)) {
            return Err(ApiError::unavailable());
        }
        let hash: B256 = serde_json::from_value(log["transactionHash"].clone())
            .map_err(|_| ApiError::unavailable())?;
        let block = chain::quantity(&log["blockNumber"])?;
        if block < from || block > to {
            return Err(ApiError::unavailable());
        }
        let expected: B256 = serde_json::from_value(log["blockHash"].clone())
            .map_err(|_| ApiError::unavailable())?;
        if block_hashes[&block] != expected {
            return Err(ApiError::unavailable());
        }
        let receipt = &receipts[&hash];
        if chain::quantity(&receipt["status"])? != 1
            || receipt["blockHash"] != log["blockHash"]
            || receipt["transactionHash"] != log["transactionHash"]
            || receipt["blockNumber"] != log["blockNumber"]
        {
            return Err(ApiError::unavailable());
        }
        let included = receipt["logs"].as_array().is_some_and(|events| {
            events.iter().any(|event| {
                event["logIndex"] == log["logIndex"]
                    && event["address"] == log["address"]
                    && event["topics"] == log["topics"]
                    && event["data"] == log["data"]
            })
        });
        if !included {
            return Err(ApiError::unavailable());
        }
        verified.push(log);
    }
    let end_block = state.rpc.block(to).await?;
    let end_hash: B256 =
        serde_json::from_value(end_block["hash"].clone()).map_err(|_| ApiError::unavailable())?;
    if block_hashes.get(&to).is_some_and(|h| *h != end_hash) {
        return Err(ApiError::unavailable());
    }
    // Recheck boundaries after RPC reads; fail closed if the provider changed the range mid-read.
    if let Some(previous) = cursor.try_get::<Option<Vec<u8>>, _>("previous_block_hash")? {
        let block = state.rpc.block(from - 1).await?;
        if chain::decode_hex(&block["hash"])? != previous {
            halt(state, "BLOCK_HASH_MISMATCH").await?;
            return Err(ApiError::unavailable());
        }
    }
    let mut tx = state.db.begin().await?;
    let current:i64=sqlx::query_scalar("SELECT next_block FROM indexer_cursors WHERE chain_id=$1 AND factory_address=$2 AND halted_reason IS NULL FOR UPDATE")
        .bind(cfg.chain_id as i64).bind(cfg.factory.as_slice()).fetch_one(&mut *tx).await?;
    if current != from as i64 {
        return Ok(());
    }
    for log in &verified {
        let topic = topic(log)?;
        let hash = chain::decode_hex(&log["transactionHash"])?;
        let emitter = chain::decode_hex(&log["address"])?;
        let inserted=sqlx::query("INSERT INTO chain_events(chain_id,block_number,block_hash,tx_hash,log_index,address,event_type,payload) VALUES($1,$2,$3,$4,$5,$6,$7,$8) ON CONFLICT DO NOTHING")
            .bind(cfg.chain_id as i64).bind(chain::quantity(&log["blockNumber"])? as i64).bind(chain::decode_hex(&log["blockHash"])?).bind(&hash).bind(chain::quantity(&log["logIndex"])? as i32).bind(&emitter).bind(topic.to_string()).bind(log).execute(&mut *tx).await?;
        if inserted.rows_affected() == 0 {
            continue;
        }
        project(&mut tx, state, log, &receipts[&B256::from_slice(&hash)]).await?;
    }
    sqlx::query("UPDATE indexer_cursors SET next_block=$3,previous_block_hash=$4,updated_at=clock_timestamp(),observed_safe_head=$5 WHERE chain_id=$1 AND factory_address=$2")
        .bind(cfg.chain_id as i64).bind(cfg.factory.as_slice()).bind((to+1) as i64).bind(end_hash.as_slice()).bind(safe_head as i64).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
}
async fn halt(state: &AppState, reason: &str) -> Result<()> {
    sqlx::query(
        "UPDATE indexer_cursors SET halted_reason=$3 WHERE chain_id=$1 AND factory_address=$2",
    )
    .bind(state.config.chain_id as i64)
    .bind(state.config.factory.as_slice())
    .bind(reason)
    .execute(&state.db)
    .await?;
    Ok(())
}
fn log_order(v: &Value) -> (u64, u64) {
    (
        chain::quantity(&v["blockNumber"]).unwrap_or(u64::MAX),
        chain::quantity(&v["logIndex"]).unwrap_or(u64::MAX),
    )
}
fn topic(v: &Value) -> Result<B256> {
    serde_json::from_value(v["topics"][0].clone()).map_err(|_| ApiError::unavailable())
}
fn recognized(t: B256) -> bool {
    [
        Factory::CampaignCreated::SIGNATURE_HASH,
        Escrow::RewardFunded::SIGNATURE_HASH,
        Escrow::CampaignActivated::SIGNATURE_HASH,
        Escrow::DistributionFinalized::SIGNATURE_HASH,
        Escrow::RewardClaimed::SIGNATURE_HASH,
        Escrow::CampaignCancelled::SIGNATURE_HASH,
        Escrow::RemainingSwept::SIGNATURE_HASH,
    ]
    .contains(&t)
}
fn decode<E: SolEvent>(v: &Value) -> Result<E> {
    let topics: Vec<B256> =
        serde_json::from_value(v["topics"].clone()).map_err(|_| ApiError::unavailable())?;
    let data = chain::decode_hex(&v["data"])?;
    E::decode_raw_log_validate(topics, &data).map_err(|_| ApiError::unavailable())
}
async fn incident(tx: &mut Transaction<'_, Postgres>, id: Uuid, reason: &str) -> Result<()> {
    sqlx::query(
        "UPDATE campaigns SET status='INTEGRITY_ERROR',updated_at=clock_timestamp() WHERE id=$1",
    )
    .bind(id)
    .execute(&mut **tx)
    .await?;
    sqlx::query("INSERT INTO audit_logs(id,campaign_id,action,metadata) VALUES($1,$2,'CHAIN_INTEGRITY_ERROR',$3)").bind(Uuid::new_v4()).bind(id).bind(json!({"reason":reason})).execute(&mut **tx).await?;
    Ok(())
}
async fn project(
    tx: &mut Transaction<'_, Postgres>,
    state: &AppState,
    log: &Value,
    receipt: &Value,
) -> Result<()> {
    let signature = topic(log)?;
    let emitter: Address =
        serde_json::from_value(log["address"].clone()).map_err(|_| ApiError::unavailable())?;
    let mut actor: Address =
        serde_json::from_value(receipt["from"].clone()).map_err(|_| ApiError::unavailable())?;
    let hash = chain::decode_hex(&log["transactionHash"])?;
    let event_kind = signature_kind(signature).ok_or_else(ApiError::unavailable)?;
    let event_campaign;
    if signature == Factory::CampaignCreated::SIGNATURE_HASH {
        if emitter != state.config.factory {
            return Err(ApiError::unavailable());
        }
        let event = decode::<Factory::CampaignCreated>(log)?;
        let c:Option<Campaign>=sqlx::query_as("SELECT * FROM campaigns WHERE chain_id=$1 AND campaign_key=$2 AND creator_wallet=$3 FOR UPDATE").bind(state.config.chain_id as i64).bind(event.campaignKey.as_slice()).bind(event.creator.as_slice()).fetch_optional(&mut **tx).await?;
        let Some(c) = c else { return Ok(()) };
        event_campaign = c.id;
        if c.status != "CONFIG_LOCKED"
            || Some(event.configHash.as_slice()) != c.config_hash.as_deref()
        {
            return incident(tx, c.id, "CREATION_CONFIG_MISMATCH").await;
        }
        actor = c.creator();
        sqlx::query(
            "UPDATE campaigns SET escrow_address=$2,status='FUNDING',version=version+1 WHERE id=$1",
        )
        .bind(c.id)
        .bind(event.escrow.as_slice())
        .execute(&mut **tx)
        .await?;
    } else {
        let c: Option<Campaign> = sqlx::query_as(
            "SELECT * FROM campaigns WHERE chain_id=$1 AND escrow_address=$2 FOR UPDATE",
        )
        .bind(state.config.chain_id as i64)
        .bind(emitter.as_slice())
        .fetch_optional(&mut **tx)
        .await?;
        let Some(c) = c else { return Ok(()) };
        event_campaign = c.id;
        if c.status == "INTEGRITY_ERROR" {
            return Ok(());
        }
        // Verified escrow bytecode enforces msg.sender. Receipt.from can be an AA bundler.
        if signature != Escrow::RewardClaimed::SIGNATURE_HASH
            && signature != Escrow::RemainingSwept::SIGNATURE_HASH
        {
            actor = c.creator();
        }
        let s = c.spec()?;
        if signature == Escrow::RewardFunded::SIGNATURE_HASH {
            let e = decode::<Escrow::RewardFunded>(log)?;
            if c.chain_state != 0
                || e.assetKind != s.reward.asset_kind.id()
                || e.token != s.reward.token_address
                || e.quantity == U256::ZERO
            {
                return incident(tx, c.id, "INVALID_FUNDING_EVENT").await;
            }
            let expected_id = crate::domain::amount(&s.reward.token_id)?;
            if s.reward.asset_kind == crate::domain::AssetKind::Erc721 {
                if e.quantity != U256::from(1)
                    || !s
                        .reward
                        .nft_inventory
                        .iter()
                        .any(|id| crate::domain::amount(id).is_ok_and(|v| v == e.tokenId))
                {
                    return incident(tx, c.id, "NFT_INVENTORY_MISMATCH").await;
                }
                let changed=sqlx::query("UPDATE reward_nft_inventory SET deposited=true WHERE campaign_id=$1 AND token_id=$2::text::numeric AND deposited=false").bind(c.id).bind(e.tokenId.to_string()).execute(&mut **tx).await?;
                if changed.rows_affected() != 1 {
                    return incident(tx, c.id, "DUPLICATE_NFT_FUNDING").await;
                }
            } else if e.tokenId != expected_id {
                return incident(tx, c.id, "TOKEN_ID_MISMATCH").await;
            }
            let changed=sqlx::query("UPDATE campaign_rewards SET funded_amount=funded_amount+$2::text::numeric WHERE campaign_id=$1 AND funded_amount+$2::text::numeric<=target_amount").bind(c.id).bind(e.quantity.to_string()).execute(&mut **tx).await?;
            if changed.rows_affected() != 1 {
                return incident(tx, c.id, "FUNDING_EXCEEDS_TARGET").await;
            }
        } else if signature == Escrow::CampaignActivated::SIGNATURE_HASH {
            let e = decode::<Escrow::CampaignActivated>(log)?;
            let funded: bool = sqlx::query_scalar(
                "SELECT funded_amount=target_amount FROM campaign_rewards WHERE campaign_id=$1",
            )
            .bind(c.id)
            .fetch_one(&mut **tx)
            .await?;
            if c.chain_state != 0
                || !funded
                || Some(e.configHash.as_slice()) != c.config_hash.as_deref()
            {
                return incident(tx, c.id, "ACTIVATION_MISMATCH").await;
            }
            sqlx::query("UPDATE campaigns SET activated=true,chain_state=1,status='SCHEDULED',listed=true,version=version+1 WHERE id=$1").bind(c.id).execute(&mut **tx).await?;
        } else if signature == Escrow::DistributionFinalized::SIGNATURE_HASH {
            let e = decode::<Escrow::DistributionFinalized>(log)?;
            let matching:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM allocation_manifests m JOIN eligibility_snapshots s ON s.id=m.snapshot_id WHERE m.campaign_id=$1 AND m.root=$2 AND m.manifest_hash=$3 AND s.snapshot_hash=$4 AND m.allocated_total=$5::text::numeric AND m.leaf_count=$6)")
                .bind(c.id).bind(e.root.as_slice()).bind(e.manifestHash.as_slice()).bind(e.eligibilityHash.as_slice()).bind(e.allocatedQuantity.to_string()).bind(u64::try_from(e.leafCount).map_err(|_|ApiError::unavailable())? as i64).fetch_one(&mut **tx).await?;
            if c.chain_state != 1 || !matching || c.status != "ALLOCATION_READY" {
                return incident(tx, c.id, "FINALIZATION_MANIFEST_MISMATCH").await;
            }
            sqlx::query("UPDATE campaigns SET chain_state=2,status='CLAIM_OPEN',version=version+1 WHERE id=$1").bind(c.id).execute(&mut **tx).await?;
        } else if signature == Escrow::RewardClaimed::SIGNATURE_HASH {
            let e = decode::<Escrow::RewardClaimed>(log)?;
            actor = e.recipient;
            if c.chain_state != 2 {
                return incident(tx, c.id, "CLAIM_BEFORE_FINALIZATION").await;
            }
            let index = u64::try_from(e.index).map_err(|_| ApiError::unavailable())?;
            let changed=sqlx::query("UPDATE allocations SET claim_tx_hash=$6,claimed_at=clock_timestamp() WHERE campaign_id=$1 AND claim_index=$2 AND recipient=$3 AND token_id=$4::text::numeric AND quantity=$5::text::numeric AND claimed_at IS NULL")
                .bind(c.id).bind(index as i64).bind(e.recipient.as_slice()).bind(e.tokenId.to_string()).bind(e.quantity.to_string()).bind(&hash).execute(&mut **tx).await?;
            // A smart account may execute through an EntryPoint; recipient binding is enforced by the escrow.
            if changed.rows_affected() != 1 {
                return incident(tx, c.id, "CLAIM_ALLOCATION_MISMATCH").await;
            }
            sqlx::query("UPDATE campaign_rewards SET claimed_amount=claimed_amount+$2::text::numeric WHERE campaign_id=$1").bind(c.id).bind(e.quantity.to_string()).execute(&mut **tx).await?;
            if s.reward.asset_kind == crate::domain::AssetKind::Erc721 {
                sqlx::query("UPDATE reward_nft_inventory SET delivered=true WHERE campaign_id=$1 AND token_id=$2::text::numeric").bind(c.id).bind(e.tokenId.to_string()).execute(&mut **tx).await?;
            }
        } else if signature == Escrow::CampaignCancelled::SIGNATURE_HASH {
            let e = decode::<Escrow::CampaignCancelled>(log)?;
            if c.chain_state != 0 || e.refundRecipient != s.refund_recipient.unwrap_or(c.creator())
            {
                return incident(tx, c.id, "CANCELLATION_MISMATCH").await;
            }
            sqlx::query("UPDATE campaigns SET chain_state=3,status='CANCELLED',version=version+1 WHERE id=$1").bind(c.id).execute(&mut **tx).await?;
            sqlx::query("UPDATE campaign_rewards SET swept_amount=funded_amount-claimed_amount WHERE campaign_id=$1").bind(c.id).execute(&mut **tx).await?;
        } else if signature == Escrow::RemainingSwept::SIGNATURE_HASH {
            let e = decode::<Escrow::RemainingSwept>(log)?;
            let expected:String=sqlx::query_scalar("SELECT (funded_amount-claimed_amount-swept_amount)::text FROM campaign_rewards WHERE campaign_id=$1").bind(c.id).fetch_one(&mut **tx).await?;
            if e.refundRecipient != s.refund_recipient.unwrap_or(c.creator())
                || e.quantity != crate::domain::amount(&expected)?
            {
                return incident(tx, c.id, "REFUND_MISMATCH").await;
            }
            sqlx::query("UPDATE campaign_rewards SET swept_amount=swept_amount+$2::text::numeric WHERE campaign_id=$1").bind(c.id).bind(e.quantity.to_string()).execute(&mut **tx).await?;
            sqlx::query(
                "UPDATE campaigns SET chain_state=4,status='CLOSED',version=version+1 WHERE id=$1",
            )
            .bind(c.id)
            .execute(&mut **tx)
            .await?;
        }
    }
    sqlx::query("UPDATE chain_transactions SET status='CONFIRMED' WHERE chain_id=$1 AND tx_hash=$2 AND expected_from=$3 AND campaign_id=$4 AND kind=$5").bind(state.config.chain_id as i64).bind(&hash).bind(actor.as_slice()).bind(event_campaign).bind(event_kind).execute(&mut **tx).await?;
    Ok(())
}
pub fn signature_kind(signature: B256) -> Option<&'static str> {
    [
        (Factory::CampaignCreated::SIGNATURE_HASH, "CREATE"),
        (Escrow::RewardFunded::SIGNATURE_HASH, "FUND"),
        (Escrow::CampaignActivated::SIGNATURE_HASH, "ACTIVATE"),
        (Escrow::DistributionFinalized::SIGNATURE_HASH, "FINALIZE"),
        (Escrow::RewardClaimed::SIGNATURE_HASH, "CLAIM"),
        (Escrow::CampaignCancelled::SIGNATURE_HASH, "CANCEL"),
        (Escrow::RemainingSwept::SIGNATURE_HASH, "SWEEP"),
    ]
    .into_iter()
    .find(|(hash, _)| *hash == signature)
    .map(|(_, kind)| kind)
}
pub async fn reconcile_transactions(state: &AppState) -> Result<()> {
    state.rpc.verify_chain().await?;
    let safe_head = state
        .rpc
        .head()
        .await?
        .saturating_sub(state.config.confirmations);
    let pending=sqlx::query("SELECT chain_id,tx_hash,campaign_id,kind,expected_from FROM chain_transactions WHERE chain_id=$1 AND status='PENDING' ORDER BY coalesce(last_checked_at,created_at),created_at LIMIT 100")
        .bind(state.config.chain_id as i64).fetch_all(&state.db).await?;
    for row in pending {
        let hash: B256 = B256::from_slice(&row.try_get::<Vec<u8>, _>("tx_hash")?);
        // Rotate even null receipts and transient failures so old pending hints cannot starve.
        sqlx::query("UPDATE chain_transactions SET last_checked_at=clock_timestamp() WHERE chain_id=$1 AND tx_hash=$2 AND status='PENDING'")
            .bind(state.config.chain_id as i64).bind(hash.as_slice()).execute(&state.db).await?;
        let receipt = state.rpc.receipt(hash).await?;
        if receipt.is_null() {
            continue;
        }
        let block = chain::quantity(&receipt["blockNumber"])?;
        if block > safe_head {
            continue;
        }
        let canonical = state.rpc.block(block).await?;
        if canonical["hash"] != receipt["blockHash"] || receipt["transactionHash"] != json!(hash) {
            return Err(ApiError::unavailable());
        }
        let kind: String = row.try_get("kind")?;
        let campaign_id: Uuid = row.try_get("campaign_id")?;
        let c = crate::services::campaigns::load(state, campaign_id).await?;
        let expected: Vec<u8> = row.try_get("expected_from")?;
        let status = if chain::quantity(&receipt["status"])? == 0 {
            let sender: Address = serde_json::from_value(receipt["from"].clone())
                .map_err(|_| ApiError::unavailable())?;
            if sender.as_slice() != expected {
                continue;
            }
            "FAILED"
        } else {
            if c.status == "INTEGRITY_ERROR" {
                continue;
            }
            let events=sqlx::query("SELECT event_type,payload,address FROM chain_events WHERE chain_id=$1 AND tx_hash=$2").bind(c.chain_id).bind(hash.as_slice()).fetch_all(&state.db).await?;
            let mut matches = false;
            for event in events {
                let signature: B256 = event
                    .try_get::<String, _>("event_type")?
                    .parse()
                    .map_err(|_| ApiError::internal())?;
                if signature_kind(signature) != Some(kind.as_str()) {
                    continue;
                }
                let emitter: Vec<u8> = event.try_get("address")?;
                let payload: Value = event.try_get("payload")?;
                if kind == "CREATE" {
                    let e = decode::<Factory::CampaignCreated>(&payload)?;
                    matches = emitter == state.config.factory.as_slice()
                        && e.campaignKey == c.key()
                        && e.creator.as_slice() == expected;
                } else if c.escrow_address.as_ref() == Some(&emitter) {
                    matches = if kind == "CLAIM" {
                        decode::<Escrow::RewardClaimed>(&payload)?
                            .recipient
                            .as_slice()
                            == expected
                    } else {
                        c.creator_wallet == expected
                    };
                }
                if matches {
                    break;
                }
            }
            if !matches {
                continue;
            }
            "CONFIRMED"
        };
        sqlx::query("UPDATE chain_transactions SET status=$3 WHERE chain_id=$1 AND tx_hash=$2 AND status='PENDING'").bind(c.chain_id).bind(hash.as_slice()).bind(status).execute(&state.db).await?;
    }
    Ok(())
}
