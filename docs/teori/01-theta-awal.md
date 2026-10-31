# 01 — Theta Awal (Initial Theta)

> Landasan teori untuk `src/initial_theta/mod.rs` (Fase 1).

## 1. Kenapa dibutuhkan titik awal

Sebelum item pertama dijawab, siklus adaptif MCAT (item selection, ability
estimation, stopping rule) butuh sebuah titik awal `theta_0` dan
ketidakpastian awal `SE_0` untuk mulai beroperasi — mis. kriteria seleksi
item D/T/A/E/W-rule (Fase 2) mengevaluasi matriks informasi Fisher relatif
terhadap `theta` saat ini, dan estimator MAP (Fase 3) butuh titik awal untuk
optimisasi numerik.

## 2. Prior populasi multivariat sebagai dasar

Pada IRT multidimensional, populasi peserta diasumsikan mengikuti distribusi
normal multivariat `theta ~ N(gmeans, gcov)`, hasil ekstraksi model
terkalibrasi (Segall, 1996; Reckase, 2009, bab 3). Model yang diidentifikasi
dengan skala standar (yang paling umum) menetapkan `gmeans = 0`, sehingga
titik tengah distribusi — pilihan titik awal yang wajar sebelum ada
informasi respons — bertepatan dengan vektor nol.

`theta-engine` menetapkan **`theta_0` literal vektor nol** sepanjang
`nfact`, bukan hasil membaca `gmeans` secara eksplisit setiap inisialisasi
(meski secara konsep keduanya berhimpit untuk model yang lazim). Ini
penting: `GroupParams` menyediakan `gmeans` untuk transparansi/debug (lih.
`InitialThetaDebug`) dan untuk fase-fase lain (mis. prior di MAP, Fase 3) —
bukan untuk menghitung `theta_0`.

`SE_0` diambil dari **akar diagonal** `gcov` populasi:

```
SE_0[i] = sqrt(gcov[i][i])
```

Hanya diagonal yang diambil — off-diagonal `gcov` menyatakan kovariansi
antar-dimensi, bukan presisi satu dimensi (lih. juga
`docs/teori/00-fondasi-irt-multidimensional.md` §1). `SE_0` dihitung dari
`gcov` **selalu**, terlepas dari apakah `theta_0` dioverride manual — dua
hal ini independen (lih. §4).

## 3. Override manual

Peneliti/operator tes kadang punya informasi eksternal tentang peserta
(mis. skor tes sebelumnya) yang lebih baik dari titik nol sebagai tebakan
awal. `theta-engine` mendukung ini lewat `ThetaStart`:

- **`ThetaStart::Fixed`** — satu vektor panjang `nfact`, dipakai untuk satu
  peserta (sesi CAT interaktif).
- **`ThetaStart::PerPersonMatrix`** — kumpulan vektor (setara matriks
  `N × nfact`), satu per peserta, dipakai untuk simulasi offline banyak
  peserta sekaligus; diindeks lewat parameter `person_index` pada
  `initialize()`.

## 4. `SE_0` independen dari override

`SE_0` dihitung dari `gcov` **sebelum** dan **terpisah dari** penentuan
`theta_0`, jadi override theta tidak pernah mengubah `SE_0`. Alasannya:
override hanya memindahkan tebakan titik awal, bukan menambah informasi
tentang presisi estimasi — ketidakpastian awal tetap ketidakpastian prior
populasi.

`tests/integration_initial_theta.rs` memverifikasi ini secara numerik
terhadap fixture `tests/fixtures/initial_theta.json`, untuk kasus default
maupun override, pada model 2D dan 3D dengan `gcov` non-diagonal: `theta_0`
bernilai `0` pada kasus default, sama dengan vektor override pada kasus
override, dan `SE_0 = sqrt(diag(gcov))` pada keduanya.

## 5. Bobot per-dimensi (`weights`)

`weights` (default `[1.0; nfact]`) diinisialisasi bersamaan `theta_0`/`SE_0`
walau baru dipakai di Fase 2. Satu vektor yang sama dibaca tiga kriteria,
dengan peran berbeda:

- `TRule` memakai `tr(W I)` dan `ARule` memakai `tr(W I^-1)`, dengan
  `W = diag(weights)`: bobot menentukan seberapa penting informasi (T) atau
  varians (A) tiap dimensi.
- `WRule` memakai `weights` sebagai koefisien skor komposit `w' theta`, dan
  memaksimalkan `w' I w`.

Bobot besar berarti dimensi (atau komposisi) itu lebih diutamakan. Default
satu semua adalah pilihan engine agar semua dimensi setara bila pemanggil
tidak menentukan apa-apa.

## Referensi

- Segall, D. O. (1996). Multidimensional adaptive testing.
  *Psychometrika*, 61(2), 331–354.
- Reckase, M. D. (2009). *Multidimensional Item Response Theory*, bab 3.
  Springer.
