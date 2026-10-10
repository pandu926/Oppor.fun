# Oppor backend

Rust backend for token promotion campaigns on Arc. The API and worker are separate binaries built from one codebase. All application code, messages, database migrations, and operational documentation use English.

## Stack

Rust 1.97.1, Axum 0.8.9, Tokio, SQLx 0.8.6, PostgreSQL, Redis, Alloy ABI encoding, S3-compatible object storage. Dependency versions are resolved in the committed `Cargo.lock`; build with `--locked`.

## Implemented flow

1. Sign in with an EIP-4361 wallet challenge. EOA and deployed EIP-1271 contract wallets are supported.
2. Create and edit a campaign draft, including social tasks and ERC-20 / ERC-721 / ERC-1155 reward configuration.
3. Lock the rules and deadlines; prepare factory creation, approvals, funding, and activation transactions.
4. Register participants and accept evidence before cutoff. Uploads use signed PUT URLs binding the exact Content-Length and Content-Type, compatible with private R2 buckets.
5. Review submissions manually after cutoff. There are no X API calls, X OAuth, scrapers, or automatic social-verification claims.
6. Lock eligibility and process a durable allocation job. Support all-eligible distributions, fixed rewards, and reproducible server raffles.
7. Publish immutable manifests and Merkle proofs; prepare creator finalization and recipient claim transactions.
8. Index verified factory/escrow logs and project funding, finalization, claims, cancellation, and refunds.

The backend never stores user private keys or sends user transactions. Users execute the returned calldata with their wallets.

## Run

Install the pinned Rust toolchain. Configure an existing PostgreSQL database, Redis, private S3 bucket, and verified factory/escrow deployment. Copy `.env.example` to `.env`, replace all deployment placeholders, and generate the two encryption/rate-limit keys independently. The binaries do not automatically read `.env`; load it through your process manager or shell.

```bash
set -a
source .env
set +a
cargo run --locked --bin oppor-api -- migrate
cargo run --locked --bin oppor-api
# In a separate process with the same environment:
cargo run --locked --bin oppor-worker
```

`APP_ENV=local` is the explicit opt-in for HTTP development and an insecure local cookie. Every other value defaults to production behavior. Production requires HTTPS origins, RPC, and object storage, PostgreSQL `sslmode=verify-full`, and `rediss://`; the session uses a `__Host-` Secure HttpOnly cookie. Serve behind a TLS reverse proxy, restrict the API's network ingress, and protect PostgreSQL/Redis/storage credentials with your secret manager. Forwarded IP headers are ignored unless the immediate peer matches `TRUSTED_PROXY_CIDRS`.

Only a verified contract deployment should populate `FACTORY_ADDRESS`, `FACTORY_CODE_HASH`, and `ESCROW_CODE_HASH`. The escrow hash pins exact runtime bytecode; deployments that embed different per-campaign Solidity immutables require a matching runtime-code verification strategy before integration. The [implemented factory/escrow](../contracts/README.md) deliberately stores configuration instead of embedding per-campaign immutables and exports a uniform runtime hash. No live Oppor deployment has been performed.

## API

Base path: `/v1`. See [the API reference](API.md) and [OpenAPI specification](openapi.json).

- Sign-in: `POST /auth/challenge`, `POST /auth/verify`.
- Session: `GET /me`, `POST /auth/logout`.
- Campaigns: `GET/POST /campaigns`, `GET /campaigns/mine`, `GET/PATCH /campaigns/{id}`.
- Rules: task editing, `POST /campaigns/{id}/lock-config`.
- Participation: registration, latest evidence, explicit submission, and private evidence access.
- Review: creator-only decisions, optimistic versions, eligibility locking.
- Distribution: creator preview, public final results/manifest/proofs, unsigned claim preparation.
- Transactions: unsigned factory/funding/activation/finalization/cancel/refund preparations and tracking hints.
- Operations: `/health/live`, `/health/ready`, `/metrics`.
- Administration: nine `/admin/*` endpoints for moderation, account suspension/session revocation, statistics, audit history, and job monitoring; see [ADMIN.md](ADMIN.md).

Unsafe requests require an exact allowed `Origin`. Session-authenticated mutations also require `X-CSRF-Token`. Business mutations require `Idempotency-Key` (8–128 alphanumeric, `-`, `_`, or `.` characters). The key is scoped to actor and operation and expires after 24 hours. Reuse the same key when retrying the same request; a changed payload returns `409`.

Amounts, token IDs, chain IDs, and claim indices in returned proofs are decimal strings. Amounts are base units, never floating-point numbers. Campaign edits wrap the complete draft in `{"expected_version": 0, "campaign": {...}}`; task and entry mutations also use optimistic versions.

## Test

```bash
cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings
bash scripts/test.sh
python3 scripts/security-audit.py
python3 -m venv target/api-validator
target/api-validator/bin/pip install -r scripts/requirements.txt
target/api-validator/bin/python scripts/validate-openapi.py
```

`scripts/test.sh` starts isolated PostgreSQL, Redis, and Moto S3 containers, compiles the actual factory/escrow and local token/wallet fixtures with Foundry, runs unit/property tests, and runs the ignored integration suite with a temporary Anvil process. Ports are bound to loopback. Override test connection URLs using `TEST_DATABASE_URL`, `TEST_REDIS_URL`, and `TEST_STORAGE_ENDPOINT`.

The OpenAPI validator also checks captured real integration responses against typed endpoint schemas. To verify the container with the default test-service ports, build `docker build -t oppor-backend:verification .` and run `python3 scripts/container-smoke.py`. This tests non-root/read-only execution, migrations, liveness, unavailable-indexer readiness, and graceful shutdown without a production deployment.

The main end-to-end test deploys the actual factory/escrow implementation and executes ERC-20 funding/finalization/claims on Anvil; only the test token and wallet are fixtures. It verifies Rust/Solidity hashes, concurrent registrations, CSRF, nonce replay, private evidence, optimistic review, idempotency, and failed transfers. Solidity tests additionally exercise ERC-721 and ERC-1155 funding, claims, refunds, receiver rejection, and reentrancy. NFT allocation has Rust coverage; API-to-chain NFT flows and live token behavior still require deployment validation.

## Security and operations

See [security boundaries](SECURITY.md), [operations runbook](RUNBOOK.md), and [verification results](VERIFICATION.md).

Security controls and passing tests do not establish an independent security audit. Production launch still needs verified/audited escrow contracts, environment-specific load testing, least-privilege infrastructure credentials, and a tested backup/restore setup. No mainnet transactions or deployment are performed by this backend implementation task.
