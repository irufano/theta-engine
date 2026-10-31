# 02 — `Mei`: Maximum Expected Information

## Quickstart

```rust
use theta_engine::{select_next_item, Criterion, SelectionConfig};

let config = SelectionConfig {
    criterion: Criterion::Mei,
    ..Default::default()
};
// person, bank, group, candidates, rng seperti di
// docs/panduan/02-item-selection-overview.md; person.nfact() harus 1.
let (chosen, _debug) =
    select_next_item(&config, &candidates, &person, &bank, &group, &mut rng)?;
```

Informasi Fisher kandidat dievaluasi di estimasi `theta` *sesudah* item itu
dijawab, dirata-ratakan atas kemungkinan jawaban berbobot peluang
prediktif.

`Criterion::Mei` tidak punya field: tidak ada `delta`, `prior`, atau
`weights` yang perlu diatur. `SelectionConfig::weights` diabaikan.

## Kapan dipakai

- Item sulit atau mudah relatif terhadap posterior: kriteria ini
  memperhitungkan bahwa jawaban itu sendiri menggeser estimasi.
- Sebagai pembanding `Mpwi`: `Mpwi` menilai informasi di bawah posterior
  saat ini, `Mei` di bawah posterior setelah respons.

## Catatan praktis

- Rumus yang dipakai adalah definisi engine (lih. dokumen teori); bila
  membandingkan dengan literatur, periksa bentuk persisnya.
- Memakai prior grup dan tidak bergantung pada `person.theta()`.
- **Hanya `nfact = 1`.** Pada `nfact > 1`, `select_next_item` dan
  `rank_candidates` mengembalikan `Err(McatError::InvalidParameter)` yang
  menyebut `Mei`; tidak ada panic dan tidak ada hasil diam-diam keliru.
- Perbarui `theta` setelah tiap jawaban lewat `ability_estimation::estimate`
  + `person.set_estimate(..)`, seperti pada kriteria lain.
- `debug: true` mengisi `candidate_scores` untuk seluruh kandidat;
  `info_before`/`info_after` tetap `None` (kriteria ini tidak memakai
  matriks informasi kumulatif). Mengaktifkan debug tidak mengubah pilihan.
- Skor tiap kandidat tersedia lewat `rank_candidates`, sehingga bisa dipakai
  `exposure_control::select_with_exposure` seperti kriteria berskor lainnya.

Teori, rumus, dan rujukan: `docs/teori/02-item-selection-mei.md`.

Contoh lengkap (lima langkah CAT satu dimensi, dibandingkan dengan `DRule`):
`cargo run --example 02_item_selection_mei`.
