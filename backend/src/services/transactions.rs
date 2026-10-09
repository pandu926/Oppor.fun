use crate::{
    auth::Auth,
    chain::{self, Claim, Escrow, Factory, Token},
    crypto,
    domain::{AssetKind, Campaign},
    error::{ApiError, Result},
    http::ApiJson,
    mutations::Mutation,
    state::AppState,
};
use alloy_primitives::{B256, U256};
use alloy_sol_types::SolCall;
use axum::{
    Json,
    extract::{Path, State},
    http::HeaderMap,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sqlx::Row;
use uuid::Uuid;

async fn checked(state: &AppState, auth: &Auth, id: Uuid, creator: bool) -> Result<Campaign> {
    let c = super::campaigns::load(state, id).await?;
    if creator {
        c.ensure_creator(auth.user_id)?;
    }
    if c.status == "INTEGRITY_ERROR" {
        return Err(ApiError::conflict(
            "INTEGRITY_ERROR",
            "The campaign is under integrity review.",
        ));
    }
    state.ensure_indexer().await?;
    if c.escrow_address.is_some() {
        state.rpc.verify_escrow(&c, &state.config).await?;
    } else {
        state.rpc.verify_factory(&state.config).await?;
    }
    Ok(c)
}
async fn current_state(state: &AppState, c: &Campaign) -> Result<u8> {
    u8::try_from(
        state
            .rpc
            .uint(c.escrow()?, Escrow::stateCall {}.abi_encode())
            .await?,
    )
    .map_err(|_| ApiError::unavailable())
}
async fn require_state(state: &AppState, c: &Campaign, expected: u8) -> Result<()> {
    if current_state(state, c).await? != expected {
        return Err(ApiError::conflict(
            "INVALID_CHAIN_STATE",
            "Refresh the campaign's onchain state.",
        ));
    }
    Ok(())
}
pub async fn create(
    State(state): State<AppState>,
    auth: Auth,
    Path(id): Path<Uuid>,
) -> Result<Json<Value>> {
    let c = checked(&state, &auth, id, true).await?;
    if c.status != "CONFIG_LOCKED" || c.escrow_address.is_some() {
        return Err(ApiError::conflict(
            "INVALID_STATE",
            "Campaign creation is only available after configuration is locked.",
        ));
    }
    let commitment: Option<Vec<u8>> =
        sqlx::query_scalar("SELECT seed_commitment FROM raffles WHERE campaign_id=$1")
            .bind(id)
            .fetch_optional(&state.db)
            .await?;
    let config = chain::configuration(
        &c,
        commitment.map(|b| B256::from_slice(&b)).unwrap_or_default(),
    )?;
    let mut prepared = state
        .rpc
        .prepare(
            &state.config,
            state.config.factory,
            auth.wallet,
            Factory::createCampaignCall { config }.abi_encode(),
            "CREATE",
        )
        .await?;
    prepared["config_hash"] = json!(crypto::hash_string(c.config_hash.as_deref()));
    Ok(Json(prepared))
}
pub async fn fund(
    State(state): State<AppState>,
    auth: Auth,
    Path(id): Path<Uuid>,
) -> Result<Json<Value>> {
    let c = checked(&state, &auth, id, true).await?;
    require_state(&state, &c, 0).await?;
    let s = c.spec()?;
    let escrow = c.escrow()?;
    if Utc::now() >= c.start_at {
        return Err(ApiError::conflict(
            "FUNDING_CLOSED",
            "Funding closes when the campaign starts.",
        ));
    }
    let funded = state
        .rpc
        .uint(escrow, Escrow::fundedQuantityCall {}.abi_encode())
        .await?;
    let target = crate::domain::positive(&s.reward.amount_base_units)?;
    let remaining = target
        .checked_sub(funded)
        .ok_or_else(ApiError::unavailable)?;
    if remaining == U256::ZERO {
        return Err(ApiError::conflict(
            "ALREADY_FUNDED",
            "The campaign reward is fully funded.",
        ));
    }
    let mut approvals = Vec::new();
    let mut missing_approval = false;
    let data = match s.reward.asset_kind {
        AssetKind::Erc20 => {
            let allowance = state
                .rpc
                .uint(
                    s.reward.token_address,
                    Token::allowanceCall {
                        owner: auth.wallet,
                        spender: escrow,
                    }
                    .abi_encode(),
                )
                .await?;
            if allowance < remaining {
                missing_approval = true;
                if allowance > U256::ZERO {
                    approvals.push(
                        state
                            .rpc
                            .prepare(
                                &state.config,
                                s.reward.token_address,
                                auth.wallet,
                                Token::approveCall {
                                    spender: escrow,
                                    amount: U256::ZERO,
                                }
                                .abi_encode(),
                                "RESET_APPROVAL",
                            )
                            .await?,
                    );
                }
                // Simulation of approve does not depend on the preceding fund operation.
                approvals.push(
                    state
                        .rpc
                        .prepare(
                            &state.config,
                            s.reward.token_address,
                            auth.wallet,
                            Token::approveCall {
                                spender: escrow,
                                amount: remaining,
                            }
                            .abi_encode(),
                            "APPROVE",
                        )
                        .await?,
                );
            }
            Escrow::fundERC20Call { amount: remaining }.abi_encode()
        }
        AssetKind::Erc721 => {
            let all = state
                .rpc
                .uint(
                    s.reward.token_address,
                    Token::isApprovedForAllCall {
                        owner: auth.wallet,
                        operator: escrow,
                    }
                    .abi_encode(),
                )
                .await?
                != U256::ZERO;
            let mut ids = Vec::new();
            for token in &s.reward.nft_inventory {
                let token_id = crate::domain::amount(token)?;
                if state
                    .rpc
                    .uint(
                        escrow,
                        Escrow::isDepositedNFTCall { tokenId: token_id }.abi_encode(),
                    )
                    .await?
                    == U256::ZERO
                {
                    ids.push(token_id);
                    if !all {
                        let approved = state
                            .rpc
                            .uint(
                                s.reward.token_address,
                                Token::getApprovedCall { tokenId: token_id }.abi_encode(),
                            )
                            .await?;
                        if approved != U256::from_be_slice(escrow.as_slice()) {
                            missing_approval = true;
                            approvals.push(
                                state
                                    .rpc
                                    .prepare(
                                        &state.config,
                                        s.reward.token_address,
                                        auth.wallet,
                                        Token::approveCall {
                                            spender: escrow,
                                            amount: token_id,
                                        }
                                        .abi_encode(),
                                        "APPROVE_NFT",
                                    )
                                    .await?,
                            );
                        }
                    }
                }
            }
            if ids.is_empty() {
                return Err(ApiError::conflict(
                    "ALREADY_FUNDED",
                    "The NFT inventory is already deposited.",
                ));
            }
            Escrow::fundERC721Call { tokenIds: ids }.abi_encode()
        }
        AssetKind::Erc1155 => {
            if state
                .rpc
                .uint(
                    s.reward.token_address,
                    Token::isApprovedForAllCall {
                        owner: auth.wallet,
                        operator: escrow,
                    }
                    .abi_encode(),
                )
                .await?
                == U256::ZERO
            {
                missing_approval = true;
                approvals.push(
                    state
                        .rpc
                        .prepare(
                            &state.config,
                            s.reward.token_address,
                            auth.wallet,
                            Token::setApprovalForAllCall {
                                operator: escrow,
                                approved: true,
                            }
                            .abi_encode(),
                            "APPROVE_COLLECTION",
                        )
                        .await?,
                );
            }
            Escrow::fundERC1155Call { amount: remaining }.abi_encode()
        }
    };
    let fund = if missing_approval {
        json!({"chain_id":c.chain_id.to_string(),"to":escrow,"data":format!("0x{}",hex::encode(data)),"value":"0","expected_sender":auth.wallet,"intent":"FUND","simulation":"REQUIRES_CONFIRMED_APPROVAL","estimated_gas":null,"expires_at":(Utc::now()+chrono::Duration::seconds(30)).to_rfc3339()})
    } else {
        state
            .rpc
            .prepare(&state.config, escrow, auth.wallet, data, "FUND")
            .await?
    };
    Ok(Json(
        json!({"approvals":approvals,"fund":fund,"remaining_quantity":remaining.to_string(),"instructions":"Confirm approvals first, then prepare funding again to simulate against the updated allowance."}),
    ))
}
pub async fn activate(
    State(state): State<AppState>,
    auth: Auth,
    Path(id): Path<Uuid>,
) -> Result<Json<Value>> {
    let c = checked(&state, &auth, id, true).await?;
    require_state(&state, &c, 0).await?;
    let s = c.spec()?;
    let escrow = c.escrow()?;
    let funded = state
        .rpc
        .uint(escrow, Escrow::fundedQuantityCall {}.abi_encode())
        .await?;
    if funded != crate::domain::positive(&s.reward.amount_base_units)? {
        return Err(ApiError::conflict(
            "FUNDING_INCOMPLETE",
            "Deposit the full reward before activating.",
        ));
    }
    if s.reward.asset_kind == AssetKind::Erc721 {
        for id in &s.reward.nft_inventory {
            if state
                .rpc
                .uint(
                    escrow,
                    Escrow::isDepositedNFTCall {
                        tokenId: crate::domain::amount(id)?,
                    }
                    .abi_encode(),
                )
                .await?
                == U256::ZERO
            {
                return Err(ApiError::conflict(
                    "NFT_INVENTORY_MISMATCH",
                    "The deposited NFT inventory does not match the locked rules.",
                ));
            }
        }
    }
    state
        .rpc
        .prepare(
            &state.config,
            escrow,
            auth.wallet,
            Escrow::activateCall {}.abi_encode(),
            "ACTIVATE",
        )
        .await
        .map(Json)
}
pub async fn finalize(
    State(state): State<AppState>,
    auth: Auth,
    Path(id): Path<Uuid>,
) -> Result<Json<Value>> {
    let c = checked(&state, &auth, id, true).await?;
    require_state(&state, &c, 1).await?;
    if c.status != "ALLOCATION_READY" {
        return Err(ApiError::conflict(
            "ALLOCATION_NOT_READY",
            "The allocation manifest is not ready.",
        ));
    }
    let row=sqlx::query("SELECT m.root,m.manifest_hash,m.allocated_total::text AS total,m.leaf_count,m.object_key,s.snapshot_hash FROM allocation_manifests m JOIN eligibility_snapshots s ON s.id=m.snapshot_id WHERE m.campaign_id=$1").bind(id).fetch_one(&state.db).await?;
    let root = B256::from_slice(&row.try_get::<Vec<u8>, _>("root")?);
    let hash = B256::from_slice(&row.try_get::<Vec<u8>, _>("manifest_hash")?);
    // Check artifact availability before asking the creator to make the root permanent.
    use object_store::ObjectStoreExt;
    state
        .storage
        .client
        .head(&object_store::path::Path::from(
            row.try_get::<String, _>("object_key")?,
        ))
        .await
        .map_err(|_| ApiError::unavailable())?;
    let call = Escrow::finalizeCall {
        root,
        allocationManifestHash: hash,
        eligibilitySnapshotHash: B256::from_slice(&row.try_get::<Vec<u8>, _>("snapshot_hash")?),
        declaredAllocatedQuantity: crate::domain::amount(&row.try_get::<String, _>("total")?)?,
        declaredLeafCount: U256::from(row.try_get::<i32, _>("leaf_count")?),
    };
    let mut result = state
        .rpc
        .prepare(
            &state.config,
            c.escrow()?,
            auth.wallet,
            call.abi_encode(),
            "FINALIZE",
        )
        .await?;
    result["distribution_root"] = json!(root);
    result["manifest_hash"] = json!(hash);
    Ok(Json(result))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClaimInput {
    pub claim_index: u32,
}
pub async fn claim(
    State(state): State<AppState>,
    auth: Auth,
    Path(id): Path<Uuid>,
    ApiJson(input): ApiJson<ClaimInput>,
) -> Result<Json<Value>> {
    let c = checked(&state, &auth, id, false).await?;
    if !c.listed {
        return Err(ApiError::missing());
    }
    require_state(&state, &c, 2).await?;
    let row=sqlx::query("SELECT a.recipient,a.token_id::text AS token_id,a.quantity::text AS quantity,a.proof_json,a.claimed_at,m.root FROM allocations a JOIN allocation_manifests m ON m.id=a.manifest_id WHERE a.campaign_id=$1 AND a.claim_index=$2")
        .bind(id).bind(input.claim_index as i64).fetch_one(&state.db).await?;
    let recipient: Vec<u8> = row.try_get("recipient")?;
    if recipient != auth.wallet.as_slice() {
        return Err(ApiError::forbidden());
    }
    if row
        .try_get::<Option<chrono::DateTime<Utc>>, _>("claimed_at")?
        .is_some()
    {
        return Err(ApiError::conflict(
            "ALREADY_CLAIMED",
            "This allocation has already been claimed.",
        ));
    }
    let root: Vec<u8> = row.try_get("root")?;
    if state
        .rpc
        .call(c.escrow()?, Escrow::distributionRootCall {}.abi_encode())
        .await?
        != root
    {
        return Err(ApiError::conflict(
            "ROOT_MISMATCH",
            "The onchain root does not match the allocation.",
        ));
    }
    let proof: Vec<B256> =
        serde_json::from_value(row.try_get("proof_json")?).map_err(|_| ApiError::internal())?;
    let allocation = Claim {
        index: U256::from(input.claim_index),
        recipient: auth.wallet,
        tokenId: crate::domain::amount(&row.try_get::<String, _>("token_id")?)?,
        quantity: crate::domain::positive(&row.try_get::<String, _>("quantity")?)?,
    };
    state
        .rpc
        .prepare(
            &state.config,
            c.escrow()?,
            auth.wallet,
            Escrow::claimCall { allocation, proof }.abi_encode(),
            "CLAIM",
        )
        .await
        .map(Json)
}
pub async fn cancel(
    State(state): State<AppState>,
    auth: Auth,
    Path(id): Path<Uuid>,
) -> Result<Json<Value>> {
    let c = checked(&state, &auth, id, true).await?;
    require_state(&state, &c, 0).await?;
    state
        .rpc
        .prepare(
            &state.config,
            c.escrow()?,
            auth.wallet,
            Escrow::cancelCall {}.abi_encode(),
            "CANCEL",
        )
        .await
        .map(Json)
}
pub async fn sweep(
    State(state): State<AppState>,
    auth: Auth,
    Path(id): Path<Uuid>,
) -> Result<Json<Value>> {
    let c = checked(&state, &auth, id, true).await?;
    state
        .rpc
        .prepare(
            &state.config,
            c.escrow()?,
            auth.wallet,
            Escrow::sweepRemainingCall {}.abi_encode(),
            "SWEEP",
        )
        .await
        .map(Json)
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TrackInput {
    pub tx_hash: B256,
    pub kind: String,
}
pub async fn track(
    State(state): State<AppState>,
    auth: Auth,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
    ApiJson(input): ApiJson<TrackInput>,
) -> Result<Json<Value>> {
    if input.tx_hash.is_zero()
        || !matches!(
            input.kind.as_str(),
            "CREATE" | "FUND" | "ACTIVATE" | "FINALIZE" | "CLAIM" | "CANCEL" | "SWEEP"
        )
    {
        return Err(ApiError::invalid("Invalid transaction hash or intent."));
    }
    let c = super::campaigns::load(&state, id).await?;
    if input.kind == "CLAIM" {
        let own: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM allocations WHERE campaign_id=$1 AND recipient=$2)",
        )
        .bind(id)
        .bind(auth.wallet.as_slice())
        .fetch_one(&state.db)
        .await?;
        if !own {
            return Err(ApiError::forbidden());
        }
    } else {
        c.ensure_creator(auth.user_id)?;
    }
    let mut op = Mutation::begin(
        &state.db,
        auth.user_id,
        format!("track:{id}"),
        &headers,
        &serde_json::to_value(&input).map_err(|_| ApiError::internal())?,
    )
    .await?;
    if let Some(v) = op.cached() {
        return Ok(Json(v));
    }
    sqlx::query("INSERT INTO chain_transactions(chain_id,tx_hash,campaign_id,kind,expected_from) VALUES($1,$2,$3,$4,$5)").bind(c.chain_id).bind(input.tx_hash.as_slice()).bind(id).bind(&input.kind).bind(auth.wallet.as_slice()).execute(&mut **op.tx()).await?;
    op.finish(json!({"tx_hash":input.tx_hash,"status":"PENDING","note":"The transaction hash is a tracking hint. Only verified chain events change campaign and claim state."})).await.map(Json)
}
