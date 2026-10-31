# 02 — Pemilihan Item (Item Selection): Overview

## Konsep dasar

`item_selection::select_next_item()` memilih item selanjutnya dari daftar
kandidat (index `ItemBank` yang belum dijawab) sesuai `SelectionConfig`.
Lih. `docs/teori/02-item-selection-overview.md` untuk landasan & kenapa
`nfact > 1` membatasi kriteria yang tersedia.

```rust
use theta_engine::{
    select_next_item, Criterion, GroupParams, PersonState, SelectionConfig,
};
use rand::rngs::StdRng;
use rand::SeedableRng;

// `bank: ItemBank` sudah ada (lih. docs/panduan/00-instalasi-dan-quickstart.md)
let group = GroupParams::standard_normal(bank.nfact().unwrap());
let person = PersonState::new(bank.nfact().unwrap());
let config = SelectionConfig::default(); // Criterion::Seq
let candidates: Vec<usize> = (0..bank.len()).collect();
let mut rng = StdRng::seed_from_u64(0); // dipakai hanya oleh Criterion::Random

let (chosen, _debug) =
    select_next_item(&config, &candidates, &person, &bank, &group, &mut rng)?;
```

## Memilih `Criterion`

| Situasi | Kriteria yang masuk akal |
|---|---|
| Kuesioner non-adaptif, urutan tetap | `Criterion::Seq` |
| Baseline exposure-neutral / kontrol eksperimen | `Criterion::Random` |
| Tidak butuh matriks informasi, cukup cepat | `Criterion::Kl`/`Criterion::KlN` |
| Presisi keseluruhan (volume elipsoid kepercayaan) | `Criterion::DRule` |
| Presisi per-dimensi, bisa diberi bobot | `Criterion::TRule` |
| Minimalkan varians estimasi (mendekati minimum error-variance) | `Criterion::ARule` |
| Cegah satu dimensi terus terabaikan | `Criterion::ERule` |
| Peduli skor komposit tertentu (`w' * theta`) | `Criterion::WRule` |
| `nfact = 1`, seleksi memperhitungkan ketidakpastian `theta` (bobot likelihood) | `Criterion::Mlwi` |
| `nfact = 1`, sama tetapi prior grup ikut menimbang (disarankan sebagai default 1D) | `Criterion::Mpwi` |
| `nfact = 1`, minimalkan varians posterior yang diharapkan | `Criterion::Mepv` |
| `nfact = 1`, informasi di estimasi sesudah respons | `Criterion::Mei` |
| `nfact = 1`, berbasis KL tanpa parameter `delta` | `Criterion::Ikl` |

`Mlwi`, `Mpwi`, `Mepv`, `Mei`, dan `Ikl` **khusus `nfact = 1`** dan tidak punya
field apa pun; pada `nfact > 1` keduanya mengembalikan `Err` (lih.
`docs/teori/02-item-selection-overview.md` §1). Pada `nfact = 1`,
D/T/A/E/W-rule sudah setara kriteria Maximum Fisher Information (informasi
kandidat di `theta` saat ini), jadi tidak ada varian `Mfi` terpisah.

Varian D/T/A/E/W-rule di atas punya field `prior: bool` — `true` memakai varian
`*Prule` (lih. §4 dokumen teori). **Disarankan `prior: true`** di awal tes
(saat item terjawab masih sedikit) untuk D/T/A/E/W-rule — tanpa prior,
`Drule`/`Erule` khususnya degenerate (nilai kriteria persis nol untuk semua
kandidat) sampai `>= nfact` item terjawab dengan arah info yang saling
bebas.

## `weights`: kapan dan bagaimana

`SelectionConfig.weights` (`Option<Vec<f64>>`, panjang harus = `nfact`)
dipakai `TRule`/`ARule`/`WRule` (+ varian prior), **diabaikan** kriteria
lain. `None` → default `vec![1.0; nfact]` (semua dimensi setara).

```rust
use theta_engine::{Criterion, SelectionConfig};

let config = SelectionConfig {
    criterion: Criterion::TRule { prior: false },
    weights: Some(vec![2.0, 0.5, 1.0]), // dimensi 1 diutamakan
    debug: false,
};
```

Gunakan `SelectionConfig::new(criterion, weights, nfact, debug)` bila ingin
validasi eksplisit (`Err(McatError::DimensionMismatch)` kalau panjang
`weights` tidak cocok `nfact`) alih-alih membangun struct literal langsung.

**Trule vs Wrule**, keduanya menerima `weights` tapi beda formula: `Trule`
menjumlahkan diagonal berbobot saja (`sum_i w_i * I[i,i]`); `Wrule` memakai
bentuk kuadratik penuh termasuk off-diagonal (`w' * I * w`) — informasi
tentang skor komposit `w' * theta`, bukan sekadar "dimensi mana yang
diprioritaskan". Pakai `Wrule` kalau yang dipedulikan benar-benar skor
gabungan (mis. total tertimbang beberapa sub-skala), `Trule` kalau cuma
mau memberi bobot relatif antar dimensi independen.

## `PersonState`: theta & item terjawab

```rust
use theta_engine::PersonState;

let mut person = PersonState::new(3); // theta/SE nol, belum ada item terjawab
// atau: PersonState::from_theta_and_se(theta0, se0) hasil Fase 1
// (initial_theta::initialize) -- lih. docs/panduan/03-ability-estimation-overview.md

person.record_answer(item_index, response_category); // catat item + kategori respons terjawab
person.set_estimate(new_theta, new_se); // update theta & SE (hasil estimasi ulang Fase 3)
```

`response_category` (ditambahkan Fase 3, dibutuhkan untuk menghitung
likelihood) diabaikan sepenuhnya oleh `select_next_item()`/`info_thetas()`
Fase 2 — hanya `Fase 3` (`ability_estimation::estimate()`) yang
memakainya.

`PersonState::info_thetas(&bank)` **menghitung ulang** matriks informasi
kumulatif dari daftar item terjawab tiap dipanggil (bukan field yang
di-cache) — otomatis konsisten begitu `set_estimate()`/`set_theta()`
dipanggil (lih. dokumen teori §2 untuk kenapa ini penting, bukan sekadar
detail implementasi).

## Kandidat & error

`candidates: &[usize]` adalah index `ItemBank` yang **belum dijawab** —
`select_next_item()` sendiri tidak melacak "item mana yang sudah dijawab",
itu tanggung jawab caller (biasanya: `(0..bank.len()).filter(|i|
!person.answered_items().contains(i))`, seperti contoh-contoh di
`examples/02_item_selection_*.rs`).

Error (`Err(McatError)`) terjadi bila: `candidates` kosong, ada index di
luar `bank`, panjang `weights` (default maupun manual) tidak cocok
`nfact`, atau kriteria khusus `nfact = 1` (`Mlwi`/`Mpwi`/`Mepv`/`Mei`/`Ikl`)
dipakai pada `nfact > 1` — tidak pernah panic.

## Debug trace

```rust
let config = SelectionConfig { debug: true, ..config };
let (chosen, debug) = select_next_item(&config, &candidates, &person, &bank, &group, &mut rng)?;
let debug = debug.unwrap();
println!("nilai kriteria tiap kandidat: {:?}", debug.candidate_scores);
println!("info sebelum/sesudah: {:?} -> {:?}", debug.info_before, debug.info_after);
```

`candidate_scores` kosong untuk `Seq`/`Random` (tidak ada nilai kriteria
untuk keduanya); `info_before`/`info_after` cuma `Some` untuk D/T/A/E/W-rule.
Untuk `Mepv` nilai kriteria adalah `-EPV` (varians posterior yang diharapkan,
dinegasikan), jadi selalu <= 0.
Mengaktifkan debug **tidak** mengubah item yang dipilih — hanya menambah
observability.

## Contoh lengkap per kriteria

```sh
cargo run --example 02_item_selection_seq
cargo run --example 02_item_selection_random
cargo run --example 02_item_selection_kl
cargo run --example 02_item_selection_d_rule
cargo run --example 02_item_selection_t_rule
cargo run --example 02_item_selection_a_rule
cargo run --example 02_item_selection_e_rule
cargo run --example 02_item_selection_w_rule
cargo run --example 02_item_selection_mlwi
cargo run --example 02_item_selection_mpwi
cargo run --example 02_item_selection_mepv
cargo run --example 02_item_selection_mei
cargo run --example 02_item_selection_ikl
```

Panduan rinci per algoritma: `docs/panduan/02-item-selection-{seq,random,
kl,d-rule,t-rule,a-rule,e-rule,w-rule,mlwi,mpwi,mepv,mei,ikl}.md`.
