# 02 — `Mpwi`: Maximum Posterior Weighted Information

## Quickstart

```rust
use theta_engine::{select_next_item, Criterion, SelectionConfig};

let config = SelectionConfig {
    criterion: Criterion::Mpwi,
    ..Default::default()
};
// person, bank, group, candidates, rng seperti di
// docs/panduan/02-item-selection-overview.md; person.nfact() harus 1.
let (chosen, _debug) =
    select_next_item(&config, &candidates, &person, &bank, &group, &mut rng)?;
```

Seperti `Mlwi`, tetapi bobotnya posterior: likelihood jawaban dikali prior
grup `N(gmeans, gcov)` dari `GroupParams`.

`Criterion::Mpwi` tidak punya field: tidak ada `delta`, `prior`, atau
`weights` yang perlu diatur. `SelectionConfig::weights` diabaikan.

## Kapan dipakai

- Pilihan default yang masuk akal untuk CAT satu dimensi: prior menstabilkan
  seleksi di awal, dan pengaruhnya mengecil sendiri saat jawaban bertambah.
- Item pertama tetap bermakna: dengan prior `N(0, 1)` dan tanpa jawaban,
  item yang informasinya memuncak di dekat `gmeans` menang.
- Jika prior tidak dipercaya (populasi peserta jauh dari `gmeans`), pakai
  `Mlwi`.

## Catatan praktis

- Prior diambil dari `GroupParams` yang diteruskan ke `select_next_item`;
  gunakan `gmeans`/`gcov` yang sama dengan yang dipakai estimasi EAP/MAP agar
  seleksi dan estimasi koheren.
- Skor adalah informasi rata-rata di bawah posterior (bobot menjumlah 1).
- **Hanya `nfact = 1`.** Pada `nfact > 1`, `select_next_item` dan
  `rank_candidates` mengembalikan `Err(McatError::InvalidParameter)` yang
  menyebut `Mpwi`; tidak ada panic dan tidak ada hasil diam-diam keliru.
- Perbarui `theta` setelah tiap jawaban lewat `ability_estimation::estimate`
  + `person.set_estimate(..)`, seperti pada kriteria lain.
- `debug: true` mengisi `candidate_scores` untuk seluruh kandidat;
  `info_before`/`info_after` tetap `None` (kriteria ini tidak memakai
  matriks informasi kumulatif). Mengaktifkan debug tidak mengubah pilihan.
- Skor tiap kandidat tersedia lewat `rank_candidates`, sehingga bisa dipakai
  `exposure_control::select_with_exposure` seperti kriteria berskor lainnya.

Teori, rumus, dan rujukan: `docs/teori/02-item-selection-mpwi.md`.

Contoh lengkap (lima langkah CAT satu dimensi, dibandingkan dengan `DRule`):
`cargo run --example 02_item_selection_mpwi`.
