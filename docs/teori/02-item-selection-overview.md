# 02 — Pemilihan Item (Item Selection): Overview

> Landasan teori untuk `src/item_selection/` (Fase 2). Lih. dokumen per
> algoritma untuk rumus & referensi rinci tiap kriteria.

## 1. Kenapa `nfact > 1` membatasi pilihan kriteria

Sebagian besar kriteria seleksi item unidimensional klasik (`MEPV`, `MLWI`,
`MPWI`, `MEI`, keluarga `IKL`) bergantung pada **integrasi kuadratur** atas
grid `theta` (mis. menghitung expected posterior variance lewat integral
numerik atas distribusi posterior). Di ruang laten multidimensional, grid
kuadratur beranak-pinak `quadpts^nfact` titik — untuk `nfact = 5` dengan
`quadpts = 21` per dimensi, itu ≈ 4 miliar titik. Ini yang disebut *curse of
dimensionality*: kriteria berbasis kuadratur langsung tidak praktis begitu
`nfact` lebih dari 2–3 (lih. juga
`docs/teori/03-ability-estimation-overview.md` untuk masalah serupa di
estimasi kemampuan).

Karena itu kriteria seleksi engine terbagi dua kelompok.

**Berlaku untuk `nfact` berapa pun** — tidak butuh integrasi kuadratur:

- **D/T/A/E/W-rule** — bersandar pada **matriks informasi Fisher**
  (turunan analitik tertutup, `O(nfact^2)` sampai `O(nfact^3)` per
  evaluasi, bukan integral), dievaluasi *pada satu titik* `theta` (estimasi
  terbaik saat ini), bukan atas seluruh distribusi posterior (Segall, 1996;
  Mulder & van der Linden, 2009).
- **KL** (`Kl`/`KlN`) — pendekatan pointwise milik engine: divergensi
  Kullback-Leibler antara dua titik ujung (`theta ± delta`), tanpa integral
  atas grid. Ini bukan indeks KL global Chang & Ying (1996) maupun KL
  multidimensi Wang, Chang & Boughton (2011); keduanya hanya literatur
  terkait (lih. `docs/teori/02-item-selection-kl.md`).
- **`Seq`/`Random`** — tidak butuh info sama sekali.

**Khusus `nfact = 1`** — merata-ratakan besaran item kandidat atas posterior
`theta` lewat grid satu dimensi (61 titik pada `[-6, 6]`, sama dengan grid
EAP):

| Kriteria | Besaran yang dirata-ratakan | Dokumen |
|---|---|---|
| `Mlwi` | informasi Fisher, bobot likelihood | `02-item-selection-mlwi.md` |
| `Mpwi` | informasi Fisher, bobot posterior | `02-item-selection-mpwi.md` |
| `Mepv` | varians posterior setelah item dijawab | `02-item-selection-mepv.md` |
| `Mei` | informasi Fisher di estimasi sesudah respons | `02-item-selection-mei.md` |
| `Ikl` | divergensi KL, bobot likelihood | `02-item-selection-ikl.md` |

Pada `nfact = 1` matriks informasi hanya 1×1, jadi D/T/A/E/W-rule semuanya
mengurutkan kandidat persis seperti kriteria **Maximum Fisher Information**
(informasi item kandidat di `theta` saat ini): total informasi hanya bergeser
oleh konstanta yang sama untuk semua kandidat (informasi item terjawab, dan
`1/σ²` bila `prior: true`). Engine karenanya tidak punya varian `Mfi`
tersendiri.

Kriteria khusus `nfact = 1` tetap ada di enum `item_selection::Criterion`
(satu enum untuk semua), sehingga pembatasannya ditegakkan saat **runtime**:
`rank_candidates`/`select_next_item` mengembalikan
`McatError::InvalidParameter` bila `PersonState::nfact() != 1`, dengan pesan
yang menyebut nama kriteria. Tidak ada hasil keliru yang dikembalikan diam-diam.
Perluasan kriteria itu ke multidimensi ada di literatur tetapi di luar
cakupan engine ini — keputusan ruang lingkup, bukan klaim bahwa kriteria itu
tidak bisa diperluas.

## 2. Matriks informasi Fisher kumulatif (`info_thetas`)

D/T/A/E/W-rule semuanya mengevaluasi fungsi (determinan, trace, invers,
eigenvalue, bentuk kuadratik) dari **matriks informasi Fisher kumulatif**
`nfact × nfact` — jumlah informasi dari seluruh item yang sudah dijawab,
ditambah kandidat yang sedang dievaluasi, semua pada titik `theta` estimasi
terkini.

**Detail penting**: `PersonState::info_thetas(bank)` **menghitung ulang**
dari daftar item terjawab setiap dipanggil — bukan mengakumulasi
inkremental. Dengan begitu info tiap item terjawab selalu dievaluasi di
estimasi `theta` **terbaru**, bukan "dibekukan" di `theta` saat item itu
dijawab. Kalau diakumulasi inkremental, info dari item-item awal akan jadi
basi begitu `theta` diestimasi ulang (Fase 3) setelah item-item berikutnya
dijawab — determinan/eigenvalue/trace yang dihitung dari campuran info
"segar" dan "basi" tidak lagi mencerminkan presisi asimtotik yang
sesungguhnya di estimasi terkini.

## 3. Ringkasan tiap kriteria

| Kriteria | Nilai yang dimaksimalkan | Bersandar pada | `weights`? |
|---|---|---|---|
| `Seq` | — (index terkecil) | — | tidak |
| `Random` | — (uniform random) | — | tidak |
| `Kl`/`KlN` | KL pointwise (dua titik ujung) item kandidat | probabilitas kandidat sendiri | tidak |
| `DRule` | `det(total_info)` | info kumulatif | tidak |
| `TRule` | `tr(W I) = sum_i w_i * total_info[i,i]` | info kumulatif | **ya** (`W = diag(w)`) |
| `ARule` | `-tr(W I^+) = -sum_i w_i * pseudo_inv(total_info)[i,i]` | info kumulatif | **ya** (`W = diag(w)`) |
| `ERule` | `min_eigenvalue(total_info)` | info kumulatif | tidak |
| `WRule` | `w' * total_info * w` | info kumulatif | **ya** (`w` = koefisien komposit) |
| `Mlwi` | `∫ I_j(θ) L(θ) dθ` | likelihood respons, grid 1D | tidak |
| `Mpwi` | `∫ I_j(θ) L(θ) g(θ) dθ` | posterior, grid 1D | tidak |
| `Mepv` | `-Σ_k P(u=k) Var(θ \| u=k)` | posterior, grid 1D | tidak |
| `Mei` | `Σ_k P(u=k) I_j(θ̂_k)` | posterior, grid 1D | tidak |
| `Ikl` | `∫ KL_j(θ̂ ‖ θ) L(θ) dθ` | likelihood respons, grid 1D | tidak |

`total_info` untuk D/T/A/E/W-rule = `info_thetas(bank)` (item terjawab) +
info Fisher kandidat pada `theta` saat ini, ditambah `inverse(gcov)` bila
`prior: true`. Kriteria khusus `nfact = 1` tidak memakai `total_info`;
rumus lengkapnya ada di dokumen masing-masing.

## 4. Varian prior (`prior: true`): informasi prior

`DRule`/`TRule`/`ARule`/`ERule`/`WRule` dengan `prior: true` menambahkan
`inverse(gcov)` (presisi prior populasi, `gcov` dari `GroupParams`) ke
`total_info` sebelum dievaluasi — pendekatan Bayesian (Segall, 1996): makin
sedikit item terjawab, makin besar pengaruh relatif prior terhadap
keputusan seleksi (mendekati prior belief tentang populasi); makin banyak
item terjawab, pengaruh prior makin kecil relatif terhadap info yang sudah
terkumpul dari data respons.

Konsekuensi praktis penting: **tanpa** prior, matriks informasi dari satu
item dikotomus tunggal (di `nfact > 1`) selalu **rank 1** (hasil kali luar
gradien terhadap satu skalar) — sebelum item pertama dijawab, `total_info`
untuk tiap kandidat individual juga rank 1, membuat `DRule` (determinan)
dan `ERule` (eigenvalue minimum) **persis nol** untuk semua kandidat
(tidak ada yang "menang" secara berarti — argmax di antara nilai identik).
Varian prior tidak kena masalah ini: `inverse(gcov)` selalu full rank
(matriks kovarians populasi valid), jadi `total_info` full rank sejak
langkah pertama.

## 5. Pasangan kriteria dan estimator kemampuan

Ini **konvensi engine ini**, bukan ketentuan dari sebuah makalah. Varian
`prior: true` menambahkan suku prior (`inverse(gcov)`) ke matriks informasi
yang dievaluasi, sehingga estimasi `theta` yang menjadi titik evaluasinya
koheren bila juga memakai prior, yaitu mode posterior (`MAP`). Varian tanpa
prior hanya memakai data respons, jadi dipasangkan dengan estimasi yang
murni likelihood (`ML`). Dengan begitu titik `theta` dan matriks informasi
yang dipakai kriteria berasal dari asumsi yang sama. Pemasangan lain tetap
sah secara teknis; ini hanya default yang disarankan.

Pilihan metode estimasi ulang `theta` (Fase 3, `ability_estimation::Method`
— lih. `docs/teori/03-ability-estimation-overview.md`) tetap menjadi
keputusan pemanggil. Pemilihan OTOMATIS `Method` berdasar `Criterion` adalah
kebijakan orkestrasi sesi CAT (belum diimplementasikan);
`ability_estimation::estimate()` menerima `Method` eksplisit dan tidak
menyimpulkannya dari `Criterion`.

## Referensi

- Segall, D. O. (1996). Multidimensional adaptive testing. *Psychometrika*,
  61(2), 331–354.
- van der Linden, W. J. (1999). Multidimensional adaptive testing with a
  minimum error-variance criterion. *Journal of Educational and Behavioral
  Statistics*, 24(4), 398–412.
- Mulder, J., & van der Linden, W. J. (2009). Multidimensional adaptive
  testing with optimal design criteria for item selection.
  *Psychometrika*, 74(2), 273–296.
- Owen, R. J. (1975). A Bayesian sequential procedure for quantal response
  in the context of adaptive mental testing. *Journal of the American
  Statistical Association*, 70(350), 351–356. (`Mepv`.)
- van der Linden, W. J. (1998). Bayesian item selection criteria for
  adaptive testing. *Psychometrika*, 63(2), 201–216. (`Mepv`, `Mpwi`,
  `Mei`.)
- Veerkamp, W. J. J., & Berger, M. P. F. (1997). Some new item selection
  criteria for adaptive testing. *Journal of Educational and Behavioral
  Statistics*, 22(2), 203–226. (`Mlwi`, dan gagasan pembobotan likelihood
  pada `Ikl`.)
- Chang, H.-H., & Ying, Z. (1996). A global information approach to
  computerized adaptive testing. *Applied Psychological Measurement*,
  20(3), 213–229. (Terkait `Kl`/`KlN`; bukan sumber rumusnya.)
- Wang, C., Chang, H.-H., & Boughton, K. A. (2011). Kullback-Leibler
  information and its applications in multi-dimensional adaptive testing.
  *Psychometrika*, 76(1), 13–39. (Terkait `Kl`/`KlN`; bukan sumber rumusnya.)
