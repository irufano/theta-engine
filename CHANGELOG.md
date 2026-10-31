# Changelog

Seluruh perubahan penting pada `theta-engine` dicatat di file ini.

Format mengikuti [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
dan proyek ini mengikuti [Semantic Versioning](https://semver.org/).

## [Unreleased]

Belum ada rilis `crates.io` — seluruh Fase 0–8 selesai di riwayat commit
ini, dikelompokkan di bawah sebagai satu entri "Added" sebelum rilis
pertama.

### Fixed

- EAP mode QMC (`Method::Eap { qmc: true }`) kini memakai prior grup:
  titik `N(0, I)` ditransformasi ke `N(gmeans, gcov)` lewat faktor
  Cholesky. Sebelumnya `gmeans`/`gcov` diabaikan sehingga hasil keliru untuk
  grup yang prior-nya bukan `N(0, I)` (selisih ke mode grid mencapai ~1.0
  pada uji regresi). `gcov` yang tidak simetris atau tidak definit positif
  kini menghasilkan `McatError`.

### Changed

- Package baru `theta-engine` (crate `theta_engine`).
- Lisensi Apache-2.0 (lih. `LICENSE`).
- Sympson–Hetter: kandidat diurutkan sekali lalu ditelusuri; bila semua
  kandidat ditolak, item peringkat teratas yang diberikan (sebelumnya item
  terakhir yang tersisa).
- Model item memakai logistik stabil tanpa batas atas logit, dan probabilitas
  kategori graded dilantai pada bilangan positif terkecil
  (`f64::MIN_POSITIVE`). Hasil di rentang normal sama dengan sebelumnya
  (selisih ~1e-12); di ekor ekstrem, informasi kini memuat nilai kecil yang
  akurat, bukan nilai jenuh.
- Klasifikasi `CI` dinyatakan sebagai uji interval `theta ± z·SE`; SE tidak
  hingga atau negatif menjadi cabang eksplisit "no decision". Hasilnya sama
  untuk `alpha` dalam (0, 0.5).

### Added

- Dukungan kompilasi `wasm32-unknown-unknown`: `rand` memakai
  `default-features = false, features = ["std_rng"]` sehingga `getrandom`
  tidak ikut ditarik. Perilaku tidak berubah: engine hanya memakai `StdRng`
  dengan seed eksplisit dari pemanggil, tidak pernah entropi OS. CI
  menjalankan `cargo check --target wasm32-unknown-unknown`.
- `Dichotomous::{mdisc, mdiff}` dan `GradedResponse::{mdisc, mdiff}`
  (`MDISC = ||a||`, `MDIFF = -d / MDISC`), diverifikasi pada toleransi
  `1e-6` terhadap nilai referensi di fixture
  `irt_probability_information.json`. `MDIFF` bernilai `0` bila semua
  slope nol.
- **Fase 0 — Fondasi IRT**: trait `ItemResponseModel`, model item
  `Dichotomous` (M2PL/M3PL/M4PL) & `GradedResponse`, tipe `ThetaVector`/
  `SEVector`/`InfoMatrix`, `ItemBank`/`GroupParams`.
- **Fase 1 — Theta Awal**: inisialisasi `theta`/SE dari populasi atau
  langsung (`initial_theta`).
- **Fase 2 — Seleksi Item**: kriteria D/T/A/E/W-rule (+ varian `*Prule`),
  KL/KLn, `seq`, `random` (`item_selection`).
- **Fase 3 — Estimasi Kemampuan**: EAP (grid & QMC), MAP, ML, WLE, EAP
  berbasis summed score, `Fixed` (`ability_estimation`).
- **Fase 4 — Stopping Rule**: `min_sem`, `delta_thetas`, klasifikasi `CI`
  & `SPRT`, `max_items`/`max_time`, hook `CustomStop` (`stopping_rule`).
- **Fase 5 — Exposure Control**: mode Sympson–Hetter & randomesque
  (`exposure_control`).
- **Fase 6 — Content Balancing**: penyempitan kandidat berbasis defisit
  proporsi kategori konten (`content_balancing`).
- **Fase 7 — Hasil Akhir**: `McatResult` — ringkasan bersih `theta`/SE/
  riwayat untuk konsumen di luar engine (`result`).
- **Fase 8 — Hardening & Kesiapan Rilis**:
  - Property-based test (`proptest`) untuk invariant lintas modul:
    probabilitas kategori selalu menjumlah 1, matriks informasi Fisher
    kumulatif selalu positive semi-definite, proporsi empiris content
    balancing tidak pernah negatif (`tests/proptest_invariants.rs`).
  - Benchmark `criterion` untuk seluruh kriteria seleksi item & metode
    estimasi kemampuan pada item bank 500 item/5 dimensi
    (`benches/theta_engine.rs`).
  - Dokumentasi API level crate lengkap — `cargo doc --no-deps` bersih
    tanpa warning (`#![deny(missing_docs)]`, seluruh item publik
    terdokumentasi, tautan intra-doc ke item privat diperbaiki).
  - `README.md`, `CHANGELOG.md`.
  - Audit keamanan dasar: tidak ada `unsafe` di `src/`; seluruh
    `.expect()`/`unreachable!()` non-test di `src/` diverifikasi hanya
    menjaga invariant yang sudah divalidasi sebelumnya (konstruktor,
    exhaustive match), bukan reachable dari input eksternal tak
    tervalidasi — semua batasan pengguna tetap lewat `Result<_,
    McatError>`.
  - CI: langkah `cargo doc --no-deps` (default & fitur `serde`) ditambah
    ke `check` job.
  - `cargo publish --dry-run` bersih.
