import type { Campaign, CampaignInput, Entry, EvidenceResult } from "./types";
export const demoWallet = "0x1111111111111111111111111111111111111111";
const titles = [
  "Orbit community",
  "Mooncat launch",
  "Genesis collection",
  "Arc creators",
  "Nova community",
  "Founders pass",
];
const descriptions = [
  "Meet the community. Help spread the word.",
  "Join the first wave of supporters.",
  "A new collection. A shared beginning.",
  "Support independent creators on Arc.",
  "Build the next chapter together.",
  "Take your part in the founding community.",
];
export const samples: Campaign[] = titles.map((title, i) => ({
  id: `00000000-0000-4000-8000-00000000000${i + 1}`,
  campaign_key: `0x${String(i + 1).padStart(64, "0")}`,
  creator_wallet: "0x2222222222222222222222222222222222222222",
  chain_id: "5042002",
  escrow_address: null,
  title,
  description: descriptions[i],
  status: "ACTIVE",
  reward: {
    asset_kind: [2, 5].includes(i) ? "ERC721" : "ERC20",
    token_address: "0x3333333333333333333333333333333333333333",
    amount_base_units: [
      "2500000000",
      "5000000000000000000000000",
      "50",
      "800000000",
      "1000000000000000000000000",
      "100",
    ][i],
    token_id: "0",
    nft_inventory: [],
  },
  distribution: {
    mode: i % 2 === 0 ? "RAFFLE" : "ALL_ELIGIBLE",
    winner_count: 100,
    capacity: 0,
    registration_limit: 10000,
    allocation_policy: "EQUAL_POOL",
    reward_per_recipient: null,
  },
  start_at: "2026-01-01T00:00:00Z",
  cutoff_at: "2099-01-01T00:00:00Z",
  review_deadline: "2099-01-03T00:00:00Z",
  claim_deadline: "2099-01-10T00:00:00Z",
  registered_count: 0,
  registration_limit: 10000,
  config_hash: null,
  rules_hash: null,
  rules: null,
  version: 0,
  verification_mode: "MANUAL",
  created_at: "2026-01-01T00:00:00Z",
  tasks: Array.from({ length: [4, 3, 5, 2, 4, 3][i] }, (_, j) => ({
    id: `task-${i}-${j}`,
    position: j,
    task_type: (
      ["X_REPOST", "DISCORD_JOIN", "X_COMMENT", "CUSTOM", "X_LIKE"] as const
    )[j],
    target_url: j === 1 ? "https://discord.com" : "https://x.com",
    instructions: [
      "Repost the launch announcement.",
      "Join the community Discord server.",
      "Comment on the launch announcement.",
      "Tell us what you like about the community.",
      "Like the launch announcement.",
    ][j],
    required: true,
  })),
}));
export const sampleRewards = [
  "2,500 USDC",
  "5M MOON",
  "50 NFTs",
  "800 USDC",
  "1M NOVA",
  "100 NFTs",
];
export const sampleDays = [2, 5, 3, 1, 7, 4];
type DemoState = {
  drafts: Campaign[];
  entries: Record<string, Entry>;
  evidence: Record<string, EvidenceResult["evidence"]>;
};
let state: DemoState = { drafts: [], entries: {}, evidence: {} };
export function loadDemo() {
  try {
    const value = localStorage.getItem("oppor:demo:v1");
    if (value) state = JSON.parse(value);
  } catch {
    /* Storage is optional. */
  }
}
function persist() {
  try {
    localStorage.setItem("oppor:demo:v1", JSON.stringify(state));
  } catch {
    /* Storage is optional. */
  }
}
export function demoCampaigns() {
  return [...samples, ...state.drafts];
}
export function createDemo(input: CampaignInput) {
  const c: Campaign = {
    ...samples[0],
    ...input,
    id: crypto.randomUUID(),
    creator_wallet: demoWallet,
    escrow_address: null,
    status: "DRAFT",
    version: 0,
    tasks: input.tasks.map((t, i) => ({
      ...t,
      instructions: t.instructions || "",
      required: t.required ?? true,
      id: crypto.randomUUID(),
      position: i,
    })),
  };
  state.drafts.push(c);
  persist();
  return c;
}
export function demoEntry(id: string) {
  return state.entries[id] ? structuredClone(state.entries[id]) : null;
}
export function demoRegister(id: string) {
  const e: Entry = {
    id: crypto.randomUUID(),
    campaign_id: id,
    payout_wallet: demoWallet,
    slot_number: 1,
    status: "REGISTERED",
    version: 0,
    submitted_at: null,
  };
  state.entries[id] = e;
  persist();
  return structuredClone(e);
}
export function demoEvidence(id: string) {
  const entry = state.entries[id];
  return {
    entry,
    x_username_declared: null,
    discord_username_declared: null,
    evidence: state.evidence[id] || [],
  } as EvidenceResult;
}
export function demoSave(id: string, task: string, text: string, url: string) {
  const e = state.entries[id];
  const items = state.evidence[id] || [];
  state.evidence[id] = [
    ...items.filter((x) => x.task_id !== task),
    {
      task_id: task,
      revision: 1,
      evidence: { text: text || null, url: url || null, upload_id: null },
      image_url: null,
    },
  ];
  e.version++;
  e.status = "REGISTERED";
  persist();
  return structuredClone(e);
}
export function demoSubmit(id: string) {
  const campaign = demoCampaigns().find((c) => c.id === id);
  if (
    campaign?.tasks?.some(
      (t) => t.required && !state.evidence[id]?.some((e) => e.task_id === t.id),
    )
  )
    throw new Error("Save evidence for every required task before submitting.");
  const e = state.entries[id];
  e.status = "SUBMITTED";
  e.version++;
  e.submitted_at = new Date().toISOString();
  persist();
  return structuredClone(e);
}
