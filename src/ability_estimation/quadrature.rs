//! Kuadratur untuk EAP grid-based & titik Quasi-Monte Carlo (QMC) untuk EAP
//! dimensi tinggi.
//!
//! - Tabel `quadpts` default per `nfact` (1→61, 2→31, 3→15, 4→9, 5→7,
//!   selebihnya 3) & rentang integrasi default `[-6, 6]`.
//! - Leaped-Halton sequence (basis = bilangan prima pertama per dimensi,
//!   leap prima `409`; konstruksi leaped Halton dari Kocis & Whiten, 1997,
//!   lih. `QMC_LEAP`) ditransformasi lewat fungsi kuantil normal standar
//!   (inverse CDF) jadi awan kuasi-`N(0,I)`; `transform_to_prior`
//!   menggesernya ke prior grup `N(gmeans, gcov)`.

use crate::error::McatError;
use crate::theta::{InfoMatrix, ThetaVector};

/// Rentang integrasi grid EAP default `[-6, 6]` — konstanta internal,
/// sengaja tidak dibuat configurable (nilai ini tetap untuk seluruh sesi
/// CAT).
pub(crate) const THETA_LIM: (f64, f64) = (-6.0, 6.0);

/// Leap (skip) untuk Halton sequence — mengurangi korelasi antar-dimensi
/// pada sequence berdimensi banyak. Titik ke-`i` memakai indeks
/// `1 + i * QMC_LEAP`, yaitu Halton "leaped" menurut Kocis, L. & Whiten, W. J.
/// (1997), "Computational investigations of low-discrepancy sequences",
/// *ACM Transactions on Mathematical Software* 23(2), 266-294. Leap yang
/// dipakai harus bilangan prima yang berbeda dari semua basis agar saling
/// koprima dengan basis; nilai 409 (bilangan prima ke-80) adalah pilihan
/// engine ini.
const QMC_LEAP: u64 = 409;

/// Tabel `quadpts` default per `nfact` (1→61, 2→31, 3→15, 4→9, 5→7,
/// selebihnya 3) — mengecil
/// seiring `nfact` supaya ukuran grid `quadpts^nfact` tidak meledak
/// kombinatorial (curse of dimensionality).
pub fn default_quadpts(nfact: usize) -> usize {
    match nfact {
        1 => 61,
        2 => 31,
        3 => 15,
        4 => 9,
        5 => 7,
        _ => 3,
    }
}

/// Grid kuadratur reguler: cartesian product `quadpts` titik per dimensi
/// pada `[-6, 6]` (linspace `quadpts` titik termasuk kedua ujung), total
/// `quadpts^nfact` titik.
///
/// Error bila `nfact == 0` atau `quadpts < 2` (linspace butuh minimal 2
/// titik untuk merentang interval `[-6, 6]`).
pub fn build_grid(nfact: usize, quadpts: usize) -> Result<Vec<ThetaVector>, McatError> {
    if nfact == 0 {
        return Err(McatError::InvalidParameter(
            "quadrature::build_grid: nfact harus >= 1".into(),
        ));
    }
    if quadpts < 2 {
        return Err(McatError::InvalidParameter(format!(
            "quadrature::build_grid: quadpts harus >= 2, didapat {quadpts}"
        )));
    }
    let axis = linspace(THETA_LIM.0, THETA_LIM.1, quadpts);
    let total = quadpts.pow(nfact as u32);
    let mut grid = Vec::with_capacity(total);
    for p in 0..total {
        let mut coords = vec![0.0; nfact];
        let mut rem = p;
        for coord in coords.iter_mut() {
            *coord = axis[rem % quadpts];
            rem /= quadpts;
        }
        grid.push(ThetaVector::from_vec(coords));
    }
    Ok(grid)
}

fn linspace(lo: f64, hi: f64, n: usize) -> Vec<f64> {
    if n == 1 {
        return vec![lo];
    }
    let step = (hi - lo) / (n - 1) as f64;
    (0..n).map(|i| lo + step * i as f64).collect()
}

/// Titik Quasi-Monte Carlo: leaped-Halton sequence (basis = bilangan prima
/// pertama per dimensi) di `[0,1]^nfact`, ditransformasi lewat invers CDF
/// normal standar — awan kuasi-`N(0, I)`, dengan leap prima 409 (Halton
/// leaped, Kocis & Whiten, 1997).
///
/// Fungsi ini sengaja hanya menghasilkan awan baku `N(0, I)`; prior grup
/// diterapkan terpisah oleh `transform_to_prior` (`theta = mu + L z`),
/// dipanggil `eap.rs` sebelum menimbang titik dengan likelihood.
pub fn build_qmc_points(nfact: usize, npts: usize) -> Result<Vec<ThetaVector>, McatError> {
    if nfact == 0 {
        return Err(McatError::InvalidParameter(
            "quadrature::build_qmc_points: nfact harus >= 1".into(),
        ));
    }
    if npts == 0 {
        return Err(McatError::InvalidParameter(
            "quadrature::build_qmc_points: npts harus >= 1".into(),
        ));
    }
    let bases = halton_bases(nfact);
    let mut points = Vec::with_capacity(npts);
    for i in 0..npts {
        let index = 1 + (i as u64) * QMC_LEAP;
        let coords: Vec<f64> = bases
            .iter()
            .map(|&base| inverse_normal_cdf(radical_inverse(index, base)))
            .collect();
        points.push(ThetaVector::from_vec(coords));
    }
    Ok(points)
}

/// Geser dan skalakan awan kuasi-`N(0, I)` menjadi awan kuasi-`N(mu, Sigma)`
/// dengan `theta_i = mu + L z_i`, `Sigma = L L^T` (faktor Cholesky).
///
/// Mengapa `mu + L z`: bila `z ~ N(0, I)` maka `theta = mu + L z` linear
/// terhadap `z`, jadi tetap normal dengan rerata `E[theta] = mu + L E[z] = mu`
/// dan kovarians `Cov[theta] = L Cov[z] L^T = L I L^T = Sigma`. Titik-titik
/// hasilnya adalah sampel dari prior grup itu sendiri. Karena itu bobot EAP
/// tetap seragam: kerapatan prior sudah terkandung dalam sebaran titik, dan
/// menimbangnya lagi dengan `dmvnorm` akan menghitung prior dua kali
/// (importance sampling dengan proposal = prior membuat rasio prior/proposal
/// sama dengan 1). Pada `mu = 0`, `Sigma = I` faktornya `L = I`, sehingga
/// titik tidak berubah sama sekali.
///
/// Error (`McatError`, bukan panic) bila dimensi tidak cocok dengan
/// `points`/`mean`, bila `cov` memuat nilai non-finite, tidak simetris, atau
/// tidak positif definit (Cholesky tidak ada).
pub(crate) fn transform_to_prior(
    points: &[ThetaVector],
    mean: &ThetaVector,
    cov: &InfoMatrix,
) -> Result<Vec<ThetaVector>, McatError> {
    let nfact = cov.nfact();
    mean.validate_nfact(nfact, "quadrature::transform_to_prior: mean")?;
    let sigma = cov.as_dmatrix();
    for i in 0..nfact {
        for j in 0..nfact {
            let (a, b) = (sigma[(i, j)], sigma[(j, i)]);
            if !a.is_finite() || !b.is_finite() {
                return Err(McatError::InvalidParameter(
                    "quadrature::transform_to_prior: gcov memuat nilai non-finite".into(),
                ));
            }
            // Cholesky nalgebra hanya membaca segitiga bawah, jadi simetri
            // diperiksa eksplisit (toleransi relatif terhadap skala elemen).
            if (a - b).abs() > 1e-10 * a.abs().max(b.abs()).max(1.0) {
                return Err(McatError::InvalidParameter(
                    "quadrature::transform_to_prior: gcov harus simetris".into(),
                ));
            }
        }
    }
    let chol = sigma.clone().cholesky().ok_or_else(|| {
        McatError::InvalidParameter(
            "quadrature::transform_to_prior: gcov harus positif definit \
             (faktorisasi Cholesky gagal)"
                .into(),
        )
    })?;
    let l = chol.l();
    if (0..nfact).any(|i| !(l[(i, i)].is_finite() && l[(i, i)] > 0.0)) {
        return Err(McatError::InvalidParameter(
            "quadrature::transform_to_prior: gcov harus positif definit \
             (diagonal faktor Cholesky tidak positif)"
                .into(),
        ));
    }

    points
        .iter()
        .map(|z| {
            z.validate_nfact(nfact, "quadrature::transform_to_prior: titik")?;
            // `L` segitiga bawah: koordinat i hanya memakai z[0..=i].
            let coords: Vec<f64> = (0..nfact)
                .map(|i| mean[i] + (0..=i).map(|k| l[(i, k)] * z[k]).sum::<f64>())
                .collect();
            Ok(ThetaVector::from_vec(coords))
        })
        .collect()
}

/// Basis Halton per dimensi: `nfact` bilangan prima pertama.
///
/// Pengaman milik engine ini (bukan bagian dari Kocis & Whiten, 1997): bila
/// leap [`QMC_LEAP`] sendiri termasuk basis, basis itu dibuang dan diganti
/// bilangan prima ke-`nfact + 1`. Alasannya, indeks titik adalah
/// `1 + i * leap`; dengan basis `b = leap` digit terendahnya selalu
/// `(1 + i * b) mod b = 1`, jadi digit paling signifikan pecahan radical
/// inverse konstan dan seluruh dimensi itu terkurung di interval sempit
/// `[1/b, 2/b)` (dimensi degenerat). Hanya terpicu bila `nfact >= 80`,
/// karena 409 adalah bilangan prima ke-80.
fn halton_bases(nfact: usize) -> Vec<u64> {
    let mut primes = first_n_primes(nfact + 1);
    if let Some(pos) = primes[..nfact].iter().position(|&p| p == QMC_LEAP) {
        primes[pos] = primes[nfact];
    }
    primes.truncate(nfact);
    primes
}

fn first_n_primes(n: usize) -> Vec<u64> {
    let mut primes: Vec<u64> = Vec::with_capacity(n);
    let mut candidate = 2u64;
    while primes.len() < n {
        if primes.iter().all(|&p| !candidate.is_multiple_of(p)) {
            primes.push(candidate);
        }
        candidate += 1;
    }
    primes
}

/// Radical inverse (van der Corput) basis `base` untuk `index` — digit
/// basis-`base` dibalik urutannya jadi pecahan `[0,1)` (padding digit nol
/// tambahan tidak mengubah nilai karena berkontribusi nol pada posisi
/// signifikan tinggi).
fn radical_inverse(mut index: u64, base: u64) -> f64 {
    let mut result = 0.0;
    let mut f = 1.0 / base as f64;
    while index > 0 {
        result += f * (index % base) as f64;
        index /= base;
        f /= base as f64;
    }
    result
}

/// Invers CDF normal standar (aproksimasi rasional Acklam, presisi relatif
/// ~1.15e-9) — fungsi kuantil normal standar, dipakai `build_qmc_points`. Hand-roll
/// murni supaya tidak menambah dependency statistik baru (mis. `statrs`),
/// mengikuti gaya `irt::probability` yang sudah hand-roll `logistic()`.
/// `p` harus di `(0, 1)` — dijamin oleh `radical_inverse` (selalu `(0,1)`
/// strict untuk `index >= 1`), jadi tidak perlu validasi tambahan di sini.
///
/// Konstanta di bawah dikutip persis presisi publikasi asli Acklam
/// (17 digit signifikan) -- `#[allow(excessive_precision)]` karena
/// memotongnya untuk memuaskan lint akan mengurangi kesetiaan terhadap
/// referensi, bukan cuma gaya kode.
#[allow(clippy::excessive_precision)]
pub(crate) fn inverse_normal_cdf(p: f64) -> f64 {
    const A: [f64; 6] = [
        -3.969683028665376e+01,
        2.209460984245205e+02,
        -2.759285104469687e+02,
        1.383577518672690e+02,
        -3.066479806614716e+01,
        2.506628277459239e+00,
    ];
    const B: [f64; 5] = [
        -5.447609879822406e+01,
        1.615858368580409e+02,
        -1.556989798598866e+02,
        6.680131188771972e+01,
        -1.328068155288572e+01,
    ];
    const C: [f64; 6] = [
        -7.784894002430293e-03,
        -3.223964580411365e-01,
        -2.400758277161838e+00,
        -2.549732539343734e+00,
        4.374664141464968e+00,
        2.938163982698783e+00,
    ];
    const D: [f64; 4] = [
        7.784695709041462e-03,
        3.224671290700398e-01,
        2.445134137142996e+00,
        3.754408661907416e+00,
    ];
    const P_LOW: f64 = 0.02425;
    const P_HIGH: f64 = 1.0 - P_LOW;

    if p < P_LOW {
        let q = (-2.0 * p.ln()).sqrt();
        (((((C[0] * q + C[1]) * q + C[2]) * q + C[3]) * q + C[4]) * q + C[5])
            / ((((D[0] * q + D[1]) * q + D[2]) * q + D[3]) * q + 1.0)
    } else if p <= P_HIGH {
        let q = p - 0.5;
        let r = q * q;
        (((((A[0] * r + A[1]) * r + A[2]) * r + A[3]) * r + A[4]) * r + A[5]) * q
            / (((((B[0] * r + B[1]) * r + B[2]) * r + B[3]) * r + B[4]) * r + 1.0)
    } else {
        let q = (-2.0 * (1.0 - p).ln()).sqrt();
        -(((((C[0] * q + C[1]) * q + C[2]) * q + C[3]) * q + C[4]) * q + C[5])
            / ((((D[0] * q + D[1]) * q + D[2]) * q + D[3]) * q + 1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn default_quadpts_matches_reference_table() {
        assert_eq!(default_quadpts(1), 61);
        assert_eq!(default_quadpts(2), 31);
        assert_eq!(default_quadpts(3), 15);
        assert_eq!(default_quadpts(4), 9);
        assert_eq!(default_quadpts(5), 7);
        assert_eq!(default_quadpts(6), 3);
        assert_eq!(default_quadpts(20), 3);
    }

    #[test]
    fn build_grid_size_matches_quadpts_pow_nfact() {
        let grid = build_grid(2, 5).unwrap();
        assert_eq!(grid.len(), 25);
        let grid3 = build_grid(3, 4).unwrap();
        assert_eq!(grid3.len(), 64);
    }

    #[test]
    fn build_grid_1d_spans_theta_lim_endpoints() {
        let grid = build_grid(1, 5).unwrap();
        let values: Vec<f64> = grid.iter().map(|t| t[0]).collect();
        assert_relative_eq!(values.iter().cloned().fold(f64::INFINITY, f64::min), -6.0);
        assert_relative_eq!(
            values.iter().cloned().fold(f64::NEG_INFINITY, f64::max),
            6.0
        );
    }

    #[test]
    fn build_grid_rejects_invalid_input() {
        assert!(build_grid(0, 5).is_err());
        assert!(build_grid(2, 1).is_err());
    }

    #[test]
    fn build_qmc_points_returns_requested_count_within_reasonable_range() {
        let pts = build_qmc_points(3, 50).unwrap();
        assert_eq!(pts.len(), 50);
        // Kuasi-normal -- semua titik harus masuk akal (bukan NaN/tak
        // terhingga, dan mayoritas dalam rentang +/-5 SD).
        for p in &pts {
            for j in 0..3 {
                assert!(p[j].is_finite());
                assert!(p[j].abs() < 6.0);
            }
        }
    }

    #[test]
    fn build_qmc_points_rejects_invalid_input() {
        assert!(build_qmc_points(0, 10).is_err());
        assert!(build_qmc_points(2, 0).is_err());
    }

    #[test]
    fn radical_inverse_matches_hand_computed_base2() {
        // index=1 -> "1" basis 2 -> 0.1_2 = 0.5
        assert_relative_eq!(radical_inverse(1, 2), 0.5);
        // index=2 -> "10" basis 2 -> radical inverse membalik urutan -> 0.01_2 = 0.25
        assert_relative_eq!(radical_inverse(2, 2), 0.25);
        // index=3 -> "11" basis 2 -> 0.11_2 = 0.75
        assert_relative_eq!(radical_inverse(3, 2), 0.75);
    }

    #[test]
    fn inverse_normal_cdf_matches_known_quantiles() {
        assert_relative_eq!(inverse_normal_cdf(0.5), 0.0, epsilon = 1e-9);
        assert_relative_eq!(inverse_normal_cdf(0.975), 1.959963984540054, epsilon = 1e-8);
        assert_relative_eq!(
            inverse_normal_cdf(0.025),
            -1.959963984540054,
            epsilon = 1e-8
        );
        assert_relative_eq!(inverse_normal_cdf(0.999), 3.090232306167813, epsilon = 1e-7);
    }

    #[test]
    fn first_n_primes_matches_known_sequence() {
        assert_eq!(first_n_primes(6), vec![2, 3, 5, 7, 11, 13]);
    }

    #[test]
    fn halton_bases_never_equal_leap_and_dimensions_are_not_constant() {
        for nfact in (1..=100).chain([150, 200]) {
            let bases = halton_bases(nfact);
            assert_eq!(bases.len(), nfact);
            assert!(
                bases.iter().all(|&b| b != QMC_LEAP),
                "nfact={nfact}: ada basis sama dengan leap"
            );
            // basis tetap berbeda satu sama lain
            let mut sorted = bases.clone();
            sorted.sort_unstable();
            sorted.dedup();
            assert_eq!(sorted.len(), nfact, "nfact={nfact}: basis duplikat");
        }

        // Titik tiap dimensi harus bervariasi luas (bukan konstan atau
        // terkurung di interval sempit) -- termasuk di nfact=100 yang
        // melewati pengaman leap.
        let nfact = 100;
        let pts = build_qmc_points(nfact, 200).unwrap();
        for j in 0..nfact {
            let (lo, hi) = pts.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |a, p| {
                (a.0.min(p[j]), a.1.max(p[j]))
            });
            assert!(
                hi - lo > 2.0,
                "dimensi {j} nyaris konstan: rentang {lo}..{hi}"
            );
        }
    }

    #[test]
    fn leap_equal_base_would_confine_dimension_to_narrow_interval() {
        // Dokumentasi pengaman: tanpa penggantian, basis = leap membuat digit
        // terendah selalu 1 sehingga nilai berada di [1/409, 2/409).
        for i in 0..50u64 {
            let u = radical_inverse(1 + i * QMC_LEAP, QMC_LEAP);
            assert!((1.0 / 409.0..2.0 / 409.0).contains(&u), "u={u}");
        }
    }

    #[test]
    fn transform_to_prior_identity_leaves_points_unchanged() {
        let z = build_qmc_points(3, 100).unwrap();
        let out = transform_to_prior(&z, &ThetaVector::zeros(3), &InfoMatrix::identity(3)).unwrap();
        assert_eq!(out, z);
    }

    #[test]
    fn transform_to_prior_gives_requested_mean_and_covariance() {
        let mean = ThetaVector::from_vec(vec![0.8, -0.5]);
        let cov = InfoMatrix::from_matrix(nalgebra::DMatrix::from_row_slice(
            2,
            2,
            &[1.5, 0.6, 0.6, 0.9],
        ))
        .unwrap();
        let z = build_qmc_points(2, 20_000).unwrap();
        let out = transform_to_prior(&z, &mean, &cov).unwrap();
        let n = out.len() as f64;
        let m: Vec<f64> = (0..2)
            .map(|j| out.iter().map(|p| p[j]).sum::<f64>() / n)
            .collect();
        assert_relative_eq!(m[0], 0.8, epsilon = 0.01);
        assert_relative_eq!(m[1], -0.5, epsilon = 0.01);
        let c = |a: usize, b: usize| {
            out.iter()
                .map(|p| (p[a] - m[a]) * (p[b] - m[b]))
                .sum::<f64>()
                / n
        };
        assert_relative_eq!(c(0, 0), 1.5, epsilon = 0.02);
        assert_relative_eq!(c(1, 1), 0.9, epsilon = 0.02);
        assert_relative_eq!(c(0, 1), 0.6, epsilon = 0.02);
    }

    #[test]
    fn transform_to_prior_rejects_bad_inputs_without_panic() {
        let z = build_qmc_points(2, 10).unwrap();
        let mean = ThetaVector::zeros(2);
        let m = |v: &[f64]| {
            InfoMatrix::from_matrix(nalgebra::DMatrix::from_row_slice(2, 2, v)).unwrap()
        };
        assert!(transform_to_prior(&z, &mean, &m(&[1.0, 2.0, 2.0, 1.0])).is_err());
        assert!(transform_to_prior(&z, &mean, &m(&[1.0, 0.5, 0.0, 1.0])).is_err());
        assert!(transform_to_prior(&z, &mean, &m(&[1.0, 1.0, 1.0, 1.0])).is_err());
        // dimensi mean tidak cocok dengan gcov
        assert!(transform_to_prior(&z, &ThetaVector::zeros(3), &InfoMatrix::identity(2)).is_err());
        // dimensi titik tidak cocok dengan gcov
        assert!(transform_to_prior(&z, &mean, &InfoMatrix::identity(3)).is_err());
    }
}
