//! `Ikl` — Integrated Kullback-Leibler: divergensi KL antara distribusi
//! respons item kandidat di estimasi `theta_hat` dan di setiap `theta`
//! lain, dirata-ratakan dengan bobot likelihood.
//!
//! ```text
//! KL_j(theta_hat || theta) = sum_k P_jk(theta_hat) * ln( P_jk(theta_hat) / P_jk(theta) )
//! IKL_j                    = integral KL_j(theta_hat || theta) * L(theta) d theta
//! ```
//!
//! Indeks KL dan gagasan "informasi global" berasal dari Chang & Ying
//! (1996); pembobotan dengan likelihood mengikuti gagasan yang sama dengan
//! `Mlwi` (Veerkamp & Berger, 1997). Item yang skornya besar adalah item
//! yang distribusi responsnya di `theta_hat` paling mudah dibedakan dari
//! distribusi di nilai `theta` lain yang masih didukung data.
//!
//! Bedanya dengan [`Criterion::Kl`](crate::Criterion::Kl): `Kl` hanya
//! membandingkan dua titik ujung `theta +- delta`, sedangkan `Ikl`
//! mengintegralkan atas seluruh grid dan tidak punya parameter `delta`.
//! Bedanya dengan indeks Chang & Ying asli: integralnya atas seluruh grid
//! `[-6, 6]` dengan bobot likelihood, bukan atas interval sempit
//! `theta_hat +- c / sqrt(n)`.

use crate::error::McatError;
use crate::irt::ItemResponseModel;
use crate::theta::ThetaVector;

use super::grid::Grid1d;

/// Skor `Ikl` satu item kandidat, dihitung di estimasi `theta_hat`.
pub(crate) fn score(
    grid: &Grid1d,
    item: &dyn ItemResponseModel,
    theta_hat: &ThetaVector,
) -> Result<f64, McatError> {
    let p_hat = item.probabilities(theta_hat)?;
    let probs = grid.probabilities(item)?;
    let mut total = 0.0;
    for (w, p) in grid.lik_weights.iter().zip(&probs) {
        let kl: f64 = p_hat
            .iter()
            .zip(p)
            .filter(|(hat, _)| **hat > 0.0)
            .map(|(hat, other)| hat * (hat.ln() - other.max(f64::MIN_POSITIVE).ln()))
            .sum();
        total += w * kl;
    }
    Ok(total)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bank::{GroupParams, ItemBank, ItemMeta};
    use crate::irt::Dichotomous;
    use crate::item_selection::PersonState;
    use approx::assert_relative_eq;

    fn grid_after(responses: &[usize]) -> Grid1d {
        let mut bank = ItemBank::new();
        bank.push(
            Box::new(Dichotomous::m2pl(vec![1.2], 0.0).unwrap()),
            ItemMeta::new(0),
        )
        .unwrap();
        let mut person = PersonState::new(1);
        for &r in responses {
            person.record_answer(0, r);
        }
        Grid1d::build(&person, &bank, &GroupParams::standard_normal(1), "Ikl").unwrap()
    }

    #[test]
    fn zero_discrimination_item_has_zero_score() {
        let grid = grid_after(&[1, 0]);
        let flat = Dichotomous::m2pl(vec![0.0], 0.2).unwrap();
        let theta_hat = ThetaVector::from_vec(vec![0.0]);
        assert_relative_eq!(
            score(&grid, &flat, &theta_hat).unwrap(),
            0.0,
            epsilon = 1e-12
        );
    }

    #[test]
    fn score_is_nonnegative_and_grows_with_discrimination() {
        let grid = grid_after(&[1, 0, 1, 0]);
        let theta_hat = ThetaVector::from_vec(vec![0.0]);
        let weak = Dichotomous::m2pl(vec![0.5], 0.0).unwrap();
        let strong = Dichotomous::m2pl(vec![2.0], 0.0).unwrap();
        let (s_weak, s_strong) = (
            score(&grid, &weak, &theta_hat).unwrap(),
            score(&grid, &strong, &theta_hat).unwrap(),
        );
        assert!(s_weak >= 0.0);
        assert!(s_strong > s_weak);
    }

    #[test]
    fn matches_independent_riemann_sum() {
        let grid = grid_after(&[1, 1, 0]);
        let (a, d) = (1.1_f64, -0.3_f64);
        let item = Dichotomous::m2pl(vec![a], d).unwrap();
        let theta_hat_value = 0.4_f64;
        let got = score(&grid, &item, &ThetaVector::from_vec(vec![theta_hat_value])).unwrap();

        let p_at = |t: f64| 1.0 / (1.0 + (-(a * t + d)).exp());
        let like_at = |t: f64| {
            let p = 1.0 / (1.0 + (-(1.2 * t)).exp());
            p * p * (1.0 - p) // dua benar, satu salah
        };
        let p1_hat = p_at(theta_hat_value);
        let n = 24001;
        let (lo, hi) = (-6.0_f64, 6.0_f64);
        let step = (hi - lo) / (n - 1) as f64;
        let (mut num, mut den) = (0.0, 0.0);
        for i in 0..n {
            let t = lo + step * i as f64;
            let p1 = p_at(t);
            let kl =
                p1_hat * (p1_hat / p1).ln() + (1.0 - p1_hat) * ((1.0 - p1_hat) / (1.0 - p1)).ln();
            num += kl * like_at(t);
            den += like_at(t);
        }
        assert_relative_eq!(got, num / den, epsilon = 1e-4);
    }
}
