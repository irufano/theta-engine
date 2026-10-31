//! `Mepv` — Minimum Expected Posterior Variance (Owen, 1975; van der
//! Linden, 1998). Pilih item yang, setelah dijawab, diharapkan membuat
//! posterior `theta` paling sempit.
//!
//! Untuk kandidat `j` dengan kategori respons `k = 0..K-1`:
//!
//! ```text
//! P(u_j = k)  = sum_g  w_g * P_jk(theta_g)              (prediktif)
//! Var_k       = Var(theta | data, u_j = k)              (posterior baru)
//! EPV_j       = sum_k  P(u_j = k) * Var_k
//! ```
//!
//! dengan `w_g` bobot posterior saat ini di grid. Engine mengembalikan
//! `-EPV_j` agar semua kriteria tetap dipilih lewat argmax.
//!
//! Hukum varians total menjamin `EPV_j <=` varians posterior saat ini, jadi
//! skor tertinggi yang mungkin adalah `-Var(theta | data)` (item yang tidak
//! memberi informasi).

use crate::error::McatError;
use crate::irt::ItemResponseModel;

use super::grid::{mean_and_variance, Grid1d};

/// Skor `Mepv` satu item kandidat: `-EPV` (makin tinggi, makin sempit
/// posterior yang diharapkan).
pub(crate) fn score(grid: &Grid1d, item: &dyn ItemResponseModel) -> Result<f64, McatError> {
    let probs = grid.probabilities(item)?;
    let mut expected_variance = 0.0;
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
        let (_, variance) = mean_and_variance(&grid.points, &joint);
        expected_variance += predictive * variance;
    }
    Ok(-expected_variance)
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
            "Mepv",
        )
        .unwrap()
    }

    #[test]
    fn uninformative_item_leaves_prior_variance_unchanged() {
        let grid = grid_no_answers();
        let flat = Dichotomous::m2pl(vec![0.0], 0.0).unwrap();
        // Prior N(0, 1): varians posterior saat ini ~ 1.
        assert_relative_eq!(score(&grid, &flat).unwrap(), -1.0, epsilon = 1e-3);
    }

    #[test]
    fn informative_item_shrinks_expected_variance_below_current() {
        let grid = grid_no_answers();
        let sharp = Dichotomous::m2pl(vec![2.0], 0.0).unwrap();
        let s = score(&grid, &sharp).unwrap();
        assert!(s > -1.0 + 1e-3, "EPV harus < varians saat ini: {s}");
        assert!(s < 0.0);
    }

    #[test]
    fn higher_discrimination_gives_higher_score() {
        let grid = grid_no_answers();
        let weak = Dichotomous::m2pl(vec![0.5], 0.0).unwrap();
        let strong = Dichotomous::m2pl(vec![2.0], 0.0).unwrap();
        assert!(score(&grid, &strong).unwrap() > score(&grid, &weak).unwrap());
    }

    #[test]
    fn matches_closed_form_for_two_category_item_by_brute_force() {
        let grid = grid_no_answers();
        let (a, d) = (1.3_f64, 0.4_f64);
        let item = Dichotomous::m2pl(vec![a], d).unwrap();
        let got = score(&grid, &item).unwrap();

        // Independen dari Grid1d: jumlah Riemann halus atas prior N(0, 1).
        let n = 24001;
        let (lo, hi) = (-6.0_f64, 6.0_f64);
        let step = (hi - lo) / (n - 1) as f64;
        // Momen (massa, sum t, sum t^2) untuk u=1 dan u=0, dibobot prior.
        let mut m1 = [0.0; 3];
        let mut m0 = [0.0; 3];
        let mut prior_mass = 0.0;
        for i in 0..n {
            let t = lo + step * i as f64;
            let prior = (-0.5 * t * t).exp();
            let p1 = 1.0 / (1.0 + (-(a * t + d)).exp());
            prior_mass += prior;
            for (m, p) in [(&mut m1, p1), (&mut m0, 1.0 - p1)] {
                m[0] += prior * p;
                m[1] += prior * p * t;
                m[2] += prior * p * t * t;
            }
        }
        let epv: f64 = [m1, m0]
            .iter()
            .map(|m| {
                let mean = m[1] / m[0];
                let var = m[2] / m[0] - mean * mean;
                (m[0] / prior_mass) * var
            })
            .sum();
        assert_relative_eq!(got, -epv, epsilon = 1e-4);
    }
}
