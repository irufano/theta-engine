# 01 — Theta Awal (Initial Theta)

## Konsep dasar

Sebelum item pertama disajikan, tiap peserta butuh titik awal vektor
kemampuan (`theta_0`), standard error awal (`SE_0`), dan bobot per-dimensi
(`weights`, dipakai belakangan oleh Fase 2). `initial_theta::initialize()`
menghitung ketiganya (lih. `docs/teori/01-theta-awal.md` untuk landasan
teorinya).

Default: `theta_0` = vektor nol, `SE_0` = `sqrt(diag(gcov))` dari
`GroupParams`, `weights` = semua `1.0`.

## `GroupParams` (`group`): wajib, dan dari mana asalnya

`group: &GroupParams` adalah parameter **wajib** di `initialize()` — bukan
`Option`, tidak bisa dilewati. Alasannya: `SE_0` **selalu** dihitung dari
`group.gcov()` (`sqrt(diag(gcov))`), apa pun isi `ThetaStart` (`Zero`,
`Fixed`, atau `PerPersonMatrix`). Override `theta_0` dan `group` adalah dua
hal yang **independen** — override cuma mengganti `theta_0`, `group` tetap
sumber tunggal untuk `SE_0` (dan `gmeans` untuk observability lewat
`InitialThetaDebug`).

`gmeans`/`gcov` di dalam `GroupParams` bukan dihitung oleh
`theta-engine` — itu parameter **populasi** hasil kalibrasi model IRT
terhadap data respons, di luar scope engine ini (kalibrasi bukan tanggung
jawab `theta-engine`, ia hanya *consumer* parameter yang sudah
dikalibrasi). Alur di dunia nyata:

1. **Kalibrasi di luar engine** dengan perangkat kalibrasi IRT
   multidimensional pilihan Anda. Selain parameter item, hasilnya memuat
   parameter populasi: rata-rata laten (`gmeans`, vektor panjang `nfact`)
   dan kovarians laten (`gcov`, matriks `nfact x nfact`). Data respons
   yang dikalibrasi di sini **bukan** log jawaban dari sesi CAT yang
   dijalankan `theta-engine` — itu data respons historis/pilot (baris = peserta,
   kolom = item, nilai = respons `0`/`1` atau kategori), biasanya dari
   administrasi tes **non-adaptif** (semua peserta menjawab semua/hampir
   semua item), dikumpulkan **sebelum** CAT dijalankan, semata untuk
   mengestimasi parameter item + populasi (`gmeans`/`gcov`) yang kemudian
   "dibekukan" jadi input tetap ke engine ini. Setelah parameter ini
   didapat, kalibrasi tidak perlu diulang tiap sesi CAT baru berjalan.
2. **Salin angka hasil kalibrasi itu ke Rust** (lewat config/JSON/DB — engine
   tidak membaca file model hasil kalibrasi langsung):
   ```rust
   use theta_engine::{GroupParams, ThetaVector};
   use theta_engine::theta::InfoMatrix;
   use nalgebra::DMatrix;

   let gmeans = ThetaVector::from_vec(vec![/* rata-rata populasi hasil kalibrasi */]);
   let gcov = InfoMatrix::from_matrix(
       DMatrix::from_row_slice(nfact, nfact, &[/* kovarians populasi, row-major */]),
   )?;
   let group = GroupParams::new(gmeans, gcov)?; // error bila dimensi gmeans != gcov
   ```

`GroupParams::standard_normal(nfact)` (`gmeans=0`, `gcov=identitas`) yang
dipakai di contoh-contoh berikut adalah **shortcut untuk development/testing
saja** — aman dipakai untuk belajar API atau unit test, tapi **jangan**
dipakai di produksi kalau kamu sudah punya hasil kalibrasi sungguhan:
`SE_0` jadi salah kalau varians populasi real bukan `1.0`, dan korelasi
antar-dimensi (off-diagonal `gcov`) ikut hilang — dua hal ini merembet ke
prior `MAP` (Fase 3) dan varian `*Prule` (Fase 2) yang juga memakai `gcov`
yang sama.

**Satu `group` dipakai untuk semua peserta** dalam satu sesi/simulasi —
beda dengan `ThetaStart` yang bisa beda per peserta (`Fixed`/
`PerPersonMatrix`). Kalau kamu ingin `gmeans`/`gcov` berbeda (mis. dua
kelompok populasi berbeda), itu berarti dua `GroupParams` terpisah, bukan
satu `GroupParams` yang "menyesuaikan" per peserta.

## Quickstart: default (tanpa override)

```rust
use theta_engine::initial_theta::{initialize, InitialThetaConfig};
use theta_engine::GroupParams;

let group = GroupParams::standard_normal(3); // gmeans=0, gcov=identitas
let config = InitialThetaConfig::default();

let (theta0, se0, weights, _debug) = initialize(&config, &group, 0)
    .expect("konfigurasi default selalu valid");

assert_eq!(theta0.as_slice(), &[0.0, 0.0, 0.0]);
assert_eq!(weights, vec![1.0, 1.0, 1.0]);
```

`person_index` (parameter ketiga) hanya relevan untuk
`ThetaStart::PerPersonMatrix` (lih. di bawah) — untuk kasus lain isi `0`.

## Kapan override manual masuk akal

Override lewat `ThetaStart::Fixed` masuk akal ketika ada informasi eksternal
tentang peserta sebelum tes dimulai — mis. skor placement test sebelumnya,
hasil tes paralel di dimensi yang sama, atau desain penelitian yang sengaja
menempatkan peserta pada titik `theta` tertentu (mis. simulasi Monte Carlo
untuk memvalidasi presisi CAT pada nilai `theta` "sebenarnya" yang diketahui).

```rust
use theta_engine::initial_theta::{initialize, InitialThetaConfig, ThetaStart};
use theta_engine::{GroupParams, ThetaVector};

let group = GroupParams::standard_normal(2);
let config = InitialThetaConfig {
    start: ThetaStart::Fixed(ThetaVector::from_vec(vec![0.5, -0.3])),
    weights: Some(vec![1.0, 2.0]), // dimensi ke-2 lebih diprioritaskan di Fase 2
    debug: false,
};

let (theta0, se0, weights, _) = initialize(&config, &group, 0).unwrap();
assert_eq!(theta0.as_slice(), &[0.5, -0.3]);
// se0 tetap dari sqrt(diag(gcov)) -- TIDAK berubah oleh override theta.
```

**Perhatikan**: dimensi `Fixed(theta)` (dan tiap entri `weights`) harus sama
dengan `nfact` model (`group.nfact()`), atau `initialize()` mengembalikan
`Err(McatError::DimensionMismatch)`.

## Simulasi offline banyak peserta (`PerPersonMatrix`)

Untuk menjalankan simulasi banyak peserta sekaligus (mis. menguji properti
statistik desain CAT lewat ribuan replikasi), tiap peserta bisa punya titik
awal berbeda:

```rust
use theta_engine::initial_theta::{initialize, InitialThetaConfig, ThetaStart};
use theta_engine::{GroupParams, ThetaVector};

let group = GroupParams::standard_normal(2);
let config = InitialThetaConfig {
    start: ThetaStart::PerPersonMatrix(vec![
        ThetaVector::from_vec(vec![0.2, -0.1]),  // peserta #0
        ThetaVector::from_vec(vec![-0.6, 0.3]),  // peserta #1
    ]),
    ..Default::default()
};

for person_index in 0..2 {
    let (theta0, _, _, _) = initialize(&config, &group, person_index).unwrap();
    println!("peserta #{person_index}: theta0 = {:?}", theta0.as_slice());
}
```

`person_index` di luar jangkauan (`>= jumlah baris`) menghasilkan
`Err(McatError::InvalidParameter(_))`.

## Debug trace

Aktifkan `debug: true` untuk melihat jalur `ThetaStart` yang dipakai serta
`gmeans`/diagonal `gcov` sumber — tidak mengubah `theta_0`/`SE_0`/`weights`
yang dikembalikan (debug hanya menambah observability):

```rust
use theta_engine::initial_theta::{initialize, InitialThetaConfig};
use theta_engine::GroupParams;

let group = GroupParams::standard_normal(2);
let config = InitialThetaConfig { debug: true, ..Default::default() };

let (_, _, _, debug) = initialize(&config, &group, 0).unwrap();
let debug = debug.unwrap();
println!("source = {:?}", debug.source);
println!("gcov_diag_used = {:?}", debug.gcov_diag_used);
```

Jalankan contoh lengkap:

```sh
cargo run --example 01_initial_theta
```

## Langkah selanjutnya

`theta_0`/`SE_0`/`weights` dari fase ini menjadi input `PersonState` yang
dipakai kriteria seleksi item D/T/A/E/W-rule (Fase 2) dan estimator
kemampuan (Fase 3) — lih.
[`02-item-selection-overview.md`](02-item-selection-overview.md) dan
[`03-ability-estimation-overview.md`](03-ability-estimation-overview.md).
