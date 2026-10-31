//! Fase 5 — Exposure Control pada MCAT: membatasi frekuensi kemunculan item
//! populer lintas peserta, dua mode (`Sympson–Hetter`/`SH` dan
//! `randomesque`/`sample`).
//!
//! Mode dideteksi dari nilai vektor `exposure` (lih.
//! [`ExposureMode::from_values`]). Rincian & rujukan akademik tiap mode di
//! modul privat `sympson_hetter` dan `randomesque`.
//!
//! **Keputusan desain**:
//!
//! 1. **`Criterion::Seq`/`Random` tidak pernah masuk exposure control.**
//!    Kedua kriteria itu memilih item tanpa menghitung `crit` (skor
//!    kandidat) sama sekali -- exposure control cuma
//!    berlaku untuk kriteria berbasis skor (`Kl`/`KlN`/`DRule`/`TRule`/
//!    `ARule`/`ERule`/`WRule`). Ditegakkan lewat
//!    [`crate::item_selection::rank_candidates`] (fungsi baru, Fase 2) yang
//!    mengembalikan `Err` untuk `Seq`/`Random` -- exposure control HARUS
//!    dipanggil dengan hasil fungsi itu, bukan `Vec` kosong buatan sendiri.
//! 2. **Status "item tidak valid untuk sesi ini" (`valid_items`) sengaja
//!    TIDAK ditambahkan sebagai field `PersonState`** (Fase 2,
//!    sudah selesai & teruji). Mengikuti preseden Fase 4
//!    (`allow_constrain_breaks` jadi parameter `bool` yang diteruskan
//!    pemanggil, bukan field `PersonState`, karena tanggung jawab
//!    lintas-fase yang belum ada orchestrator-nya):
//!    [`select_with_exposure`] menerima `valid_items: &mut [bool]` sebagai
//!    parameter eksplisit, dimiliki & disiapkan pemanggil (nantinya
//!    orkestrator sesi eksternal). Ini juga menyiapkan Fase 6 (constraint
//!    `independent` yang JUGA memutasi `valid_items`) untuk berbagi
//!    struktur yang sama tanpa desain ulang `PersonState`.
//! 3. **`rank_candidates` (Fase 2) diekstrak dari `select_next_item`**,
//!    bukan diimplementasikan ulang di sini -- exposure control murni
//!    mengonsumsi `Vec<(ItemIndex, f64)>` yang sudah ada, konsisten dengan
//!    Prinsip Desain #1 (jangan duplikasi logika skor kriteria).

mod randomesque;
mod sympson_hetter;

use rand::Rng;

use crate::error::McatError;
use crate::item_selection::ItemIndex;

/// Mode exposure control -- enum tertutup persis 2 mode (selain
/// `"none"`, yang di Rust direpresentasikan dengan **tidak memanggil**
/// [`select_with_exposure`] sama sekali -- lih. `Criterion` Fase 2 untuk
/// pola serupa: ketiadaan varian, bukan varian `None`, untuk state
/// "fitur ini nonaktif").
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ExposureMode {
    /// Seleksi terima/tolak per-item; nilai diindeks per **ID item**
    /// (panjang `n_items`). Lih. modul privat `sympson_hetter`.
    SympsonHetter(Vec<f64>),
    /// Top-*n* random; nilai diindeks per **posisi penyajian** (panjang
    /// `n_items`, elemen ke-`k` dipakai saat `k` item sudah terjawab).
    /// Lih. modul privat `randomesque`.
    Randomesque(Vec<u32>),
}

impl ExposureMode {
    /// Deteksi mode otomatis dari nilai vektor exposure:
    /// - Semua nilai `∈ [0, 1]` → [`ExposureMode::SympsonHetter`].
    /// - Selain itu (ada nilai `> 1`, atau ada nilai negatif) → mode
    ///   "sample"/[`ExposureMode::Randomesque`], TAPI seluruh elemen (bukan
    ///   cuma yang `> 1`) wajib bilangan bulat `>= 1` -- syaratnya
    ///   `trunc(v) == v && trunc(v) >= 1` (`trunc` memotong ke arah nol,
    ///   bukan pembulatan) -- error `InvalidParameter`
    ///   bila tidak.
    pub fn from_values(values: &[f64]) -> Result<Self, McatError> {
        let all_in_unit_interval = values.iter().all(|&v| (0.0..=1.0).contains(&v));
        if all_in_unit_interval {
            return Ok(ExposureMode::SympsonHetter(values.to_vec()));
        }

        let mut integers = Vec::with_capacity(values.len());
        for &v in values {
            let truncated = v.trunc();
            if truncated != v || truncated < 1.0 {
                return Err(McatError::InvalidParameter(format!(
                    "ExposureMode::from_values: nilai {v} tidak valid untuk mode randomesque/\
                     'sample' -- begitu ada elemen > 1 di vektor exposure, SELURUH elemen wajib \
                     bilangan bulat >= 1"
                )));
            }
            integers.push(truncated as u32);
        }
        Ok(ExposureMode::Randomesque(integers))
    }

    fn len(&self) -> usize {
        match self {
            ExposureMode::SympsonHetter(v) => v.len(),
            ExposureMode::Randomesque(v) => v.len(),
        }
    }
}

/// Konfigurasi exposure control.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ExposureControlConfig {
    /// Mode exposure control yang dipakai.
    pub mode: ExposureMode,
    /// Aktifkan jejak debug (prinsip desain #7). Default `false`.
    pub debug: bool,
}

impl ExposureControlConfig {
    /// Konstruktor yang memvalidasi panjang vektor exposure `== n_items`.
    pub fn new(mode: ExposureMode, n_items: usize, debug: bool) -> Result<Self, McatError> {
        if mode.len() != n_items {
            return Err(McatError::DimensionMismatch {
                expected: n_items,
                actual: mode.len(),
                context: "ExposureControlConfig::new: vektor exposure",
            });
        }
        Ok(Self { mode, debug })
    }
}

/// Jejak debug opsional dari [`select_with_exposure`] -- hanya `Some` bila
/// `ExposureControlConfig::debug == true`; tidak memengaruhi item yang
/// dipilih (prinsip desain #7).
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ExposureControlDebug {
    /// Sympson–Hetter: `(item, diterima)` tiap kandidat yang di-draw, urut
    /// peringkat -- pada jalur normal entri terakhir adalah `chosen`; pada
    /// fallback semua-ditolak seluruh entri `false` dan `chosen` sama dengan
    /// item entri pertama (pilihan paksa tidak punya draw).
    /// Randomesque: `(item, true)` untuk tiap kandidat di himpunan top-*n*
    /// yang dipertimbangkan sebelum pemilihan uniform random (urutan tidak
    /// bermakna peringkat, hanya keanggotaan).
    pub attempts: Vec<(ItemIndex, bool)>,
    /// `true` bila `chosen` (mode Sympson–Hetter) disajikan lewat fallback
    /// semua-ditolak (kandidat berperingkat teratas), bukan lolos draw
    /// uniform normal. Selalu `false` untuk mode Randomesque (tidak ada
    /// pilihan paksa di sana).
    pub forced_accept: bool,
}

/// Pilih item selanjutnya dari `candidate_scores` (hasil
/// [`crate::item_selection::rank_candidates`] -- **HARUS** sudah terfilter
/// agar tidak memuat item yang `valid_items[..] == false`, yaitu item
/// belum dijawab difilter dengan `valid_items` sebelum skor dihitung)
/// sesuai `config.mode`.
///
/// `position_in_test` = jumlah item yang sudah terjawab peserta ini
/// (0-based, mis. `person.n_answered()`) -- dipakai mode `Randomesque` utk
/// mengindeks vektor exposure per posisi (elemen
/// ke-`position_in_test`). `valid_items` (panjang `n_items` di bank, diindeks per ID
/// item) dimutasi in-place oleh mode `SympsonHetter` untuk item yang
/// ditolak -- pemanggil WAJIB meneruskan slice yang SAMA di langkah
/// berikutnya sesi peserta ini supaya invalidasi persisten sepanjang sesi
/// (lih. catatan modul di atas soal kenapa ini bukan field `PersonState`).
/// `rng` dipakai kedua mode untuk draw acak.
///
/// Error bila `candidate_scores` kosong (kehabisan item untuk
/// disajikan), atau `position_in_test` di luar jangkauan vektor
/// exposure (mode `Randomesque`).
pub fn select_with_exposure(
    config: &ExposureControlConfig,
    candidate_scores: &[(ItemIndex, f64)],
    position_in_test: usize,
    valid_items: &mut [bool],
    rng: &mut impl Rng,
) -> Result<(ItemIndex, Option<ExposureControlDebug>), McatError> {
    match &config.mode {
        ExposureMode::SympsonHetter(exposure) => {
            let outcome =
                sympson_hetter::select(exposure, candidate_scores, valid_items, rng, config.debug)?;
            let debug = config.debug.then_some(ExposureControlDebug {
                attempts: outcome.attempts,
                forced_accept: outcome.forced_accept,
            });
            Ok((outcome.chosen, debug))
        }
        ExposureMode::Randomesque(exposure) => {
            let exposure_for_position = *exposure.get(position_in_test).ok_or_else(|| {
                McatError::InvalidParameter(format!(
                    "select_with_exposure: position_in_test {position_in_test} di luar \
                     jangkauan vektor exposure (panjang {})",
                    exposure.len()
                ))
            })?;
            let outcome = randomesque::select(exposure_for_position, candidate_scores, rng)?;
            let debug = config.debug.then_some(ExposureControlDebug {
                attempts: outcome
                    .considered
                    .into_iter()
                    .map(|idx| (idx, true))
                    .collect(),
                forced_accept: false,
            });
            Ok((outcome.chosen, debug))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    #[test]
    fn from_values_all_unit_interval_is_sympson_hetter() {
        let mode = ExposureMode::from_values(&[0.0, 0.5, 1.0]).unwrap();
        assert_eq!(mode, ExposureMode::SympsonHetter(vec![0.0, 0.5, 1.0]));
    }

    #[test]
    fn from_values_any_value_above_one_is_randomesque() {
        let mode = ExposureMode::from_values(&[1.0, 3.0, 1.0]).unwrap();
        assert_eq!(mode, ExposureMode::Randomesque(vec![1, 3, 1]));
    }

    #[test]
    fn from_values_randomesque_rejects_non_integer_values() {
        let err = ExposureMode::from_values(&[1.0, 2.5]).unwrap_err();
        assert!(matches!(err, McatError::InvalidParameter(_)));
    }

    #[test]
    fn from_values_randomesque_rejects_values_below_one() {
        // 0.5 sendirian di [0,1] -> tapi 4.0 memicu mode sample; begitu mode
        // sample, SEMUA elemen (termasuk 0.5) wajib integer >= 1.
        let err = ExposureMode::from_values(&[0.5, 4.0]).unwrap_err();
        assert!(matches!(err, McatError::InvalidParameter(_)));
    }

    #[test]
    fn exposure_control_config_new_rejects_length_mismatch() {
        let mode = ExposureMode::SympsonHetter(vec![0.5, 0.5]);
        let err = ExposureControlConfig::new(mode, 3, false).unwrap_err();
        assert_eq!(
            err,
            McatError::DimensionMismatch {
                expected: 3,
                actual: 2,
                context: "ExposureControlConfig::new: vektor exposure",
            }
        );
    }

    #[test]
    fn select_with_exposure_dispatches_to_sympson_hetter() {
        let config =
            ExposureControlConfig::new(ExposureMode::SympsonHetter(vec![0.0, 1.0]), 2, true)
                .unwrap();
        let scores = [(0usize, 100.0), (1usize, 1.0)];
        let mut valid = vec![true; 2];
        let mut rng = StdRng::seed_from_u64(1);
        let (chosen, debug) =
            select_with_exposure(&config, &scores, 0, &mut valid, &mut rng).unwrap();
        assert_eq!(chosen, 1);
        assert!(!valid[0]);
        assert!(debug.is_some());
    }

    #[test]
    fn select_with_exposure_dispatches_to_randomesque_and_indexes_by_position() {
        let config =
            ExposureControlConfig::new(ExposureMode::Randomesque(vec![1, 1, 3]), 3, true).unwrap();
        let scores = [(0usize, 1.0), (1usize, 5.0), (2usize, 3.0)];
        let mut valid = vec![true; 3];
        let mut rng = StdRng::seed_from_u64(2);
        // position_in_test=0 -> exposure[0]=1 -> deterministik argmax.
        let (chosen, _) = select_with_exposure(&config, &scores, 0, &mut valid, &mut rng).unwrap();
        assert_eq!(chosen, 1);
    }

    #[test]
    fn select_with_exposure_errors_on_position_out_of_range() {
        let config =
            ExposureControlConfig::new(ExposureMode::Randomesque(vec![1, 1]), 2, false).unwrap();
        let scores = [(0usize, 1.0)];
        let mut valid = vec![true; 2];
        let mut rng = StdRng::seed_from_u64(3);
        let err = select_with_exposure(&config, &scores, 5, &mut valid, &mut rng).unwrap_err();
        assert!(matches!(err, McatError::InvalidParameter(_)));
    }

    #[test]
    fn select_with_exposure_errors_on_empty_candidates() {
        let config =
            ExposureControlConfig::new(ExposureMode::SympsonHetter(vec![0.5, 0.5]), 2, false)
                .unwrap();
        let mut valid = vec![true; 2];
        let mut rng = StdRng::seed_from_u64(4);
        assert!(select_with_exposure(&config, &[], 0, &mut valid, &mut rng).is_err());
    }

    #[test]
    fn debug_off_does_not_change_chosen_item_vs_debug_on() {
        let scores = [(0usize, 100.0), (1usize, 1.0)];

        let config_off =
            ExposureControlConfig::new(ExposureMode::SympsonHetter(vec![1.0, 1.0]), 2, false)
                .unwrap();
        let mut valid_off = vec![true; 2];
        let mut rng_off = StdRng::seed_from_u64(9);
        let (chosen_off, debug_off) =
            select_with_exposure(&config_off, &scores, 0, &mut valid_off, &mut rng_off).unwrap();
        assert!(debug_off.is_none());

        let config_on =
            ExposureControlConfig::new(ExposureMode::SympsonHetter(vec![1.0, 1.0]), 2, true)
                .unwrap();
        let mut valid_on = vec![true; 2];
        let mut rng_on = StdRng::seed_from_u64(9);
        let (chosen_on, debug_on) =
            select_with_exposure(&config_on, &scores, 0, &mut valid_on, &mut rng_on).unwrap();
        assert!(debug_on.is_some());

        assert_eq!(chosen_off, chosen_on);
    }
}
