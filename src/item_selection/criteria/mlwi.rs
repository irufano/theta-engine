//! `Mlwi` — Maximum Likelihood Weighted Information (Veerkamp & Berger,
//! 1997). Informasi Fisher item kandidat dirata-ratakan atas `theta`
//! dengan bobot likelihood respons yang sudah terkumpul:
//!
//! ```text
//! MLWI_j = integral I_j(theta) * L(theta) d theta
//! ```
//!
//! Beda dari kriteria informasi di satu titik (`I_j(theta_hat)`), MLWI tidak
//! menganggap estimasi `theta_hat` sudah benar. Di awal tes, ketika
//! likelihood masih lebar, item dinilai atas seluruh rentang `theta` yang
//! masih masuk akal; ketika likelihood menyempit di sekitar `theta_hat`,
//! nilainya mendekati `I_j(theta_hat)`.

use crate::error::McatError;
use crate::irt::ItemResponseModel;

use super::grid::{weighted_information, Grid1d};

/// Skor `Mlwi` satu item kandidat: informasi Fisher berbobot likelihood
/// ternormalisasi (bobot menjumlah 1; hanya urutan antarkandidat yang
/// bermakna).
pub(crate) fn score(grid: &Grid1d, item: &dyn ItemResponseModel) -> Result<f64, McatError> {
    weighted_information(grid, &grid.lik_weights, item)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bank::{GroupParams, ItemBank, ItemMeta};
    use crate::irt::Dichotomous;
    use crate::item_selection::PersonState;
    use approx::assert_relative_eq;

    fn bank_with_answered_probe() -> ItemBank {
        let mut bank = ItemBank::new();
        // item 0: dipakai sebagai item terjawab.
        bank.push(
            Box::new(Dichotomous::m2pl(vec![1.2], 0.0).unwrap()),
            ItemMeta::new(0),
        )
        .unwrap();
        bank
    }

    #[test]
    fn matches_independent_riemann_sum_of_information_times_likelihood() {
        let bank = bank_with_answered_probe();
        let mut person = PersonState::new(1);
        person.record_answer(0, 1);
        let grid = Grid1d::build(&person, &bank, &GroupParams::standard_normal(1), "Mlwi").unwrap();

        // Kandidat 2PL: a = 1.4, d = -0.7  (b = 0.5).
        let candidate = Dichotomous::m2pl(vec![1.4], -0.7).unwrap();
        let got = score(&grid, &candidate).unwrap();

        // Hitung ulang tanpa Grid1d: jumlah Riemann halus, rumus 2PL
        // `I = a^2 P Q` ditulis langsung.
        let (a_c, d_c) = (1.4_f64, -0.7_f64);
        let (a_0, d_0) = (1.2_f64, 0.0_f64);
        let n = 24001;
        let (lo, hi) = (-6.0_f64, 6.0_f64);
        let step = (hi - lo) / (n - 1) as f64;
        let (mut num, mut den) = (0.0, 0.0);
        for i in 0..n {
            let t = lo + step * i as f64;
            let like = 1.0 / (1.0 + (-(a_0 * t + d_0)).exp()); // jawaban benar
            let p = 1.0 / (1.0 + (-(a_c * t + d_c)).exp());
            num += a_c * a_c * p * (1.0 - p) * like;
            den += like;
        }
        assert_relative_eq!(got, num / den, epsilon = 1e-4);
    }

    #[test]
    fn prefers_item_whose_information_peaks_where_likelihood_concentrates() {
        // Sepuluh jawaban benar-salah bergantian pada item b = 1 menaruh
        // likelihood di sekitar theta = 1.
        let mut bank = ItemBank::new();
        bank.push(
            Box::new(Dichotomous::m2pl(vec![1.5], -1.5).unwrap()),
            ItemMeta::new(0),
        )
        .unwrap();
        let mut person = PersonState::new(1);
        for r in [1, 0, 1, 0, 1, 0, 1, 0] {
            person.record_answer(0, r);
        }
        let grid = Grid1d::build(&person, &bank, &GroupParams::standard_normal(1), "Mlwi").unwrap();

        let near = Dichotomous::m2pl(vec![1.5], -1.5).unwrap(); // b = 1
        let far = Dichotomous::m2pl(vec![1.5], 3.0).unwrap(); // b = -2
        assert!(score(&grid, &near).unwrap() > score(&grid, &far).unwrap());
    }

    #[test]
    fn zero_discrimination_item_has_zero_score() {
        let bank = bank_with_answered_probe();
        let grid = Grid1d::build(
            &PersonState::new(1),
            &bank,
            &GroupParams::standard_normal(1),
            "Mlwi",
        )
        .unwrap();
        let flat = Dichotomous::m2pl(vec![0.0], 0.3).unwrap();
        assert_relative_eq!(score(&grid, &flat).unwrap(), 0.0, epsilon = 1e-15);
    }
}
