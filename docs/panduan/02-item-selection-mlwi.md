# 02 — `Mlwi`: Maximum Likelihood Weighted Information

## Quickstart

```rust
use theta_engine::{select_next_item, Criterion, SelectionConfig};

let config = SelectionConfig {
    criterion: Criterion::Mlwi,
    ..Default::default()
};
// person, bank, group, candidates, rng seperti di
// docs/panduan/02-item-selection-overview.md; person.nfact() harus 1.
let (chosen, _debug) =
    select_next_item(&config, &candidates, &person, &bank, &group, &mut rng)?;
```

Informasi Fisher tiap kandidat dirata-ratakan atas `theta` dengan bobot
likelihood jawaban yang sudah ada, bukan hanya dievaluasi di estimasi titik.

`Criterion::Mlwi` tidak punya field: tidak ada `delta`, `prior`, atau
`weights` yang perlu diatur. `SelectionConfig::weights` diabaikan.

## Kapan dipakai

- Awal tes, ketika `theta_hat` masih tidak pasti dan item yang paling
  informatif di titik itu bisa meleset.
- Anda tidak ingin prior memengaruhi seleksi (bobot hanya likelihood). Bila
  prior ingin ikut, pakai `Mpwi`.
- Tes panjang: begitu posterior menyempit, hasilnya sama dengan kriteria
  informasi di satu titik (`DRule`), jadi tidak ada risiko lebih buruk.

## Catatan praktis

- **Item pertama.** Tanpa jawaban, likelihood datar dan skor tiap item
  hanya rata-rata informasinya atas `[-6, 6]`; item dengan `a` sama tetapi
  `b` berbeda hampir seri (lihat langkah 0 di contoh). Untuk item pertama
  pakai `Mpwi`/`Mepv`/`Mei`/`Ikl`, atau tetapkan item awal sendiri.
- Skor yang dicetak adalah informasi rata-rata berbobot (bobot menjumlah 1),
  bukan integral mentah; hanya urutannya yang bermakna.
- **Hanya `nfact = 1`.** Pada `nfact > 1`, `select_next_item` dan
  `rank_candidates` mengembalikan `Err(McatError::InvalidParameter)` yang
  menyebut `Mlwi`; tidak ada panic dan tidak ada hasil diam-diam keliru.
- Perbarui `theta` setelah tiap jawaban lewat `ability_estimation::estimate`
  + `person.set_estimate(..)`, seperti pada kriteria lain.
- `debug: true` mengisi `candidate_scores` untuk seluruh kandidat;
  `info_before`/`info_after` tetap `None` (kriteria ini tidak memakai
  matriks informasi kumulatif). Mengaktifkan debug tidak mengubah pilihan.
- Skor tiap kandidat tersedia lewat `rank_candidates`, sehingga bisa dipakai
  `exposure_control::select_with_exposure` seperti kriteria berskor lainnya.

Teori, rumus, dan rujukan: `docs/teori/02-item-selection-mlwi.md`.

Contoh lengkap (lima langkah CAT satu dimensi, dibandingkan dengan `DRule`):
`cargo run --example 02_item_selection_mlwi`.
