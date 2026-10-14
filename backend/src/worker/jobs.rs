use crate::{
    crypto,
    error::{ApiError, Result},
    mutations::lock_campaign,
    services::{allocations, campaigns},
    state::AppState,
};
use alloy_primitives::{Address, B256};
use serde_json::{Value, json};
use sqlx::{Postgres, Row};
use uuid::Uuid;

pub async fn tick(state: &AppState, worker: Uuid) -> Result<()> {
    let job=sqlx::query("UPDATE jobs SET state='RUNNING',attempts=attempts+1,worker_id=$1,locked_until=clock_timestamp()+interval '5 minutes' WHERE id=(SELECT id FROM jobs WHERE attempts<8 AND available_at<=clock_timestamp() AND (state='PENDING' OR (state='RUNNING' AND locked_until<clock_timestamp())) ORDER BY available_at FOR UPDATE SKIP LOCKED LIMIT 1) RETURNING id,campaign_id,attempts")
        .bind(worker).fetch_optional(&state.db).await?;
    let Some(row) = job else { return Ok(()) };
    let job_id: Uuid = row.try_get("id")?;
    let campaign_id: Uuid = row.try_get("campaign_id")?;
    let attempts: i32 = row.try_get("attempts")?;
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(240),
        build(state, campaign_id, job_id, worker),
    )
    .await
    .unwrap_or_else(|_| Err(ApiError::unavailable()));
    if let Err(error) = result {
        let delay = (2i32.pow(attempts as u32)).min(600);
        sqlx::query("UPDATE jobs SET state=$3,last_error=$4,available_at=clock_timestamp()+make_interval(secs=>$5),worker_id=NULL,locked_until=NULL WHERE id=$1 AND worker_id=$2 AND state='RUNNING'")
            .bind(job_id).bind(worker).bind(if attempts>=8{"FAILED"}else{"PENDING"}).bind(error.code).bind(delay as f64).execute(&state.db).await?;
        return Err(error);
    }
    Ok(())
}
async fn build(state: &AppState, id: Uuid, job: Uuid, worker: Uuid) -> Result<()> {
    let c = campaigns::load(state, id).await?;
    if c.status != "ELIGIBILITY_LOCKED" || chrono::Utc::now() >= c.review_deadline {
        return Err(ApiError::conflict(
            "ALLOCATION_EXPIRED",
            "The allocation is no longer available for finalization.",
        ));
    }
    let snapshot = sqlx::query(
        "SELECT id,snapshot_hash,artifact FROM eligibility_snapshots WHERE campaign_id=$1",
    )
    .bind(id)
    .fetch_one(&state.db)
    .await?;
    let snapshot_id: Uuid = snapshot.try_get("id")?;
    let snapshot_hash = B256::from_slice(&snapshot.try_get::<Vec<u8>, _>("snapshot_hash")?);
    let snapshot_artifact: Value = snapshot.try_get("artifact")?;
    let rows = sqlx::query(
        "SELECT entry_id,payout_wallet FROM snapshot_entries WHERE snapshot_id=$1 ORDER BY ordinal",
    )
    .bind(snapshot_id)
    .fetch_all(&state.db)
    .await?;
    let entries: Vec<(Uuid, Address)> = rows
        .iter()
        .map(|r| {
            Ok((
                r.try_get("entry_id")?,
                Address::from_slice(&r.try_get::<Vec<u8>, _>("payout_wallet")?),
            ))
        })
        .collect::<Result<_>>()?;
    let raffle =
        sqlx::query("SELECT encrypted_seed,seed_commitment FROM raffles WHERE campaign_id=$1")
            .bind(id)
            .fetch_optional(&state.db)
            .await?;
    let seed = if let Some(row) = raffle {
        let seed = crypto::decrypt_seed(
            &state.config.seed_key,
            c.key(),
            &row.try_get::<Vec<u8>, _>("encrypted_seed")?,
        )?;
        let commitment = crypto::seed_commitment(c.key(), B256::new(*seed));
        if row.try_get::<Vec<u8>, _>("seed_commitment")? != commitment.as_slice() {
            return Err(ApiError::internal());
        }
        Some(B256::new(*seed))
    } else {
        None
    };
    let cc = c.clone();
    let ee = entries.clone();
    let result =
        tokio::task::spawn_blocking(move || allocations::build(&cc, snapshot_hash, &ee, seed))
            .await
            .map_err(|_| ApiError::internal())??;
    let spec = c.spec()?;
    let artifact = json!({"schema_version":1,"chain_id":c.chain_id.to_string(),"escrow":c.escrow()?,"campaign_key":c.key(),"config_hash":crypto::hash_string(c.config_hash.as_deref()),"eligibility_hash":snapshot_hash,"asset_kind":spec.reward.asset_kind,"reward_token":spec.reward.token_address,"distribution_mode":spec.distribution.mode,"algorithm_version":"allocation-v1","root":result.root,"leaf_count":result.allocations.len(),"allocated_quantity":result.total.to_string(),"allocations":result.allocations});
    let bytes = crypto::canonical(&artifact)?;
    let hash = alloy_primitives::keccak256(&bytes);
    let object_key = format!("manifests/{hash}.json");
    state
        .storage
        .put_immutable(&object_key, bytes.into())
        .await?;
    state
        .storage
        .put_immutable(
            &format!("snapshots/{snapshot_hash}.json"),
            crypto::canonical(&snapshot_artifact)?.into(),
        )
        .await?;
    if let Some(transcript) = &result.transcript {
        state
            .storage
            .put_immutable(
                &format!("raffles/{id}.json"),
                crypto::canonical(transcript)?.into(),
            )
            .await?;
    }
    let mut tx = state.db.begin().await?;
    let locked = lock_campaign(&mut tx, id).await?;
    let valid:bool=sqlx::query_scalar("SELECT worker_id=$2 AND state='RUNNING' AND locked_until>clock_timestamp() FROM jobs WHERE id=$1 FOR UPDATE").bind(job).bind(worker).fetch_one(&mut *tx).await?;
    if !valid
        || locked.status != "ELIGIBILITY_LOCKED"
        || crate::mutations::db_now(&mut tx).await? >= locked.review_deadline
    {
        return Err(ApiError::conflict(
            "JOB_LEASE_EXPIRED",
            "The job lost its lease or the campaign expired.",
        ));
    }
    let manifest = Uuid::new_v4();
    sqlx::query("INSERT INTO allocation_manifests(id,campaign_id,snapshot_id,root,manifest_hash,object_key,artifact,leaf_count,allocated_total) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9::text::numeric)")
        .bind(manifest).bind(id).bind(snapshot_id).bind(result.root.as_slice()).bind(hash.as_slice()).bind(&object_key).bind(&artifact).bind(result.allocations.len() as i32).bind(result.total.to_string()).execute(&mut *tx).await?;
    let entry_by_wallet: std::collections::HashMap<_, _> =
        entries.iter().map(|(id, w)| (*w, *id)).collect();
    for (chunk_index, chunk) in result.allocations.chunks(500).enumerate() {
        let mut q = sqlx::QueryBuilder::<Postgres>::new(
            "INSERT INTO allocations(id,campaign_id,manifest_id,claim_index,entry_id,recipient,token_id,quantity,leaf_hash,proof_json) ",
        );
        q.push_values(chunk.iter().enumerate(), |mut b, (i, a)| {
            b.push_bind(Uuid::new_v4())
                .push_bind(id)
                .push_bind(manifest)
                .push_bind((chunk_index * 500 + i) as i32)
                .push_bind(entry_by_wallet[&a.recipient])
                .push_bind(a.recipient.as_slice().to_vec());
            b.push_bind(a.token_id.clone())
                .push_unseparated("::text::numeric");
            b.push_bind(a.quantity.clone())
                .push_unseparated("::text::numeric");
            b.push_bind(result.leaves[chunk_index * 500 + i].to_vec())
                .push_bind(json!(a.proof));
        });
        q.build().execute(&mut *tx).await?;
    }
    if let Some(transcript) = &result.transcript {
        sqlx::query("UPDATE raffles SET revealed_seed=$2,transcript=$3,executed_at=clock_timestamp() WHERE campaign_id=$1 AND revealed_seed IS NULL")
            .bind(id).bind(seed.ok_or_else(ApiError::internal)?.as_slice()).bind(transcript).execute(&mut *tx).await?;
    }
    sqlx::query("UPDATE campaigns SET status='ALLOCATION_READY',version=version+1,updated_at=clock_timestamp() WHERE id=$1").bind(id).execute(&mut *tx).await?;
    sqlx::query("UPDATE jobs SET state='SUCCEEDED',worker_id=NULL,locked_until=NULL,last_error=NULL WHERE id=$1 AND worker_id=$2").bind(job).bind(worker).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO audit_logs(id,campaign_id,action,metadata) VALUES($1,$2,'ALLOCATION_PUBLISHED',$3)").bind(Uuid::new_v4()).bind(id).bind(json!({"root":result.root,"manifest_hash":hash})).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
}

pub async fn maintenance(state: &AppState) -> Result<()> {
    sqlx::query("UPDATE jobs SET state='FAILED',last_error='RETRY_LIMIT_EXCEEDED',worker_id=NULL,locked_until=NULL WHERE state='RUNNING' AND attempts>=8 AND locked_until<clock_timestamp()")
        .execute(&state.db).await?;
    // Receipt reconciliation never projects rewards; indexed contract events remain authoritative.
    if let Err(error) = super::indexer::reconcile_transactions(state).await {
        tracing::warn!(
            code = error.code,
            "Transaction hint reconciliation is temporarily unavailable"
        );
    }
    sqlx::query("DELETE FROM auth_nonces WHERE expires_at<clock_timestamp()-interval '1 day'")
        .execute(&state.db)
        .await?;
    sqlx::query("DELETE FROM sessions WHERE expires_at<clock_timestamp()-interval '1 day' OR revoked_at<clock_timestamp()-interval '1 day'").execute(&state.db).await?;
    sqlx::query("DELETE FROM idempotency_keys WHERE expires_at<clock_timestamp()")
        .execute(&state.db)
        .await?;
    let pending=sqlx::query("SELECT id,staging_key FROM uploads WHERE expires_at<clock_timestamp()-interval '1 hour' AND status='PENDING' ORDER BY expires_at LIMIT 100").fetch_all(&state.db).await?;
    for row in pending {
        state
            .storage
            .delete(&row.try_get::<String, _>("staging_key")?)
            .await?;
        sqlx::query("UPDATE uploads SET status='DELETED' WHERE id=$1 AND status='PENDING'")
            .bind(row.try_get::<Uuid, _>("id")?)
            .execute(&state.db)
            .await?;
    }
    let staged=sqlx::query("SELECT id,staging_key FROM uploads WHERE status='COMPLETE' AND expires_at<clock_timestamp() AND staging_deleted_at IS NULL ORDER BY expires_at LIMIT 100").fetch_all(&state.db).await?;
    for row in staged {
        state
            .storage
            .delete(&row.try_get::<String, _>("staging_key")?)
            .await?;
        sqlx::query("UPDATE uploads SET staging_deleted_at=clock_timestamp() WHERE id=$1")
            .bind(row.try_get::<Uuid, _>("id")?)
            .execute(&state.db)
            .await?;
    }
    // Eligibility and claim rights are checked against timestamps in each request; no cron correctness dependency.
    Ok(())
}
