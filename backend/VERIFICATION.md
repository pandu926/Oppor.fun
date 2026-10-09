# Verification record

Date: 2026-10-09. Local Linux x86_64 workspace, AMD EPYC virtual CPU, eight visible CPUs. The host also runs other workloads. These measurements are reproducible local baselines, not production service-level guarantees.

## Completed checks

| Check | Result |
| --- | --- |
| `cargo fmt --all -- --check` | Passed |
| `cargo clippy --locked --all-targets -- -D warnings` | Passed |
| `cargo test --locked --lib` | 16 unit/property tests passed; the performance test is explicitly ignored in the default run |
| `cargo test --locked --test integration -- --ignored --nocapture` | Complete integration scenario passed against PostgreSQL, Redis, S3 fixture, and Anvil |
| `cargo test --locked --release --lib` | 16 unit/property tests passed in the pre-administration baseline; current unit/property coverage passed in the default profile |
| `cargo test --locked --release --test integration -- --ignored --nocapture` | Release integration scenario passed |
| Solidity implementation | 68 contract tests passed; backend integration uses actual factory/escrow; see [contract verification](../contracts/VERIFICATION.md) |
| Release allocation benchmark | Passed, 10,000 allocations and all proofs verified |
| OpenAPI source inventory and schema validation | 43 paths, 47 operations; typed schemas and 115 captured real integration responses validated |
| Dependency audit | Zero active vulnerable/unsound/yanked packages; maintenance/unused-lockfile advisories reported |
| Application Compose configuration | Validated without resolving production secrets |
| `cargo build --locked --release --bins` | API and worker binaries built successfully |
| Docker build with pinned base-image digests | Passed |
| Container smoke | Passed: UID 10001, read-only filesystem, migrations, liveness, fail-closed readiness, protected admin routing, API/worker SIGTERM shutdown, production HTTPS default |

The latest integration scenario includes all five database migrations, malformed evidence rejection, evidence immutability, fair transaction reconciliation, stale-indexer readiness, and the administrative flows below. The CI workflow repeats formatting, strict linting, unit/infrastructure tests, typed API response validation, dependency scanning, and container smoke checks; hosted CI has not been run from this workspace.

## End-to-end coverage

The integration scenario performs actual local EVM transactions through the actual factory/escrow implementation and a token fixture and verifies balances and indexed events. It covers:

- EOA and EIP-1271 sign-in; concurrent nonce consumption, replay rejection, duplicate cookie handling, logout, and contract-wallet revocation.
- Origin/CSRF enforcement, private draft ownership, transactional idempotency, changed-payload conflicts, and immutable rule protection in PostgreSQL.
- Factory deployment preparation, ERC-20 approval, funding, activation, and trusted bytecode checks.
- Concurrent last-slot registration, unique payout wallets, evidence uploads, size/type rejection, private upload ownership, immutable evidence copies, and participant evidence access restrictions.
- Cutoff rejection, explicit submissions, creator-only review, pending-review blocking, stale review versions, and immutable eligibility locking.
- Durable allocation publication into PostgreSQL and S3, deterministic raffle replay, and Rust/Solidity golden Merkle-leaf agreement.
- Finalization and public artifact hashes, wrong-recipient rejection, failed token transfer simulation, successful recipient claim, duplicate-claim rejection, premature sweep rejection, and exact token balances.
- Late transaction callbacks, canonical receipt reconciliation, event replay without double counting, fresh/stale readiness, and indexer metrics from shared database state.
- Administrator allowlist enforcement across read routes, denied creator mutations, admin CSRF, stale session rejection, protected operator accounts, atomic audited moderation, idempotent retries, changed-payload conflicts, and concurrent moderation version conflicts.
- Immediate hide/unhide across warmed list caches, nonparticipant detail/new-registration rejection, preserved creator/participant access, administrator pagination/filter validation, successful/failed job monitoring, suspension/login rejection, restoration without resurrecting revoked sessions, standalone session revocation, and audit access.

Unit/property tests additionally exercise ERC-20 rounding and insufficient pools, uint256 boundaries, fixed ERC-1155 quantities, numeric ERC-721 inventory ordering, cross-escrow proof separation, authenticated seed decryption, unique raffle selections, every tested Merkle tree size, ordered unique snapshots, trusted proxy parsing, PostgreSQL TLS parameter overrides (including aliases), and Redis insecure-TLS options.

## Performance baseline

Release build: optimizations enabled, thin LTO, one code-generation unit.

| Workload | Measurement |
| --- | --- |
| Build 10,000 ERC-20 allocations, generate every proof, verify every proof | 161.75 ms |
| Same build plus canonical JSON serialization | 320.22 ms |
| Serialized allocation list with proofs | 10,428,395 bytes |
| HTTP listing, 200 requests, concurrency 32, one finalized campaign, warm Redis cache, loopback TCP | 801.98 requests/second |
| HTTP p50 / p95 / p99 | 27.02 / 112.64 / 127.99 ms |

Allocation measurements are the pre-administration baseline and exclude database insertion, storage uploads, and network latency. The HTTP numbers above were measured with administration enabled and include a PostgreSQL moderation revision read even on cache hits to enforce visibility changes immediately. The HTTP run took place after the Docker build completed; shared-host scheduling can still affect tail latency. Measurements do not cover uncached listings, authenticated writes, remote RPC/storage, many campaigns, sustained traffic, or full 100,000-recipient memory sizing. Debug builds were also exercised and are slower; do not use their throughput as a release capacity estimate.

Reproduce:

```bash
bash scripts/test.sh
cargo test --locked --release --lib large_distribution_performance -- --ignored --nocapture
cargo test --locked --release --test integration -- --ignored --nocapture
```

The tests emit `ALLOCATION_PERFORMANCE` / `HTTP_PERFORMANCE` JSON and write local reports to `target/allocation-performance.json` / `target/http-performance.json`. Repeated runs overwrite the reports. Test infrastructure uses an isolated Compose project with loopback ports; stop it using `docker compose -f compose.test.yml down` after testing.

## Limits of this record

Anvil is an EVM integration fixture, not a live Arc deployment. Moto is an S3 protocol fixture, not a production durability/IAM service. ERC-721/ERC-1155 allocation and calldata have Rust coverage. The Solidity suite validates local NFT funding/claim/refund transfers against the implemented escrow; API-to-chain NFT integration and live token behavior remain unvalidated. Production TLS connections, real storage POST-policy enforcement, remote-provider behavior, deep reorg recovery, database restore, environment-specific load, and container rollout require deployment validation.

The backend prepares transactions and does not custody funds. No mainnet deployment or transaction was performed. This implementation has not undergone an independent penetration test or smart-contract audit. See [SECURITY.md](SECURITY.md) and [RUNBOOK.md](RUNBOOK.md) for the explicit trust and operational boundaries.
