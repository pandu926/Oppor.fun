# Spesifikasi Backend

## 1. Lingkup dan stack

Backend menyimpan campaign, bukti, review, hasil raffle, alokasi, proof, dan proyeksi event escrow. Tidak menjalankan API X, scraper X, OAuth X, ataupun pemeriksaan otomatis X.

| Komponen | Pilihan |
|---|---|
| Bahasa | Rust stable, versi dipin melalui `rust-toolchain.toml` |
| HTTP | Axum, Tokio, Tower / tower-http |
| Database | PostgreSQL melalui SQLx |
| Cache / rate limit | Redis |
| EVM | Alloy; ABI berasal dari artifact kontrak yang sama |
| Data | Serde, UUID, UTC timestamp, integer/U256 |
| Observability | tracing, metrik Prometheus-compatible |
| Evidence / manifest | Object storage S3-compatible; provider belum dipilih |
| Pengujian | Rust integration tests, PostgreSQL/Redis isolated, Foundry untuk kontrak |

Axum mendukung handler, extractor, shared state, dan middleware Tower. SQLx dipakai untuk query dan transaksi. Ikuti dokumentasi resmi dan pin versi dependency, bukan `latest`: [Axum](https://docs.rs/axum/latest/axum/), [SQLx](https://docs.rs/sqlx/latest/sqlx/).

## 2. Struktur aplikasi

```text
backend/
  Cargo.toml
  Cargo.lock
  rust-toolchain.toml
  migrations/
  src/
    main.rs
    config.rs
    state.rs
    error.rs
    http/{routes,auth,campaigns,entries,reviews,allocations,uploads}.rs
    domain/{campaign,entry,reward,distribution}.rs
    services/{auth,campaign,review,snapshot,raffle,allocation}.rs
    repositories/{campaigns,entries,jobs,chain_events}.rs
    chain/{client,abi,indexer,transactions}.rs
    crypto/{wallet_auth,merkle,raffle,canonical_json}.rs
    workers/{runner,cutoff,indexer,manifest,reconcile}.rs
  tests/
```

Mulai sebagai satu codebase. Binary API dan worker dapat dijalankan terpisah. API tidak memegang database transaction saat menunggu RPC atau object storage.

`AppState` berisi `PgPool`, Redis connection manager, RPC client read-only, object storage client, konfigurasi, dan clock. Handler memanggil service; service menangani aturan dan transaction; repository menangani query.

## 3. Representasi data

- Waktu API: ISO-8601 UTC. Database: `timestamptz`. Kontrak: Unix seconds.
- Alamat EVM: `bytea` sepanjang 20 byte di DB; hex `0x...` pada API. Normalize sebelum uniqueness; jangan case-sensitive string equality.
- Hash: `bytea` sepanjang 32 byte di DB; hex pada API.
- Chain ID: string integer desimal pada API/manifest, integer tervalidasi di Rust/DB. Placeholder pada contoh wajib diganti dengan chain ID environment; bukan nama jaringan.
- Amount/token ID: integer desimal string pada API, `numeric(78,0)` di DB, U256 di Rust. Terapkan batas `0 <= value <= 2^256-1`; SQL numeric saja tidak menegakkan batas U256.
- Jumlah reward selalu base units; jangan float. `decimals` hanya untuk format display.
- JSON aturan diserialisasi kanonis dan di-hash; metode di [dokumen integrasi](INTEGRATION_AND_TESTING.md).
- UUID adalah ID offchain. `campaign_key` bytes32 ditetapkan sekali; tidak diregenerasi saat retry.
- DB transaction menggunakan database clock untuk penerimaan pengajuan. Diterima jika validasi/write memperoleh lock sebelum cutoff dan timestamp DB masih `< cutoff`; request masuk ke proxy sebelumnya tidak menjamin diterima.
- Evaluasi batas waktu memakai `clock_timestamp()` setelah memperoleh lock; `now()` PostgreSQL adalah waktu awal transaction dan dapat stale setelah menunggu lock. Jadwal harus whole seconds agar representasi kontrak dan backend identik.

## 4. State machine

### Campaign

| State | Makna / transisi |
|---|---|
| `DRAFT` | Tugas/reward bisa diedit. |
| `CONFIG_LOCKED` | Konfigurasi pendanaan dikunci; transaksi escrow sedang disiapkan. |
| `FUNDING` | Escrow tercatat, pendanaan/aktivasi belum dikonfirmasi. |
| `SCHEDULED` | Escrow activated; waktu mulai belum tiba. |
| `ACTIVE` | Mulai tercapai, waktu belum cutoff. |
| `REVIEWING` | Cutoff tercapai; bukti tidak bisa diedit. |
| `ELIGIBILITY_LOCKED` | Semua submission yang diajukan telah direview; snapshot permanent dibuat. |
| `ALLOCATION_READY` | Manifest, proof, dan root lengkap, menunggu creator finalize. |
| `CLAIM_OPEN` | Event finalize telah diverifikasi; waktu claim belum selesai. |
| `COMPLETED` | Seluruh alokasi diklaim atau masa claim berakhir. Tidak berarti semua reward sudah ditarik. |
| `CANCELLED` | Kontrak membatalkan escrow sebelum activation. |
| `EXPIRED` | Tidak difinalisasi sampai review deadline. |
| `CLOSED` | Event sweep sisa berhasil. |

Status waktu dapat diturunkan dari jadwal dan state onchain. Tidak bergantung pada cron agar submission ditolak tepat waktu. Status offchain tidak dapat membatalkan hak onchain.

### Entry

`REGISTERED → SUBMITTED → ELIGIBLE | DISQUALIFIED`

- `REGISTERED` yang tidak submit sebelum cutoff menjadi `NOT_SUBMITTED`.
- Untuk raffle, eligible menjadi `WINNER` atau `NOT_SELECTED` pada hasil distribusi, tanpa mengubah keputusan review.
- Alokasi claim disimpan terpisah: `UNCLAIMED → CLAIMED` hanya setelah event valid.
- `PENDING_REVIEW` tidak masuk undian. Lock menolak bila masih ada entry `SUBMITTED` tanpa keputusan.
- Review dapat direvisi sebelum snapshot lock; semua revisi tercatat. Setelah lock, review dan wallet payout tidak dapat diganti.

## 5. Autentikasi dan otorisasi

### Wallet login

Gunakan challenge bertipe Sign-In with Ethereum/EIP-4361:

1. Server membuat nonce acak single-use dengan TTL 5 menit.
2. Message memuat domain, URI, chain ID, address, nonce, issued-at, expiration-time.
3. Server memverifikasi semua field terhadap konfigurasi, bukan hanya recovery signature.
4. EOA menggunakan verifikasi signature standar; wallet kontrak menggunakan EIP-1271 pada chain yang disetujui.
5. Nonce dikonsumsi atomik. Login paralel dengan nonce sama hanya satu berhasil.
6. Buat session acak; simpan hash token, bukan token asli, di PostgreSQL. Cookie `HttpOnly`, `Secure`, `SameSite=Lax` dengan expiry. Usulan default session 7 hari.

Nonce dan session authoritative di PostgreSQL; Redis boleh cache. Tidak ada private key user di server. Nonce berhasil login tidak boleh dipakai lagi meski server crash.

### Hak akses

- Publik: browse campaign, aturan, statistik, manifest final, dan proof publik yang sudah dibuka.
- Peserta: entry/bukti sendiri dan status review sendiri.
- Creator: campaign miliknya, bukti peserta campaign itu, review, snapshot lock, dan prepare finalize.
- Moderator: hide listing abusive. Tidak boleh mengganti root atau mengambil reward.
- Worker: index event dan menghitung artifacts. Tidak otomatis mendapatkan hak creator.

Cookie mutation membutuhkan pemeriksaan `Origin` serta CSRF token. CORS memakai daftar origin spesifik. Jangan wildcard credentialed CORS.

## 6. Skema PostgreSQL

DDL berikut adalah model target, bukan migration yang telah dijalankan. Semua tabel memakai UTC `created_at`; mutable rows memakai `updated_at` dan optimistic `version` jika diperlukan.

| Tabel | Kolom utama | Constraint / index penting |
|---|---|---|
| `users` | id, primary_wallet, display_name | unique(primary_wallet), length(address)=20 |
| `auth_nonces` | nonce_hash, wallet, chain_id, expires_at, consumed_at, message_hash | unique(nonce_hash) |
| `sessions` | id, user_id, token_hash, expires_at, revoked_at | unique(token_hash), index(user_id) |
| `campaigns` | id, campaign_key, creator_id, creator_wallet, chain_id, escrow_address, slug, title, description, status, mode, start_at, cutoff_at, review_deadline, claim_deadline, capacity, winner_count, rules_json, config_hash, version | unique(chain_id,campaign_key), unique(chain_id,escrow_address), unique(slug); ordered timestamps |
| `campaign_rewards` | id, campaign_id, asset_kind, token_address, decimals, token_id, target_amount, funded_amount, swept_amount | unique(campaign_id); positive target |
| `reward_nft_inventory` | campaign_id, token_id, state, allocation_id | unique(campaign_id,token_id); untuk ERC-721 |
| `tasks` | id, campaign_id, position, task_type, target_url, instructions, required, evidence_schema | unique(campaign_id,position) |
| `entries` | id, campaign_id, user_id, payout_wallet, slot_number, x_username_declared, discord_username_declared, status, submitted_at, latest_review_id | unique(campaign_id,user_id), unique(campaign_id,payout_wallet), unique(campaign_id,slot_number) |
| `submissions` | id, entry_id, task_id, revision, evidence_json, object_key, received_at | unique(entry_id,task_id,revision) |
| `entry_reviews` | id, entry_id, reviewer_id, decision, reason, entry_version, created_at | append-only; index(entry_id,created_at) |
| `eligibility_snapshots` | id, campaign_id, snapshot_hash, manifest_key, eligible_count, locked_at | unique(campaign_id) |
| `snapshot_entries` | snapshot_id, ordinal, entry_id, payout_wallet | unique(snapshot_id,ordinal), unique(snapshot_id,payout_wallet) |
| `raffles` | campaign_id, seed_commitment, encrypted_seed, revealed_seed, algorithm_version, eligible_hash, winners_hash, executed_at | unique(campaign_id) |
| `allocation_manifests` | id, campaign_id, snapshot_id, root, manifest_hash, object_key, leaf_count, allocated_total, algorithm_version | unique(campaign_id); immutable after published |
| `allocations` | id, campaign_id, manifest_id, claim_index, entry_id, recipient, token_id, quantity, leaf_hash, proof_json, claim_tx_hash, claimed_at | unique(campaign_id,claim_index); qty positive |
| `chain_transactions` | chain_id, tx_hash, campaign_id, kind, expected_from, status, block_number, receipt_error | unique(chain_id,tx_hash) |
| `chain_events` | chain_id, block_number, block_hash, tx_hash, log_index, address, event_type, payload | unique(chain_id,tx_hash,log_index) |
| `indexer_cursors` | chain_id, factory_address, next_block, previous_block_hash | unique(chain_id,factory_address) |
| `jobs` | id, kind, dedupe_key, payload, state, attempts, available_at, locked_until, worker_id, last_error | unique(dedupe_key), index(state,available_at) |
| `outbox` | id, event_type, aggregate_id, payload, published_at | index(published_at) |
| `idempotency_keys` | actor_id, route, key, request_hash, state, response_status, response_json, expires_at | unique(actor_id,route,key) |
| `audit_logs` | id, actor_id, campaign_id, action, before_hash, after_hash, request_id, created_at | append-only |

Foreign keys menghubungkan semua child ke parent. Setelah konfigurasi dikunci, larang mutation tugas/reward/jadwal melalui service dan pembatasan DB yang relevan. Submission task harus berasal dari campaign entry yang sama; enforce composite foreign keys atau transaction validation, bukan FK terpisah saja.

Indexes tambahan: campaign(status,start_at), campaign(status,cutoff_at), entries(campaign_id,status), allocations(recipient,campaign_id), events(address,block_number).

### Transaction boundaries

- Registrasi: `SELECT ... FOR UPDATE` campaign, periksa status/waktu/kuota, insert entry dan increment counter dalam transaction yang sama. Unique constraints tetap pertahanan terakhir.
- Submission: lock campaign dan entry; periksa wallet, cutoff, seluruh required task, insert revision; audit/outbox ikut transaction.
- Review: lock campaign lalu entry; cek creator, status reviewing, versi; append review dan update keputusan atomik.
- Lock eligibility: lock campaign; pastikan tidak ada submitted entry belum direview; snapshot seluruh eligible dengan urutan kanonis; insert job/outbox; commit. Tidak ada review baru setelah lock.
- Publication manifest: transaction menyimpan allocations dan manifest lengkap; artifacts harus tersedia sebelum status ready dipublikasikan.

Gunakan urutan lock campaign → entry di semua endpoint untuk mengurangi deadlock. Retry serialization/deadlock secara terbatas; operasi tetap idempotent.

## 7. Redis

| Key | Kegunaan | Usulan TTL |
|---|---|---|
| `ratelimit:ip:{hash}:{bucket}` | Auth/upload/public requests | sesuai window |
| `ratelimit:user:{id}:{route}` | Submission/review flood | sesuai window |
| `campaign:list:{queryHash}` | Listing publik | 30 detik |
| `campaign:stats:{id}` | Statistik derived | 15 detik |
| `asset:metadata:{chain}:{token}` | Decimals/name/symbol | 1 jam |
| `indexer:health:{chain}` | Health hint | 30 detik |

Rate limit increment + expiry atomik melalui Lua atau primitive yang sesuai. Redis lock bukan perlindungan dana/kuota/snapshot. Semua exclusivity bisnis memakai PostgreSQL transaction dan constraints.

Saat Redis mati: database state tetap utuh; listing baca PostgreSQL; authenticated mutations menggunakan fallback rate limit terbatas atau `503`. Tidak memulai mutation sensitif tanpa autentikasi hanya karena cache gagal.

## 8. API REST `/v1`

### Konvensi

- JSON, cursor pagination, default page 25, maksimum 100.
- Amount berupa string; address hex; timestamps UTC.
- Mutation non-idempotent menerima `Idempotency-Key`.
- `request_id` di response/error dan logs; jangan expose SQL/RPC internals.
- Usulan format error: `{ "error": { "code": "CUTOFF_PASSED", "message": "Campaign sudah ditutup.", "request_id": "..." } }`.
- `400` format invalid; `401` tanpa session; `403` salah role; `404` resource tak tersedia; `409` state/kuota/version conflict; `422` aturan invalid; `429` rate limit; `503` dependency belum tersedia.

### Auth dan publik

| Method / path | Input atau hasil |
|---|---|
| `POST /auth/challenge` | wallet + chain_id → message + nonce expiry |
| `POST /auth/verify` | message + signature → session cookie |
| `POST /auth/logout` | revoke session |
| `GET /me` | wallet dan profile |
| `GET /campaigns` | filter status, asset_kind, mode; sort newest/ending; cursor |
| `GET /campaigns/{id}` | detail, funded reward, jadwal, tugas, aturan, verification_mode=MANUAL |
| `GET /campaigns/{id}/results` | snapshot hash, penerima dan manifest final; tanpa evidence pribadi |
| `GET /campaigns/{id}/manifest` | URL immutable manifest + hash |
| `GET /health/live` | proses hidup |
| `GET /health/ready` | DB, worker/indexer freshness dan dependency diperlukan |

### Creator

| Method / path | Aturan |
|---|---|
| `POST /campaigns` | create draft |
| `PATCH /campaigns/{id}` | draft saja, expected_version |
| `POST /campaigns/{id}/tasks` | draft saja |
| `PATCH /campaigns/{id}/tasks/{task_id}` | draft saja |
| `DELETE /campaigns/{id}/tasks/{task_id}` | draft saja |
| `POST /campaigns/{id}/lock-config` | validasi dan hash konfigurasi; membuat seed commitment jika raffle |
| `POST /campaigns/{id}/prepare-create` | unsigned factory calldata, target chain dan expected state |
| `POST /campaigns/{id}/prepare-fund` | unsigned approve/deposit calldata, jumlah remaining |
| `POST /campaigns/{id}/prepare-activate` | hanya bila receipt funding dan deadline valid |
| `POST /campaigns/{id}/transactions` | tx_hash + kind; hint indexer, bukan bukti sukses |
| `GET /campaigns/{id}/entries` | hanya creator, filter review state |
| `GET /campaigns/{id}/entries/{entry_id}/evidence` | hanya creator/peserta terkait |
| `POST /campaigns/{id}/entries/{entry_id}/review` | ELIGIBLE/DISQUALIFIED + alasan + expected_version |
| `POST /campaigns/{id}/lock-eligibility` | freeze, validasi pending review, enqueue allocation |
| `GET /campaigns/{id}/allocation-preview` | root, manifest hash, totals, warnings nyata |
| `POST /campaigns/{id}/prepare-finalize` | unsigned escrow finalize; hash harus identik preview |
| `POST /campaigns/{id}/prepare-cancel` | escrow DRAFT saja |
| `POST /campaigns/{id}/prepare-sweep` | mengikuti contract timestamp/state |

### Peserta

| Method / path | Aturan |
|---|---|
| `POST /campaigns/{id}/entries` | register wallet session dan identitas sosial self-declared |
| `GET /campaigns/{id}/my-entry` | state dan bukti sendiri |
| `PUT /campaigns/{id}/my-entry/submissions/{task_id}` | revision evidence sebelum cutoff |
| `POST /campaigns/{id}/my-entry/submit` | required evidence lengkap; status SUBMITTED |
| `POST /uploads/presign` | ukuran/type terbatas, entry ownership |
| `POST /uploads/{upload_id}/complete` | cek storage object sebelum attach evidence |
| `GET /campaigns/{id}/allocations/{wallet}` | manifest/proofs final; wallet proof publik, evidence tidak |
| `POST /campaigns/{id}/prepare-claim` | claim_index → unsigned claim calldata |

Tidak ada endpoint `POST /claim` yang meminta server memegang private key atau mengirim reward. Backend menyiapkan calldata; wallet penerima mengirim transaksi.

### Contoh draft

```json
{
  "title": "Promosi token contoh",
  "chain_id": "CHAIN_ID_FROM_CONFIG",
  "reward": {
    "asset_kind": "ERC20",
    "token_address": "0x1111111111111111111111111111111111111111",
    "amount_base_units": "200000000000000000000000000"
  },
  "distribution": {"mode": "RAFFLE", "winner_count": 100},
  "start_at": "2026-10-12T00:00:00Z",
  "cutoff_at": "2026-10-19T00:00:00Z",
  "review_deadline": "2026-10-22T00:00:00Z",
  "claim_deadline": "2026-11-05T00:00:00Z",
  "tasks": [{"type": "X_REPOST", "target_url": "https://x.com/example/status/123", "required": true}]
}
```

Tanggal/jumlah/alamat ini hanya contoh; API chain_id nyata berupa string integer desimal dari konfigurasi deployment. Backend menolak URL target invalid, jumlah nol, atau jadwal yang tidak berurutan.

## 9. Evidence

- Self-declared handle tidak dipakai sebagai autentikasi atau dedup manusia.
- Bukti wajib hanya kelengkapan format; status tidak otomatis eligible.
- Tautan X boleh disimpan tanpa server mengambil kontennya.
- Jangan fetch arbitrary URL dari input; hindari SSRF dan jangan mengikuti redirect internal.
- File usulan maksimum 5 MB; JPEG/PNG/WebP; validasi magic bytes; jangan izinkan HTML/SVG aktif.
- Storage private dengan URL bertanda tangan TTL pendek. Manifest final publik tidak berisi screenshot/handle/private evidence.
- Draft unggahan kedaluwarsa dibersihkan worker. Retensi evidence setelah campaign ditetapkan dalam kebijakan produk; usulan 90 hari, bukan angka yang sudah disepakati.

## 10. Jobs dan indexer

Jobs durable di PostgreSQL. Worker mengambil batch kecil menggunakan `FOR UPDATE SKIP LOCKED`, memberi lease, lalu menjalankan operasi di luar transaction. Lease expired memungkinkan retry; dedupe key memastikan efek idempotent, bukan exactly-once delivery.

Jobs: `CLOSE_SUBMISSIONS`, `BUILD_SNAPSHOT_ARTIFACT`, `RUN_RAFFLE`, `BUILD_ALLOCATIONS`, `INDEX_LOG_RANGE`, `RECONCILE_ESCROW`, `EXPIRE_CAMPAIGN`, `CLEAN_UPLOADS`.

Retry usulan exponential backoff 1s, 5s, 30s, 2m, 10m; setelah batas masuk failed/dead-letter yang terlihat operator. Review/claim tidak hilang karena job gagal.

Indexer:

1. Baca factory logs dari deployment block dan discover escrow resmi.
2. Untuk rentang blok, baca log hanya address factory/escrow yang terdaftar.
3. Verifikasi receipt success, event signature, emitter, chain ID, creator/config hash dan parameters.
4. Insert event, update proyeksi, dan cursor pada transaction DB yang sama.
5. Unique event key mencegah double count. Jangan menjumlahkan USDC Transfer dan event escrow sebagai dua reward.
6. Replay event membangun state yang sama. Tx hash dari frontend tidak cukup untuk funded/finalized/claimed.
7. Simpan block hash dan checkpoint; provider inconsistency menghentikan projection dan memicu reconcile. Finality tidak membenarkan mengabaikan wrong RPC atau database restore.

Cursor dan daftar escrow tetap di DB. Batasi rentang `eth_getLogs`, retry dengan rentang lebih kecil saat provider membatasi response. Event claim dari semua penerima dipantau walau tidak ada frontend callback.

## 11. Konfigurasi dan operasi

```text
DATABASE_URL
REDIS_URL
HTTP_BIND
PUBLIC_ORIGIN
ALLOWED_ORIGINS
ARC_RPC_URL
ARC_CHAIN_ID
FACTORY_ADDRESS
FACTORY_DEPLOYMENT_BLOCK
OBJECT_STORAGE_ENDPOINT
OBJECT_STORAGE_BUCKET
OBJECT_STORAGE_ACCESS_KEY
OBJECT_STORAGE_SECRET_KEY
RAFFLE_SEED_ENCRYPTION_KEY
SESSION_TTL_SECONDS
INDEXER_MAX_LAG_SECONDS
```

Tidak ada `X_API_KEY` atau private key pengguna. Raffle key melindungi seed unrevealed; rotasi key harus menjaga kemampuan membuka campaign yang sudah berjalan.

- Readiness gagal jika DB tidak dapat dijangkau. RPC/indexer stale menonaktifkan prepare transaksi sensitif dengan error jelas; browsing data cached bisa tetap tersedia.
- Structured logs: request ID, campaign ID, job ID, chain ID, tx hash. Redact session, signature, evidence pribadi, credentials.
- Metrik: latency/error rate, DB pool saturation, jobs pending/retries, indexer lag, review backlog, manifest availability, failed claim simulations.
- Backup PostgreSQL + manifest/object storage; lakukan restore drill dan replay chain events.
- Shutdown graceful: stop menerima request, selesaikan/lepaskan job leases, flush telemetry.
- SQLx checked queries memakai schema yang dimigrasikan dan metadata offline bila digunakan; CI memastikan migrations dan query metadata tidak stale.

## 12. Batas MVP

MVP tetap mencakup proses lengkap draft → escrow → manual review → raffle/all eligible → claim. Tidak memasukkan AI, API X, indexing seluruh token Arc, private-key custody, sponsor gas, marketplace NFT, ataupun smart contract upgrade otomatis.

NFT dan ERC-1155 memiliki spesifikasi di dokumen kontrak. Implementasi dapat dirilis bertahap dengan UI hanya menawarkan asset kind yang benar-benar didukung deployment; jangan menerima reward yang belum bisa didistribusikan.
