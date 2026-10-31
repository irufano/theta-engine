//! Grid posterior satu dimensi, dipakai bersama oleh kriteria `Mepv`,
//! `Mlwi`, `Mpwi`, `Mei`, dan `Ikl`.
//!
//! Kelima kriteria itu merata-ratakan sebuah besaran item kandidat atas
//! ketidakpastian `theta`, bukan mengevaluasinya di satu titik estimasi.
//! Integralnya dihitung dengan aturan trapesium di atas grid reguler
//! `[-6, 6]` yang sama dengan grid EAP (`quadrature::default_quadpts(1)`
//! titik): titik ujung berbobot setengah, karena likelihood murni (prior
//! datar) tidak mendekati nol di ujung `+6` dan jumlah polos akan
//! menggelembungkan normalisasinya. Karena
//! integral itu hanya dipakai untuk **mengurutkan** kandidat, bobot grid
//! dinormalisasi menjumlah 1; konstanta pengali yang sama untuk semua
//! kandidat tidak mengubah urutan.
//!
//! Hanya valid untuk `nfact == 1`: grid kartesius `quadpts^nfact` meledak di
//! dimensi tinggi (lih. `docs/teori/02-item-selection-overview.md`).

use crate::ability_estimation::quadrature::{build_grid, default_quadpts};
use crate::ability_estimation::{density::log_dmvnorm, log_likelihood_at};
use crate::bank::{GroupParams, ItemBank};
use crate::error::McatError;
use crate::irt::ItemResponseModel;
use crate::item_selection::PersonState;
use crate::theta::ThetaVector;
use std::ops::Neg;

/// Grid satu dimensi beserta dua set bobot ternormalisasi.
pub(crate) struct Grid1d {
    /// Titik grid, urut naik dari `-6` ke `6`.
    pub points: Vec<f64>,
    /// Titik yang sama sebagai `ThetaVector` satu dimensi (siap dipakai
    /// `ItemResponseModel`).
    pub thetas: Vec<ThetaVector>,
    /// Bobot likelihood saja (prior datar), menjumlah 1.
    pub lik_weights: Vec<f64>,
    /// Bobot posterior (likelihood dikali prior grup), menjumlah 1.
    pub post_weights: Vec<f64>,
}

impl Grid1d {
    /// Bangun grid dari item terjawab `person` dan prior `group`.
    ///
    /// `criterion` hanya dipakai untuk pesan error. Error bila `person`
    /// bukan satu dimensi, atau tidak ada titik grid bermassa finite.
    pub(crate) fn build(
        person: &PersonState,
        bank: &ItemBank,
        group: &GroupParams,
        criterion: &str,
    ) -> Result<Self, McatError> {
        if person.nfact() != 1 {
            return Err(McatError::InvalidParameter(format!(
                "rank_candidates: kriteria {criterion} hanya berlaku untuk nfact = 1, \
                 didapat nfact = {}",
                person.nfact()
            )));
        }
        let thetas = build_grid(1, default_quadpts(1))?;
        let points: Vec<f64> = thetas.iter().map(|t| t[0]).collect();

        let last = thetas.len() - 1;
        let mut log_lik = Vec::with_capacity(thetas.len());
        let mut log_post = Vec::with_capacity(thetas.len());
        for (g, t) in thetas.iter().enumerate() {
            // Bobot trapesium: titik ujung dihitung setengah.
            let log_rule = if g == 0 || g == last {
                std::f64::consts::LN_2.neg()
            } else {
                0.0
            };
            let ll = log_likelihood_at(person, bank, t)? + log_rule;
            let lp = log_dmvnorm(t, group.gmeans(), group.gcov())?;
            log_lik.push(ll);
            log_post.push(ll + lp);
        }

        Ok(Self {
            points,
            thetas,
            lik_weights: normalize_log_weights(&log_lik, criterion)?,
            post_weights: normalize_log_weights(&log_post, criterion)?,
        })
    }

    /// Probabilitas kategori `item` di setiap titik grid: `[titik][kategori]`.
    pub(crate) fn probabilities(
        &self,
        item: &dyn ItemResponseModel,
    ) -> Result<Vec<Vec<f64>>, McatError> {
        self.thetas.iter().map(|t| item.probabilities(t)).collect()
    }
}

/// `exp(log_w - max)` lalu dinormalisasi menjumlah 1. Error bila tidak ada
/// log-bobot finite (likelihood/prior tak terdefinisi di seluruh grid).
fn normalize_log_weights(log_w: &[f64], criterion: &str) -> Result<Vec<f64>, McatError> {
    let max = log_w.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    if !max.is_finite() {
        return Err(McatError::InvalidParameter(format!(
            "rank_candidates: kriteria {criterion}: tidak ada titik grid bermassa posterior \
             finite -- periksa parameter item/grup"
        )));
    }
    let raw: Vec<f64> = log_w.iter().map(|lw| (lw - max).exp()).collect();
    let total: f64 = raw.iter().sum();
    Ok(raw.into_iter().map(|w| w / total).collect())
}

/// `sum_g weights[g] * I_item(theta_g)`: informasi Fisher `item` dirata-ratakan
/// atas grid dengan bobot `weights` (likelihood untuk `Mlwi`, posterior untuk
/// `Mpwi`).
pub(crate) fn weighted_information(
    grid: &Grid1d,
    weights: &[f64],
    item: &dyn ItemResponseModel,
) -> Result<f64, McatError> {
    let mut total = 0.0;
    for (theta, w) in grid.thetas.iter().zip(weights) {
        total += w * item.information(theta)?.get(0, 0);
    }
    Ok(total)
}

/// Momen pertama dan kedua dari `weights` atas `points`, dikembalikan sebagai
/// `(mean, variance)`. Varians dilantai nol (selisih momen bisa sedikit
/// negatif karena pembulatan).
pub(crate) fn mean_and_variance(points: &[f64], weights: &[f64]) -> (f64, f64) {
    let mass: f64 = weights.iter().sum();
    let mean = points.iter().zip(weights).map(|(x, w)| x * w).sum::<f64>() / mass;
    let second = points
        .iter()
        .zip(weights)
        .map(|(x, w)| x * x * w)
        .sum::<f64>()
        / mass;
    (mean, (second - mean * mean).max(0.0))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bank::ItemMeta;
    use crate::irt::Dichotomous;
    use approx::assert_relative_eq;

    fn bank_1d() -> ItemBank {
        let mut bank = ItemBank::new();
        bank.push(
            Box::new(Dichotomous::m2pl(vec![1.5], 0.0).unwrap()),
            ItemMeta::new(0),
        )
        .unwrap();
        bank
    }

    #[test]
    fn weights_sum_to_one_and_prior_only_grid_is_symmetric() {
        let bank = bank_1d();
        let grid = Grid1d::build(
            &PersonState::new(1),
            &bank,
            &GroupParams::standard_normal(1),
            "Mpwi",
        )
        .unwrap();
        assert_eq!(grid.points.len(), default_quadpts(1));
        assert_relative_eq!(grid.lik_weights.iter().sum::<f64>(), 1.0, epsilon = 1e-12);
        assert_relative_eq!(grid.post_weights.iter().sum::<f64>(), 1.0, epsilon = 1e-12);
        // Tanpa item terjawab likelihood datar, jadi bobot likelihood
        // seragam dan bobot posterior sama dengan prior: simetris.
        // Prior datar: bobot interior seragam, ujung setengahnya (trapesium).
        assert_relative_eq!(
            grid.lik_weights[1],
            1.0 / (grid.points.len() - 1) as f64,
            epsilon = 1e-12
        );
        assert_relative_eq!(
            grid.lik_weights[0],
            0.5 * grid.lik_weights[1],
            epsilon = 1e-12
        );
        let n = grid.points.len();
        assert_relative_eq!(
            grid.post_weights[0],
            grid.post_weights[n - 1],
            epsilon = 1e-12
        );
        let (mean, var) = mean_and_variance(&grid.points, &grid.post_weights);
        assert_relative_eq!(mean, 0.0, epsilon = 1e-9);
        assert_relative_eq!(var, 1.0, epsilon = 1e-3);
    }

    #[test]
    fn multidimensional_person_is_rejected() {
        let bank = bank_1d();
        let err = Grid1d::build(
            &PersonState::new(2),
            &bank,
            &GroupParams::standard_normal(2),
            "Mepv",
        )
        .err()
        .unwrap();
        assert!(err.to_string().contains("Mepv"));
        assert!(err.to_string().contains("nfact = 1"));
    }

    #[test]
    fn correct_answer_shifts_posterior_mass_upward() {
        let bank = bank_1d();
        let mut person = PersonState::new(1);
        person.record_answer(0, 1);
        let grid = Grid1d::build(&person, &bank, &GroupParams::standard_normal(1), "Mlwi").unwrap();
        let (mean_post, _) = mean_and_variance(&grid.points, &grid.post_weights);
        let (mean_lik, _) = mean_and_variance(&grid.points, &grid.lik_weights);
        assert!(mean_post > 0.0);
        // Prior menarik ke nol, jadi posterior lebih dekat ke nol daripada
        // likelihood saja.
        assert!(mean_lik > mean_post);
    }
}
