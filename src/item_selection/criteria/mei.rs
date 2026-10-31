//! `Mei` — Maximum Expected Information (van der Linden, 1998). Informasi
//! Fisher item kandidat dievaluasi di estimasi `theta` **setelah** item itu
//! dijawab, lalu dirata-ratakan atas kemungkinan jawaban.
//!
//! Untuk kandidat `j` dengan kategori respons `k = 0..K-1`:
//!
//! ```text
//! P(u_j = k)   = sum_g  w_g * P_jk(theta_g)             (prediktif)
//! theta_hat_k  = E[theta | data, u_j = k]               (EAP baru)
//! MEI_j        = sum_k  P(u_j = k) * I_j(theta_hat_k)
//! ```
//!
//! dengan `w_g` bobot posterior saat ini di grid. Berbeda dari `Mpwi`, yang
//! merata-ratakan informasi atas posterior **saat ini**, `Mei` memperhitungkan
//! bahwa jawaban item itu sendiri menggeser estimasi.

use crate::error::McatError;
use crate::irt::ItemResponseModel;
use crate::theta::ThetaVector;

use super::grid::{mean_and_variance, Grid1d};

/// Skor `Mei` satu item kandidat: informasi yang diharapkan di estimasi
/// `theta` sesudah respons.
pub(crate) fn score(grid: &Grid1d, item: &dyn ItemResponseModel) -> Result<f64, McatError> {
    let probs = grid.probabilities(item)?;
    let mut expected_information = 0.0;
    for k in 0..item.n_categories() {
        let joint: Vec<f64> = grid
            .post_weights
            .iter()
            .zip(&probs)
            .map(|(w, p)| w * p[k])
            .collect();
        let predictive: f64 = joint.iter().sum();
        if predictive <= 0.0 {
            continue;
        }
        let (updated_theta, _) = mean_and_variance(&grid.points, &joint);
        let info = item
            .information(&ThetaVector::from_vec(vec![updated_theta]))?
            .get(0, 0);
        expected_information += predictive * info;
    }
    Ok(expected_information)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bank::{GroupParams, ItemBank, ItemMeta};
    use crate::irt::Dichotomous;
    use crate::item_selection::PersonState;
    use approx::assert_relative_eq;

    fn grid_no_answers() -> Grid1d {
        let mut bank = ItemBank::new();
        bank.push(
            Box::new(Dichotomous::m2pl(vec![1.0], 0.0).unwrap()),
            ItemMeta::new(0),
        )
        .unwrap();
        Grid1d::build(
            &PersonState::new(1),
            &bank,
            &GroupParams::standard_normal(1),
            "Mei",
        )
        .unwrap()
    }

    #[test]
    fn zero_discrimination_item_has_zero_score() {
        let grid = grid_no_answers();
        let flat = Dichotomous::m2pl(vec![0.0], 0.0).unwrap();
        assert_relative_eq!(score(&grid, &flat).unwrap(), 0.0, epsilon = 1e-15);
    }

    #[test]
    fn symmetric_item_and_prior_give_mirrored_updates_with_equal_information() {
        // Item a=1.5, d=0 dan prior N(0,1) simetris: respons benar dan salah
        // menggeser estimasi ke +x dan -x dengan peluang sama, dan I(+x) =
        // I(-x) untuk 2PL tanpa tebakan, sehingga MEI sama dengan
        // I(x) di titik geser tunggal itu.
        let grid = grid_no_answers();
        let item = Dichotomous::m2pl(vec![1.5], 0.0).unwrap();
        let got = score(&grid, &item).unwrap();

        let probs = grid.probabilities(&item).unwrap();
        let joint: Vec<f64> = grid
            .post_weights
            .iter()
            .zip(&probs)
            .map(|(w, p)| w * p[1])
            .collect();
        let (shift, _) = mean_and_variance(&grid.points, &joint);
        let p = 1.0 / (1.0 + (-(1.5 * shift)).exp());
        let expected = 1.5_f64.powi(2) * p * (1.0 - p);
        assert_relative_eq!(got, expected, epsilon = 1e-9);
    }

    #[test]
    fn item_matched_to_prior_mean_beats_distant_item() {
        let grid = grid_no_answers();
        let matched = Dichotomous::m2pl(vec![1.5], 0.0).unwrap(); // b = 0
        let distant = Dichotomous::m2pl(vec![1.5], 6.0).unwrap(); // b = -4
        assert!(score(&grid, &matched).unwrap() > score(&grid, &distant).unwrap());
    }
}
