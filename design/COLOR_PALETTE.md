# Oppor: riset palet warna

Tanggal: 9 Oktober 2026. Pilihan pengguna terbaru: **Indigo + Pearl**. Larangan pengguna: tidak memakai hijau. Mockup aplikasi terbaru tersedia di [INDIGO_MOCKUP.md](INDIGO_MOCKUP.md).

Mockup dibuat menggunakan built-in imagegen: [perbandingan tiga palet](oppor-color-directions-v1.png). Gambar adalah konsep visual dan data campaign contoh; hex token di dokumen ini menjadi acuan implementasi karena warna raster tidak dijamin persis.

## Opsi

| Peran | Cobalt + Slate | Indigo + Pearl | Amber + Graphite |
|---|---|---|---|
| Primary | `#2563EB` | `#4338CA` | `#F59E0B` |
| Canvas | `#F8FAFC` | `#FAFAFC` | `#18181B` |
| Card | `#FFFFFF` | `#FFFFFF` | `#27272A` |
| Sidebar | `#0F172A` | `#18181B` | `#09090B` |
| Teks utama | `#0F172A` | `#18181B` | `#FAFAFA` |
| Teks sekunder | `#64748B` | `#71717A` | `#A1A1AA` |
| Border dekoratif | `#E2E8F0` | `#E4E4E7` | `#3F3F46` |
| Teks tombol primary | `#FFFFFF` | `#FFFFFF` | `#18181B` |

Rekomendasi awal: Cobalt + Slate. Pengguna kemudian memilih Indigo + Pearl; mockup terbaru menggunakan navigasi terang dan aksen indigo. Ini keputusan desain, bukan klaim bahwa satu warna menaikkan conversion.

Indigo + Pearl adalah alternatif dengan aksen lebih pekat. Amber + Graphite adalah pilihan dark-first dengan tombol amber berteks gelap.

## Penggunaan

- Mayoritas area memakai warna netral; primary untuk tombol utama, filter terpilih, link, dan focus indicator.
- Badge escrow funded memakai warna aksen lembut dan label teks; tidak memakai hijau.
- Status tugas pending/review/eligible/disqualified harus memiliki label dan ikon, bukan hanya warna.
- Funded berarti reward tersimpan, bukan creator terverifikasi atau peluang profit.
- Brand token campaign tidak harus menjadi warna UI. Mockup memakai emblem contoh tanpa hijau.
- Border pada tabel adalah dekoratif. Batas kontrol interaktif dan focus ring perlu kontras yang diuji tersendiri.

## Kontras

Perhitungan lokal memakai relative luminance sRGB dan rasio WCAG, dibulatkan dua desimal:

| Foreground / background | Rasio |
|---|---|
| Putih / cobalt `#2563EB` | 5,17:1 |
| Putih / indigo `#4338CA` | 7,90:1 |
| Graphite `#18181B` / amber `#F59E0B` | 8,25:1 |
| Slate `#64748B` / canvas `#F8FAFC` | 4,55:1 |
| Navy `#0F172A` / canvas `#F8FAFC` | 17,06:1 |

WCAG menetapkan minimum 4,5:1 untuk teks biasa dan 3:1 untuk teks besar. Pemeriksaan pasangan di atas bukan sertifikasi seluruh UI; hover, disabled, focus, error, badge, dan dark-mode combinations tetap perlu diuji ketika implementasi dibuat. [W3C Contrast Minimum](https://www.w3.org/WAI/WCAG22/Understanding/contrast-minimum.html).

## Referensi dan metode

Radix menyediakan skala neutral dan accent untuk UI serta varian light/dark. Riset ini memakai pendekatan pembagian peran tersebut, dengan hex pilihan sendiri; bukan salinan exact Radix scale. [Radix scales](https://www.radix-ui.com/colors/docs/palette-composition/scales), [Radix usage](https://www.radix-ui.com/colors/docs/overview/usage).

Ketiga mockup menampilkan struktur dan informasi yang sama agar perbandingan fokus pada warna. Brief lengkap imagegen disimpan di [IMAGEGEN_PROMPT.txt](IMAGEGEN_PROMPT.txt).
