# theta-engine

Engine Rust untuk **Multidimensional Computerized Adaptive Testing (MCAT)**, `nfact > 1` (multidimensional IRT). Kasus satu dimensi (`nfact = 1`) juga didukung. Didesain untuk dipakai sebagai dependency biasa di web API Rust (mis. Axum) atau project Rust lain, tanpa terikat framework tertentu.

`theta-engine` **tidak** melakukan kalibrasi model IRT dari data respons
mentah (EM/MH-RM, marginal maximum likelihood). Engine ini menerima
parameter item yang **sudah terkalibrasi** (slope, intercept, guessing,
dst.) sebagai input, lalu menjalankan siklus adaptif di atasnya:
theta awal → seleksi item → estimasi kemampuan → stopping rule → exposure
control → content balancing → hasil akhir.

## Status

Fase 0–8 selesai — fondasi IRT, siklus adaptif penuh, dan hardening rilis
(property test, benchmark, audit keamanan, dokumentasi API lengkap). Lihat
[`CHANGELOG.md`](CHANGELOG.md) untuk ringkasan per fase.

## Instalasi

Belum dipublikasikan ke crates.io. Untuk memakainya dari project Rust lain:

```toml
[dependencies]
theta-engine = { git = "https://github.com/irufano/theta-engine" }
# atau, dari checkout lokal:
# theta-engine = { path = "../theta-engine" }
# atau, setelah dipublikasikan:
# theta-engine = "0.1"
```

Fitur opsional `serde` mengaktifkan `Serialize`/`Deserialize` untuk
`McatResult` dan tipe pendukungnya (dibutuhkan untuk persist hasil lewat
HTTP/DB):

```toml
theta-engine = { version = "0.1", features = ["serde"] }
```

Dependency inti hanya [`nalgebra`](https://crates.io/crates/nalgebra)
(aljabar linear), [`rand`](https://crates.io/crates/rand),
[`thiserror`](https://crates.io/crates/thiserror), dan
[`log`](https://crates.io/crates/log) — tidak ada dependency ke framework
web apa pun.

## Quickstart

Menghitung probabilitas & informasi Fisher satu item (fondasi IRT, Fase 0):

```rust
use theta_engine::irt::{Dichotomous, ItemResponseModel};
use theta_engine::theta::ThetaVector;

let item = Dichotomous::m2pl(vec![1.2, 0.8], 0.3).unwrap();
let theta = ThetaVector::from_vec(vec![0.5, -0.2]);

let p = item.probabilities(&theta).unwrap(); // [P(u=0), P(u=1)]
let info = item.information(&theta).unwrap(); // matriks 2x2
```

Lihat [`docs/panduan/00-instalasi-dan-quickstart.md`](docs/panduan/00-instalasi-dan-quickstart.md)
untuk quickstart lengkap fondasi IRT. Contoh lain per fase ada di
[`examples/`](examples/) (`cargo run --example <nama>`).

## Dokumentasi

- [`docs/teori/`](docs/teori/) — landasan teori & rujukan akademik tiap
  algoritma (mulai dari
  [`00-fondasi-irt-multidimensional.md`](docs/teori/00-fondasi-irt-multidimensional.md)).
- [`docs/panduan/`](docs/panduan/) — panduan penggunaan API per fase
  (mulai dari
  [`00-instalasi-dan-quickstart.md`](docs/panduan/00-instalasi-dan-quickstart.md)).
- [`wasm/README.md`](wasm/README.md) — binding WebAssembly (paket npm
  `@dayatech/theta-engine-wasm`) untuk backend Node/Bun: pemakaian, kontrak
  galat, build, dan pengujian.
- `cargo doc --no-deps --open` — dokumentasi API level crate (`rustdoc`).

## Scope & Non-Scope

**Tujuan:**
- Siklus MCAT lengkap (theta awal sampai hasil akhir) dengan hasil numerik
  yang terverifikasi: integration test membandingkan output engine dengan
  nilai referensi beku di `tests/fixtures/` pada toleransi numerik kecil.
- API murni Rust (trait + struct), framework-agnostic — tidak ada
  dependency web di `[dependencies]` maupun `[dev-dependencies]`.
- Tiap algoritma diuji & didemonstrasikan independen (unit test, integration
  test, example).
- Landasan teori tiap algoritma didokumentasikan dengan rujukan akademik
  (lih. [`docs/teori/`](docs/teori/)).

**Di luar scope:**
- **Kalibrasi/estimasi parameter item** (EM, MH-RM, marginal maximum
  likelihood) — dilakukan dengan perangkat kalibrasi IRT terpisah; engine
  ini hanya mengonsumsi parameter hasilnya.
- **Shadow testing** / optimal test design berbasis linear programming
  untuk constraint konten yang kompleks.
- Antarmuka penyajian tes (UI/GUI) — library ini hanya menyediakan logika
  administrasi tes adaptif; penyajian item jadi urusan aplikasi pemanggil.
- Model IRT unidimensional bukan target utama, meski fondasi IRT cukup umum
  sehingga `nfact == 1` otomatis valid sebagai kasus khusus. Untuk kasus itu
  engine menyediakan lima kriteria seleksi tambahan yang hanya berlaku pada
  `nfact = 1` (`Mlwi`, `Mpwi`, `Mepv`, `Mei`, `Ikl`; lih.
  [`docs/panduan/02-item-selection-overview.md`](docs/panduan/02-item-selection-overview.md)).
  Selain itu, `Criterion::DRule` dan kawan-kawannya pada `nfact = 1` setara
  Maximum Fisher Information.

## Referensi

Rujukan akademik utama algoritma engine ini (daftar lengkap per algoritma
ada di bagian Referensi tiap dokumen [`docs/teori/`](docs/teori/)):

- Reckase, M. D. (2009). *Multidimensional Item Response Theory*. Springer.
- Segall, D. O. (1996). Multidimensional adaptive testing. *Psychometrika*,
  61(2), 331–354.
- Mulder, J., & van der Linden, W. J. (2009). Multidimensional adaptive
  testing with optimal design criteria for item selection. *Psychometrika*,
  74(2), 273–296.
- van der Linden, W. J. (1999). Multidimensional adaptive testing with a
  minimum error-variance criterion. *Journal of Educational and Behavioral
  Statistics*, 24(4), 398–412.
- Chang, H.-H., & Ying, Z. (1996). A global information approach to
  computerized adaptive testing. *Applied Psychological Measurement*,
  20(3), 213–229. (Literatur terkait kriteria KL; kriteria `KL`/`KLn` di
  engine ini adalah pendekatan pointwise sendiri, bukan indeks global ini.)
- Bock, R. D., & Aitkin, M. (1981). Marginal maximum likelihood estimation
  of item parameters: Application of an EM algorithm. *Psychometrika*,
  46(4), 443–459.
- Warm, T. A. (1989). Weighted likelihood estimation of ability in item
  response theory. *Psychometrika*, 54(3), 427–450.
- Wald, A. (1945). Sequential tests of statistical hypotheses. *Annals of
  Mathematical Statistics*, 16(2), 117–186.
- van der Linden, W. J., & Glas, C. A. W. (Eds.) (2010). *Elements of
  Adaptive Testing*. Springer.
- Sympson, J. B., & Hetter, R. D. (1985). Controlling item-exposure rates
  in computerized adaptive testing. *Proceedings of the 27th Annual Meeting
  of the Military Testing Association*, 973–977.
- Kingsbury, G. G., & Zara, A. R. (1989). Procedures for selecting items
  for computerized adaptive tests. *Applied Measurement in Education*,
  2(4), 359–375.
- Veldkamp, B. P., & van der Linden, W. J. (2002). Multidimensional
  adaptive testing with constraints on test content. *Psychometrika*,
  67(4), 575–588.

## Lisensi

Dilisensikan di bawah [Apache License 2.0](LICENSE).
