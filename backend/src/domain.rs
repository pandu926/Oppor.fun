use crate::error::{ApiError, Result};
use alloy_primitives::{Address, B256, U256};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::FromRow;
use uuid::Uuid;

pub const MAX_ENTRIES: u32 = 100_000;
pub const MAX_NFTS: usize = 100;

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AssetKind {
    Erc20,
    Erc721,
    Erc1155,
}
impl AssetKind {
    pub fn id(self) -> u8 {
        match self {
            Self::Erc20 => 0,
            Self::Erc721 => 1,
            Self::Erc1155 => 2,
        }
    }
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DistributionMode {
    AllEligible,
    Raffle,
}
impl DistributionMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::AllEligible => "ALL_ELIGIBLE",
            Self::Raffle => "RAFFLE",
        }
    }
    pub fn id(self) -> u8 {
        if self == Self::Raffle { 1 } else { 0 }
    }
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AllocationPolicy {
    EqualPool,
    FixedReward,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Reward {
    pub asset_kind: AssetKind,
    pub token_address: Address,
    pub amount_base_units: String,
    #[serde(default = "zero")]
    pub token_id: String,
    #[serde(default)]
    pub nft_inventory: Vec<String>,
}
fn zero() -> String {
    "0".into()
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Distribution {
    pub mode: DistributionMode,
    #[serde(default)]
    pub winner_count: u32,
    #[serde(default)]
    pub capacity: u32,
    #[serde(default = "default_limit")]
    pub registration_limit: u32,
    #[serde(default = "equal_pool")]
    pub allocation_policy: AllocationPolicy,
    pub reward_per_recipient: Option<String>,
}
fn default_limit() -> u32 {
    MAX_ENTRIES
}
fn equal_pool() -> AllocationPolicy {
    AllocationPolicy::EqualPool
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TaskInput {
    pub task_type: String,
    pub target_url: String,
    #[serde(default)]
    pub instructions: String,
    #[serde(default = "yes")]
    pub required: bool,
}
fn yes() -> bool {
    true
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CampaignInput {
    pub title: String,
    #[serde(default)]
    pub description: String,
    pub chain_id: String,
    pub reward: Reward,
    pub distribution: Distribution,
    pub start_at: DateTime<Utc>,
    pub cutoff_at: DateTime<Utc>,
    pub review_deadline: DateTime<Utc>,
    pub claim_deadline: DateTime<Utc>,
    pub refund_recipient: Option<Address>,
    pub tasks: Vec<TaskInput>,
}
impl CampaignInput {
    pub fn validate(&self, chain: u64, now: DateTime<Utc>) -> Result<()> {
        if self.chain_id != chain.to_string() {
            return Err(ApiError::invalid("The campaign chain is not supported."));
        }
        if self.title.trim().chars().count() < 3
            || self.title.chars().count() > 100
            || self.description.chars().count() > 5000
        {
            return Err(ApiError::invalid("Invalid campaign title or description."));
        }
        if self.reward.token_address.is_zero() || self.refund_recipient.is_some_and(|v| v.is_zero())
        {
            return Err(ApiError::invalid(
                "Reward and refund addresses cannot be zero.",
            ));
        }
        let times = [
            self.start_at,
            self.cutoff_at,
            self.review_deadline,
            self.claim_deadline,
        ];
        if times
            .iter()
            .any(|v| v.timestamp() < 0 || v.timestamp_subsec_nanos() != 0)
            || !(now < times[0] && times.windows(2).all(|w| w[0] < w[1]))
        {
            return Err(ApiError::invalid(
                "Campaign deadlines must be future, strictly ordered, whole-second timestamps.",
            ));
        }
        if (self.claim_deadline - self.review_deadline).num_hours() < 24 {
            return Err(ApiError::invalid(
                "The claim window must last at least 24 hours after the review deadline.",
            ));
        }
        if self.tasks.is_empty() || self.tasks.len() > 20 {
            return Err(ApiError::invalid(
                "A campaign must have between one and twenty tasks.",
            ));
        }
        for t in &self.tasks {
            t.validate()?;
        }
        if !self.tasks.iter().any(|t| t.required) {
            return Err(ApiError::invalid("At least one task must be required."));
        }
        let p = positive(&self.reward.amount_base_units)?;
        let token_id = amount(&self.reward.token_id)?;
        let d = &self.distribution;
        if d.registration_limit == 0
            || d.registration_limit > MAX_ENTRIES
            || d.capacity > MAX_ENTRIES
            || (d.capacity > 0 && d.registration_limit < d.capacity)
        {
            return Err(ApiError::invalid("Invalid participant limit."));
        }
        if (d.mode == DistributionMode::Raffle
            && (d.winner_count == 0 || d.winner_count > MAX_ENTRIES))
            || (d.mode == DistributionMode::AllEligible && d.winner_count != 0)
        {
            return Err(ApiError::invalid("Invalid raffle winner count."));
        }
        if d.allocation_policy == AllocationPolicy::FixedReward {
            if d.capacity == 0 || self.reward.asset_kind == AssetKind::Erc721 {
                return Err(ApiError::invalid(
                    "Fixed rewards require a bounded capacity and a divisible reward asset.",
                ));
            }
            let r = positive(
                d.reward_per_recipient
                    .as_deref()
                    .ok_or_else(|| ApiError::invalid("A fixed reward amount is required."))?,
            )?;
            if r.checked_mul(U256::from(d.capacity)) != Some(p) {
                return Err(ApiError::invalid(
                    "The reward pool must equal the fixed reward times participant capacity.",
                ));
            }
        } else if d.reward_per_recipient.is_some() {
            return Err(ApiError::invalid(
                "A per-recipient amount is only allowed for fixed rewards.",
            ));
        }
        if self.reward.asset_kind != AssetKind::Erc1155 && token_id != U256::ZERO {
            return Err(ApiError::invalid(
                "Token ID must be zero for ERC-20 and ERC-721 campaign configuration.",
            ));
        }
        if self.reward.asset_kind == AssetKind::Erc721 {
            let ids = self
                .reward
                .nft_inventory
                .iter()
                .map(|s| amount(s))
                .collect::<Result<Vec<_>>>()?;
            let mut sorted = ids.clone();
            sorted.sort();
            sorted.dedup();
            if ids.is_empty()
                || ids.len() > MAX_NFTS
                || sorted.len() != ids.len()
                || p != U256::from(ids.len())
            {
                return Err(ApiError::invalid(
                    "NFT inventory must contain at most 100 unique IDs and match the target count.",
                ));
            }
            if (d.mode == DistributionMode::AllEligible
                && (d.capacity == 0 || d.capacity as usize > ids.len()))
                || (d.mode == DistributionMode::Raffle && d.winner_count as usize > ids.len())
            {
                return Err(ApiError::invalid(
                    "NFT inventory does not cover the declared recipient limit.",
                ));
            }
        } else if !self.reward.nft_inventory.is_empty() {
            return Err(ApiError::invalid(
                "NFT inventory is only allowed for ERC-721.",
            ));
        }
        Ok(())
    }
    pub fn effective_limit(&self) -> i32 {
        if self.distribution.capacity > 0 {
            self.distribution.capacity as i32
        } else {
            self.distribution.registration_limit as i32
        }
    }
}
impl TaskInput {
    pub fn validate(&self) -> Result<()> {
        if !matches!(
            self.task_type.as_str(),
            "X_REPOST" | "X_LIKE" | "X_COMMENT" | "X_TAG" | "DISCORD_JOIN" | "CUSTOM"
        ) || self.instructions.chars().count() > 2000
        {
            return Err(ApiError::invalid("Invalid task type or instructions."));
        }
        validate_url(&self.target_url)?;
        let u = url::Url::parse(&self.target_url)
            .map_err(|_| ApiError::invalid("Invalid task URL."))?;
        let host = u.host_str().unwrap_or_default();
        if self.task_type.starts_with("X_")
            && !matches!(
                host,
                "x.com" | "www.x.com" | "twitter.com" | "www.twitter.com"
            )
        {
            return Err(ApiError::invalid("X tasks must link to X or Twitter."));
        }
        if self.task_type == "DISCORD_JOIN"
            && !matches!(host, "discord.gg" | "discord.com" | "www.discord.com")
        {
            return Err(ApiError::invalid("Discord tasks must link to Discord."));
        }
        Ok(())
    }
}
pub fn amount(v: &str) -> Result<U256> {
    if v.is_empty()
        || v.len() > 78
        || !v.bytes().all(|c| c.is_ascii_digit())
        || (v.len() > 1 && v.starts_with('0'))
    {
        return Err(ApiError::invalid(
            "Amounts must be canonical decimal integer strings.",
        ));
    }
    U256::from_str_radix(v, 10).map_err(|_| ApiError::invalid("The amount exceeds uint256."))
}
pub fn positive(v: &str) -> Result<U256> {
    let a = amount(v)?;
    if a == U256::ZERO {
        Err(ApiError::invalid("The amount must be positive."))
    } else {
        Ok(a)
    }
}
pub fn validate_url(v: &str) -> Result<()> {
    if v.len() > 2048 {
        return Err(ApiError::invalid("The URL is too long."));
    }
    let u = url::Url::parse(v).map_err(|_| ApiError::invalid("Invalid URL."))?;
    if u.scheme() != "https"
        || u.host_str().is_none()
        || !u.username().is_empty()
        || u.password().is_some()
    {
        return Err(ApiError::invalid(
            "Only HTTPS URLs without credentials are allowed.",
        ));
    }
    Ok(())
}

#[derive(Clone, Debug, FromRow)]
pub struct Campaign {
    pub id: Uuid,
    pub campaign_key: Vec<u8>,
    pub creator_id: Uuid,
    pub creator_wallet: Vec<u8>,
    pub chain_id: i64,
    pub escrow_address: Option<Vec<u8>>,
    pub title: String,
    pub description: String,
    pub status: String,
    pub mode: String,
    pub start_at: DateTime<Utc>,
    pub cutoff_at: DateTime<Utc>,
    pub review_deadline: DateTime<Utc>,
    pub claim_deadline: DateTime<Utc>,
    pub capacity: i32,
    pub registration_limit: i32,
    pub registered_count: i32,
    pub winner_count: i32,
    pub spec_json: Value,
    pub rules_json: Option<Value>,
    pub rules_hash: Option<Vec<u8>>,
    pub config_hash: Option<Vec<u8>>,
    pub activated: bool,
    pub chain_state: i16,
    pub version: i64,
    pub listed: bool,
    pub moderation_hidden: bool,
    pub moderation_version: i64,
    pub moderation_reason: Option<String>,
    pub moderation_updated_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}
impl Campaign {
    pub fn spec(&self) -> Result<CampaignInput> {
        serde_json::from_value(self.spec_json.clone()).map_err(|_| ApiError::internal())
    }
    pub fn key(&self) -> B256 {
        B256::from_slice(&self.campaign_key)
    }
    pub fn creator(&self) -> Address {
        Address::from_slice(&self.creator_wallet)
    }
    pub fn escrow(&self) -> Result<Address> {
        self.escrow_address
            .as_ref()
            .map(|b| Address::from_slice(b))
            .ok_or_else(|| {
                ApiError::conflict("ESCROW_NOT_CREATED", "The escrow has not been confirmed.")
            })
    }
    pub fn ensure_creator(&self, user: Uuid) -> Result<()> {
        if self.creator_id == user {
            Ok(())
        } else {
            Err(ApiError::forbidden())
        }
    }
    pub fn ensure_open(&self, now: DateTime<Utc>) -> Result<()> {
        if !self.activated
            || self.chain_state != 1
            || matches!(
                self.status.as_str(),
                "INTEGRITY_ERROR" | "EXPIRED" | "CLOSED" | "CANCELLED"
            )
        {
            return Err(ApiError::conflict(
                "CAMPAIGN_NOT_ACTIVE",
                "The campaign is not accepting participants.",
            ));
        }
        if now < self.start_at {
            return Err(ApiError::conflict(
                "CAMPAIGN_NOT_STARTED",
                "The campaign has not started.",
            ));
        }
        if now >= self.cutoff_at {
            return Err(ApiError::conflict(
                "CUTOFF_PASSED",
                "The submission deadline has passed.",
            ));
        }
        Ok(())
    }
    pub fn ensure_review(&self, now: DateTime<Utc>) -> Result<()> {
        if !self.activated
            || self.chain_state != 1
            || !matches!(self.status.as_str(), "ACTIVE" | "SCHEDULED" | "REVIEWING")
        {
            return Err(ApiError::conflict(
                "REVIEW_CLOSED",
                "Eligibility decisions are locked or the campaign is inactive.",
            ));
        }
        if now < self.cutoff_at || now >= self.review_deadline {
            return Err(ApiError::conflict(
                "REVIEW_CLOSED",
                "Review is only available between cutoff and the review deadline.",
            ));
        }
        Ok(())
    }
    pub fn display_status(&self, now: DateTime<Utc>) -> &str {
        if matches!(
            self.status.as_str(),
            "INTEGRITY_ERROR" | "CANCELLED" | "CLOSED"
        ) {
            return &self.status;
        }
        if self.chain_state == 2 && now >= self.claim_deadline {
            return "COMPLETED";
        }
        if self.chain_state == 1 && now >= self.review_deadline {
            return "EXPIRED";
        }
        if matches!(self.status.as_str(), "SCHEDULED" | "ACTIVE" | "REVIEWING") && self.activated {
            if now >= self.cutoff_at {
                "REVIEWING"
            } else if now >= self.start_at {
                "ACTIVE"
            } else {
                "SCHEDULED"
            }
        } else {
            &self.status
        }
    }
}

#[derive(Debug, Clone, FromRow)]
pub struct Entry {
    pub id: Uuid,
    pub campaign_id: Uuid,
    pub user_id: Uuid,
    pub payout_wallet: Vec<u8>,
    pub slot_number: i32,
    pub x_username_declared: Option<String>,
    pub discord_username_declared: Option<String>,
    pub status: String,
    pub submitted_at: Option<DateTime<Utc>>,
    pub version: i64,
}
