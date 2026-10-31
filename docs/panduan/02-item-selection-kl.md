# 02 — `KL`/`KLn`: Pendekatan KL Pointwise Engine

## Quickstart

```rust
use theta_engine::{select_next_item, Criterion, SelectionConfig};

let config_kl = SelectionConfig {
    criterion: Criterion::Kl { delta: 0.1 },
    ..Default::default()
};

let config_kln = SelectionConfig {
    criterion: Criterion::KlN { delta: 0.1 },
    ..Default::default()
};
```

`delta` sama untuk kedua varian — untuk `KlN`, `select_next_item()`
mengalikannya dengan `sqrt(person.n_answered())` secara internal (lih.
dokumen teori). Ini pendekatan pointwise milik engine, bukan indeks KL
global Chang & Ying (1996). Nilai umum untuk `delta`: `0.1`–`0.3` pada skala `theta`
standar (`N(0,1)`).

## Kapan dipakai

- `KL`: interval lokal tetap masuk akal sepanjang tes; lebih murah
  komputasi dibanding D/T/A/E/W-rule (tidak perlu determinan/invers/
  eigenvalue matriks `nfact × nfact`).
- `KLn`: interval evaluasi ingin ikut melebar seiring item terjawab
  bertambah. Pelebaran `sqrt(n)` adalah konvensi engine (kebalikan dari
  interval yang menyempit pada Chang & Ying), bukan hasil dari literatur.
- Cocok jadi kriteria default saat `nfact` besar (>4) dan D/T/A/E/W-rule
  jadi lebih mahal komputasi (matriks makin besar) — `KL`/`KLn` biayanya
  tidak bergantung `nfact` selain lewat panjang vektor probabilitas.

## Catatan praktis

- `delta` terlalu kecil → semua kandidat punya `KL` mirip (kurang
  diskriminatif). `delta` terlalu besar → kedua titik terlalu jauh dari
  estimasi, sehingga struktur lokal di sekitarnya terabaikan.
- `KLn` di `n_answered = 0` (`delta_efektif = 0`) menghasilkan `KL = 0`
  untuk **semua** kandidat (kedua titik evaluasi berimpit) — pemenang di
  langkah pertama hanya ditentukan aturan seri. Ini akibat langsung dari
  definisi, bukan bug;
  kombinasikan dengan `start_item` eksplisit (di luar Fase 2, kebijakan
  level orkestrasi sesi) untuk item pertama.

Contoh lengkap (perbandingan `KL` vs `KLn` seiring `n_answered` bertambah):
`cargo run --example 02_item_selection_kl`.
