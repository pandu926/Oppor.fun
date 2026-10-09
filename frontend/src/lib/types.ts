// Generated from backend/openapi.json. Run python3 scripts/generate-types.py.
export type Error = {
  error: {
    code: string;
    message: string;
    request_id: string;
  };
};

export type VersionInput = {
  expected_version: number;
};

export type Task = {
  task_type:
    "X_REPOST" | "X_LIKE" | "X_COMMENT" | "X_TAG" | "DISCORD_JOIN" | "CUSTOM";
  target_url: string;
  instructions?: string;
  required?: boolean;
};

export type Reward = {
  asset_kind: "ERC20" | "ERC721" | "ERC1155";
  token_address: string;
  amount_base_units: string;
  token_id?: string;
  nft_inventory?: string[];
};

export type Distribution = {
  mode: "ALL_ELIGIBLE" | "RAFFLE";
  winner_count?: number;
  capacity?: number;
  registration_limit?: number;
  allocation_policy?: "EQUAL_POOL" | "FIXED_REWARD";
  reward_per_recipient?: string | null;
};

export type CampaignInput = {
  title: string;
  description?: string;
  chain_id: string;
  reward: Reward;
  distribution: Distribution;
  start_at: string;
  cutoff_at: string;
  review_deadline: string;
  claim_deadline: string;
  refund_recipient?: string | null;
  tasks: Task[];
};

export type UpdateInput = {
  expected_version: number;
  campaign: CampaignInput;
};

export type TaskEdit = {
  expected_version: number;
  task: Task;
};

export type ChallengeInput = {
  wallet: string;
  chain_id: string;
};

export type VerifyInput = {
  message: string;
  signature: string;
};

export type RegisterInput = {
  x_username?: string | null;
  discord_username?: string | null;
};

export type EvidenceInput = {
  expected_version: number;
  text?: string | null;
  url?: string | null;
  upload_id?: string | null;
};

export type ReviewInput = {
  expected_version: number;
  decision: "ELIGIBLE" | "DISQUALIFIED";
  reason: string;
};

export type ClaimInput = {
  claim_index: number;
};

export type TrackInput = {
  tx_hash: string;
  kind:
    "CREATE" | "FUND" | "ACTIVATE" | "FINALIZE" | "CLAIM" | "CANCEL" | "SWEEP";
};

export type UploadInput = {
  campaign_id: string;
  content_type: "image/png" | "image/jpeg" | "image/webp";
  size_bytes: number;
};

export type PreparedTransaction = {
  chain_id: string;
  to: string;
  data: string;
  value: string;
  expected_sender: string;
  intent: string;
  estimated_gas?: string | null;
  expires_at: string;
  config_hash?: string;
  distribution_root?: string;
  manifest_hash?: string;
  simulation?: string;
};

export type AdminProfile = {
  user_id: string;
  wallet: string;
  role: unknown;
  reauthenticate_at: string;
  permissions: string[];
};

export type ModerationInput = {
  expected_version: number;
  hidden: boolean;
  reason: string;
};

export type SuspensionInput = {
  expected_version: number;
  suspended: boolean;
  reason: string;
};

export type RevokeInput = {
  expected_version: number;
  reason: string;
};

export type ModerationResult = {
  campaign_id: string;
  hidden: boolean;
  version: number;
};

export type SuspensionResult = {
  user_id: string;
  suspended: boolean;
  version: number;
  revoked_sessions: number;
};

export type RevokeResult = {
  user_id: string;
  version: number;
  revoked_sessions: number;
};

export type AdminUser = {
  id: string;
  wallet: string;
  created_at: string;
  suspended: boolean;
  version: number;
  suspension_reason: string | null;
  suspension_updated_at: string | null;
};

export type ModerationState = {
  hidden: boolean;
  version: number;
  reason: string | null;
  updated_at: string | null;
};

export type AdminCampaign = {
  campaign: Campaign;
  moderation: ModerationState;
};

export type AdminAudit = {
  id: string;
  actor_id: string | null;
  campaign_id: string | null;
  action: string;
  metadata: Record<string, unknown>;
  created_at: string;
};

export type AdminJob = {
  id: string;
  campaign_id: string;
  kind: string;
  status: "PENDING" | "RUNNING" | "FAILED" | "SUCCEEDED";
  attempts: number;
  available_at: string;
  locked_until: string | null;
  last_error: string | null;
  created_at: string;
};

export type AdminUserPage = {
  items: AdminUser[];
  next_cursor: string | null;
};

export type AdminCampaignPage = {
  items: AdminCampaign[];
  next_cursor: string | null;
};

export type AdminAuditPage = {
  items: AdminAudit[];
  next_cursor: string | null;
};

export type AdminJobPage = {
  items: AdminJob[];
  next_cursor: string | null;
};

export type AdminCounts = {
  users: number;
  suspended_users: number;
  campaigns: number;
  hidden_campaigns: number;
  public_campaigns: number;
  entries: number;
  claimed_allocations: number;
  jobs: {
    PENDING?: number;
    RUNNING?: number;
    FAILED?: number;
    SUCCEEDED?: number;
  };
};

export type AdminIndexer = {
  next_block: string;
  observed_safe_head: string | null;
  updated_at: string;
  halted_reason: string | null;
  healthy: boolean;
};

export type AdminStats = {
  counts: AdminCounts;
  indexer: AdminIndexer | null;
  generated_at: string;
};

export type HealthResult = {
  status: string;
};

export type ChallengeResult = {
  message: string;
  expires_at: string;
};

export type Profile = {
  user_id: string;
  wallet: string;
};

export type SessionResult = {
  user_id: string;
  wallet: string;
  csrf_token: string;
  expires_in: number;
};

export type LogoutResult = {
  logged_out: boolean;
};

export type TaskView = {
  task_type:
    "X_REPOST" | "X_LIKE" | "X_COMMENT" | "X_TAG" | "DISCORD_JOIN" | "CUSTOM";
  target_url: string;
  instructions: string;
  required: boolean;
  id: string;
  position: number;
};

export type LockedRules = {
  schema_version: number;
  campaign_key: string;
  chain_id: string;
  creator: string;
  refund_recipient: string;
  reward: Reward;
  distribution: Distribution;
  task_ids: string[];
  tasks: Task[];
  starts_at: string;
  cutoff_at: string;
  review_deadline: string;
  claim_deadline: string;
  verification_mode: string;
  raffle_algorithm: string;
  raffle_seed_commitment: string | null;
  claim_policy: string;
  refund_policy: string;
};

export type Campaign = {
  id: string;
  campaign_key: string;
  creator_wallet: string;
  chain_id: string;
  escrow_address: string | null;
  title: string;
  description: string;
  status: string;
  reward: Reward;
  distribution: Distribution;
  start_at: string;
  cutoff_at: string;
  review_deadline: string;
  claim_deadline: string;
  registered_count: number;
  registration_limit: number;
  config_hash: string | null;
  rules_hash: string | null;
  rules: LockedRules | null;
  version: number;
  verification_mode: string;
  created_at: string;
  tasks?: TaskView[];
};

export type CampaignPage = {
  items: Campaign[];
  next_cursor: string | null;
};

export type TaskMutation = {
  task_id: string;
  version: number;
};

export type DeleteTaskResult = {
  deleted: boolean;
  version: number;
};

export type ConfigResult = {
  campaign_id: string;
  config_hash: string;
  rules_hash: string;
  rules: LockedRules;
  version: number;
};

export type Entry = {
  id: string;
  campaign_id: string;
  payout_wallet: string;
  slot_number: number;
  status: string;
  version: number;
  submitted_at: string | null;
  review?: {
    decision: string;
    reason: string;
  };
};

export type EntryPage = {
  items: Entry[];
  next_cursor: string | null;
};

export type EntryMutation = {
  entry_id: string;
  status: string;
  version: number;
};

export type EvidenceMutation = {
  entry_id: string;
  task_id: string;
  revision: number;
  version: number;
  status: string;
};

export type Evidence = {
  text: string | null;
  url: string | null;
  upload_id: string | null;
};

export type EvidenceItem = {
  task_id: string;
  revision: number;
  evidence: Evidence;
  image_url: string | null;
};

export type EvidenceResult = {
  entry: Entry;
  x_username_declared: string | null;
  discord_username_declared: string | null;
  evidence: EvidenceItem[];
};

export type EligibilityResult = {
  campaign_id: string;
  status: string;
  snapshot_hash: string;
  eligible_count: number;
  version: number;
};

export type AllocationPreview = {
  root: string;
  manifest_hash: string;
  leaf_count: number;
  allocated_quantity: string;
  manifest_url: string;
  trust_model: string;
};

export type Results = {
  eligibility_hash: string;
  eligible_snapshot_url: string;
  raffle_transcript_url: string | null;
  root: string;
  manifest_hash: string;
  leaf_count: number;
  manifest_url: string;
};

export type ManifestResult = {
  manifest_hash: string;
  url: string;
  expires_in: number;
};

export type Allocation = {
  index: string;
  recipient: string;
  token_id: string;
  quantity: string;
  proof: string[];
  claim_tx_hash: string | null;
  claimed_at: string | null;
};

export type AllocationResult = {
  campaign_id: string;
  escrow: string;
  claim_deadline: string;
  allocations: Allocation[];
};

export type TrackResult = {
  tx_hash: string;
  status: string;
  note: string;
};

export type UploadPolicy = {
  url: string;
  method: unknown;
  fields: Record<string, unknown>;
  expires_in: number;
  instructions: string;
};

export type UploadResult = {
  upload_id: string;
  upload: UploadPolicy;
};

export type UploadComplete = {
  upload_id: string;
  status: unknown;
};

export type FundingResult = {
  approvals: PreparedTransaction[];
  fund: PreparedTransaction;
  remaining_quantity: string;
  instructions: string;
};
