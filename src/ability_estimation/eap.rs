//! EAP (Expected A Posteriori) — grid-based & QMC-based, termasuk
//! pembangunan grid/bobotnya.
//!
//! - Grid-based: titik grid reguler, bobot = `dmvnorm(grid; gmeans, gcov)`.
//! - QMC-based: awan kuasi-`N(0, I)` dari `quadrature::build_qmc_points`
//!   ditransformasi menjadi kuasi-`N(gmeans, gcov)` lewat `theta = mu + L z`
//!   (`gcov = L L^T`, lih. `quadrature::transform_to_prior`). Titik-titik itu
//!   sudah sampel dari prior grup, jadi bobotnya **seragam**: menimbang lagi
//!   dengan densitas prior akan menghitung prior dua kali. Posterior mean/SE
//!   dihitung dari bobot likelihood saja, sama seperti mode grid.
//!
//! Hanya memakai item yang **sudah dijawab** (`person.answered_items()` +
//! `person.responses()`) — item yang belum dijawab tidak berkontribusi ke
//! likelihood; hanya kategori item yang terobservasi yang dipilih.

use crate::bank::{GroupParams, ItemBank};
use crate::error::McatError;
use crate::item_selection::PersonState;
use crate::theta::{SEVector, ThetaVector};

use super::density::log_dmvnorm;
use super::log_likelihood_at;
use super::quadrature::{build_grid, build_qmc_points, transform_to_prior};

/// Hasil EAP + jumlah titik kuadratur dipakai (untuk `AbilityEstimationDebug`).
pub(crate) struct EapResult {
    pub theta: ThetaVector,
    pub se: SEVector,
    pub n_quadrature_points: usize,
}

/// EAP grid-based (`qmc=false`) atau QMC-based (`qmc=true`). `quadpts` =
/// jumlah titik per dimensi (grid, total `quadpts^nfact`) atau jumlah titik
/// total (QMC).
pub(crate) fn estimate(
    person: &PersonState,
    bank: &ItemBank,
    group: &GroupParams,
    quadpts: usize,
    qmc: bool,
) -> Result<EapResult, McatError> {
    let nfact = person.nfact();

    let (grid, log_weights) = if qmc {
        // Titik adalah sampel dari prior grup sendiri (`mu + L z`), jadi
        // bobotnya seragam: menimbang lagi dengan densitas prior akan
        // menghitung prior dua kali.
        let z = build_qmc_points(nfact, quadpts)?;
        let points = transform_to_prior(&z, group.gmeans(), group.gcov())?;
        let log_w = vec![0.0; points.len()]; // seragam -- konstanta, hilang saat dinormalisasi
        (points, log_w)
    } else {
        let points = build_grid(nfact, quadpts)?;
        let mut log_w = Vec::with_capacity(points.len());
        for p in &points {
            log_w.push(log_dmvnorm(p, group.gmeans(), group.gcov())?);
        }
        (points, log_w)
    };
    let n = grid.len();

    let log_lik = log_likelihood_per_grid_point(person, bank, &grid)?;

    let log_post_max = log_lik
        .iter()
        .zip(&log_weights)
        .map(|(l, w)| l + w)
        .fold(f64::NEG_INFINITY, f64::max);
    let mut norm_const = 0.0;
    let mut num = vec![0.0; nfact];
    let mut post_w_cache = Vec::with_capacity(n);
    for g in 0..n {
        let lw = log_lik[g] + log_weights[g];
        let post_w = (lw - log_post_max).exp();
        post_w_cache.push(post_w);
        norm_const += post_w;
        for j in 0..nfact {
            num[j] += grid[g][j] * post_w;
        }
    }
    if norm_const == 0.0 || !norm_const.is_finite() {
        // Setelah log-posterior dikurangi maksimumnya, suku terbesar bernilai
        // 1, jadi konstanta normalisasi hanya nol/non-finite bila maksimum itu
        // sendiri `-inf`/NaN: tidak ada titik kuadratur dengan massa posterior
        // finite (mis. likelihood/prior tak terdefinisi di seluruh titik).
        // Estimasi titik (`Method::Map`) tidak bergantung pada kuadratur;
        // titik QMC menyebar mengikuti prior grup, bukan rentang grid tetap.
        return Err(McatError::InvalidParameter(
            "ability_estimation::eap::estimate: tidak ada titik kuadratur bermassa posterior \
             finite (konstanta normalisasi nol atau non-finite) -- periksa parameter item/grup, \
             atau pakai Method::Map yang tidak memerlukan kuadratur"
                .into(),
        ));
    }
    let theta_hat: Vec<f64> = num.iter().map(|v| v / norm_const).collect();

    // SE dari momen kedua posterior tertimbang (hanya diagonal):
    // `SE = sqrt(diag(vcov))`.
    let mut var = vec![0.0; nfact];
    for g in 0..n {
        for j in 0..nfact {
            var[j] += (grid[g][j] - theta_hat[j]).powi(2) * post_w_cache[g];
        }
    }
    let se: Vec<f64> = var.iter().map(|v| (v / norm_const).sqrt()).collect();

    Ok(EapResult {
        theta: ThetaVector::from_vec(theta_hat),
        se: SEVector::from_vec(se),
        n_quadrature_points: n,
    })
}

/// `log L(theta_g) = sum` atas item terjawab `log P_{response}(theta_g)`,
/// untuk tiap titik grid `g`. Item belum dijawab tidak berkontribusi.
/// Memakai `log_likelihood_at` (dipakai juga `map.rs`/`ml.rs`/`wle.rs`)
/// sekali per titik grid.
fn log_likelihood_per_grid_point(
    person: &PersonState,
    bank: &ItemBank,
    grid: &[ThetaVector],
) -> Result<Vec<f64>, McatError> {
    grid.iter()
        .map(|theta_g| log_likelihood_at(person, bank, theta_g))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bank::ItemMeta;
    use crate::irt::Dichotomous;
    use approx::assert_relative_eq;

    fn bank_1item_1d(a: f64, d: f64) -> ItemBank {
        let mut bank = ItemBank::new();
        bank.push(
            Box::new(Dichotomous::m2pl(vec![a], d).unwrap()),
            ItemMeta::new(0),
        )
        .unwrap();
        bank
    }

    /// Sanity check dasar 1D: item simetris (`a=1, d=0`) + prior `N(0,1)`
    /// simetris -> posterior mean untuk respons benar HARUS persis negatif
    /// dari posterior mean untuk respons salah (simetri analitik integral,
    /// bukan cuma tanda). Dihitung ulang independen lewat brute-force
    /// Riemann sum langsung di test ini (bukan lewat `build_grid` produksi)
    /// untuk verifikasi nilai numerik, bukan cuma struktur.
    #[test]
    fn eap_grid_1d_matches_independent_brute_force_and_is_symmetric() {
        let bank = bank_1item_1d(1.5, 0.0);
        let group = GroupParams::standard_normal(1);

        let mut person_correct = PersonState::new(1);
        person_correct.record_answer(0, 1);
        let mut person_incorrect = PersonState::new(1);
        person_incorrect.record_answer(0, 0);

        let quadpts = 401; // grid halus untuk mendekati integral kontinu
        let correct = estimate(&person_correct, &bank, &group, quadpts, false).unwrap();
        let incorrect = estimate(&person_incorrect, &bank, &group, quadpts, false).unwrap();

        assert_relative_eq!(correct.theta[0], -incorrect.theta[0], epsilon = 1e-9);
        assert!(
            correct.theta[0] > 0.0,
            "respons benar harus menarik theta positif"
        );

        // Brute-force independen: integral kontinu didekati Riemann sum
        // langsung (bukan reuse `build_grid`), dibandingkan terhadap hasil
        // produksi di atas.
        let item = bank.item(0).unwrap();
        let n = 20001;
        let lo = -6.0;
        let hi = 6.0;
        let step = (hi - lo) / (n - 1) as f64;
        let mut num = 0.0;
        let mut den = 0.0;
        for i in 0..n {
            let t = lo + step * i as f64;
            let theta = ThetaVector::from_vec(vec![t]);
            let p1 = item.probabilities(&theta).unwrap()[1];
            let prior = (-0.5 * t * t).exp(); // proporsional N(0,1), konstanta hilang saat normalisasi
            let w = p1 * prior;
            num += t * w;
            den += w;
        }
        let expected_theta = num / den;
        assert_relative_eq!(correct.theta[0], expected_theta, epsilon = 1e-3);
    }

    #[test]
    fn eap_returns_prior_mean_when_no_items_answered() {
        let bank = bank_1item_1d(1.0, 0.0);
        let mean = ThetaVector::from_vec(vec![0.4]);
        let group = GroupParams::new(mean.clone(), crate::theta::InfoMatrix::identity(1)).unwrap();
        let person = PersonState::new(1);
        let result = estimate(&person, &bank, &group, 201, false).unwrap();
        assert_relative_eq!(result.theta[0], 0.4, epsilon = 1e-6);
    }

    #[test]
    fn qmc_and_grid_agree_approximately_on_same_scenario() {
        let bank = bank_1item_1d(1.2, 0.0);
        let group = GroupParams::standard_normal(1);
        let mut person = PersonState::new(1);
        person.record_answer(0, 1);

        let grid_result = estimate(&person, &bank, &group, 401, false).unwrap();
        let qmc_result = estimate(&person, &bank, &group, 4000, true).unwrap();

        assert_relative_eq!(grid_result.theta[0], qmc_result.theta[0], epsilon = 0.05);
    }

    // ---- Prior grup pada mode QMC (regresi) ---------------------------

    fn bank_nd(items: &[(&[f64], f64)]) -> ItemBank {
        let mut bank = ItemBank::new();
        for (i, (a, d)) in items.iter().enumerate() {
            bank.push(
                Box::new(Dichotomous::m2pl(a.to_vec(), *d).unwrap()),
                ItemMeta::new(i),
            )
            .unwrap();
        }
        bank
    }

    fn group_nd(means: &[f64], cov_row_major: &[f64]) -> GroupParams {
        let n = means.len();
        GroupParams::new(
            ThetaVector::from_vec(means.to_vec()),
            crate::theta::InfoMatrix::from_matrix(nalgebra::DMatrix::from_row_slice(
                n,
                n,
                cov_row_major,
            ))
            .unwrap(),
        )
        .unwrap()
    }

    fn person_nd(nfact: usize, responses: &[usize]) -> PersonState {
        let mut person = PersonState::new(nfact);
        for (i, &r) in responses.iter().enumerate() {
            person.record_answer(i, r);
        }
        person
    }

    /// Selisih mutlak maksimum (theta, SE) antara EAP QMC dan EAP grid pada
    /// skenario yang sama.
    fn qmc_vs_grid_gap(
        person: &PersonState,
        bank: &ItemBank,
        group: &GroupParams,
        grid_quadpts: usize,
        qmc_points: usize,
    ) -> (f64, f64) {
        let grid = estimate(person, bank, group, grid_quadpts, false).unwrap();
        let qmc = estimate(person, bank, group, qmc_points, true).unwrap();
        let mut d_theta: f64 = 0.0;
        let mut d_se: f64 = 0.0;
        for j in 0..person.nfact() {
            d_theta = d_theta.max((grid.theta[j] - qmc.theta[j]).abs());
            d_se = d_se.max((grid.se[j] - qmc.se[j]).abs());
        }
        (d_theta, d_se)
    }

    const COV_2D: [f64; 4] = [1.5, 0.6, 0.6, 0.9];
    const COV_3D: [f64; 9] = [1.5, 0.6, 0.3, 0.6, 0.9, -0.2, 0.3, -0.2, 1.1];

    fn scenario_2d() -> (ItemBank, PersonState) {
        let bank = bank_nd(&[(&[1.2, 0.4], 0.3), (&[0.5, 1.1], -0.2), (&[0.9, 0.9], 0.0)]);
        (bank, person_nd(2, &[1, 0, 1]))
    }

    fn scenario_3d() -> (ItemBank, PersonState) {
        let bank = bank_nd(&[
            (&[1.0, 0.3, 0.2], 0.2),
            (&[0.2, 1.1, 0.4], -0.3),
            (&[0.3, 0.2, 1.2], 0.1),
            (&[0.8, 0.8, 0.5], 0.0),
        ]);
        (bank, person_nd(3, &[1, 1, 0, 1]))
    }

    // Toleransi QMC vs grid: pada 20000 titik leaped-Halton, selisih yang
    // teramati hanya ~2e-4 (prior N(0, I), acuan galat QMC + diskretisasi
    // grid), ~4e-5 (2D, prior non-standar) dan ~4e-4 untuk theta / ~1.2e-3
    // untuk SE (3D). 0.01 memberi ruang >= 8x atas yang terburuk, namun
    // ~70x di bawah bias yang muncul ketika prior diabaikan (selisih theta
    // 0.72 di 2D dan 1.0 di 3D), sehingga regresi tetap tertangkap.
    const QMC_GRID_TOLERANCE: f64 = 0.01;

    #[test]
    fn qmc_matches_grid_with_standard_normal_group_as_accuracy_reference_2d() {
        let (bank, person) = scenario_2d();
        let group = GroupParams::standard_normal(2);
        let (d_theta, d_se) = qmc_vs_grid_gap(&person, &bank, &group, 61, 20_000);
        assert!(d_theta < QMC_GRID_TOLERANCE / 2.0, "theta gap {d_theta}");
        assert!(d_se < QMC_GRID_TOLERANCE / 2.0, "se gap {d_se}");
    }

    #[test]
    fn qmc_applies_group_means_and_covariance_like_grid_2d() {
        let (bank, person) = scenario_2d();
        let group = group_nd(&[0.8, -0.5], &COV_2D);
        let (d_theta, d_se) = qmc_vs_grid_gap(&person, &bank, &group, 61, 20_000);
        assert!(d_theta < QMC_GRID_TOLERANCE, "theta gap {d_theta}");
        assert!(d_se < QMC_GRID_TOLERANCE, "se gap {d_se}");
    }

    #[test]
    fn qmc_applies_group_means_and_covariance_like_grid_3d() {
        let (bank, person) = scenario_3d();
        let group = group_nd(&[0.8, -0.5, 0.3], &COV_3D);
        let (d_theta, d_se) = qmc_vs_grid_gap(&person, &bank, &group, 41, 20_000);
        assert!(d_theta < QMC_GRID_TOLERANCE, "theta gap {d_theta}");
        assert!(d_se < QMC_GRID_TOLERANCE, "se gap {d_se}");
    }

    #[test]
    fn qmc_theta_moves_toward_group_means_when_few_items_answered() {
        // Satu item lemah: likelihood hampir datar, jadi posterior didominasi
        // prior dan theta harus bergeser dari hasil prior N(0, I) ke arah
        // `gmeans` di tiap dimensi.
        let bank = bank_nd(&[(&[0.3, 0.3], 0.0)]);
        let person = person_nd(2, &[1]);
        let means = [0.8, -0.5];
        let shifted = estimate(&person, &bank, &group_nd(&means, &COV_2D), 20_000, true).unwrap();
        let standard = estimate(
            &person,
            &bank,
            &GroupParams::standard_normal(2),
            20_000,
            true,
        )
        .unwrap();
        assert!(shifted.theta[0] - standard.theta[0] > 0.5);
        assert!(shifted.theta[1] - standard.theta[1] < -0.3);
    }

    #[test]
    fn qmc_with_standard_normal_group_is_identical_to_untransformed_points() {
        // Dengan gmeans=0 dan gcov=I transformasi prior adalah identitas
        // (L = I): hasilnya harus SAMA PERSIS dengan memakai awan kuasi-
        // N(0, I) apa adanya (perilaku sebelum prior diterapkan).
        let (bank, person) = scenario_2d();
        let group = GroupParams::standard_normal(2);
        let npts = 3000;
        let result = estimate(&person, &bank, &group, npts, true).unwrap();

        let points = build_qmc_points(2, npts).unwrap();
        let log_lik = log_likelihood_per_grid_point(&person, &bank, &points).unwrap();
        let max_ll = log_lik.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let w: Vec<f64> = log_lik.iter().map(|l| (l - max_ll).exp()).collect();
        let total: f64 = w.iter().sum();
        let expected: Vec<f64> = (0..2)
            .map(|j| points.iter().zip(&w).map(|(p, wi)| p[j] * wi).sum::<f64>() / total)
            .collect();
        for (j, &want) in expected.iter().enumerate() {
            assert_eq!(result.theta[j], want);
        }
    }

    #[test]
    fn qmc_rejects_non_positive_definite_or_asymmetric_covariance_without_panic() {
        let (bank, person) = scenario_2d();
        let bad_covs: [(&str, [f64; 4]); 4] = [
            ("indefinit", [1.0, 2.0, 2.0, 1.0]),
            ("tak simetris", [1.0, 0.5, 0.0, 1.0]),
            ("singular", [1.0, 1.0, 1.0, 1.0]),
            ("NaN", [1.0, f64::NAN, f64::NAN, 1.0]),
        ];
        for (name, cov) in bad_covs {
            let group = group_nd(&[0.0, 0.0], &cov);
            let result = estimate(&person, &bank, &group, 100, true);
            assert!(
                matches!(result, Err(McatError::InvalidParameter(_))),
                "gcov {name} harus ditolak sebagai InvalidParameter"
            );
        }
    }

    #[test]
    fn n_quadrature_points_matches_grid_and_qmc_sizes() {
        let bank = bank_1item_1d(1.0, 0.0);
        let group = GroupParams::standard_normal(2);
        let mut bank2d = ItemBank::new();
        bank2d
            .push(
                Box::new(Dichotomous::m2pl(vec![1.0, 0.5], 0.0).unwrap()),
                ItemMeta::new(0),
            )
            .unwrap();
        let _ = bank;
        let mut person = PersonState::new(2);
        person.record_answer(0, 1);

        let grid_result = estimate(&person, &bank2d, &group, 5, false).unwrap();
        assert_eq!(grid_result.n_quadrature_points, 25);

        let qmc_result = estimate(&person, &bank2d, &group, 123, true).unwrap();
        assert_eq!(qmc_result.n_quadrature_points, 123);
    }

    #[test]
    fn out_of_bank_answered_item_errors_not_panics() {
        let bank = bank_1item_1d(1.0, 0.0);
        let group = GroupParams::standard_normal(1);
        let mut person = PersonState::new(1);
        person.record_answer(999, 1);
        assert!(estimate(&person, &bank, &group, 21, false).is_err());
    }
}
