# 03 — Estimasi Kemampuan (Ability Estimation): Overview

> Landasan teori untuk `src/ability_estimation/` (Fase 3). Lih. dokumen per
> method untuk rumus & referensi rinci tiap `Method`.

## 1. Lingkup: estimator untuk satu pola respons

`ability_estimation::estimate()` mengestimasi `theta` (dan SE-nya) dari
**satu** pola respons (satu peserta) dengan method yang dipilih caller
(`Eap`/`Map`/`Ml`/`Wle`/`EapSum`/`Fixed`). Ia hanya estimator — logika
sesi CAT seperti fallback otomatis `ML -> MAP` pada pola respons ekstrem
bukan bagian darinya (lih. §5).

## 2. Curse of dimensionality: grid kuadratur `quadpts^nfact`

`EAP` (dan `EapSum`) grid-based mengintegrasikan posterior lewat kuadratur
numerik: jumlah titik grid = `quadpts^nfact` (cartesian product `quadpts`
titik per dimensi). Ini meledak kombinatorial seiring `nfact` naik — problem
yang sama seperti dibahas di `docs/teori/02-item-selection-overview.md` §1
untuk kriteria seleksi item berbasis kuadratur.

Engine menahan ledakan ini dengan **mengecilkan `quadpts` default seiring
`nfact` naik**:

| `nfact` | `quadpts` default | Total titik grid |
|---|---|---|
| 1 | 61 | 61 |
| 2 | 31 | 961 |
| 3 | 15 | 3.375 |
| 4 | 9 | 6.561 |
| 5 | 7 | 16.807 |
| `>5` | 3 | `3^nfact` |

`ability_estimation::quadrature::default_quadpts(nfact)` mengimplementasikan
tabel ini. `quadpts` bisa dioverride manual lewat
`EstimationConfig::quadpts`. Rentang integrasi (`theta_lim`) tetap `[-6,6]`
untuk seluruh `nfact` — konstanta internal, sengaja tidak configurable
(nilainya tetap untuk seluruh sesi CAT).

Trade-off tabel di atas jelas: `quadpts` mengecil supaya total titik tetap
terkendali, tapi **resolusi per-dimensi** ikut menurun (`quadpts=3` untuk
`nfact>5` berarti cuma 3 titik `{-6, 0, 6}` per dimensi — sangat kasar).
Inilah motivasi Quasi-Monte Carlo di §3.

## 3. Motivasi Quasi-Monte Carlo

Alternatif untuk integrasi dimensi tinggi: alih-alih grid REGULER (yang
butuh `quadpts^nfact` titik untuk resolusi seragam di semua dimensi), pakai
**titik quasi-random** yang tersebar merata (low-discrepancy) di ruang
`nfact`-dimensi dengan jumlah titik **total** yang ditentukan bebas (tidak
harus `quadpts^nfact`) — memungkinkan resolusi jauh lebih baik per titik
untuk `nfact` besar, dengan total titik yang sama.

Engine memakai **leaped-Halton sequence** (`quadrature::build_qmc_points`):
basis bilangan prima pertama per dimensi, dengan "leap" (skip) prima `409`
(indeks titik ke-`i` adalah `1 + i * 409`). Konstruksi Halton "leaped" ini
berasal dari Kocis & Whiten (1997), "Computational investigations of
low-discrepancy sequences", *ACM Transactions on Mathematical Software*
23(2), 266–294; leap mengurangi korelasi antar-dimensi pada sequence
berdimensi banyak. Titik `[0,1]^nfact` hasil Halton ditransformasi lewat
invers CDF normal standar jadi awan kuasi-`N(0,I)`.

Pengaman milik engine ini (bukan bagian dari Kocis & Whiten): bila leap
sendiri termasuk basis, basis itu diganti bilangan prima berikutnya. Basis
sama dengan leap membuat digit terendah indeks `1 + i * 409` selalu 1,
sehingga dimensi itu konstan di interval sempit `[1/409, 2/409)` (dimensi
degenerat). Ini hanya terpicu pada `nfact >= 80` (409 adalah bilangan prima
ke-80); test `halton_bases_never_equal_leap_and_dimensions_are_not_constant`
memeriksa sampai `nfact = 200`.

**Prior grup.** Awan kuasi-`N(0,I)` disesuaikan dengan prior sesungguhnya
lewat `theta = gmeans + L z` dengan `gcov = L L^T` (Cholesky), sehingga
titik adalah sampel dari `N(gmeans, gcov)`. Bobot EAP untuk titik QMC
karenanya **seragam** (kerapatan prior sudah ada dalam sebaran titik;
menimbang dengan `dmvnorm` lagi akan menghitung prior dua kali), sedangkan
grid biasa memakai bobot `dmvnorm(titik; gmeans, gcov)`. Penurunan &
implikasi praktis di `docs/teori/03-ability-estimation-eap.md` §2.

## 4. Peringatan `EAP` + `nfact > 4` tanpa QMC

Grid EAP adalah hasil kali kartesius, jadi totalnya `quadpts^nfact` titik
dan tiap titik menghitung likelihood seluruh item terjawab. Tabel §2
mengecilkan `quadpts` supaya total titik terkendali, tetapi di `nfact > 4`
resolusinya menjadi terlalu kasar: `nfact = 6` dengan `quadpts = 3` memberi
`3^6 = 729` titik berspasi 6,0 pada `[-6, 6]`; menaikkan ke 7 per dimensi
memberi `7^6 = 117.649` titik, dan `7^8 = 5.764.801` pada `nfact = 8`.
Optimisasi langsung (`MAP`/`ML`/`WLE`) tidak butuh grid, dan QMC memakai
jumlah titik total yang dipilih sendiri.
`ability_estimation::estimate()` karenanya mengeluarkan peringatan lewat
`log::warn!` saat `Method::Eap { qmc: false }` dipakai dengan `nfact > 4`,
menyebut ukuran grid itu dan menunjuk `Method::Eap { qmc: true }` atau
`Map` sebagai alternatif. Ini sengaja peringatan, bukan error keras:
estimasi tetap dijalankan, karena hasilnya masih sah, hanya kurang akurat.

## 5. Yang BUKAN cakupan Fase 3

- **Fallback otomatis `ML -> MAP`** untuk pola respons ekstrem (semua item
  terjawab bernilai identik) — ini logika level SESI CAT, bukan
  `ability_estimation::estimate()` itu sendiri. `Method::Ml` di Fase 3
  sengaja dibiarkan divergen apa adanya (lih.
  `docs/teori/03-ability-estimation-ml.md`) — keputusan tentang KAPAN
  beralih metode adalah kebijakan orkestrasi sesi, bukan properti estimator
  itu sendiri.
- **Pemilihan estimator otomatis** (`ML`/`MAP` berdasar kriteria seleksi
  item yang dipakai) — dibahas di
  `docs/teori/02-item-selection-overview.md` §5, juga level orkestrasi sesi.

## Peta dokumen per-method

| Method | Teori | Panduan |
|---|---|---|
| `Eap { qmc }` | `03-ability-estimation-eap.md` | `03-ability-estimation-eap.md` |
| `Map` | `03-ability-estimation-map.md` | `03-ability-estimation-map.md` |
| `Ml` | `03-ability-estimation-ml.md` | `03-ability-estimation-ml.md` |
| `Wle` | `03-ability-estimation-wle.md` | `03-ability-estimation-wle.md` |
| `EapSum` | `03-ability-estimation-eap-sum.md` | `03-ability-estimation-eap-sum.md` |
| `Fixed` | `03-ability-estimation-fixed.md` | `03-ability-estimation-fixed.md` |

## Referensi

- Bock, R. D., & Aitkin, M. (1981). Marginal maximum likelihood estimation
  of item parameters: Application of an EM algorithm. *Psychometrika*,
  46(4), 443–459.
- Bock, R. D., & Mislevy, R. J. (1982). Adaptive EAP estimation of ability
  in a microcomputer environment. *Applied Psychological Measurement*,
  6(4), 431–444.
- Kocis, L., & Whiten, W. J. (1997). Computational investigations of
  low-discrepancy sequences. *ACM Transactions on Mathematical Software*,
  23(2), 266–294.
- Warm, T. A. (1989). Weighted likelihood estimation of ability in item
  response theory. *Psychometrika*, 54(3), 427–450.
- Reckase, M. D. (2009). *Multidimensional Item Response Theory*, bab 5.
  Springer.
