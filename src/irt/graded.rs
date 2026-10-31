//! Graded response model (GRM) multidimensional (politomus, respons
//! terurut) — mengikuti Samejima (1969) dan Muraki & Carlson (1995) untuk
//! versi multidimensional.

use nalgebra::DMatrix;

use super::model::ItemResponseModel;
use super::probability::{
    add_fisher_term, linear_predictor, logistic, logistic_derivative, mdiff, mdisc,
};
use crate::error::McatError;
use crate::theta::{InfoMatrix, ThetaVector};

/// Item graded response multidimensional: `a` (slope per dimensi), `d`
/// (boundary/threshold per kategori, panjang `ncat - 1`, **harus** terurut
/// menurun `d[0] > d[1] > ... > d[k]`, mengikuti urutan kategori respons
/// menaik).
#[derive(Debug, Clone, PartialEq)]
pub struct GradedResponse {
    a: Vec<f64>,
    d: Vec<f64>,
}

impl GradedResponse {
    /// Error bila `a` kosong, `d` punya kurang dari 1 boundary, atau `d`
    /// tidak terurut menurun (boundary tak terurut membuat selisih
    /// probabilitas kumulatif negatif untuk sebagian kategori — karena itu
    /// ditolak eksplisit saat konstruksi, sesuai prinsip desain #5).
    pub fn new(a: Vec<f64>, d: Vec<f64>) -> Result<Self, McatError> {
        if a.is_empty() {
            return Err(McatError::InvalidParameter(
                "GradedResponse: `a` (slope) tidak boleh kosong".into(),
            ));
        }
        if d.is_empty() {
            return Err(McatError::InvalidParameter(
                "GradedResponse: `d` (boundary) harus punya minimal 1 elemen".into(),
            ));
        }
        if d.windows(2).any(|w| w[0] <= w[1]) {
            return Err(McatError::InvalidParameter(format!(
                "GradedResponse: `d` harus terurut menurun (d[0] > d[1] > ...), didapat {d:?}"
            )));
        }
        Ok(Self { a, d })
    }

    /// Slope per dimensi.
    pub fn a(&self) -> &[f64] {
        &self.a
    }

    /// Boundary kategori (terurut menurun).
    pub fn d(&self) -> &[f64] {
        &self.d
    }

    /// Daya beda multidimensional `MDISC = ||a||` (Reckase, 2009). Satu
    /// nilai untuk seluruh boundary item.
    pub fn mdisc(&self) -> f64 {
        mdisc(&self.a)
    }

    /// Kesulitan multidimensional per boundary `MDIFF_k = -d_k / MDISC`,
    /// searah `d` (menaik karena `d` menurun). Semua `0.0` bila semua
    /// slope nol.
    pub fn mdiff(&self) -> Vec<f64> {
        let overall = self.mdisc();
        self.d.iter().map(|&dk| mdiff(dk, overall)).collect()
    }

    /// Kurva batas kategori `Pstar_k = P(respons >= kategori k+1 | theta)`
    /// untuk `k = 0..=nint+1`, dengan `Pstar_0 = 1` dan `Pstar_{nint+1} = 0`
    /// sebagai batas implisit. Mengembalikan `(pstar, qstar, deriv)`:
    /// `qstar_k = 1 - Pstar_k` dihitung langsung dari `logistic(-z)` (bukan
    /// dengan pengurangan), dan `deriv_k = Pstar_k * (1 - Pstar_k)`.
    fn boundary_curves(&self, theta: &[f64]) -> BoundaryCurves {
        let n = self.d.len() + 2;
        let mut pstar = Vec::with_capacity(n);
        let mut qstar = Vec::with_capacity(n);
        let mut deriv = Vec::with_capacity(n);
        pstar.push(1.0);
        qstar.push(0.0);
        deriv.push(0.0);
        for &dk in &self.d {
            let z = linear_predictor(&self.a, dk, theta);
            pstar.push(logistic(z));
            qstar.push(logistic(-z));
            deriv.push(logistic_derivative(z));
        }
        pstar.push(0.0);
        qstar.push(1.0);
        deriv.push(0.0);
        BoundaryCurves {
            pstar,
            qstar,
            deriv,
        }
    }

    /// Probabilitas kategori `P_k = Pstar_{k-1} - Pstar_k` **sebelum**
    /// dilantai. Secara matematis selisih ini `>= 0` (boundary menurun),
    /// tetapi pembulatan bisa membuatnya `0` atau sedikit negatif. Bila
    /// kedua kurva berada di sisi atas (`Pstar_k > 0.5`), selisih dihitung
    /// sebagai `Qstar_k - Qstar_{k-1}` (identik secara matematis) supaya
    /// kategori kecil tidak hilang oleh pengurangan dua bilangan yang
    /// dekat ke 1.
    fn raw_category_probabilities(curves: &BoundaryCurves) -> Vec<f64> {
        (0..curves.pstar.len() - 1)
            .map(|k| {
                if curves.pstar[k + 1] > 0.5 {
                    curves.qstar[k + 1] - curves.qstar[k]
                } else {
                    curves.pstar[k] - curves.pstar[k + 1]
                }
            })
            .collect()
    }
}

/// Kurva batas kumulatif beserta komplemen dan turunannya terhadap logit.
struct BoundaryCurves {
    pstar: Vec<f64>,
    qstar: Vec<f64>,
    deriv: Vec<f64>,
}

impl ItemResponseModel for GradedResponse {
    fn nfact(&self) -> usize {
        self.a.len()
    }

    fn n_categories(&self) -> usize {
        self.d.len() + 1
    }

    /// Probabilitas kategori, dilantai di `f64::MIN_POSITIVE`. Lantai ini
    /// satu-satunya penjaga numerik: ia hanya memastikan `ln P_k` tetap
    /// finite untuk kode likelihood, dan tidak dipakai untuk hal lain
    /// (informasi Fisher memakai probabilitas mentah, lih. `information`).
    /// Tidak ada batas atas: pada `f64` batas atas `1 - eps` tidak
    /// berpengaruh (`P_k <= 1` sudah dijamin oleh selisih dua nilai di
    /// `[0, 1]`).
    fn probabilities(&self, theta: &ThetaVector) -> Result<Vec<f64>, McatError> {
        theta.validate_nfact(self.nfact(), "GradedResponse::probabilities")?;
        let curves = self.boundary_curves(theta.as_slice());
        let probs = Self::raw_category_probabilities(&curves)
            .into_iter()
            .map(|p| p.max(f64::MIN_POSITIVE))
            .collect();
        Ok(probs)
    }

    fn information(&self, theta: &ThetaVector) -> Result<InfoMatrix, McatError> {
        theta.validate_nfact(self.nfact(), "GradedResponse::information")?;
        let curves = self.boundary_curves(theta.as_slice());
        let probs = Self::raw_category_probabilities(&curves);
        let nfact = self.nfact();

        // grad(Pstar_k) = a * Pstar_k * (1 - Pstar_k); batas Pstar_0 dan
        // Pstar_last tetap (deriv = 0, jadi gradiennya 0).
        let mut m = DMatrix::zeros(nfact, nfact);
        for (k, &p_k) in probs.iter().enumerate() {
            let grad_p: Vec<f64> = self
                .a
                .iter()
                .map(|aj| aj * (curves.deriv[k] - curves.deriv[k + 1]))
                .collect();
            // Suku `grad_p grad_p^T / P_k`: gradien kategori berorde `P_k`
            // di ekor kurva logistik, jadi suku -> 0 saat `P_k -> 0`.
            // `add_fisher_term` memberi kontribusi 0 bila `P_k` nol, negatif
            // akibat pembulatan, atau tidak di atas lantai `MIN_POSITIVE`,
            // dan bila gradiennya persis 0 -- bukan `Inf`/`NaN`.
            add_fisher_term(&mut m, &grad_p, p_k);
        }
        InfoMatrix::from_matrix(m)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn probabilities_sum_to_one() {
        let item = GradedResponse::new(vec![1.2, -0.5], vec![1.5, 0.2, -1.0]).unwrap();
        let theta = ThetaVector::from_vec(vec![0.4, -0.2]);
        let p = item.probabilities(&theta).unwrap();
        assert_eq!(p.len(), 4);
        assert_relative_eq!(p.iter().sum::<f64>(), 1.0, epsilon = 1e-9);
    }

    #[test]
    fn probabilities_are_monotonic_with_boundary_order() {
        // theta ekstrem tinggi -> massa probabilitas terkonsentrasi di kategori tertinggi.
        let item = GradedResponse::new(vec![1.0], vec![2.0, 0.0, -2.0]).unwrap();
        let high = item
            .probabilities(&ThetaVector::from_vec(vec![10.0]))
            .unwrap();
        let low = item
            .probabilities(&ThetaVector::from_vec(vec![-10.0]))
            .unwrap();
        assert!(high[3] > high[0]); // kategori tertinggi dominan saat theta besar
        assert!(low[0] > low[3]); // kategori terendah dominan saat theta kecil
    }

    #[test]
    fn unordered_boundaries_rejected_at_construction() {
        assert!(GradedResponse::new(vec![1.0], vec![0.0, 0.5]).is_err()); // tidak menurun
        assert!(GradedResponse::new(vec![1.0], vec![0.0, 0.0]).is_err()); // sama, bukan ketat menurun
    }

    #[test]
    fn empty_a_or_d_rejected() {
        assert!(GradedResponse::new(vec![], vec![0.0]).is_err());
        assert!(GradedResponse::new(vec![1.0], vec![]).is_err());
    }

    #[test]
    fn mdisc_and_mdiff_per_boundary() {
        let item = GradedResponse::new(vec![3.0, 4.0], vec![2.0, 0.0, -1.0]).unwrap();
        assert_relative_eq!(item.mdisc(), 5.0);
        let mdiff = item.mdiff();
        assert_eq!(mdiff.len(), 3);
        assert_relative_eq!(mdiff[0], -0.4);
        assert_relative_eq!(mdiff[1], 0.0);
        assert!(mdiff[1].is_sign_positive());
        assert_relative_eq!(mdiff[2], 0.2);
    }

    #[test]
    fn mdiff_is_zero_for_every_boundary_when_all_slopes_are_zero() {
        let item = GradedResponse::new(vec![0.0, 0.0], vec![1.0, -1.0]).unwrap();
        assert_eq!(item.mdisc(), 0.0);
        assert_eq!(item.mdiff(), vec![0.0, 0.0]);
    }

    #[test]
    fn dimension_mismatch_errors_not_panics() {
        let item = GradedResponse::new(vec![1.0, 1.0], vec![1.0, -1.0]).unwrap();
        let theta = ThetaVector::from_vec(vec![0.0]);
        assert!(item.probabilities(&theta).is_err());
        assert!(item.information(&theta).is_err());
    }

    /// Dua kategori (nint=1) graded response harus setara dengan 2PL
    /// (probabilitas & informasi identik).
    #[test]
    fn two_category_graded_matches_2pl_equivalent() {
        use super::super::dichotomous::Dichotomous;

        let a = vec![1.1, 0.6];
        let d = 0.3_f64;
        let graded = GradedResponse::new(a.clone(), vec![d]).unwrap();
        let dich = Dichotomous::m2pl(a, d).unwrap();

        let theta = ThetaVector::from_vec(vec![0.5, -0.4]);
        let p_graded = graded.probabilities(&theta).unwrap();
        let p_dich = dich.probabilities(&theta).unwrap();
        assert_relative_eq!(p_graded[0], p_dich[0], epsilon = 1e-9);
        assert_relative_eq!(p_graded[1], p_dich[1], epsilon = 1e-9);

        let info_graded = graded.information(&theta).unwrap();
        let info_dich = dich.information(&theta).unwrap();
        assert_relative_eq!(info_graded.get(0, 0), info_dich.get(0, 0), epsilon = 1e-8);
        assert_relative_eq!(info_graded.get(1, 1), info_dich.get(1, 1), epsilon = 1e-8);
    }

    fn extreme_item() -> GradedResponse {
        GradedResponse::new(vec![1.1, 0.6], vec![1.5, 0.2, -1.2]).unwrap()
    }

    fn extreme_thetas() -> Vec<ThetaVector> {
        let mut out = Vec::new();
        for t in [50.0, -50.0, 400.0, -400.0, 1e4, -1e4] {
            out.push(ThetaVector::from_vec(vec![t, t]));
            out.push(ThetaVector::from_vec(vec![t, -t]));
        }
        out
    }

    #[test]
    fn probabilities_are_finite_and_in_unit_interval_at_extreme_theta() {
        let item = extreme_item();
        for theta in extreme_thetas() {
            for p in item.probabilities(&theta).unwrap() {
                assert!(
                    p.is_finite() && (0.0..=1.0).contains(&p),
                    "theta={:?}: p={p}",
                    theta.as_slice()
                );
            }
        }
    }

    #[test]
    fn probabilities_are_at_least_the_floor_at_extreme_theta() {
        let item = extreme_item();
        for theta in extreme_thetas() {
            for p in item.probabilities(&theta).unwrap() {
                assert!(p >= f64::MIN_POSITIVE && p.ln().is_finite());
            }
        }
    }

    #[test]
    fn probabilities_sum_to_one_at_extreme_theta() {
        let item = extreme_item();
        for theta in extreme_thetas() {
            let sum: f64 = item.probabilities(&theta).unwrap().iter().sum();
            assert!(
                (sum - 1.0).abs() < 1e-12,
                "theta={:?}: sum={sum}",
                theta.as_slice()
            );
        }
    }

    #[test]
    fn information_is_finite_at_extreme_theta() {
        let item = extreme_item();
        for theta in extreme_thetas() {
            let info = item.information(&theta).unwrap();
            for i in 0..2 {
                for j in 0..2 {
                    assert!(
                        info.get(i, j).is_finite(),
                        "theta={:?}: info[{i},{j}]={}",
                        theta.as_slice(),
                        info.get(i, j)
                    );
                }
            }
        }
    }

    #[test]
    fn information_is_symmetric_at_extreme_theta() {
        let item = extreme_item();
        for theta in extreme_thetas() {
            let info = item.information(&theta).unwrap();
            assert_eq!(
                info.get(0, 1),
                info.get(1, 0),
                "theta={:?}",
                theta.as_slice()
            );
        }
    }

    #[test]
    fn information_is_zero_in_far_tail_where_all_gradients_vanish() {
        let item = extreme_item();
        let info = item
            .information(&ThetaVector::from_vec(vec![1e4, 1e4]))
            .unwrap();
        assert_eq!(info.get(0, 0), 0.0);
        assert_eq!(info.get(1, 1), 0.0);
    }

    #[test]
    fn small_upper_category_is_not_lost_to_cancellation() {
        // theta tinggi: kategori terendah ~ exp(-(z_1)) tetapi pengurangan naif
        // 1 - Pstar_1 membulat ke 0 untuk z_1 > ~36.
        let item = GradedResponse::new(vec![1.0], vec![2.0, 0.0]).unwrap();
        let p = item
            .probabilities(&ThetaVector::from_vec(vec![40.0]))
            .unwrap();
        assert_relative_eq!(p[0], (-42.0_f64).exp(), max_relative = 1e-9);
    }

    #[test]
    fn probabilities_match_naive_difference_in_normal_range() {
        let a = [1.1_f64, 0.6_f64];
        let d = [1.5_f64, 0.2, -1.2];
        let item = GradedResponse::new(a.to_vec(), d.to_vec()).unwrap();
        for t in [-4.0, -1.5, 0.0, 0.8, 3.0] {
            let theta = ThetaVector::from_vec(vec![t, -0.5 * t]);
            let base = a[0] * theta[0] + a[1] * theta[1];
            let mut pstar = vec![1.0];
            pstar.extend(d.iter().map(|dk| 1.0 / (1.0 + (-(dk + base)).exp())));
            pstar.push(0.0);
            let p = item.probabilities(&theta).unwrap();
            for k in 0..p.len() {
                let expected = pstar[k] - pstar[k + 1];
                assert!(
                    (p[k] - expected).abs() < 1e-15,
                    "t={t} k={k}: {} vs {expected}",
                    p[k]
                );
            }
        }
    }
}
