use crate::{
    config::Config,
    domain::Campaign,
    error::{ApiError, Result},
};
use alloy_primitives::{Address, B256, Bytes, U256, keccak256};
use alloy_sol_types::{SolCall, SolValue, sol};
use serde_json::{Value, json};
use std::{
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::Duration,
};

sol! {
    struct CampaignConfig {
        bytes32 campaignKey; address creator; address refundRecipient; uint8 assetKind;
        address rewardToken; uint256 rewardTokenId; uint256 targetQuantity;
        uint64 startsAt; uint64 cutoffAt; uint64 reviewDeadline; uint64 claimDeadline;
        uint8 mode; uint32 participantCapacity; uint32 winnerCount; bytes32 rulesHash; bytes32 raffleSeedCommitment;
    }
    struct Claim { uint256 index; address recipient; uint256 tokenId; uint256 quantity; }
    interface Factory {
        function createCampaign(CampaignConfig config) external returns(address escrow);
        event CampaignCreated(address indexed creator, bytes32 indexed campaignKey, address indexed escrow, bytes32 configHash);
    }
    interface Escrow {
        function configHash() external view returns(bytes32);
        function state() external view returns(uint8);
        function distributionRoot() external view returns(bytes32);
        function fundedQuantity() external view returns(uint256);
        function allocatedQuantity() external view returns(uint256);
        function claimedQuantity() external view returns(uint256);
        function isDepositedNFT(uint256 tokenId) external view returns(bool);
        function fundERC20(uint256 amount) external;
        function fundERC721(uint256[] tokenIds) external;
        function fundERC1155(uint256 amount) external;
        function activate() external;
        function finalize(bytes32 root,bytes32 allocationManifestHash,bytes32 eligibilitySnapshotHash,uint256 declaredAllocatedQuantity,uint256 declaredLeafCount) external;
        function claim(Claim allocation,bytes32[] proof) external;
        function cancel() external;
        function sweepRemaining() external;
        event RewardFunded(uint8 assetKind,address indexed token,uint256 tokenId,uint256 quantity);
        event CampaignActivated(bytes32 indexed configHash);
        event DistributionFinalized(bytes32 indexed root,bytes32 manifestHash,bytes32 eligibilityHash,uint256 allocatedQuantity,uint256 leafCount);
        event RewardClaimed(uint256 indexed index,address indexed recipient,uint256 tokenId,uint256 quantity);
        event CampaignCancelled(address indexed refundRecipient);
        event RemainingSwept(address indexed refundRecipient,uint256 quantity);
    }
    interface Token {
        function approve(address spender,uint256 amount) external returns(bool);
        function allowance(address owner,address spender) external view returns(uint256);
        function getApproved(uint256 tokenId) external view returns(address);
        function isApprovedForAll(address owner,address operator) external view returns(bool);
        function setApprovalForAll(address operator,bool approved) external;
        function supportsInterface(bytes4 interfaceId) external view returns(bool);
    }
    interface ContractWallet {
        function isValidSignature(bytes32 hash,bytes signature) external view returns(bytes4);
    }
}

pub fn configuration(c: &Campaign, seed: B256) -> Result<CampaignConfig> {
    let s = c.spec()?;
    Ok(CampaignConfig {
        campaignKey: c.key(),
        creator: c.creator(),
        refundRecipient: s.refund_recipient.unwrap_or(c.creator()),
        assetKind: s.reward.asset_kind.id(),
        rewardToken: s.reward.token_address,
        rewardTokenId: crate::domain::amount(&s.reward.token_id)?,
        targetQuantity: crate::domain::positive(&s.reward.amount_base_units)?,
        startsAt: s.start_at.timestamp() as u64,
        cutoffAt: s.cutoff_at.timestamp() as u64,
        reviewDeadline: s.review_deadline.timestamp() as u64,
        claimDeadline: s.claim_deadline.timestamp() as u64,
        mode: s.distribution.mode.id(),
        participantCapacity: s.distribution.capacity,
        winnerCount: s.distribution.winner_count,
        rulesHash: crate::crypto::parse_hash(
            c.rules_hash.as_deref().ok_or_else(ApiError::internal)?,
        )?,
        raffleSeedCommitment: seed,
    })
}
pub fn configuration_hash(config: &CampaignConfig) -> B256 {
    keccak256(config.abi_encode())
}

#[derive(Clone)]
pub struct Rpc {
    client: reqwest::Client,
    url: Arc<str>,
    counter: Arc<AtomicU64>,
    chain_id: u64,
}
impl Rpc {
    pub fn new(config: &Config) -> Result<Self> {
        Ok(Self {
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(8))
                .connect_timeout(Duration::from_secs(3))
                .redirect(reqwest::redirect::Policy::none())
                .build()
                .map_err(|_| ApiError::internal())?,
            url: config.rpc_url.clone().into(),
            counter: Arc::new(AtomicU64::new(1)),
            chain_id: config.chain_id,
        })
    }
    pub async fn request(&self, method: &str, params: Value) -> Result<Value> {
        let id = self.counter.fetch_add(1, Ordering::Relaxed);
        let mut r = self
            .client
            .post(self.url.as_ref())
            .json(&json!({"jsonrpc":"2.0","id":id,"method":method,"params":params}))
            .send()
            .await
            .map_err(|_| ApiError::unavailable())?;
        if !r.status().is_success() {
            return Err(ApiError::unavailable());
        }
        let mut data = Vec::new();
        while let Some(chunk) = r.chunk().await.map_err(|_| ApiError::unavailable())? {
            if data.len() + chunk.len() > 8 * 1024 * 1024 {
                return Err(ApiError::unavailable());
            }
            data.extend(chunk);
        }
        let value: Value = serde_json::from_slice(&data).map_err(|_| ApiError::unavailable())?;
        if value.get("id") != Some(&json!(id)) || value.get("jsonrpc") != Some(&json!("2.0")) {
            return Err(ApiError::unavailable());
        }
        if value.get("error").is_some() {
            tracing::warn!(rpc_method = method, "RPC rejected an operation");
            return Err(ApiError::conflict(
                "CHAIN_OPERATION_REJECTED",
                "The chain rejected this operation. Refresh state and retry.",
            ));
        }
        value
            .get("result")
            .cloned()
            .ok_or_else(ApiError::unavailable)
    }
    pub async fn verify_chain(&self) -> Result<()> {
        let v = self.request("eth_chainId", json!([])).await?;
        if quantity(&v)? != self.chain_id {
            return Err(ApiError::unavailable());
        }
        Ok(())
    }
    pub async fn code(&self, address: Address) -> Result<Vec<u8>> {
        let v = self
            .request("eth_getCode", json!([address, "latest"]))
            .await?;
        decode_hex(&v)
    }
    pub async fn call(&self, to: Address, data: Vec<u8>) -> Result<Vec<u8>> {
        let v = self
            .request(
                "eth_call",
                json!([{"to":to,"data":format!("0x{}",hex::encode(data))},"latest"]),
            )
            .await?;
        decode_hex(&v)
    }
    pub async fn uint(&self, to: Address, data: Vec<u8>) -> Result<U256> {
        let b = self.call(to, data).await?;
        if b.len() != 32 {
            return Err(ApiError::unavailable());
        }
        Ok(U256::from_be_slice(&b))
    }
    pub async fn head(&self) -> Result<u64> {
        quantity(&self.request("eth_blockNumber", json!([])).await?)
    }
    pub async fn block(&self, number: u64) -> Result<Value> {
        self.request(
            "eth_getBlockByNumber",
            json!([format!("0x{number:x}"), false]),
        )
        .await
    }
    pub async fn receipt(&self, hash: B256) -> Result<Value> {
        self.request("eth_getTransactionReceipt", json!([hash]))
            .await
    }
    pub async fn logs(&self, from: u64, to: u64, addresses: &[Address]) -> Result<Vec<Value>> {
        let result=self.request("eth_getLogs",json!([{"fromBlock":format!("0x{from:x}"),"toBlock":format!("0x{to:x}"),"address":addresses}])).await?;
        let logs = result.as_array().ok_or_else(ApiError::unavailable)?.clone();
        if logs.len() > 50_000 {
            return Err(ApiError::unavailable());
        }
        Ok(logs)
    }
    pub async fn verify_factory(&self, config: &Config) -> Result<()> {
        self.verify_chain().await?;
        if keccak256(self.code(config.factory).await?) != config.factory_code_hash {
            return Err(ApiError::unavailable());
        }
        Ok(())
    }
    pub async fn verify_escrow(&self, c: &Campaign, config: &Config) -> Result<()> {
        self.verify_chain().await?;
        let address = c.escrow()?;
        if keccak256(self.code(address).await?) != config.escrow_code_hash {
            return Err(ApiError::unavailable());
        }
        let hash = self
            .call(address, Escrow::configHashCall {}.abi_encode())
            .await?;
        if Some(hash.as_slice()) != c.config_hash.as_deref() {
            return Err(ApiError::conflict(
                "CONFIG_MISMATCH",
                "The escrow configuration does not match this campaign.",
            ));
        }
        Ok(())
    }
    pub async fn prepare(
        &self,
        config: &Config,
        to: Address,
        from: Address,
        data: Vec<u8>,
        intent: &str,
    ) -> Result<Value> {
        self.verify_chain().await?;
        let transaction =
            json!({"from":from,"to":to,"data":format!("0x{}",hex::encode(&data)),"value":"0x0"});
        self.request("eth_call", json!([transaction, "latest"]))
            .await?;
        let gas = self
            .request("eth_estimateGas", json!([transaction]))
            .await?;
        let estimate = quantity(&gas)?;
        Ok(
            json!({"chain_id":config.chain_id.to_string(),"to":to,"data":format!("0x{}",hex::encode(data)),"value":"0","expected_sender":from,"intent":intent,"estimated_gas":estimate.to_string(),"expires_at":(chrono::Utc::now()+chrono::Duration::seconds(30)).to_rfc3339()}),
        )
    }
    pub async fn valid_wallet_signature(
        &self,
        address: Address,
        message: &str,
        signature: &str,
    ) -> Result<bool> {
        self.verify_chain().await?;
        let sig =
            hex::decode(signature.strip_prefix("0x").ok_or_else(|| {
                ApiError::bad("INVALID_SIGNATURE", "Invalid signature encoding.")
            })?)
            .map_err(|_| ApiError::bad("INVALID_SIGNATURE", "Invalid signature encoding."))?;
        if sig.len() > 4096 {
            return Ok(false);
        }
        let parsed: siwe::Message = message
            .parse()
            .map_err(|_| ApiError::bad("INVALID_MESSAGE", "Invalid sign-in message."))?;
        if self.code(address).await?.is_empty() {
            let Ok(sig): std::result::Result<[u8; 65], _> = sig.try_into() else {
                return Ok(false);
            };
            return Ok(parsed.verify_eip191(&sig).is_ok());
        }
        let hash = B256::new(
            parsed
                .eip191_hash()
                .map_err(|_| ApiError::bad("INVALID_MESSAGE", "Invalid sign-in message."))?,
        );
        let call = ContractWallet::isValidSignatureCall {
            hash,
            signature: Bytes::from(sig),
        };
        match self.call(address, call.abi_encode()).await {
            Ok(result) => Ok(result.len() == 32 && result[..4] == [0x16, 0x26, 0xba, 0x7e]),
            Err(e) if e.code == "CHAIN_OPERATION_REJECTED" => Ok(false),
            Err(e) => Err(e),
        }
    }
}
pub fn quantity(v: &Value) -> Result<u64> {
    let s = v
        .as_str()
        .and_then(|s| s.strip_prefix("0x"))
        .ok_or_else(ApiError::unavailable)?;
    u64::from_str_radix(s, 16).map_err(|_| ApiError::unavailable())
}
pub fn decode_hex(v: &Value) -> Result<Vec<u8>> {
    hex::decode(
        v.as_str()
            .and_then(|s| s.strip_prefix("0x"))
            .ok_or_else(ApiError::unavailable)?,
    )
    .map_err(|_| ApiError::unavailable())
}
