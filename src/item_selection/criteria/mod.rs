//! Implementasi tiap kriteria seleksi item — satu file per algoritma
//! (lih. `docs/teori/02-item-selection-overview.md`). Fungsi di sini murni
//! (`total_info`/`theta`/`weights` sudah dibentuk oleh caller), dispatcher
//! ada di `item_selection::select_next_item` (`../mod.rs`).
//!
//! `mepv`, `mlwi`, `mpwi`, `mei`, dan `ikl` khusus `nfact = 1`: kelimanya
//! memakai grid posterior satu dimensi dari `grid`.

pub(crate) mod a_rule;
pub(crate) mod d_rule;
pub(crate) mod e_rule;
pub(crate) mod grid;
pub(crate) mod ikl;
pub(crate) mod kl;
pub(crate) mod mei;
pub(crate) mod mepv;
pub(crate) mod mlwi;
pub(crate) mod mpwi;
pub(crate) mod random;
pub(crate) mod seq;
pub(crate) mod t_rule;
pub(crate) mod w_rule;
