# 02 — `Ikl`: Integrated Kullback-Leibler

## Quickstart

```rust
use theta_engine::{select_next_item, Criterion, SelectionConfig};

let config = SelectionConfig {
    criterion: Criterion::Ikl,
    ..Default::default()
};
// person, bank, group, candidates, rng seperti di
// docs/panduan/02-item-selection-overview.md; person.nfact() harus 1.
let (chosen, _debug) =
    select_next_item(&config, &candidates, &person, &bank, &group, &mut rng)?;
```

Divergensi KL antara distribusi respons kandidat di `theta` saat ini dan di
setiap `theta` lain, dirata-ratakan dengan bobot likelihood. Berbeda dari
`Kl`/`KlN`, tidak ada parameter `delta`.

`Criterion::Ikl` tidak punya field: tidak ada `delta`, `prior`, atau
`weights` yang perlu diatur. `SelectionConfig::weights` diabaikan.

## Kapan dipakai

- Anda ingin kriteria berbasis KL tanpa memilih `delta`: seluruh `theta`
  ditimbang sesuai dukungan data.
- Item pertama: berbeda dari `KlN` (yang bernilai nol untuk semua kandidat di
  langkah pertama), `Ikl` tetap terdefinisi karena memakai `person.theta()`.

## Catatan praktis

- **Bergantung pada `person.theta()`.** Satu-satunya kriteria 1D yang
  memakai estimasi titik; pastikan `set_estimate` dipanggil setelah tiap
  jawaban agar skor memakai estimasi terbaru.
- Bobot hanya likelihood (tanpa prior).
- Bukan indeks Chang & Ying asli (yang mengintegralkan atas interval sempit
  yang menyempit); lih. dokumen teori untuk perbedaannya.
- **Hanya `nfact = 1`.** Pada `nfact > 1`, `select_next_item` dan
  `rank_candidates` mengembalikan `Err(McatError::InvalidParameter)` yang
  menyebut `Ikl`; tidak ada panic dan tidak ada hasil diam-diam keliru.
- Perbarui `theta` setelah tiap jawaban lewat `ability_estimation::estimate`
  + `person.set_estimate(..)`, seperti pada kriteria lain.
- `debug: true` mengisi `candidate_scores` untuk seluruh kandidat;
  `info_before`/`info_after` tetap `None` (kriteria ini tidak memakai
  matriks informasi kumulatif). Mengaktifkan debug tidak mengubah pilihan.
- Skor tiap kandidat tersedia lewat `rank_candidates`, sehingga bisa dipakai
  `exposure_control::select_with_exposure` seperti kriteria berskor lainnya.

Teori, rumus, dan rujukan: `docs/teori/02-item-selection-ikl.md`.

Contoh lengkap (lima langkah CAT satu dimensi, dibandingkan dengan `DRule`):
`cargo run --example 02_item_selection_ikl`.
