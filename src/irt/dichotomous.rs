//! Model item dikotomus multidimensional — M2PL/M3PL/M4PL sebagai kasus
//! khusus dari satu parameterisasi `(a, d, g, u)`, dengan probabilitas dan
//! informasi Fisher mengikuti Reckase (2009, bab 4).

use nalgebra::DMatrix;

use super::model::ItemResponseModel;
use super::probability::{linear_predictor, logistic, logistic_derivative, mdiff, mdisc};
use crate::error::McatError;
use crate::theta::{InfoMatrix, ThetaVector};

/// Item dikotomus multidimensional: `a` (slope per dimensi), `d`
/// (intercept/difficulty), `g` (guessing/lower asymptote), `u` (upper
/// asymptote). `g=0, u=1` → M2PL; `u=1` → M3PL; keduanya bebas → M4PL.
#[derive(Debug, Clone, PartialEq)]
pub struct Dichotomous {
    a: Vec<f64>,
    d: f64,
    g: f64,
    u: f64,
}

impl Dichotomous {
    /// Konstruktor umum M4PL. Error bila `a` kosong, `g`/`u` di luar
    /// `[0, 1]`, atau `u <= g` (asymptote tidak valid).
    pub fn new(a: Vec<f64>, d: f64, g: f64, u: f64) -> Result<Self, McatError> {
        if a.is_empty() {
            return Err(McatError::InvalidParameter(
                "Dichotomous: `a` (slope) tidak boleh kosong".into(),
            ));
        }
        if !(0.0..=1.0).contains(&g) {
            return Err(McatError::InvalidParameter(format!(
                "Dichotomous: `g` harus di [0,1], didapat {g}"
            )));
        }
        if !(0.0..=1.0).contains(&u) {
            return Err(McatError::InvalidParameter(format!(
                "Dichotomous: `u` harus di [0,1], didapat {u}"
            )));
        }
        if u <= g {
            return Err(McatError::InvalidParameter(format!(
                "Dichotomous: `u` harus > `g`, didapat u={u}, g={g}"
            )));
        }
        Ok(Self { a, d, g, u })
    }

    /// M2PL: `g=0, u=1`.
    pub fn m2pl(a: Vec<f64>, d: f64) -> Result<Self, McatError> {
        Self::new(a, d, 0.0, 1.0)
    }

    /// M3PL: `u=1`, `g` bebas.
    pub fn m3pl(a: Vec<f64>, d: f64, g: f64) -> Result<Self, McatError> {
        Self::new(a, d, g, 1.0)
    }

    /// M4PL: `g` & `u` bebas.
    pub fn m4pl(a: Vec<f64>, d: f64, g: f64, u: f64) -> Result<Self, McatError> {
        Self::new(a, d, g, u)
    }

    /// Slope per dimensi.
    pub fn a(&self) -> &[f64] {
        &self.a
    }

    /// Intercept/difficulty.
    pub fn d(&self) -> f64 {
        self.d
    }

    /// Guessing/lower asymptote.
    pub fn g(&self) -> f64 {
        self.g
    }

    /// Upper asymptote.
    pub fn u(&self) -> f64 {
        self.u
    }

    /// Daya beda multidimensional `MDISC = ||a||` (Reckase, 2009).
    /// Tidak bergantung pada `d`, `g`, `u`.
    pub fn mdisc(&self) -> f64 {
        mdisc(&self.a)
    }

    /// Kesulitan multidimensional `MDIFF = -d / MDISC` (Reckase, 2009).
    /// `0.0` bila semua slope nol.
    pub fn mdiff(&self) -> f64 {
        mdiff(self.d, self.mdisc())
    }
}

impl ItemResponseModel for Dichotomous {
    fn nfact(&self) -> usize {
        self.a.len()
    }

    fn n_categories(&self) -> usize {
        2
    }

    fn probabilities(&self, theta: &ThetaVector) -> Result<Vec<f64>, McatError> {
        theta.validate_nfact(self.nfact(), "Dichotomous::probabilities")?;
        let z = linear_predictor(&self.a, self.d, theta.as_slice());
        let (p0, p1) = self.category_probabilities(z);
        Ok(vec![p0, p1])
    }

    fn information(&self, theta: &ThetaVector) -> Result<InfoMatrix, McatError> {
        theta.validate_nfact(self.nfact(), "Dichotomous::information")?;
        let z = linear_predictor(&self.a, self.d, theta.as_slice());
        let (p0, p1) = self.category_probabilities(z);
        // dP1/dtheta_j = (u - g) * pstar * (1 - pstar) * a_j, dan
        // dP0/dtheta_j = -dP1/dtheta_j. Karena outer(-g, -g) = outer(g, g),
        // info = outer(grad1, grad1) * (1/P1 + 1/P0).
        let factor = (self.u - self.g) * logistic_derivative(z);
        let grad1: Vec<f64> = self.a.iter().map(|aj| factor * aj).collect();
        let nfact = self.nfact();
        let mut m = DMatrix::zeros(nfact, nfact);
        // Tanpa clamp pada logit, `P0` atau `P1` bisa persis 0 (M2PL pada
        // |z| sangat besar). Gradien `dP/dtheta` berorde `P` di ekor logistik,
        // jadi `grad^2 / P -> 0` saat `P -> 0`: bila salah satu kategori tidak
        // di atas `f64::MIN_POSITIVE`, kontribusinya dianggap 0 (limit),
        // bukan `Inf`/`NaN` dari `1/0`. Bentuk `grad_i * grad_j * scale`
        // dipertahankan agar urutan operasi (dan hasil bit demi bit) di
        // rentang normal sama dengan sebelumnya.
        if p1 > f64::MIN_POSITIVE && p0 > f64::MIN_POSITIVE {
            let scale = 1.0 / p1 + 1.0 / p0;
            for i in 0..nfact {
                for j in 0..nfact {
                    m[(i, j)] = grad1[i] * grad1[j] * scale;
                }
            }
        }
        InfoMatrix::from_matrix(m)
    }
}

impl Dichotomous {
    /// `(P(u=0), P(u=1))` pada logit `z`. Kedua sisi dihitung langsung dari
    /// sisi logistik masing-masing (`P*(z)` dan `P*(-z)`), bukan
    /// `1 - P(u=1)`, supaya kategori yang kecil di ekor kurva tidak hilang
    /// oleh pembulatan: `P(u=0) = (1 - u) + (u - g) * P*(-z)`.
    fn category_probabilities(&self, z: f64) -> (f64, f64) {
        let span = self.u - self.g;
        let p1 = self.g + span * logistic(z);
        let p0 = (1.0 - self.u) + span * logistic(-z);
        (p0, p1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn probabilities_sum_to_one_2pl() {
        let item = Dichotomous::m2pl(vec![1.0, 0.5], 0.2).unwrap();
        let theta = ThetaVector::from_vec(vec![0.3, -0.7]);
        let p = item.probabilities(&theta).unwrap();
        assert_relative_eq!(p[0] + p[1], 1.0, epsilon = 1e-12);
    }

    #[test]
    fn probabilities_sum_to_one_3pl_and_4pl() {
        let m3pl = Dichotomous::m3pl(vec![1.2], -0.5, 0.2).unwrap();
        let theta = ThetaVector::from_vec(vec![1.0]);
        let p3 = m3pl.probabilities(&theta).unwrap();
        assert_relative_eq!(p3[0] + p3[1], 1.0, epsilon = 1e-12);

        let m4pl = Dichotomous::m4pl(vec![1.2], -0.5, 0.15, 0.95).unwrap();
        let p4 = m4pl.probabilities(&theta).unwrap();
        assert_relative_eq!(p4[0] + p4[1], 1.0, epsilon = 1e-12);
        // guessing floor & upper asymptote menyempitkan rentang P1
        assert!(p4[1] >= 0.15 && p4[1] <= 0.95);
    }

    #[test]
    fn invalid_asymptotes_rejected_at_construction() {
        assert!(Dichotomous::new(vec![1.0], 0.0, 0.5, 0.5).is_err()); // u <= g
        assert!(Dichotomous::new(vec![1.0], 0.0, -0.1, 1.0).is_err()); // g < 0
        assert!(Dichotomous::new(vec![], 0.0, 0.0, 1.0).is_err()); // a kosong
    }

    #[test]
    fn mdisc_and_mdiff_follow_reckase_definition() {
        let item = Dichotomous::m2pl(vec![3.0, 4.0], 2.0).unwrap();
        assert_relative_eq!(item.mdisc(), 5.0);
        assert_relative_eq!(item.mdiff(), -0.4);
    }

    #[test]
    fn mdisc_and_mdiff_ignore_guessing_and_upper_asymptote() {
        let m2pl = Dichotomous::m2pl(vec![1.0, 1.0], 0.5).unwrap();
        let m4pl = Dichotomous::m4pl(vec![1.0, 1.0], 0.5, 0.2, 0.9).unwrap();
        assert_relative_eq!(m4pl.mdisc(), m2pl.mdisc());
        assert_relative_eq!(m4pl.mdiff(), m2pl.mdiff());
    }

    #[test]
    fn mdiff_is_positive_zero_when_intercept_is_zero() {
        let item = Dichotomous::m2pl(vec![1.0, 1.0], 0.0).unwrap();
        assert_eq!(item.mdiff(), 0.0);
        assert!(item.mdiff().is_sign_positive());
    }

    #[test]
    fn mdiff_is_zero_when_all_slopes_are_zero() {
        let item = Dichotomous::m2pl(vec![0.0, 0.0], 1.5).unwrap();
        assert_eq!(item.mdisc(), 0.0);
        assert_eq!(item.mdiff(), 0.0);
    }

    #[test]
    fn dimension_mismatch_errors_not_panics() {
        let item = Dichotomous::m2pl(vec![1.0, 1.0], 0.0).unwrap();
        let theta = ThetaVector::from_vec(vec![0.0]);
        assert!(item.probabilities(&theta).is_err());
        assert!(item.information(&theta).is_err());
    }

    /// Sanity check 1 dimensi terhadap turunan analitik 2PL:
    /// `I(theta) = a^2 * P * (1 - P)`.
    #[test]
    fn information_matches_closed_form_2pl_1d() {
        let a = 1.3_f64;
        let d = 0.4_f64;
        let item = Dichotomous::m2pl(vec![a], d).unwrap();
        let theta = ThetaVector::from_vec(vec![0.6]);
        let p = item.probabilities(&theta).unwrap();
        let info = item.information(&theta).unwrap();
        let expected = a * a * p[1] * p[0];
        assert_relative_eq!(info.get(0, 0), expected, epsilon = 1e-10);
    }

    /// Sanity check 3PL 1 dimensi terhadap turunan analitik:
    /// `I(theta) = a^2 * ((u-g) * Pstar * (1-Pstar))^2 / (P1 * P0)`.
    #[test]
    fn information_matches_closed_form_3pl_1d() {
        let a = 0.9_f64;
        let d = -0.2_f64;
        let g = 0.2_f64;
        let item = Dichotomous::m3pl(vec![a], d, g).unwrap();
        let theta = ThetaVector::from_vec(vec![-0.4]);
        let z = a * theta[0] + d;
        let pstar = 1.0 / (1.0 + (-z).exp());
        let p1 = g + (1.0 - g) * pstar;
        let p0 = 1.0 - p1;
        let expected = (a * (1.0 - g) * pstar * (1.0 - pstar)).powi(2) / (p1 * p0);
        let info = item.information(&theta).unwrap();
        assert_relative_eq!(info.get(0, 0), expected, epsilon = 1e-10);
    }

    #[test]
    fn information_is_symmetric_2d() {
        let item = Dichotomous::m2pl(vec![0.8, 1.4], 0.1).unwrap();
        let theta = ThetaVector::from_vec(vec![0.2, -0.3]);
        let info = item.information(&theta).unwrap();
        assert_relative_eq!(info.get(0, 1), info.get(1, 0), epsilon = 1e-12);
    }

    /// Titik `theta` 2D ekstrem: logit M2PL jauh melewati rentang ~36 tempat
    /// `P*` membulat ke 0 atau 1.
    fn extreme_thetas() -> Vec<ThetaVector> {
        let mut out = Vec::new();
        for t in [50.0, -50.0, 400.0, -400.0, 1e4, -1e4] {
            out.push(ThetaVector::from_vec(vec![t, t]));
            out.push(ThetaVector::from_vec(vec![t, -t]));
        }
        out
    }

    fn extreme_items() -> Vec<(&'static str, Dichotomous)> {
        vec![
            ("m2pl", Dichotomous::m2pl(vec![1.2, 0.8], 0.3).unwrap()),
            ("m3pl", Dichotomous::m3pl(vec![1.2, 0.8], 0.3, 0.2).unwrap()),
            (
                "m4pl",
                Dichotomous::m4pl(vec![1.2, 0.8], 0.3, 0.15, 0.95).unwrap(),
            ),
        ]
    }

    #[test]
    fn probabilities_are_finite_and_in_unit_interval_at_extreme_theta() {
        for (name, item) in extreme_items() {
            for theta in extreme_thetas() {
                for p in item.probabilities(&theta).unwrap() {
                    assert!(
                        p.is_finite() && (0.0..=1.0).contains(&p),
                        "{name} theta={:?}: p={p}",
                        theta.as_slice()
                    );
                }
            }
        }
    }

    #[test]
    fn probabilities_sum_to_one_at_extreme_theta() {
        for (name, item) in extreme_items() {
            for theta in extreme_thetas() {
                let sum: f64 = item.probabilities(&theta).unwrap().iter().sum();
                assert!(
                    (sum - 1.0).abs() < 1e-12,
                    "{name} theta={:?}: sum={sum}",
                    theta.as_slice()
                );
            }
        }
    }

    #[test]
    fn information_is_finite_at_extreme_theta() {
        for (name, item) in extreme_items() {
            for theta in extreme_thetas() {
                let info = item.information(&theta).unwrap();
                for i in 0..2 {
                    for j in 0..2 {
                        assert!(
                            info.get(i, j).is_finite(),
                            "{name} theta={:?}: info[{i},{j}]={}",
                            theta.as_slice(),
                            info.get(i, j)
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn information_is_symmetric_at_extreme_theta() {
        for (name, item) in extreme_items() {
            for theta in extreme_thetas() {
                let info = item.information(&theta).unwrap();
                assert_eq!(
                    info.get(0, 1),
                    info.get(1, 0),
                    "{name} theta={:?}",
                    theta.as_slice()
                );
            }
        }
    }

    #[test]
    fn information_is_zero_in_far_tail_where_probability_underflows() {
        // |z| = 1e4: P*(z) = 1 atau 0 persis, limit informasinya 0.
        let item = Dichotomous::m2pl(vec![1.2, 0.8], 0.3).unwrap();
        for t in [1e4, -1e4] {
            let info = item
                .information(&ThetaVector::from_vec(vec![t, t]))
                .unwrap();
            assert_eq!(info.get(0, 0), 0.0);
            assert_eq!(info.get(1, 1), 0.0);
        }
    }

    #[test]
    fn information_keeps_tiny_tail_value_beyond_old_clamp() {
        // z = 40 (> 35, batas clamp lama): 2PL 1D memberi a^2 * P * Q ~ exp(-40).
        let item = Dichotomous::m2pl(vec![1.0], 0.0).unwrap();
        let info = item
            .information(&ThetaVector::from_vec(vec![40.0]))
            .unwrap();
        assert_relative_eq!(info.get(0, 0), (-40.0_f64).exp(), max_relative = 1e-9);
    }

    #[test]
    fn information_matches_naive_formula_in_normal_range() {
        // Rumus naif lama dengan logit diklem: factor * a * ... * (1/P1 + 1/P0).
        let (a, d, g, u) = ([1.2_f64, 0.8_f64], 0.3_f64, 0.15_f64, 0.95_f64);
        let item = Dichotomous::m4pl(a.to_vec(), d, g, u).unwrap();
        for t in [-3.0, -1.0, 0.0, 0.7, 2.5] {
            let theta = ThetaVector::from_vec(vec![t, 0.5 * t]);
            let z = d + a[0] * theta[0] + a[1] * theta[1];
            let pstar = 1.0 / (1.0 + (-z.clamp(-35.0, 35.0)).exp());
            let p1 = g + (u - g) * pstar;
            let p0 = 1.0 - p1;
            let factor = (u - g) * pstar * (1.0 - pstar);
            let scale = 1.0 / p1 + 1.0 / p0;
            let info = item.information(&theta).unwrap();
            for i in 0..2 {
                for j in 0..2 {
                    let expected = factor * a[i] * factor * a[j] * scale;
                    assert_relative_eq!(info.get(i, j), expected, max_relative = 1e-12);
                }
            }
        }
    }
}
