//! `Mpwi` — Maximum Posterior Weighted Information (van der Linden, 1998).
//! Sama seperti `Mlwi`, tetapi bobotnya posterior (likelihood dikali prior
//! grup), bukan likelihood saja:
//!
//! ```text
//! MPWI_j = integral I_j(theta) * L(theta) * g(theta) d theta
//! ```
//!
//! dengan `g` kerapatan `N(gmeans, gcov)` dari `GroupParams`. Prior menahan
//! kriteria agar tidak terseret ke ekor yang tidak masuk akal saat tes baru
//! berjalan; makin banyak item terjawab, makin kecil pengaruhnya.

use crate::error::McatError;
use crate::irt::ItemResponseModel;

use super::grid::{weighted_information, Grid1d};

/// Skor `Mpwi` satu item kandidat: informasi Fisher berbobot posterior
/// ternormalisasi (bobot menjumlah 1; hanya urutan antarkandidat yang
/// bermakna).
pub(crate) fn score(grid: &Grid1d, item: &dyn ItemResponseModel) -> Result<f64, McatError> {
    weighted_information(grid, &grid.post_weights, item)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bank::{GroupParams, ItemBank, ItemMeta};
    use crate::irt::Dichotomous;
    use crate::item_selection::PersonState;
    use crate::theta::{InfoMatrix, ThetaVector};
    use approx::assert_relative_eq;

    fn empty_bank() -> ItemBank {
        let mut bank = ItemBank::new();
        bank.push(
            Box::new(Dichotomous::m2pl(vec![1.0], 0.0).unwrap()),
            ItemMeta::new(0),
        )
        .unwrap();
        bank
    }

    #[test]
    fn matches_independent_riemann_sum_of_information_times_prior() {
        let bank = empty_bank();
        let group = GroupParams::new(
            ThetaVector::from_vec(vec![0.5]),
            InfoMatrix::from_matrix(nalgebra::DMatrix::from_row_slice(1, 1, &[0.64])).unwrap(),
        )
        .unwrap();
        let grid = Grid1d::build(&PersonState::new(1), &bank, &group, "Mpwi").unwrap();

        let candidate = Dichotomous::m2pl(vec![1.3], -0.4).unwrap();
        let got = score(&grid, &candidate).unwrap();

        let (a, d) = (1.3_f64, -0.4_f64);
        let (mu, var) = (0.5_f64, 0.64_f64);
        let n = 24001;
        let (lo, hi) = (-6.0_f64, 6.0_f64);
        let step = (hi - lo) / (n - 1) as f64;
        let (mut num, mut den) = (0.0, 0.0);
        for i in 0..n {
            let t = lo + step * i as f64;
            let prior = (-(t - mu).powi(2) / (2.0 * var)).exp();
            let p = 1.0 / (1.0 + (-(a * t + d)).exp());
            num += a * a * p * (1.0 - p) * prior;
            den += prior;
        }
        assert_relative_eq!(got, num / den, epsilon = 1e-4);
    }

    #[test]
    fn with_no_answers_prefers_item_centered_on_prior_mean() {
        let bank = empty_bank();
        let group =
            GroupParams::new(ThetaVector::from_vec(vec![1.0]), InfoMatrix::identity(1)).unwrap();
        let grid = Grid1d::build(&PersonState::new(1), &bank, &group, "Mpwi").unwrap();

        let centered = Dichotomous::m2pl(vec![1.5], -1.5).unwrap(); // b = 1
        let off_center = Dichotomous::m2pl(vec![1.5], 1.5).unwrap(); // b = -1
        assert!(score(&grid, &centered).unwrap() > score(&grid, &off_center).unwrap());
    }

    #[test]
    fn differs_from_mlwi_when_prior_is_informative() {
        // Satu jawaban benar: likelihood menaruh massa di theta tinggi,
        // prior N(-1, 0.25) menariknya turun, jadi urutan dua kandidat bisa
        // berbeda antara bobot likelihood dan bobot posterior.
        let bank = empty_bank();
        let mut person = PersonState::new(1);
        person.record_answer(0, 1);
        let group = GroupParams::new(
            ThetaVector::from_vec(vec![-1.0]),
            InfoMatrix::from_matrix(nalgebra::DMatrix::from_row_slice(1, 1, &[0.25])).unwrap(),
        )
        .unwrap();
        let grid = Grid1d::build(&person, &bank, &group, "Mpwi").unwrap();

        let high = Dichotomous::m2pl(vec![2.0], -4.0).unwrap(); // b = 2
        let low = Dichotomous::m2pl(vec![2.0], 2.0).unwrap(); // b = -1
        let lik_prefers_high = super::super::mlwi::score(&grid, &high).unwrap()
            > super::super::mlwi::score(&grid, &low).unwrap();
        let post_prefers_low = score(&grid, &low).unwrap() > score(&grid, &high).unwrap();
        assert!(lik_prefers_high);
        assert!(post_prefers_low);
    }
}
