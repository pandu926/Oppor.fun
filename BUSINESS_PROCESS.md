# Proses Bisnis Platform Campaign Token

Tanggal: 9 Oktober 2026  
Nama kerja: **Oppor** (`oppai.fun`), belum ditetapkan sebagai nama final.

## 1. Konsep

Platform mempertemukan creator token yang ingin mempromosikan proyeknya dengan peserta yang ingin memperoleh reward dari tugas promosi.

Creator bebas menentukan tugas, jenis hadiah, jumlah hadiah, dan cara pembagiannya. Peserta memilih sendiri campaign yang menurut mereka menarik. Platform tidak menetapkan minimum nilai reward atau menjamin campaign akan ramai.

Contoh: creator mengalokasikan sejumlah token yang setara dengan 20% supply saat campaign dibuat, menyetorkannya ke escrow, lalu menawarkan reward kepada peserta yang menyelesaikan tugas sebelum cutoff.

Persentase tersebut diterjemahkan menjadi jumlah token yang jelas. Creator harus memiliki dan menyetorkan tokennya; platform tidak bisa mengambil supply yang berada di wallet orang lain. Nilai reward token tidak dijamin dalam dolar.

## 2. Peran

| Peran | Tanggung jawab |
|---|---|
| Creator | Membuat campaign, mendanai reward, menentukan syarat, memeriksa peserta, dan memfinalisasi penerima. |
| Peserta / airdropper | Memilih campaign, mengerjakan tugas, mengirim klaim atau bukti, dan melakukan claim reward. |
| Platform | Menampilkan campaign, mencatat partisipasi, mengelola status, menjalankan raffle, dan menyediakan halaman claim. |
| Smart contract escrow | Menahan reward, memberlakukan aturan distribusi, mencegah claim ganda, dan mengembalikan sisa sesuai aturan. |

## 3. Tugas dan Reward

### Tugas

Creator dapat memilih dan menggabungkan tugas seperti:

- Join Discord server.
- Retweet / repost di X.
- Like post di X.
- Comment pada post tertentu.
- Tag akun sesuai syarat campaign.
- Tugas custom dengan instruksi dan bukti yang ditentukan creator.

### Reward

Reward dapat berupa:

- Token milik creator sendiri.
- Token ERC-20 lain, termasuk USDC.
- NFT.
- Kombinasi reward jika didukung implementasi.

NFT unik membutuhkan penetapan penerima per NFT. Reward NFT untuk semua peserta hanya dapat ditawarkan jika jumlah NFT yang tersedia mencukupi. Dukungan beberapa jenis aset tidak otomatis berarti kontraknya memiliki mekanisme transfer yang sama.

## 4. Dua Cara Distribusi

| Metode | Aturan |
|---|---|
| Raffle | Pemenang diundi dari peserta yang lolos pemeriksaan. Jumlah pemenang dan reward ditentukan sebelum campaign aktif. |
| Semua peserta memenuhi syarat | Setiap peserta yang lolos mendapat reward sesuai formula pembagian yang diumumkan. |

Untuk metode semua peserta lolos, creator memilih aturan yang didukung platform, misalnya:

- Pool token dibagi rata kepada seluruh peserta yang lolos.
- Reward tetap per peserta, dengan batas peserta yang ditentukan dan didanai sejak awal.

Jika ada kuota, aturan siapa yang mendapat slot harus diumumkan sebelum campaign aktif. Jangan menjanjikan reward tetap kepada peserta tanpa batas dari pool yang terbatas.

## 5. Alur Campaign

### A. Creator membuat draft

Creator mengisi:

- Nama, deskripsi, gambar, dan tautan proyek/token.
- Daftar tugas dan syarat kelulusan.
- Jenis aset dan jumlah reward.
- Metode distribusi: raffle atau semua peserta yang memenuhi syarat.
- Waktu mulai dan cutoff pengerjaan/pengiriman bukti.
- Kuota jika ada, serta jumlah pemenang jika raffle.
- Aturan pembagian, batas waktu pemeriksaan, batas waktu claim, dan penanganan sisa reward.

### B. Creator mendanai campaign

Creator menyetorkan reward ke smart contract escrow. Platform memeriksa transaksi pendanaan sebelum campaign menjadi aktif.

Halaman campaign menampilkan aset dan jumlah reward yang benar-benar sudah didanai. Pengumuman reward tanpa deposit belum menjadi campaign aktif.

### C. Peserta memilih dan mendaftar

Peserta melihat tugas, reward, kuota, cutoff, dan metode pembagian, lalu menghubungkan wallet untuk menerima hadiah.

Karena platform tidak memakai API X, username X yang dicantumkan peserta bersifat informasi yang diajukan sendiri. Username itu belum merupakan bukti kepemilikan akun atau bukti pengerjaan tugas.

### D. Peserta mengerjakan dan mengirim klaim

Peserta mengerjakan tugas di platform terkait, kemudian menandai tugas sebagai dikerjakan dan mengirim bukti yang diminta creator, seperti username, tautan, screenshot, atau keterangan.

Status pada tahap ini adalah **diajukan**, bukan **terverifikasi**. Menekan tombol tugas atau mengirim screenshot tidak otomatis membuktikan tugas selesai.

### E. Cutoff menutup pengajuan

Setelah cutoff, pendaftaran dan pengiriman bukti ditutup. Bukti yang sudah masuk dipertahankan untuk pemeriksaan. Aturan mengenai perubahan bukti setelah penutupan harus jelas.

Backend memperbarui status sesuai waktu; smart contract menerapkan batas waktu pada fungsi yang relevan. Perubahan status tidak berarti kontrak menjalankan transaksi sendiri.

### F. Creator memeriksa peserta

Creator memeriksa peserta setelah cutoff dan menandai:

- **Lolos:** memenuhi syarat.
- **Gagal:** tidak memenuhi syarat, dengan alasan.
- **Belum diperiksa:** masih menunggu keputusan.

Platform **tidak memakai API X atau penyedia API X pihak ketiga**. Pemeriksaan retweet, like, comment, dan tag dilakukan manual oleh creator. Sistem tidak bisa otomatis mengetahui tindakan X yang belum diperiksa.

Peserta boleh mengajukan klaim sebelum pemeriksaan, tetapi klaim yang tidak memenuhi syarat akan didiskualifikasi berdasarkan hasil review. Status gagal otomatis mengeluarkan peserta dari daftar calon penerima.

Pemeriksaan hanya membuktikan keadaan yang dapat dilihat saat review. Peserta bisa menghapus atau membatalkan tindakan setelahnya; platform tidak memantau X terus-menerus.

### G. Menentukan penerima

**Raffle:** setelah daftar peserta lolos selesai dan dikunci, platform menjalankan undian sesuai jumlah pemenang dan aturan campaign.

**Semua peserta lolos:** platform menghitung alokasi berdasarkan daftar peserta yang dikunci dan formula yang sudah diumumkan.

Creator memfinalisasi hasil. Daftar penerima dan alokasinya tidak boleh diganti sepihak setelah claim dibuka.

Raffle harus benar-benar menggunakan mekanisme undian. Jika creator memilih pemenang sendiri, tampilkan sebagai pemilihan manual. Sumber randomness dan cara mengaudit undian masih merupakan keputusan implementasi; dokumen ini tidak menganggap randomness onchain sudah tersedia.

### H. Peserta claim reward

Peserta membuka halaman campaign, melihat alokasinya, dan menekan **Claim** dari wallet yang terdaftar.

Smart contract memeriksa hak claim, mentransfer reward, dan mencatat bahwa alokasi tersebut sudah diklaim. Peserta tidak perlu memberi approval atas aset walletnya untuk menerima reward biasa.

Model distribusi adalah **pull / claim**, bukan creator mengirim satu per satu kepada seluruh penerima.

### I. Campaign selesai

Campaign menampilkan jumlah peserta, peserta lolos/gagal, pemenang, alokasi, dan reward yang sudah diklaim.

Sisa reward hanya dapat dikembalikan sesuai aturan yang diumumkan dan diberlakukan kontrak. Creator tidak boleh menarik reward yang masih menjadi hak claim peserta sebelum masa claim berakhir.

## 6. Status

### Campaign

`Draft → Funded → Active → Reviewing → Finalized / Claim Open → Completed`

Cutoff memindahkan campaign aktif ke tahap review. Status pembatalan atau kedaluwarsa diperlukan untuk kasus campaign tidak dilanjutkan; aturan penanganannya harus ditetapkan sebelum aktivasi.

### Peserta

`Registered → Submitted → Eligible / Disqualified`

Setelah finalisasi:

- Raffle: `Eligible → Winner / Not Selected → Claimed` untuk pemenang.
- Semua peserta lolos: `Eligible → Allocated → Claimed`.

Peserta yang belum diperiksa tidak otomatis dianggap lolos.

## 7. Batas Kepercayaan

Escrow menjamin reward tersedia dan distribusi mengikuti alokasi yang difinalisasi. Escrow tidak bisa menilai sendiri apakah peserta benar-benar retweet atau apakah creator menilai bukti secara adil.

Hasil pemeriksaan manual bergantung pada creator. Platform menyimpan keputusan dan alasan agar prosesnya bisa ditelusuri, tetapi tidak mengklaim verifikasi sosial yang sepenuhnya otomatis atau trustless.

Satu wallet atau satu username tidak membuktikan satu manusia. Perlindungan terhadap multi-account dapat dikembangkan kemudian; versi awal tidak menjanjikan anti-Sybil sempurna.

## 8. Pembagian Pekerjaan Sistem

| Komponen | Menangani |
|---|---|
| Aplikasi / database | Campaign, tugas, peserta, bukti, review, undian, dan tampilan alokasi. |
| Smart contract | Deposit reward, otorisasi finalisasi, hak claim, pencegahan claim ganda, dan aturan penarikan sisa. |

Hasil review offchain dihubungkan ke kontrak melalui daftar alokasi atau komitmen alokasi, misalnya Merkle root. Pilihan teknisnya belum ditetapkan. Kontrak harus membatasi total pembayaran sesuai reward yang didanai.

Untuk efisiensi, hindari mengunggah satu transaksi penetapan alokasi untuk setiap peserta jika satu komitmen distribusi dapat mencukupi.

## 9. Tabel Data Awal

| Tabel | Isi utama |
|---|---|
| `campaigns` | Creator, proyek, status, jadwal, metode distribusi, kuota, dan aturan. |
| `campaign_rewards` | Aset reward, jumlah/token ID, dan transaksi pendanaan. |
| `tasks` | Jenis tugas, target, instruksi, dan syarat bukti. |
| `entries` | Peserta, wallet penerima, waktu daftar, status review, dan alasan gagal. |
| `submissions` | Bukti per tugas dan waktu pengiriman. |
| `allocations` | Penerima final, reward, bukti claim jika diperlukan, dan transaksi claim. |

Informasi tidak harus disimpan seluruhnya onchain. Database menyimpan proses bisnis; kontrak menyimpan aturan dan state yang diperlukan untuk menjaga reward.

## 10. Biaya dan Pendapatan

- Creator membayar gas pendanaan dan finalisasi campaign.
- Peserta membayar gas saat claim, dalam USDC di Arc.
- Peserta tetap membutuhkan sedikit USDC meskipun hadiah berupa token creator atau NFT.
- Tidak ada anggaran API X dalam rancangan ini.
- Pendapatan platform dapat berupa fee pembuatan campaign atau layanan promosi tambahan. Harga dan mekanismenya belum ditetapkan.
- Reward campaign bukan otomatis pendapatan platform. Potongan reward hanya boleh dilakukan jika diumumkan sebelum pendanaan dan diatur jelas.

## 11. Aturan yang Perlu Ditetapkan Saat Implementasi

Keputusan berikut belum menjadi kesepakatan final:

- Jenis reward yang didukung pada rilis pertama dan dukungan reward gabungan.
- Batas waktu creator menyelesaikan review, serta penanganan creator yang menghilang.
- Pembatalan campaign dan pengembalian reward, terutama setelah peserta mulai mengerjakan.
- Masa claim dan pengembalian reward yang tidak diklaim.
- Mekanisme raffle, sumber randomness, dan audit hasil undian.
- Penanganan jumlah peserta lolos yang lebih kecil dari jumlah pemenang raffle.
- Penanganan keberatan terhadap keputusan review.
- Besaran fee platform.

Aturan tersebut harus terlihat sebelum peserta ikut dan sebelum creator mengaktifkan campaign. Perubahan setelah campaign aktif tidak boleh mengurangi hak reward yang sudah dijanjikan.

## 12. Prinsip Produk

**Creator bebas menawarkan reward. Peserta bebas memilih campaign. Platform menyediakan pasar, pencatatan tugas, pemeriksaan manual, dan claim hadiah yang sudah didanai.**

Campaign menarik memperoleh partisipasi karena penawarannya. Platform tidak menjanjikan jumlah peserta, kenaikan harga token, atau keberhasilan promosi.
