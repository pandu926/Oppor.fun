# API reference

Base URL: `/v1`. Machine-readable contract: [openapi.json](openapi.json). Regenerate using `python3 scripts/generate-openapi.py`; use `--check` in CI. Endpoints with JSON request schemas reject unknown fields.

## Authentication

Send `POST /auth/challenge` with `{"wallet":"0x…","chain_id":"5042"}`. Chain ID must match the configured deployment; `5042` is an illustrative value, not a production-network assertion. Sign the exact returned `message` with the wallet's personal-sign method. Send `POST /auth/verify` with `message` and `signature`.

The response sets an HttpOnly session cookie and returns `user_id`, `wallet`, `csrf_token`, and `expires_in`. Keep the CSRF token in memory and send `X-CSRF-Token` on session-authenticated mutations. Every unsafe method requires the configured `Origin`. Cross-origin browser calls must send credentials. Logout revokes the stored session.

Deployed contract wallets use EIP-1271. Their original signature is revalidated on authenticated requests; a changed/revoked wallet signature revokes the session. Undeployed counterfactual wallets/EIP-6492 are not supported.

## Campaign draft

Example request to `POST /campaigns`:

```json
{
  "title": "Orbit community launch",
  "description": "Complete community tasks and submit evidence.",
  "chain_id": "5042",
  "reward": {
    "asset_kind": "ERC20",
    "token_address": "0x1111111111111111111111111111111111111111",
    "amount_base_units": "2500000000",
    "token_id": "0",
    "nft_inventory": []
  },
  "distribution": {
    "mode": "RAFFLE",
    "winner_count": 100,
    "capacity": 0,
    "registration_limit": 10000,
    "allocation_policy": "EQUAL_POOL",
    "reward_per_recipient": null
  },
  "start_at": "2026-11-01T00:00:00Z",
  "cutoff_at": "2026-11-08T00:00:00Z",
  "review_deadline": "2026-11-10T00:00:00Z",
  "claim_deadline": "2026-11-17T00:00:00Z",
  "refund_recipient": null,
  "tasks": [{
    "task_type": "X_REPOST",
    "target_url": "https://x.com/example/status/123",
    "instructions": "Submit evidence of your repost.",
    "required": true
  }]
}
```

All example addresses, dates, and chain IDs must be replaced. A campaign accepts one reward asset. ERC-721 inventory lists token IDs; target amount equals inventory count, maximum 100. ERC-1155 accepts one configured token ID. `FIXED_REWARD` requires positive `capacity` and target funding equal to reward per recipient times capacity. Positive capacity is the effective registration cap. Otherwise `registration_limit` applies, up to 100,000.

`PATCH /campaigns/{id}` accepts a full replacement draft under `campaign`, together with `expected_version`. Full draft replacement regenerates task IDs; use individual task endpoints to preserve existing draft task IDs. A locked draft is immutable. Public GET returns campaign metadata and ordered tasks; unpublished drafts are visible only to the creator. Detail includes `verification_mode: "MANUAL"`.

Campaign listing supports `limit`, opaque `cursor`, `mode`, `asset_kind`, derived `status`, and `sort=newest|ending`. `GET /campaigns/mine` includes private creator drafts. Public listing may be cached for 15 seconds; detail, authorization, and transaction checks remain authoritative. Do not infer claim availability from a cached listing.

## Transaction preparation

Creation: lock config → prepare-create → wallet executes → wait for indexing. Funding: prepare-fund returns `approvals` and `fund`. Confirm approvals, then call prepare-fund again so funding can be simulated. ERC-20 approvals use the required amount; ERC-721 uses per-token approvals; ERC-1155 requires collection approval. Activate only after full recognized funding.

Prepare endpoints return `chain_id`, `to`, `data`, `value`, `expected_sender`, `intent`, `estimated_gas`, and `expires_at`, plus intent-specific hashes. No private key is accepted. The preparation TTL is a frontend freshness hint, not an onchain deadline extension. Verify the displayed intent and use the designated wallet/chain.

`POST /campaigns/{id}/transactions` stores `tx_hash` and `kind` as a tracking hint. The backend does not mark funds or claims confirmed from that callback. The indexer reads all relevant logs even when no tracking callback was sent. Maintenance reconciles late callbacks against canonical receipts and already verified events. A reverted EOA transaction from the expected sender is marked failed; a replacement or an account-abstraction revert without a matching sender may remain pending. Authoritative campaign/claim state comes from indexed events.

## Participation and evidence

`POST /campaigns/{id}/entries` takes optional self-declared `x_username` and `discord_username`. The payout wallet is the authenticated wallet and cannot be redirected in the request. One wallet/user can register once per campaign. Creators cannot register in their own campaign.

Save evidence with `PUT /campaigns/{id}/my-entry/submissions/{task_id}`:

```json
{"expected_version":0,"text":"My task evidence","url":"https://x.com/example/status/456","upload_id":null}
```

At least one evidence field must be present. HTTPS links are stored, never fetched or used as proof of task completion. Editing evidence returns the entry to `REGISTERED`; participants must explicitly resubmit. `POST /campaigns/{id}/my-entry/submit` takes `expected_version` and requires evidence for all required tasks.

For images, call `POST /uploads/presign` with `campaign_id`, `content_type`, and `size_bytes`. The result contains `upload_id` and `upload: {url, method:"POST", fields, expires_in}`. Send a multipart form containing those fields and the file last. The signed S3 policy binds the exact object key, MIME type, and file size. Call `POST /uploads/{upload_id}/complete` afterward. The backend checks ownership, declared size, image magic bytes, and campaign cutoff, then copies the bytes to a private immutable key. An uploaded staging object does not count as submitted evidence.

PNG/JPEG/WebP only, maximum five MiB, maximum 40 active uploads per entry. This is format/size validation, not malware scanning or social verification. Evidence image URLs are short-lived and only issued to the participant or creator. Object storage remains private. Apply restrictive bucket CORS for the frontend origin.

## Review and distribution

Creator entry listing uses `limit`, UUID `after`, and `status`. Evidence access requires the creator or entry owner. Reviews take `expected_version`, `decision=ELIGIBLE|DISQUALIFIED`, and a nonempty `reason`; each decision is appended to the audit history.

Review is permitted from cutoff until review deadline. Lock-eligibility requires all submitted entries to have a decision and at least two minutes remaining before review deadline. Entries never submitted do not become eligible. Locking creates an immutable ordered snapshot and a durable allocation job.

`GET /campaigns/{id}/allocation-preview` returns root, manifest hash, leaf count, allocated quantity, short-lived manifest URL, and the trust model. The creator executes prepare-finalize once the artifact is ready.

Public results are available only after matching finalization is confirmed. `/results` returns snapshot/raffle/manifest artifact URLs and hashes; it never returns private evidence. `/manifest` returns the canonical manifest URL and hash. `/allocations/{wallet}` returns that wallet's allocations, proof arrays, claim hashes, and claim timestamps. Artifacts and proofs can be exported for direct contract claims during API outages.

Prepare-claim accepts `{"claim_index":0}`. The authenticated wallet must equal the allocation recipient. Claim indices in exported proofs are decimal strings; the API request uses a bounded JSON integer. Claim outcomes are confirmed only through verified escrow events.

## Administration

Nine admin endpoints use an allowlisted operator wallet and a session less than 15 minutes old. They support campaign hiding/restoration, account suspension/restoration, session revocation, platform counters, audit logs, and job monitoring. Admin writes require CSRF, idempotency, an optimistic administration version, and a reason. Empty `ADMIN_WALLETS` disables access. See [ADMIN.md](ADMIN.md) for setup, request examples, filters, and the exact effect on existing participation and claim rights.

## Errors

Errors use `{"error":{"code":"CUTOFF_PASSED","message":"…","request_id":"…"}}` and a matching `X-Request-ID` header. Invalid JSON/unknown fields return `400`; unauthenticated `401`; unauthorized or CSRF/origin rejection `403`; unavailable resources `404`; state/version/idempotency conflicts `409`; business validation `422`; oversized bodies `413`; rate limits `429`; dependencies `503`; request timeout `504`.

Do not retry a failed business rule blindly. Refresh versions/state first. For network failures or request timeouts, retry the original mutation with the same idempotency key.
