# Spesifikasi Smart Contract Escrow

## 1. Tujuan

Creator mendanai campaign, mengunci ketentuan, lalu menetapkan alokasi setelah cutoff. Peserta mengambil reward dengan Merkle proof. Kontrak mencegah claim ganda, pergantian root, dan penarikan reward selama hak claim masih terbuka.

Dokumen ini merupakan spesifikasi awal. Implementasi yang sesuai ABI backend, pengujian, dan batas keamanan aktual tersedia di [contracts/README.md](../contracts/README.md). Kontrak belum diaudit independen atau di-deploy ke Arc.

## 2. Arsitektur

```text
CampaignFactory
  └── CampaignEscrow terpisah per campaign
        ├── immutable configuration
        ├── satu reward asset / satu koleksi
        ├── funding inventory
        ├── satu Merkle root final
        └── claimed bitmap
```

- Factory mendepoy escrow dengan constructor; baseline tidak memakai upgradeable proxy.
- Factory mencatat creator, campaign key, escrow address, dan config hash.
- Satu creator tidak dapat mengambil reward escrow campaign lain.
- Tidak ada owner platform dengan hak sweep atau mengganti root.
- Tidak ada keeper yang otomatis mengirim reward; claim dan refund membutuhkan transaksi.
- Backend tidak menjadi signer dana. Transaksi creator/peserta berasal dari wallet masing-masing.

Escrow yang salah deployment tidak diperbaiki dengan mengganti storage: creator membatalkan sebelum aktivasi dan membuat deployment baru. Factory dapat dihentikan dari membuat escrow baru jika mekanisme pause dipilih, tetapi tidak boleh membekukan claim escrow lama.

## 3. Asset yang didukung

| Kind | Inventory | Quantity leaf |
|---|---|---|
| `ERC20` | Satu token address; target funded base units | Jumlah token base units; tokenId=0 |
| `ERC721` | Satu collection address; daftar token ID unik | quantity=1; tokenId harus inventory |
| `ERC1155` | Satu collection + satu token ID; target jumlah units | quantity positif; tokenId sama dengan campaign |

Rancangan awal belum mendukung reward campuran. Kombinasi ERC-20 + NFT membutuhkan bundle allocations dan akuntansi per aset; jangan berpura-pura satu jumlah mencakup keduanya.

### Arc USDC

USDC memiliki native interface untuk gas dan ERC-20 interface untuk aplikasi. Native menggunakan 18 decimals; ERC-20 menggunakan 6 decimals. Reward USDC di escrow menggunakan interface ERC-20 dan base units 6 decimals, bukan `msg.value`.

`1 USDC reward = 1_000_000` base units. Gas wallet dihitung dari native 18-decimal amount. Jangan mengakui native balance dan ERC-20 balance sebagai dua aset terpisah. [Panduan resmi Arc](https://www.arc.io/blog/usdc-for-every-action-how-arc-simplifies-building-onchain).

Alamat USDC dan RPC harus dicocokkan dengan dokumentasi resmi serta hasil read chain pada environment deployment. Tidak ada konstanta deployment mainnet Oppor yang dianggap telah tervalidasi di sini.

### Batas dukungan token

- ERC-20 standar dengan jumlah transfer yang tidak berubah adalah baseline.
- Tolak fee-on-transfer, rebasing, reflection, dan token yang debit/credit-nya tidak sesuai jumlah yang diharapkan. `SafeERC20` saja tidak mendeteksi perubahan jumlah.
- Funding memeriksa delta balance escrow sama dengan amount.
- Claim ERC-20 memeriksa delta escrow dan penerima sesuai amount; jika gagal, seluruh claim revert termasuk bitmap/counter.
- Token dapat diblacklist/paused oleh issuer di luar escrow. Escrow tidak menjamin token tersebut selalu dapat ditransfer.
- NFT memakai safe transfer sehingga wallet kontrak penerima perlu mendukung receiver interface.
- Tidak menerima arbitrary external calls dari input claim.

Gunakan OpenZeppelin `SafeERC20`, `MerkleProof`, dan storage-based `ReentrancyGuard` dengan dependency versi dipin. Jangan menganggap penggunaan library berarti escrow custom sudah diaudit. Referensi: [ERC-20](https://docs.openzeppelin.com/contracts/5.x/api/token/erc20), [Utils](https://docs.openzeppelin.com/contracts/5.x/api/utils).

## 4. Config immutable

```solidity
enum AssetKind { ERC20, ERC721, ERC1155 }
enum DistributionMode { ALL_ELIGIBLE, RAFFLE }
enum CampaignState { DRAFT, ACTIVE, FINALIZED, CANCELLED, CLOSED }

struct CampaignConfig {
    bytes32 campaignKey;
    address creator;
    address refundRecipient;
    AssetKind assetKind;
    address rewardToken;
    uint256 rewardTokenId;      // ERC1155; 0 for ERC20/ERC721
    uint256 targetQuantity;     // ERC20 units / ERC721 count / ERC1155 units
    uint64 startsAt;
    uint64 cutoffAt;
    uint64 reviewDeadline;
    uint64 claimDeadline;
    DistributionMode mode;
    uint32 participantCapacity; // 0 = uncapped pool-sharing campaign
    uint32 winnerCount;         // >0 for raffle, 0 otherwise
    bytes32 rulesHash;
    bytes32 raffleSeedCommitment; // zero for ALL_ELIGIBLE
}
```

Rules hash mengikat manifest aturan, tasks, inventory ERC-721 bila relevan, algoritme alokasi, seed commitment, dan kebijakan claim/refund. Kontrak mengikat config terstruktur dan rules hash melalui `configHash`; definisinya di dokumen integrasi.

Constructor memeriksa:

- creator, refundRecipient, rewardToken tidak zero; rewardToken memiliki code.
- `now < startsAt < cutoffAt < reviewDeadline < claimDeadline`.
- targetQuantity positif, fits counter; rulesHash tidak zero.
- mode/winnerCount konsisten; raffle commitment tidak zero.
- ERC20 rewardTokenId=0; ERC721 rewardTokenId=0, target count maksimal 100 sebagai batas usulan per campaign; ERC1155 ID boleh nol.
- Asset interface checks untuk NFT; jangan hanya mempercayai label dari frontend.

Semua deadline adalah waktu mutlak. Claim tidak mendapat perpanjangan diam-diam bila creator terlambat finalize. Claim window minimum dari reviewDeadline ke claimDeadline merupakan validasi produk yang harus diumumkan.

## 5. Factory ABI

```solidity
function createCampaign(CampaignConfig calldata config)
    external returns (address escrow);

function campaignOf(address creator, bytes32 campaignKey)
    external view returns (address);

event CampaignCreated(
    address indexed creator,
    bytes32 indexed campaignKey,
    address indexed escrow,
    bytes32 configHash
);
```

`config.creator == msg.sender`; uniqueness `(creator,campaignKey)` enforced. Creator ditetapkan dari pemanggil, bukan arbitrary address pihak ketiga. Gunakan `new CampaignEscrow(config)` pada baseline; alamat diketahui dari receipt event, bukan tebakan frontend.

Factory create tidak mendanai reward secara otomatis. Frontend menyiapkan transaksi approval/deposit terpisah. Deployment gas tidak sama dengan claim gas.

## 6. Escrow storage

```text
config + configHash
state
fundedQuantity
allocatedQuantity
claimedQuantity
leafCount
claimedLeafCount
distributionRoot
manifestHash
eligibilityHash
mapping(uint256 bitmapWord => uint256 bits) claimedBitmap
ERC721: deposited[tokenId], delivered[tokenId], inventory[]
expected NFT deposit context (sender/tokenId/quantity)
```

Quantities dihitung dalam satuan sesuai asset kind. Inventory ERC-721 memakai count untuk funded/allocated/claimed dan qty=1 setiap leaf; token ID bukan jumlah.

## 7. Escrow ABI dan fungsi

### Read

```solidity
function state() external view returns (CampaignState);
function configHash() external view returns (bytes32);
function distributionRoot() external view returns (bytes32);
function isClaimed(uint256 claimIndex) external view returns (bool);
function isDepositedNFT(uint256 tokenId) external view returns (bool);
```

Selain itu expose configuration dan funded/allocated/claimed counters. Read calldata tidak boleh menjadi satu-satunya sumber metadata campaign; rules manifest tersedia terpisah.

### Pendanaan

```solidity
function fundERC20(uint256 amount) external;
function fundERC721(uint256[] calldata tokenIds) external;
function fundERC1155(uint256 amount) external;
function activate() external;
```

Funding hanya creator, state DRAFT, sebelum startsAt; amount positif dan total tidak melampaui target. Memerlukan approval pada reward token/collection. Partial funding boleh, tetapi activation membutuhkan fundedQuantity tepat target.

ERC-721 memeriksa ID unik, inventory limit, ownership creator, serta menerima transfer melalui receiver callback yang diharapkan. ERC-1155 hanya satu ID configured. Receiver callback memeriksa token contract, sender, ID, amount dan active deposit context; unsolicited safe transfers ditolak. Batch ERC-1155 tidak didukung baseline.

Jangan menerapkan `nonReentrant` pada receiver callback yang dipanggil dari fungsi funding ber-guard; callback harus dibatasi oleh deposit context, sementara funding/claim/refund memiliki guard.

`activate` hanya creator, sebelum startsAt, fundedQuantity == targetQuantity. Setelah activation tidak ada perubahan tasks, config, additional funding resmi, atau cancellation. Backend hanya menerima peserta ketika activated dan startsAt telah tercapai.

Direct token transfer yang tidak melalui funding tidak menaikkan fundedQuantity dan tidak mengaktifkan campaign. Native USDC transfer juga tidak menjadi pendanaan resmi.

### Finalisasi

```solidity
function finalize(
    bytes32 root,
    bytes32 allocationManifestHash,
    bytes32 eligibilitySnapshotHash,
    uint256 declaredAllocatedQuantity,
    uint256 declaredLeafCount
) external;
```

Syarat:

- Hanya creator; state ACTIVE.
- `cutoffAt <= now < reviewDeadline`.
- manifestHash dan eligibilityHash tidak zero.
- declaredAllocatedQuantity <= fundedQuantity.
- Nonempty: root != 0, leafCount > 0, allocated > 0.
- Empty: root=0, leafCount=0, allocated=0; hasil tanpa eligible/pemenang tetap dicatat dengan manifest valid.
- Untuk ERC721: allocated quantity == leaf count dan tidak melampaui jumlah NFT.
- Batas leafCount usulan 100.000; bukan loop saat finalize.

Set root dan semua counters satu kali; state FINALIZED. Tidak ada `updateRoot`, `replaceManifest`, atau finalisasi ulang. Claim langsung tersedia setelah event confirmed, sampai claimDeadline.

**Batas penting:** Merkle root biasa tidak membuktikan penjumlahan leaf atau uniqueness NFT. Contract membatasi total yang bisa dibayarkan dan inventory, tetapi root jahat bisa membuat sebagian claim gagal. Backend memvalidasi seluruh manifest; peserta tetap mempercayai creator menetapkan hasil yang benar. Merkle-sum atau dual attestation bukan bagian baseline.

### Claim

```solidity
struct Claim {
    uint256 index;
    address recipient;
    uint256 tokenId;
    uint256 quantity;
}

function claim(Claim calldata allocation, bytes32[] calldata proof) external;
```

Syarat:

- state FINALIZED dan `now < claimDeadline`.
- `msg.sender == recipient`; recipient tidak zero, bukan escrow sendiri.
- index < leafCount; bitmap belum claimed.
- quantity positif; asset kind/token ID sesuai configuration.
- ERC721 quantity=1, ID deposited dan belum delivered.
- Merkle proof sah untuk leaf yang mengikat chain, escrow, campaign, index, recipient, token, ID, quantity.
- `claimedQuantity + quantity <= allocatedQuantity`.

Urutan: validasi → mark bitmap/update counters dan NFT delivered → transfer reward → validate ERC20 deltas → emit. Seluruh operasi `nonReentrant`. Revert transfer membatalkan semua perubahan.

Tidak ada user approval untuk menerima reward; approval hanya pada creator saat funding. Tidak ada penggantian recipient atau redirect reward di calldata. Penerima kontrak menggunakan panggilan dari wallet kontrak tersebut; callback reward harus didukung.

Bitmap: word=`index >> 8`, bit=`index & 255`, mask=`1 << bit`. Index satu campaign tidak dapat mengonsumsi index campaign lain karena escrow terpisah dan domain leaf berbeda.

Baseline claim satu alokasi per transaksi. ERC20/ERC1155 memakai satu leaf per penerima; ERC721 satu leaf per token ID. Bila seorang peserta mendapat beberapa NFT, beberapa claim mungkin diperlukan; batch claim merupakan perluasan, bukan janji satu transaksi untuk semua jenis hadiah.

### Cancel dan refund

```solidity
function cancel() external;
function sweepRemaining() external;
```

`cancel`: hanya creator, state DRAFT. Mark CANCELLED, lalu kembalikan pendanaan yang tercatat ke immutable refundRecipient. Jika refund transfer gagal, seluruh transaksi revert. Draft yang gagal diaktivasi karena startsAt terlewat masih bisa dibatalkan creator.

`sweepRemaining`: boleh dipanggil siapa pun, tetapi selalu membayar refundRecipient dan tidak pernah caller.

Syarat salah satu:

- state ACTIVE dan `now >= reviewDeadline`: creator tidak finalize, campaign expired.
- state FINALIZED dan `now >= claimDeadline`: periode claim selesai.
- state DRAFT dan `now >= reviewDeadline`: draft abandoned.

Mark CLOSED sebelum transfer, gunakan nonReentrant. ERC20/1155 mengembalikan sisa pendanaan tercatat; ERC721 mengembalikan deposited IDs yang belum delivered. Iterasi ERC721 dibatasi inventory maksimum 100, sehingga refund tidak memiliki loop peserta yang tidak terbatas.

Tidak ada early sweep karena claimedCount == leafCount: ini menghindari menarik dana berdasarkan declared count sebelum deadline. Pengembalian lebih awal dapat ditambahkan nanti hanya dengan aturan yang benar-benar menjamin tidak ada hak claim tersisa.

NFT refundRecipient harus dapat menerima safe transfer. Validasi pada draft dan simulasikan sebelum activation; bila recipient gagal saat refund, dana tetap dalam escrow sampai transfer tersebut bisa berhasil. Jangan menjanjikan recovery universal.

Tidak ada arbitrary admin rescue. Aset asing atau transfer tidak tercatat dapat terjebak pada baseline; frontend tidak mengarahkan pengguna ke transfer langsung. Dukungan recovery aset tidak tercatat adalah fitur terpisah yang tidak boleh membuka penarikan reward aktif.

## 8. Events

```solidity
event RewardFunded(uint8 assetKind, address indexed token, uint256 tokenId, uint256 quantity);
event CampaignActivated(bytes32 indexed configHash);
event DistributionFinalized(
    bytes32 indexed root,
    bytes32 manifestHash,
    bytes32 eligibilityHash,
    uint256 allocatedQuantity,
    uint256 leafCount
);
event RewardClaimed(
    uint256 indexed index,
    address indexed recipient,
    uint256 tokenId,
    uint256 quantity
);
event CampaignCancelled(address indexed refundRecipient);
event RemainingSwept(address indexed refundRecipient, uint256 quantity);
```

Event escrow tidak membutuhkan campaign ID karena address sudah memetakan campaign. Backend mengikat event ke registry factory resmi dan chain ID. Event memuat quantity base units; tidak ada konversi decimals dalam contract event.

## 9. Errors

Custom errors minimal: `Unauthorized`, `InvalidConfig`, `InvalidState`, `WrongAssetKind`, `FundingClosed`, `FundingExceeded`, `FundingIncomplete`, `UnexpectedNFTTransfer`, `DuplicateNFT`, `CutoffNotReached`, `ReviewExpired`, `ClaimExpired`, `AlreadyClaimed`, `InvalidProof`, `InvalidAllocation`, `AllocationExceeded`, `UnsupportedTokenBehavior`, `RefundNotAvailable`.

Backend memetakan errors ke pesan yang jelas. Jangan memperlakukan simulation success sebagai jaminan transaksi akan berhasil ketika dieksekusi.

## 10. Invariant

1. `claimedQuantity <= allocatedQuantity <= fundedQuantity <= targetQuantity` setelah finalisasi.
2. Root, recipient domain, config dan deadline tidak berubah setelah activation/finalization masing-masing.
3. Satu index hanya dibayar sekali.
4. Satu NFT inventory hanya dikirim sekali.
5. Satu campaign tidak memakai reward campaign lain.
6. Caller refund tidak menerima reward kecuali memang refundRecipient.
7. Dana resmi tidak ditarik selama masa claim terbuka.
8. Transfer gagal tidak mengonsumsi hak claim.
9. Panggilan callback tidak melewati checks lewat reentrancy.
10. Tidak ada loop seluruh peserta saat finalize/claim/refund.

Balance riil token tidak selalu sama dengan counter karena unsolicited transfers atau token issuer behavior. Counters mencatat kewajiban yang diakui, bukan semua token yang pernah tiba.

## 11. Deployment

- Solidity compiler dan optimizer dipin; pilih EVM target yang didukung Arc dan dependency yang dipilih.
- Verifikasi alamat token, chain ID, RPC, factory deployment block dan bytecode.
- Deploy dan uji di environment yang sesuai sebelum mainnet.
- Publikasikan verified source, ABI, compiler settings dan address registry.
- Foundry unit/fuzz/invariant tests wajib untuk operasi dana; daftar di dokumen pengujian.
- Jangan menyalin klaim biaya rata-rata jaringan menjadi estimasi claim. Ukur gas claim ERC20/ERC721/ERC1155, proof panjang berbeda, cold/warm slots, dan initial balance penerima.
- Tidak ada deployment atau transaksi dana yang telah dilakukan sebagai bagian dokumentasi ini.
