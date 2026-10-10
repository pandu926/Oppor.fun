# Production operations

Oppor runs at `https://oppor.fun` behind Cloudflare and the existing VPS reverse proxy. This runbook covers the deployed infrastructure, contract activation, release publishing, and recovery. Commands assume `/root/arc` on the production VPS.

## Deployment status

The frontend, documentation, and wallet deployment page are served over verified public HTTPS. Cloudflare uses Full (strict) encryption to the origin. PostgreSQL and Redis run on an internal Docker network with persistent volumes, verified TLS, and no published database ports. Five database migrations have been applied. The Rust runtime image has been built and exercised against those services. Private R2 storage has been tested with signed PUT requests and metadata tampering checks. An encrypted backup has been uploaded to R2 and restored into an isolated database successfully.

Campaign API activation is pending an Arc mainnet factory deployment. Until a deployment is verified, `/v1/` returns a structured 503 and the frontend uses its existing demo data configuration. Infrastructure readiness does not mean real campaign transactions are enabled.

## Service layout

| Component              | Configuration                 | Exposure                                                    |
| ---------------------- | ----------------------------- | ----------------------------------------------------------- |
| Shared origin proxy    | `ops/nginx/origin.conf`       | Public HTTP and HTTPS; only Oppor virtual hosts are managed |
| Static frontend        | `ops/compose.web.yml`         | Private port 18820 behind the origin proxy                  |
| PostgreSQL             | `ops/compose.production.yml`  | TLS, internal service network                               |
| Redis                  | `ops/compose.production.yml`  | TLS, authentication, internal service network               |
| API                    | Compose `application` profile | Private port 18821; activated after factory verification    |
| Indexer and jobs       | Compose `application` profile | No inbound ports                                            |
| Migration runner       | Compose `operations` profile  | One-shot, database access only                              |
| Evidence and artifacts | R2 `oppor-production`         | Private; signed access                                      |
| Recovery archives      | R2 `oppor-backups`            | Private; encrypted before upload                            |

Credentials are stored under `/root/.config/oppor`, outside the repository. Do not put environment files, backup keys, TLS keys, or Cloudflare credentials into Git. The checked-in Compose files intentionally refer to private host paths.

## Deploy the release factory

1. Open `https://oppor.fun/setup/deploy` and connect a browser wallet.
2. Prepare the deployment. The page verifies Arc mainnet chain ID `5042`, checks the wallet account, estimates gas, and checks native USDC for the network fee.
3. Review the network, account, bytecode hash, zero native transfer value, and fee estimate. Sign the contract creation transaction from the wallet.
4. After two confirmations and runtime bytecode verification, download the deployment manifest.
5. Place the manifest on the VPS. It contains public deployment metadata, not a private key.

The factory has no platform owner or upgrade key. Deploying it does not assign an application administrator or fund a campaign. Failed confirmation polling can be retried using the displayed transaction hash without sending a second deployment transaction.

## Activate campaign transactions

Optional administrator wallets are configured explicitly. Set `ADMIN_WALLETS` to a comma-separated EVM address allowlist when running activation; an existing allowlist is preserved when the variable is omitted. An empty allowlist grants no administration access. A Reown project identifier enables WalletConnect QR and mobile connections; browser wallet connections work without it. Configure `oppor.fun` in the Reown project allowlist.

```sh
# Optional public configuration; never provide a wallet private key.
export ADMIN_WALLETS=0xYourAdministratorAddress
export VITE_WALLETCONNECT_PROJECT_ID=your_reown_project_id
bash ops/activate.sh /secure/path/oppor-mainnet-deployment.json
```

Replace the placeholders with valid values or omit the optional variables. Activation verifies the RPC chain, creation transaction input, zero native value, successful receipt, contract address, two confirmations, and the release runtime code hash. It derives the deployment block from the receipt rather than trusting supplied metadata. A wrong network, stale build, or mismatched contract stops activation.

The script runs migrations, applies runtime grants, starts the API and worker, and requires database, Redis, and verified chain-indexer readiness before publishing the live frontend and public API route. If readiness fails, public API routing is not enabled. Diagnose the services before retrying:

```sh
docker compose -f ops/compose.production.yml --profile application ps
docker compose -f ops/compose.production.yml logs --tail 100 api worker
curl --fail http://127.0.0.1:18821/v1/health/ready
```

`/v1/metrics` remains private. Readiness reports dependency health and indexer freshness; process liveness alone is insufficient for activation.

## Database identities and migrations

`oppor_owner` is the private PostgreSQL bootstrap role. API and worker never use its credentials. `oppor_migrator` owns the database and schema but has no superuser, role-creation, or database-creation privileges. `oppor_app` has restricted application access and cannot modify migration history or update/delete protected append-only operational records. Database triggers provide additional append-only enforcement.

```sh
docker compose -f ops/compose.production.yml --profile operations run --rm migrate
docker compose -f ops/compose.production.yml exec -T postgres \
  psql -U oppor_migrator -d oppor -v ON_ERROR_STOP=1 < ops/runtime-grants.sql
```

Migration mode needs only a valid database URL and the CA certificate. It does not require a factory address, signing wallet, Redis, or storage credentials. Apply grants after every migration that introduces tables. Production PostgreSQL connections require `sslmode=verify-full`; Redis requires `rediss` with CA verification.

## Provision another VPS

Install Docker Compose, the pinned Rust toolchain, Node.js, OpenSSL, age, and the frontend dependencies. Restore Cloudflare credentials into the root-only credentials file. Build the backend runtime image and publish an initial frontend release before starting the web service. The shared origin proxy and Cloudflare DNS/TLS configuration must be installed separately; do not overwrite virtual hosts for other applications.

```sh
python3 ops/provision-services.py
docker compose -f ops/compose.production.yml up -d postgres redis
node ops/provision-runtime-role.mjs
node ops/configure-production.mjs
docker compose -f ops/compose.production.yml --profile operations run --rm migrate
docker compose -f ops/compose.production.yml exec -T postgres \
  psql -U oppor_migrator -d oppor -v ON_ERROR_STOP=1 < ops/runtime-grants.sql
```

Service provisioning preserves existing credentials and certificates. The frontend container must already exist when generating the application environment because its exact private proxy address is verified. For a recovered installation, restore the original seed encryption key and database before activation rather than creating a new raffle key for existing campaigns.

## Build and publish

```sh
(cd backend && cargo build --release --locked)
docker build -f ops/backend-runtime.Dockerfile -t oppor-backend:production .
(cd frontend && npm ci && npm run build)
node ops/publish-web.mjs
docker compose -f ops/compose.web.yml up -d web
```

The host-built runtime image uses a pinned Debian trixie base to match the host binaries' glibc requirements. The separate `backend/Dockerfile` remains available for a complete containerized source build. Never mix host binaries with an older runtime libc.

Publishing copies a complete build into `/var/lib/oppor/web/releases`, retains hashed assets in a shared directory, and atomically switches the relative `current` symlink. The frontend mounts the parent directory, so replacement releases appear without deleting files being served. `release.json` records the Git commit and publish time. Keep earlier releases for rollback; never change a hashed asset in place.

## Storage uploads

R2 accepts the signed PUT flow used by this release. The browser uploads the raw file body with its declared content type. The signature binds the exact content length, content type, and host, and expires after five minutes. Completion checks the object size, image signature, and digest before copying it into an immutable artifact path. Accepted uploads are PNG, JPEG, and WebP, up to 5 MiB.

Bucket CORS permits the production origin and required GET, HEAD, and PUT requests. Neither artifacts nor backup buckets have public anonymous access enabled. Use authenticated application responses and expiring signed URLs to retrieve private evidence.

## Backup and recovery

A daily root cron job runs `ops/backup.sh`. It takes a custom-format PostgreSQL dump and includes application configuration and service credentials, including the raffle seed encryption key, in the same recovery set. The archive is encrypted with age before it is uploaded to the private `oppor-backups` bucket. Local encrypted copies are retained for seven days. Remote archives are retained until an explicit retention policy is configured.

```sh
bash ops/backup.sh
bash ops/restore-check.sh /var/lib/oppor/backups/oppor-TIMESTAMP.tar.gz.age
```

The restore check decrypts the archive, creates a temporary database, restores the dump, verifies migration history, and removes the temporary database. It does not overwrite the production database. This check has been executed successfully for the initial production recovery archive.

Store an independent, secure copy of `/root/.config/oppor/production/backup-age.key` off the VPS. Losing that key makes encrypted backups unreadable. A VPS-only key is insufficient for disaster recovery. Full recovery also requires new service certificates, private configuration, restored database grants, deployment verification, and worker readiness; do not publish live traffic until those checks pass. Redis sessions and rate-limit counters are not included in database backups; users reauthenticate after recovery.

## TLS and ingress maintenance

Cloudflare Universal SSL terminates browser TLS. The origin certificate covers the apex and `www` and expires on 10 October 2027. Renew it before expiry, test the shared proxy configuration, and reload only after the test succeeds. The private service CA and PostgreSQL/Redis certificates have separate lifetimes; monitor both.

The origin trusts `CF-Connecting-IP` only from the current documented Cloudflare networks. It overwrites forwarded client addresses before passing requests to the frontend proxy. The backend trusts only that frontend container's exact private address. After recreating the frontend container, rerun `node ops/configure-production.mjs` and recreate the application services when active so the allowlist reflects the current address. Never trust all Docker clients or arbitrary forwarded IP headers.

## Validation performed

Backend unit tests, the campaign-to-claim security/concurrency integration suite, OpenAPI response validation, and Rust Clippy checks cover this release. Infrastructure smoke checks verify PostgreSQL/Redis TLS, restricted database rights, and the Arc mainnet RPC. Live R2 checks cover successful upload, signed metadata rejection, readback, and cleanup. Browser tests cover campaign flows, documentation, administration, wallet session invalidation, and deployment rejection guards. These are internal checks; they do not constitute an independent smart contract audit.
