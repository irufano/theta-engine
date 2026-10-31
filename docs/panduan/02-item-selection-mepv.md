# 02 — `Mepv`: Minimum Expected Posterior Variance

## Quickstart

```rust
use theta_engine::{select_next_item, Criterion, SelectionConfig};

let config = SelectionConfig {
    criterion: Criterion::Mepv,
    ..Default::default()
};
// person, bank, group, candidates, rng seperti di
// docs/panduan/02-item-selection-overview.md; person.nfact() harus 1.
let (chosen, _debug) =
    select_next_item(&config, &candidates, &person, &bank, &group, &mut rng)?;
```

Mensimulasikan tiap kemungkinan jawaban kandidat, membentuk posterior
barunya, dan memilih item yang diharapkan membuat posterior paling sempit.
Nilai kriteria adalah `-EPV`, jadi selalu <= 0 dan makin dekat ke 0 makin
baik.

`Criterion::Mepv` tidak punya field: tidak ada `delta`, `prior`, atau
`weights` yang perlu diatur. `SelectionConfig::weights` diabaikan.

## Kapan dipakai

- Tujuan Anda eksplisit meminimalkan ketidakpastian `theta` (SE), bukan
  memaksimalkan informasi.
- Item politomus: varians dijumlahkan atas semua kategori, tanpa asumsi
  khusus dikotomus.
- Anda ingin kriteria yang langsung berhubungan dengan aturan berhenti
  `min_sem`: skornya adalah varians yang diharapkan sesudah item.

## Catatan praktis

- Skor dibatasi: `-Var(posterior saat ini) <= skor <= 0`. Item tanpa
  diskriminasi bernilai tepat batas bawah itu, sehingga selisih skor
  terhadap `-Var(saat ini)` langsung menunjukkan berapa varians yang
  diharapkan turun.
- Biaya `O(kandidat x kategori x 61)` per langkah (momen posterior dihitung
  per kategori per kandidat).
- **Hanya `nfact = 1`.** Pada `nfact > 1`, `select_next_item` dan
  `rank_candidates` mengembalikan `Err(McatError::InvalidParameter)` yang
  menyebut `Mepv`; tidak ada panic dan tidak ada hasil diam-diam keliru.
- Perbarui `theta` setelah tiap jawaban lewat `ability_estimation::estimate`
  + `person.set_estimate(..)`, seperti pada kriteria lain.
- `debug: true` mengisi `candidate_scores` untuk seluruh kandidat;
  `info_before`/`info_after` tetap `None` (kriteria ini tidak memakai
  matriks informasi kumulatif). Mengaktifkan debug tidak mengubah pilihan.
- Skor tiap kandidat tersedia lewat `rank_candidates`, sehingga bisa dipakai
  `exposure_control::select_with_exposure` seperti kriteria berskor lainnya.

Teori, rumus, dan rujukan: `docs/teori/02-item-selection-mepv.md`.

Contoh lengkap (lima langkah CAT satu dimensi, dibandingkan dengan `DRule`):
`cargo run --example 02_item_selection_mepv`.
