# Security boundaries

## Authentication and authorization

Sign-in uses the exact server-generated EIP-4361 message, bound to the configured domain, URI, chain, wallet, nonce, and expiry. A PostgreSQL transaction consumes each challenge once. EOA signatures and deployed EIP-1271 wallets are verified against the configured chain. Contract-wallet session signatures are revalidated on authenticated requests; revocation or an unavailable verifier fails closed.

Session and CSRF credentials use independent random values; the database stores their hashes. Production cookies are `__Host-` prefixed, Secure, HttpOnly, and SameSite=Lax. Authenticated mutations require a matching CSRF token and an exact allowed Origin. Authorization checks apply to campaigns, entries, reviews, upload initiation/completion, and private evidence. Payout addresses come from the authenticated wallet and cannot be redirected in evidence or claim requests.

Wallet authentication does not establish one human per wallet. This implementation does not promise Sybil resistance, verified social actions, or automatic fraud detection. Creators review submitted evidence manually. X and Discord URLs are evidence references; the server does not fetch them or call social APIs.

Platform administration is restricted to operator-configured `ADMIN_WALLETS` and sessions created within 15 minutes. Admin changes revalidate/lock the session in their transaction and require CSRF, idempotency, optimistic versions, and audit reasons. No endpoint grants roles. Suspension serializes with user/session creation and transactional writes, revokes sessions, and prevents new login. Admin campaign moderation preserves onchain rights and published allocations. Allowlisted administrator accounts are protected from HTTP suspension/revocation. See [ADMIN.md](ADMIN.md).

## State integrity

Business changes and their idempotency records commit together. Idempotency keys are scoped to the actor and operation; the same key with a different canonical payload fails. Campaign and entry locks serialize registrations and deadline-sensitive edits. Optimistic versions reject stale edits/reviews. Database constraints independently enforce address/hash lengths, uint256 bounds, wallet uniqueness, reward budgets, and immutable configuration/snapshot/allocation fields.

Amounts remain integer base-unit decimal strings. Distribution inputs must contain at most 100,000 strictly ordered unique wallets. Allocation checks enforce the funded budget, unique recipients, NFT inventory assignment, and every generated proof. Merkle leaves bind the chain, escrow, campaign, index, recipient, asset, token ID, and quantity. A claim preparation never changes the recipient.

## Chain trust

RPC endpoints are operator configuration. Prepared transactions verify the chain ID and pinned runtime code; the indexer verifies receipts and canonical block hashes before committing deduplicated events and projections. A continuity mismatch halts indexing rather than attempting a speculative rollback. A finalization inconsistent with the published root, manifest, snapshot, or budget quarantines the campaign.

Transaction callbacks are tracking hints. Reward state changes come from verified indexed contract events. The backend never holds user private keys, broadcasts user transactions, or treats an HTTP callback as proof of funding or a claim. Public claims/results fail closed when indexing is stale or the onchain root differs.

Production integration assumes a verified, non-upgradeable factory and escrow ABI matching `src/chain.rs`. Pinning proxy runtime bytecode alone does not pin its implementation. Per-instance immutables also require a deliberate code-verification strategy. The implemented contracts in `../contracts` store configuration to keep runtime hashes uniform. The integration suite deploys these contracts with test-only token/wallet fixtures; the implementation has not been independently audited. An RPC provider and the configured finality depth remain trust dependencies.

## Raffle trust

A 32-byte random seed is committed when rules are locked and encrypted with AES-256-GCM using the campaign key as authenticated context. Eligibility is snapshotted once. The worker uses deterministic rejection-sampled shuffling and publishes the seed and transcript with the allocation. Retry fencing and immutable publication prevent an ordinary job retry from producing a new draw.

The creator controls eligibility and the operator controls the server seed. Reproducibility detects changes to the published draw; it does not establish unbiased, externally verifiable randomness or eliminate selective refusal to publish. This is explicitly described as a reproducible server raffle, not VRF.

## Files and availability

Evidence is stored in a private bucket. Signed POST policies bind key, content type, exact size, and a five-minute expiry. Completion verifies ownership, deadline, byte count, and PNG/JPEG/WebP signatures, then copies bytes to an immutable private evidence object. Read URLs expire after two minutes. Format checks are not an image decoder or malware scanner. Configure restrictive storage CORS and response content handling; do not serve evidence as executable application content.

The API caps JSON bodies at 64 KiB, concurrent requests at 256, request duration at 15 seconds, and database statement/lock times at 8/3 seconds. Redis applies per-IP rate limits. Unsafe requests fail closed if that limiter is unavailable; read traffic can proceed with local concurrency limits. Forwarded IPs are trusted only from explicitly configured proxy CIDRs, never from arbitrary clients. Avoid overlapping trust ranges with public ingress.

Production configuration requires HTTPS origins/RPC/storage, PostgreSQL `sslmode=verify-full`, and certificate-verified `rediss://`. Duplicate PostgreSQL TLS parameters and their aliases cannot override validation; Redis's `#insecure` option is rejected. Network policy, credential rotation, storage IAM, TLS certificate roots, backups, secret injection, and retention are deployment responsibilities. Keep logs free of signatures, cookies, CSRF tokens, evidence, and connection URLs. `/metrics` must be restricted at the ingress.

## Dependency audit

Run `python3 scripts/security-audit.py` with `cargo-audit` installed. It reports all lockfile advisories and fails on vulnerable, unsound, or yanked dependencies in the actual normal/build graph for all targets. There is no blanket advisory ignore list.

SQLx's lockfile includes optional MySQL dependencies even though this backend builds only its PostgreSQL features. The RSA Marvin advisory (RUSTSEC-2023-0071) is currently in that unused graph. `paste` is an active SQLx macro dependency with an unmaintained advisory; that maintenance risk is reported, not hidden. Enabling new features requires rerunning the graph-aware audit. A passing dependency scan is not an application or smart-contract audit.

## Reporting

Do not post session credentials, private evidence, or exploitable deployment details in public issues. Use the repository owner's private security-reporting channel when configured. No reporting email or production incident contact is invented by this implementation.
