# Integrasi, Distribusi, dan Pengujian

## 1. Sumber kebenaran

| Informasi | Sumber |
|---|---|
| Deskripsi, tugas, evidence, keputusan review | PostgreSQL + immutable snapshot artifacts |
| Reward resmi tersedia, activation, root final, claim, refund | Escrow onchain |
| Random seed dan hasil raffle awal | Artifact undian server yang dipublikasikan |
| Cache listing/rate limit | Redis, bukan authority transaksi |

API tidak boleh menandai claim sukses berdasarkan HTTP callback atau tx hash user. Receipt dan event harus diperiksa. Jika root onchain berbeda dari root yang dipublikasikan backend, blokir proof/calldata normal dan tampilkan integrity incident; jangan diam-diam membangun ulang alokasi dari root asing.

## 2. Config hash dan canonical serialization

Aturan bisnis menjadi JSON kanonis: schema version, tasks (urutan tetap), reward parameters/inventory, mode, capacity, winner count, formula pembagian, waktu, claim/refund policy, raffle algorithm dan seed commitment.

- Gunakan JSON Canonicalization Scheme / RFC 8785 dengan implementasi diuji; semua integer besar berupa strings.
- Normalisasi timestamp ke Unix seconds string; address ke lowercase hex; task IDs stabil.
- `rulesHash = keccak256(UTF8(canonicalRulesJson))`.
- `configHash = keccak256(abi.encode(config))` memakai urutan struct tepat dari artifact Solidity. Tidak memakai JSON hashing untuk menggantikan ABI hashing.
- Rules manifest tidak berisi screenshot atau identitas sosial pribadi.
- Setelah lock-config, editor tidak mengubah payload yang mengikat configHash. Perubahan berarti draft/deployment baru.

Raffle commitment dibuat sebelum rulesHash/configHash agar tidak ada dependency melingkar.

## 3. Merkle leaf v1

Satu leaf memiliki tuple ABI:

```text
uint256 chainId
address escrow
bytes32 campaignKey
uint256 index
address recipient
uint8 assetKind
address rewardToken
uint256 tokenId
uint256 quantity
```

Hash:

```solidity
bytes32 leaf = keccak256(bytes.concat(keccak256(abi.encode(
    block.chainid,
    address(this),
    campaignKey,
    allocation.index,
    allocation.recipient,
    uint8(assetKind),
    rewardToken,
    allocation.tokenId,
    allocation.quantity
))));
```

Double hash sesuai konvensi StandardMerkleTree; jangan `abi.encodePacked` untuk tuple ini. Rust ABI encoder menghasilkan bytes yang sama dengan Solidity. [Referensi OpenZeppelin Merkle Tree](https://github.com/OpenZeppelin/merkle-tree).

### Tree construction

Untuk menghindari perbedaan Rust dan generator lain, gunakan konvensi OpenZeppelin StandardMerkleTree yang dipin:

1. Bentuk leaf dari tuple dengan index yang sudah ditetapkan.
2. Sort leaf hashes ascending lexicographic bytes; simpan mapping claim index → tree position.
3. Buat array `tree` sepanjang `2*n-1`.
4. Masukkan leaf sorted ke posisi `tree[tree.len()-1-i]`.
5. Untuk node internal dari bawah ke atas: `tree[i] = hashPair(tree[2*i+1], tree[2*i+2])`.
6. `hashPair(a,b) = keccak256(min(a,b) || max(a,b))`, bytes32 comparison.
7. Root=`tree[0]`. Proof berupa sibling path hingga root, tanpa direction bits.

Jangan mengganti dengan algoritme duplicate-last atau promote-odd. Untuk n=1 root=leaf dan proof kosong. Untuk n=0 gunakan root zero, leafCount=0; tidak membangun array dengan panjang negatif.

Claim index adalah ID alokasi, bukan posisi leaf setelah sort. Domain chain/escrow/campaign mencegah reuse proof antar deployment. Proof publik boleh dibagikan; kontrak membatasi recipient/caller.

## 4. Manifest final

```json
{
  "schema_version": 1,
  "chain_id": "configured-chain-id",
  "escrow": "0x...",
  "campaign_key": "0x...",
  "config_hash": "0x...",
  "eligibility_hash": "0x...",
  "distribution_mode": "RAFFLE",
  "algorithm_version": "allocation-v1",
  "root": "0x...",
  "leaf_count": 100,
  "allocated_quantity": "100000000",
  "allocations": [
    {"index": "0", "recipient": "0x...", "token_id": "0", "quantity": "1000000", "proof": ["0x..."]}
  ]
}
```

Ini contoh bentuk, bukan manifest valid: ellipsis/chain placeholder harus diganti dan semua tuple lengkap tersedia pada export nyata. Root dihitung dari allocations, bukan dari manifestHash; manifestHash dihitung terakhir dari canonical manifest, sehingga tidak ada self-hash circular.

- Artifact ditulis ke content-addressed key, misalnya `manifests/{hash}.json`.
- Semua proof diperiksa lokal sebelum publication.
- Total quantities dan uniqueness diperiksa menggunakan integer exact.
- Archive manifest + eligible snapshot + raffle transcript; backup dan export untuk claimant agar outage API tidak menghentikan claim yang sudah terbuka.
- Jangan membutuhkan wallet login untuk membaca proof final; proof bukan secret.
- Endpoint proof juga memeriksa root onchain dan deadline terkini. Storage evidence tetap private.

## 5. Eligible snapshot

Setelah cutoff dan seluruh submitted entries direview:

1. Lock campaign di DB.
2. Freeze daftar eligible dan payout wallet.
3. Sort wallet ascending berdasarkan 20-byte address.
4. Set ordinal `0..N-1`; satu wallet satu entry dalam campaign.
5. Canonical artifact publik memuat version, campaignKey, wallets, N, cutoff, lockedAt; tidak memuat username/evidence.
6. `eligibilityHash = keccak256(canonical artifact bytes)`.

Reviewer tidak dapat mengubah daftar setelah lock. Tidak ada re-roll bila hasil undian dianggap tidak menarik. N=0 tetap menghasilkan artifact nonempty dengan daftar kosong.

## 6. Raffle v1

### Model yang dipilih

Raffle awal adalah undian server yang bisa direproduksi. Ini tidak membuktikan server/creator tidak berkolusi: server mengetahui seed sebelum snapshot dan creator memilih eligible secara manual. Label produk harus menyebut **undian server yang dapat diaudit**, bukan VRF atau randomness trustless.

Jika fairness terhadap operator dibutuhkan, implementasi berikutnya memakai randomness independen yang tersedia/terverifikasi pada Arc atau future beacon dengan round yang dipin sebelum snapshot. Jangan menganggap Chainlink VRF otomatis tersedia di Arc.

### Seed dan komitmen

- Generate seed 32-byte dari OS CSPRNG saat lock-config.
- Simpan seed encrypted di DB; jangan tulis pada log atau response sebelum snapshot locked.
- Commitment: `keccak256(abi.encode(bytes32("OPPOR_RAFFLE_SEED_V1"), campaignKey, seed))`.
- Commitment masuk config/rules; tidak pernah diganti.
- Setelah eligibility lock, reveal seed satu kali, verifikasi commitment, lalu jalankan algoritme.
- Draw seed: `keccak256(abi.encode(bytes32("OPPOR_RAFFLE_DRAW_V1"), campaignKey, eligibilityHash, seed))`.

Server restart harus membuka seed lama, bukan generate seed baru. Jika seed hilang/corrupt, campaign masuk distribution failure dan tidak mengundi ulang dengan seed baru; refund mengikuti review deadline kontrak.

### Sampling tanpa bias modulo

Untuk mengambil k pemenang tanpa replacement:

1. `k = min(requestedWinnerCount, N)`; kebijakan saat peserta kurang ini tercantum sejak campaign dibuat.
2. Mulai array ordinal `[0,1,...,N-1]` dari snapshot.
3. Jalankan partial Fisher–Yates untuk `i=0..k-1`.
4. Ambil random integer uniform `r` dalam `[0,N-i)` menggunakan rejection sampling dari hash stream.
5. Swap positions `i` dan `i+r`; prefix k adalah pemenang dengan urutan draw.

Hash stream: `x = U256_BE(keccak256(abi.encode(drawSeed, counter)))`; counter mulai nol dan bertambah pada setiap sample termasuk yang ditolak.

Untuk bound `b>0`, definisikan `t = 2^256 mod b`, dapat dihitung tanpa representasi 2^256 sebagai `((U256_MAX % b)+1) % b` dengan arithmetic yang tidak overflow pada bound kecil ini. Tolak x bila x<t; selain itu `r=x % b`. Bound=1 selalu menghasilkan nol. Backend tidak memakai default PRNG/platform-specific shuffle.

Publikasikan seed, commitment, snapshot hash, algorithm version, counter akhir, dan urutan pemenang. Menjalankan ulang input yang sama harus menghasilkan output yang sama.

Untuk raffle server, manifest generation/finalize harus selesai sebelum reviewDeadline. Lock terlalu dekat deadline berisiko expired; backend menampilkan sisa waktu dan menolak pekerjaan jika tidak cukup untuk batas operasi yang dikonfigurasi.

## 7. Formula alokasi

### ERC20 / ERC1155 pool dibagi rata

Untuk pool P base units dan penerima K:

- Jika K=0: manifest kosong, allocated=0.
- Jika K>0 dan P<K: tolak finalisasi karena reward minimum satu base unit tidak tersedia untuk semua penerima.
- `q = floor(P/K)`, `remainder=P mod K`.
- Setiap penerima mendapat q. Remainder tidak dialokasikan dan kembali setelah claimDeadline.
- `allocated = q*K <= P`.

Raffle memakai K=min(winnerCount,N). ALL_ELIGIBLE memakai K=N. Urutkan final allocations menurut recipient untuk penetapan claim index; urutan draw disimpan terpisah.

### Reward tetap

Reward R per penerima memerlukan capacity C dengan target funding `P=R*C`. Kuota registrasi first-come ditentukan transaction DB; tidak boleh menjanjikan R kepada entries tanpa slot. Setelah review, K peserta eligible mendapat R dan remainder tidak dialokasikan. Peserta gagal tidak diganti setelah cutoff kecuali aturan waitlist telah dirancang sebelum activation; baseline tanpa waitlist.

Untuk mode RAFFLE, K pada formula reward tetap adalah jumlah pemenang, `min(winnerCount,N)`, dan hanya pemenang memperoleh R. Baseline tetap memakai pendanaan konservatif R*C; funding berdasarkan winnerCount saja memerlukan varian aturan terpisah. Rules JSON harus menentukan `allocation_policy=EQUAL_POOL` atau `FIXED_REWARD` dan nilai R jika digunakan.

Batas operasional finalisasi adalah 100.000 leaf. Campaign pool-sharing dengan capacity=0 tetap memiliki batas registrasi layanan yang diumumkan di rules sebelum aktivasi, maksimal 100.000 pada baseline; uncapped berarti tanpa kuota reward tetap, bukan resource tanpa batas.

### ERC721

- Inventory token ID dari funding harus sama dengan inventory manifest aturan yang di-hash.
- Backend memeriksa kecocokan inventory sebelum prepare-activate/finalize. Kontrak hanya menyimpan rulesHash dan tidak membaca JSON inventory; creator yang bertransaksi langsung tetap berada dalam batas kepercayaan creator yang dijelaskan sebelumnya.
- Urutkan token ID numerically ascending; setiap recipient mendapat satu NFT pada baseline.
- ALL_ELIGIBLE capacity <= inventory count, sehingga seluruh eligible bisa mendapat NFT.
- RAFFLE winnerCount <= inventory count; K=min(winnerCount,N).
- Pada raffle, token IDs ascending dipasangkan dengan urutan pemenang draw; pada all eligible, dipasangkan dengan eligible recipient order.
- Bangun allocations/index setelah pasangan tersebut ditetapkan. NFT tidak terbagi pecahan.
- NFT sisa di-refund setelah claimDeadline.

## 8. Prepare transaction

Response standar:

```json
{
  "chain_id": "configured-chain-id",
  "to": "0x...",
  "data": "0x...",
  "value": "0",
  "expected_sender": "0x...",
  "intent": "CLAIM",
  "config_hash": "0x...",
  "distribution_root": "0x...",
  "expires_at": "2026-10-20T12:00:30Z"
}
```

TTL response hanya membantu UI; bukan expiry contract authorization. Contract memakai deadline immutable. Claim recipient menjadi expected_sender. Creator intents menggunakan creator wallet.

Backend memeriksa chainId RPC, contract bytecode registry, event/config, simulation dan gas estimate; tidak memaksa nonce atau fixed gas dari rata-rata jaringan. Frontend menampilkan ringkasan sebelum signature, memakai chain yang benar, lalu melaporkan tx hash sebagai tracking hint.

Untuk ERC20 funding: approve amount tepat ke escrow → fundERC20 → activate. Jangan request infinite approval sebagai default. Retry prepare tidak otomatis membuat deployment/pembayaran baru.

## 9. Idempotency dan race conditions

- Idempotency key sama + payload sama menghasilkan response yang sama; payload berbeda menghasilkan 409.
- Pembacaan/penyimpanan idempotency dilakukan dengan business transaction. Mutation yang gagal sebelum commit tidak ditandai sukses.
- Cache Redis kehilangan data tidak menggandakan campaign, entry, snapshot, raffle atau allocation manifest.
- Dua creator review request untuk entry yang sama memerlukan expected_version; satu stale gagal.
- Snapshot lock melawan review/submission memakai row lock dan cutoff check yang sama.
- Dua claim transaksi index sama: satu berhasil, lainnya revert; backend cukup memproyeksikan satu event.
- Claim gagal tidak membuat allocation CLAIMED.
- Transaksi replace/cancel di wallet ditangani lewat receipt/state, bukan tx hash pertama saja.

## 10. Penanganan gagal

| Kasus | Perilaku |
|---|---|
| RPC down / wrong chain | Jangan siapkan transaksi sensitif; retry indexer, browsing DB tetap mungkin. |
| Redis down | Cache miss; jangan hilangkan state bisnis atau mengizinkan rate-limit bypass tak terbatas. |
| DB down | Mutation gagal 503; tidak menerima bukti yang tidak tersimpan. |
| Object upload gagal | Evidence belum lengkap; jangan mark submitted hanya dari presign. |
| Creator tidak review | Campaign belum final; setelah reviewDeadline reward dapat direfund sesuai aturan. |
| Tidak ada eligible | Manifest kosong; tidak ada claim; refund setelah deadline. |
| Winner kurang dari requested | Gunakan seluruh eligible sesuai kebijakan min yang diumumkan. |
| Seed hilang | Tidak redraw; incident, expiry/refund. |
| Manifest storage gagal | Jangan expose ready/finalize; retry dengan hash/content yang sama. |
| Root chain tidak sama artifact | Integrity incident; jangan expose proof yang tampak sah. |
| Token transfer revert | Claim tetap unclaimed; jangan menjanjikan issuer token bisa diperbaiki platform. |
| User tidak punya USDC gas | Tampilkan kebutuhan gas; tidak menjanjikan free claim. |
| Claim deadline lewat | Claim revert; UI tutup claim berdasarkan chain time. |
| Indexer callback terlewat | Poll logs/reconcile menemukan event; tidak perlu user klik claim lagi. |

## 11. Pengujian backend

### Unit

- State transitions, timestamp boundary, formula allocations, U256 parsing, canonical rules hashing.
- Eligibility sorting dan deterministic IDs.
- Raffle N=0, N=1, K=0 invalid config, K>N, rejection sampling, uniqueness winners.
- Wallet login message validation, nonce replay, session expiry dan EIP-1271 paths.

### Integration PostgreSQL/Redis

- Migrations clean install dan upgrade; FK/unique/U256 bounds.
- Registrasi paralel capacity=1: hanya satu slot; uniqueness wallet normalisasi.
- Review versus snapshot lock: snapshot tidak berisi setengah update.
- Submission tepat cutoff ditolak secara konsisten.
- Idempotency repeated request tidak menggandakan efek.
- Crash setelah DB commit sebelum enqueue: outbox/job tetap dapat diproses.
- Worker lease expired dan retry tidak menghasilkan draw/manifests berbeda.
- Redis flush/down tidak menghilangkan nonce/session authority DB.
- Evidence access antar peserta/campaign ditolak.
- Event duplicate/replay menghasilkan projection sama.

## 12. Pengujian kontrak Foundry

### Unit dan adversarial tokens

- Constructor invalid config, salah mode, timeline, zero addresses.
- Partial funding, overfund, funding after start, activation incomplete.
- Unauthorized funding/activation/finalize/cancel.
- ERC20 returning false/no return/revert; fee-on-transfer/rebasing mock ditolak.
- NFT unsolicited callback, wrong sender/token/ID, duplicate ID, receiver revert.
- Finalize before cutoff, tepat cutoff, tepat review deadline.
- Finalize dua kali dan root mutation tidak tersedia.
- Empty distribution dengan root=0/counters=0; root kosong nonempty ditolak.
- Proof wrong chain/escrow/campaign/recipient/index/tokenId/quantity ditolak.
- Bitmap indices 0,255,256 dan batas leafCount.
- Double claim, changed recipient, transfer reentrancy, issuer pause.
- Claim tepat claimDeadline ditolak; sweep tepat deadline boleh.
- Cancel draft, cancel activated ditolak; refund caller tidak mengambil reward.
- ERC721 delivered bitmap dan repeated leaf token ID tidak membayar dua kali.
- Root invalid sum tidak melampaui escrow budget; dokumentasikan starvation risk sebagai creator trust, bukan invariant yang sudah diselesaikan.

### Fuzz / invariant

- Semua urutan fund/activate/finalize/claim/sweep menjaga counters.
- Total payment tidak melebihi budget recognized.
- Callback tidak memungkinkan pembayaran kedua.
- Tidak ada akses cross-campaign.
- Transfer failure mengembalikan state sebelumnya.

## 13. Golden vectors lintas bahasa

Wajib sebelum deployment:

1. ABI configHash Solidity identik Alloy/Rust.
2. Leaf double hash identik Rust, Solidity, dan OpenZeppelin generator yang versinya dipin.
3. Tree/proof identik untuk n=1,2,3,5,100; terutama odd sizes.
4. Amount U256 besar, ERC721 token ID nol dan besar, USDC 6 decimals, native gas 18 decimals.
5. Manifest hash dan eligible snapshot hash reproducible byte-for-byte.
6. Raffle transcript fixed seed menghasilkan winners sama setelah restart dan pada build berbeda.

Generator JavaScript OpenZeppelin boleh digunakan sebagai oracle test fixture; backend produksi tetap Rust. Tidak menjadikan JavaScript worker sebagai dependency bisnis hanya untuk membangun root.

## 14. Acceptance criteria end-to-end

- Creator membuat campaign dengan reward didanai dan aturan immutable.
- Peserta wallet-auth mendaftar, mengirim bukti, dan tidak bisa mengubah setelah cutoff.
- Creator review manual tanpa panggilan API X.
- Pending review tidak lolos otomatis; lock menghasilkan snapshot immutable.
- Raffle bisa direproduksi dan tidak memiliki tombol reroll.
- ALL_ELIGIBLE benar-benar memberikan alokasi kepada semua eligible sesuai aturan/kuota.
- Manifest tersedia sebelum creator finalize; hash/root cocok onchain.
- Peserta claim dari wallet penerima dan menerima asset sesuai alokasi.
- Claim kedua gagal; indexer memproyeksikan claim pertama secara idempotent.
- Refund tidak tersedia sebelum deadline yang mengikat hak peserta.
- Restart API/worker, Redis loss, dan delayed receipt tidak menggandakan reward.
- User dapat export proof/manifest dan claim langsung walau API sedang tidak tersedia.

## 15. Implementasi bertahap

1. Pin rules/schema/ABI dan deadline/refund policy.
2. Rust workspace, migrations, wallet auth, campaign/task/entry endpoints.
3. Manual review, eligibility lock, deterministic allocations dan Merkle golden vectors.
4. Escrow ERC20 + factory, unit/fuzz/invariant tests.
5. Funding/finalize/claim API preparations dan event indexer.
6. Raffle transcript, ERC721/ERC1155 inventory, serta tests tambahan.
7. End-to-end environment, backup/export, deployment verification.

Ini urutan engineering, bukan klaim seluruh implementasi production selesai satu hari. Semua asset kind hanya dibuka di UI ketika end-to-end path-nya sudah teruji.

## 16. Referensi

- [Axum](https://docs.rs/axum/latest/axum/)
- [SQLx](https://docs.rs/sqlx/latest/sqlx/)
- [PostgreSQL explicit locking](https://www.postgresql.org/docs/current/explicit-locking.html)
- [Redis rate limiter patterns](https://redis.io/docs/latest/commands/incr/)
- [EIP-4361 wallet sign-in](https://eips.ethereum.org/EIPS/eip-4361)
- [EIP-1271 contract-wallet signatures](https://eips.ethereum.org/EIPS/eip-1271)
- [RFC 8785 JSON canonicalization](https://www.rfc-editor.org/rfc/rfc8785)
- [OpenZeppelin Merkle Tree](https://github.com/OpenZeppelin/merkle-tree)
- [OpenZeppelin Contracts](https://docs.openzeppelin.com/contracts/5.x/)
- [Arc USDC interfaces](https://www.arc.io/blog/usdc-for-every-action-how-arc-simplifies-building-onchain)
