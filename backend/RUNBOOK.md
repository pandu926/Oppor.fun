# Operations runbook

## Deployment

Build with Rust 1.97.1 and the committed lockfile, or use the supplied multi-stage Dockerfile. Run `oppor-api` and `oppor-worker` as separate non-root services. `compose.yml` is an application-only template using external PostgreSQL, Redis, and S3; `compose.test.yml` is exclusively an isolated test fixture.

1. Provision PostgreSQL with verified TLS, an authenticated TLS Redis instance, and a private S3-compatible bucket. Enable database point-in-time recovery and storage durability/versioning. Configure bucket CORS for the frontend origin and PUT uploads only; keep public access blocked.
2. Verify the factory/escrow ABI, chain ID, runtime bytecode hashes, deployment block, token support, and finality policy. The repository contains no production escrow deployment. Do not use Solidity fixtures for real funds.
3. Inject production environment variables through a secret manager. Set `APP_ENV=production`; use `postgres://...?...sslmode=verify-full`, `rediss://...`, and HTTPS RPC/storage URLs. Configure the required CA roots. Generate `RAFFLE_SEED_ENCRYPTION_KEY` and `RATE_LIMIT_KEY` independently with `openssl rand -hex 32`.
4. Set `PUBLIC_ORIGIN` and `ALLOWED_ORIGINS` to exact frontend origins. Set `TRUSTED_PROXY_CIDRS` only for the actual ingress proxies. Restrict direct network access to the API, database, Redis, and storage. Terminate HTTPS at the ingress; keep `/v1/metrics` private. The frontend and API should use an appropriate same-site deployment for cookie authentication.
5. Apply migrations as a separate controlled operation: `oppor-api migrate`. Migration mode connects only to PostgreSQL and still requires verified TLS in production; it does not need a factory deployment, Redis, or object-storage credentials. Use a schema-owning migration identity; runtime identities should have table DML/sequence privileges without schema creation or role administration. Both runtime services use the same tables and need their prescribed update rights.
6. Start the worker and API. Wait for `/v1/health/ready` to return 200 after the indexer catches up. Liveness alone does not mean funds-related operations are available. Route traffic only to ready API instances.

```bash
docker compose --env-file .env -f compose.yml build
docker compose --env-file .env -f compose.yml --profile operations run --rm migrate
docker compose --env-file .env -f compose.yml up -d api worker
```

The template binds the API to host loopback. Adjust networking for your ingress. Supply an environment-specific RPC, database, Redis, storage, and contract configuration; no default production credentials are provided. Pin final container image digests in the deployment system.

## Administrator configuration

Apply migration `0005_platform_administration.sql`, then configure `ADMIN_WALLETS` on every API replica. Empty disables administrator access. Operators sign in through the wallet flow and reauthenticate every 15 minutes for admin requests. See [ADMIN.md](ADMIN.md) for moderation, suspension, session revocation, immutable auditing, and protected administrator accounts. Changing the allowlist requires a configuration rollout; it is not self-service through the public API.

Campaign-list cache keys now include a PostgreSQL moderation revision. Cache hits perform a small database read to prevent stale hidden campaigns from being served after moderation commits. Earlier cache-only throughput measurements should not be interpreted as current-path capacity guarantees.

## Capacity and behavior

The database pool defaults to 24 connections per process (two processes consume up to 48). Account for every replica and migration process when sizing PostgreSQL. SQLx pool acquisition times out after three seconds. Concurrency is bounded at 256 per API process; Redis rate limits are shared. List responses have a 15-second optional cache, while authorization and private detail responses remain database-backed.

Indexing processes confirmed ranges of up to 50 blocks and fetches at most eight distinct receipts/blocks concurrently. The worker runs one allocation job at a time per process; CPU construction uses a blocking thread. Job leases last five minutes, execution is capped at four minutes, and retries are bounded to eight attempts. Lost leases cannot commit a result. Multiple workers use PostgreSQL `SKIP LOCKED`; additional indexers serialize cursor commits. Benchmark on the actual RPC, token contracts, storage provider, dataset, and hardware before choosing production targets.

Allocation jobs cannot complete after the review deadline. The API requires at least two minutes remaining when eligibility is locked. Allow a larger operational margin for large campaigns, congestion, RPC failures, and creator finalization. Claim rights and cutoff checks depend on timestamps checked in requests/contracts, not on a maintenance cron advancing campaign labels.

## Monitoring

- `/v1/health/live`: process liveness.
- `/v1/health/ready`: PostgreSQL, Redis, and an unhalted, fresh indexer no more than 100 confirmed blocks behind. It does not continuously probe storage.
- `/v1/metrics`: request/failure totals, aggregate latency, pool occupancy, and indexer timestamp. These are basic counters, not a complete distributed tracing or histogram system.
- PostgreSQL: pool saturation, lock waits, connection limits, failed jobs, pending jobs approaching review deadlines, pending transaction age, and indexer lag/halt reasons.
- Storage/RPC: request errors, upload failures, publication errors, provider throttling, and discrepancies between provider heads.

Logs are JSON with safe error codes and HTTP request IDs. Configure central retention and alerting; do not enable raw payload or credential logging. The worker retries transient errors. An indexer continuity halt requires an operator investigation.

## Recovery

**Database unavailable:** fail readiness and database-backed operations; restore connectivity or invoke the database recovery procedure. Do not treat Redis as a substitute for authoritative state.

**Redis unavailable:** authenticated/business writes fail closed under the rate-limit policy. Public reads remain bounded locally. Restore Redis, check readiness, and expect previously cached listings to repopulate.

**RPC outage or stale indexer:** transaction preparation/public proofs fail closed. Restore RPC access, verify chain and bytecode pins, then allow indexing to catch up. User wallet transactions already sent must be reconciled from canonical logs after recovery.

**Worker crash:** its unfinished lease expires and another worker resumes the same snapshot/seed. Objects use immutable keys and byte equality checks. An exhausted crashed eighth attempt is marked failed during maintenance. Do not generate a new seed to retry a campaign. Investigate the recorded safe error, deadline, artifact availability, and deployment state before a controlled retry.

**Reorganization/continuity mismatch:** indexing halts. Freeze affected write paths, compare independent provider history, identify the canonical divergence, and restore/replay projections from a consistent database checkpoint preceding the divergence. This release deliberately does not implement arbitrary automated deep-reorg rollback. Do not simply clear a halt flag while keeping incompatible claim counters or finalization projections.

**Integrity error:** do not publish alternate proofs or override the mismatched root. Preserve artifacts and audit records, inspect creator finalization and the verified contract, and communicate the affected campaign state through the product.

**Lost encryption key:** unrevealed raffle seeds cannot be recovered. Back up the key securely alongside database recovery procedures. This release supports one active seed key and has no automatic rotation/key-version migration. Keep the existing key until all affected raffle seeds are revealed or implement and test an explicit re-encryption migration.

## Retention and backup

Maintenance removes expired challenges/sessions, expired idempotency rows, abandoned staging uploads, and completed staging copies. Private completed evidence, eligibility artifacts, manifests, audit records, and campaign history are not automatically erased. Configure and document their retention deliberately. Apply staging lifecycle rules separately from immutable finalized manifests; deleting active claim artifacts breaks proof availability even though onchain rights remain.

Back up PostgreSQL, finalized artifacts, and the seed encryption key as a consistent recovery set. Test recovery in isolation, verify recorded manifest hashes and deployed roots, and rerun the indexer against canonical history before opening traffic. Never restore production user credentials or evidence into public development environments.

## Rollout and shutdown

The API and worker respond to SIGTERM. API requests drain; the worker allows up to 30 seconds before exit. A long allocation may be recovered through lease expiry. Set an orchestrator grace period above 30 seconds, remove readiness before termination, and do not run destructive schema changes alongside old instances. Review migrations and take a recoverable backup before each release.
