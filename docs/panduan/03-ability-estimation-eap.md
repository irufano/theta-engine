# 03 — `Eap`: Expected A Posteriori

## Quickstart

```rust
use theta_engine::ability_estimation::estimate;
use theta_engine::{EstimationConfig, Method};

// Grid-based (default quadpts dari tabel nfact)
let config_grid = EstimationConfig {
    method: Method::Eap { qmc: false },
    ..Default::default()
};

// QMC-based, quadpts dinaikkan manual untuk manfaat penuh (lih. §di bawah)
let config_qmc = EstimationConfig {
    method: Method::Eap { qmc: true },
    quadpts: Some(2000),
    ..Default::default()
};

let ((theta, se), _) = estimate(&config_grid, &person, &bank, &group)?;
```

## Kapan pakai grid vs QMC

- **Grid (`qmc: false`)**: default yang wajar untuk `nfact <= 4` — murah,
  akurat, tidak butuh tuning tambahan.
- **QMC (`qmc: true`)**: disarankan untuk `nfact > 4` — grid di dimensi
  ini dipaksa memakai `quadpts` sangat kecil (`<=7`, lih. tabel overview),
  QMC memungkinkan jumlah titik total lebih besar tanpa `quadpts^nfact`.
  **Naikkan `quadpts` manual** (mis. beberapa ribu) — default tetap
  mengikuti tabel `nfact` yang sama seperti grid (engine tidak
  membesarkannya otomatis untuk QMC).

Peringatan otomatis (`log::warn!`) muncul saat `Eap { qmc: false }` dipakai
dengan `person.nfact() > 4` — pasang logger (`env_logger`, `tracing-log`,
dst.) untuk melihatnya; tidak menghentikan eksekusi (bukan error).

## Catatan: prior grup pada mode QMC

Titik QMC adalah cuplikan dari prior grup itu sendiri: awan kuasi-`N(0,I)`
ditransformasi menjadi `N(gmeans, gcov)` dengan `theta = gmeans + L z`
(`gcov = L Lᵀ`, faktor Cholesky), lalu diberi bobot seragam (lih.
`docs/teori/03-ability-estimation-eap.md` §2). Hasilnya konsisten dengan mode
grid untuk grup mana pun. `gcov` harus simetris dan definit positif; kalau
tidak, `estimate` mengembalikan error, bukan panic.

Sebelum perbaikan ini, mode QMC mengabaikan `gmeans`/`gcov` dan hasilnya
keliru untuk grup yang prior-nya bukan `N(0, I)`.

## Debug trace: jumlah titik kuadratur

```rust
let config = EstimationConfig { method: Method::Eap { qmc: false }, debug: true, ..Default::default() };
let (_, debug) = estimate(&config, &person, &bank, &group)?;
println!("n_quadrature_points = {:?}", debug.unwrap().n_quadrature_points);
```

`n_quadrature_points` = `quadpts^nfact` untuk grid, `quadpts` (jumlah total
titik, sesuai parameter) untuk QMC.

Contoh lengkap (grid vs QMC pada pola respons yang sama, perbandingan
waktu komputasi): `cargo run --example 03_ability_estimation_eap`.
