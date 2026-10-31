//! Mode klasifikasi `CI` — uji interval kepercayaan per-dimensi terhadap
//! cutoff, keputusan **per-dimensi** (`AboveCutoff`/`BelowCutoff`/
//! `NoDecision`), stop hanya bila **semua** dimensi terklasifikasi
//! (`all()`).
//!
//! Dimensi `i` punya interval kepercayaan satu-sisi-ke-dua-arah
//! `[theta[i] - z·SE[i], theta[i] + z·SE[i]]` dengan `z = z_{1-alpha}`
//! (kuantil normal standar; `alpha = (1 - tingkat_kepercayaan) / 2`).
//! Dimensi terklasifikasi bila cutoff berada **di luar** interval itu
//! (Weiss & Kingsbury, 1984; Eggen & Straetmans, 2000):
//!
//! ```text
//! batas_bawah[i] > cutoff[i]   -> AboveCutoff  (met)
//! batas_atas[i]  < cutoff[i]   -> BelowCutoff  (met)
//! selain itu                   -> NoDecision   (cutoff di dalam interval,
//!                                               atau tepat di batasnya)
//! stop_now = all(met)
//! ```
//! Pertidaksamaan **tegas**: cutoff yang tepat berimpit dengan salah satu
//! batas interval belum dianggap di luar. `SE` tidak terhingga atau
//! tidak terdefinisi (`NaN`, `Inf`, atau bertanda negatif) adalah cabang
//! eksplisit "tanpa keputusan": interval tidak informatif, jadi dimensi itu
//! tidak pernah terklasifikasi. Keputusan **per-dimensi** dihitung independen
//! dari gerbang `all()` dan diekspos langsung supaya pemanggil bisa melihat
//! status tiap dimensi walau belum semuanya yakin (mis. "satu dimensi sudah
//! above cutoff, satu masih no decision").

use crate::stopping_rule::ClassifyDecision;
use crate::theta::{SEVector, ThetaVector};

/// Nilai kritis `z_{1-alpha}`; kuantil normal standar simetris, jadi sama
/// dengan `|Φ⁻¹(alpha)|`. Dipakai bersama oleh [`evaluate`] dan [`interval`]
/// supaya batas yang dilaporkan identik dengan batas yang menentukan keputusan.
fn critical_z(alpha: f64) -> f64 {
    crate::ability_estimation::quadrature::inverse_normal_cdf(alpha).abs()
}

/// Mengembalikan `(met_per_dimensi, klasifikasi_per_dimensi)`. `cutoffs`
/// harus sudah divalidasi panjangnya == `theta.nfact()` oleh pemanggil.
pub(crate) fn evaluate(
    theta: &ThetaVector,
    se: &SEVector,
    cutoffs: &[f64],
    alpha: f64,
) -> (Vec<bool>, Vec<ClassifyDecision>) {
    let z = critical_z(alpha);
    let nfact = theta.nfact();
    let mut met = Vec::with_capacity(nfact);
    let mut decisions = Vec::with_capacity(nfact);
    for i in 0..nfact {
        let se_i = se[i];
        let decision = if !se_i.is_finite() || se_i.is_sign_negative() {
            // Interval tidak terdefinisi/tak terbatas: tanpa keputusan.
            ClassifyDecision::NoDecision
        } else {
            let half_width = z * se_i;
            let (lower, upper) = (theta[i] - half_width, theta[i] + half_width);
            if lower > cutoffs[i] {
                ClassifyDecision::AboveCutoff
            } else if upper < cutoffs[i] {
                ClassifyDecision::BelowCutoff
            } else {
                ClassifyDecision::NoDecision
            }
        };
        met.push(decision != ClassifyDecision::NoDecision);
        decisions.push(decision);
    }
    (met, decisions)
}

/// Batas `(bawah, atas)` interval kepercayaan per-dimensi, `theta ± z_{1-alpha}·SE`
/// -- kriteria `evaluate` ekuivalen "cutoff berada di luar interval ini".
/// Hanya untuk jejak debug; tidak memengaruhi keputusan. `SE` `NaN` dilaporkan
/// sebagai interval tak terbatas (batas -> `-Inf`/`+Inf`), sejalan dengan
/// cabang "tanpa keputusan" di `evaluate`.
pub(crate) fn interval(theta: &ThetaVector, se: &SEVector, alpha: f64) -> (Vec<f64>, Vec<f64>) {
    let z = critical_z(alpha);
    (0..theta.nfact())
        .map(|i| {
            let se_i = if se[i].is_nan() { f64::INFINITY } else { se[i] };
            (theta[i] - z * se_i, theta[i] + z * se_i)
        })
        .unzip()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interval_excludes_cutoff_exactly_when_dimension_is_classified() {
        let theta = ThetaVector::from_vec(vec![3.0, 0.05]);
        let se = SEVector::from_vec(vec![0.3, 1.0]);
        let cutoffs = [0.0, 0.0];
        let (met, _) = evaluate(&theta, &se, &cutoffs, 0.05);
        let (lower, upper) = interval(&theta, &se, 0.05);
        for i in 0..2 {
            assert!(lower[i] < theta[i] && theta[i] < upper[i]);
            let outside = cutoffs[i] < lower[i] || cutoffs[i] > upper[i];
            assert_eq!(met[i], outside, "dimensi {i}");
        }
    }

    #[test]
    fn interval_with_nan_se_is_unbounded() {
        let theta = ThetaVector::from_vec(vec![1.0]);
        let se = SEVector::from_vec(vec![f64::NAN]);
        let (lower, upper) = interval(&theta, &se, 0.05);
        assert_eq!(lower, vec![f64::NEG_INFINITY]);
        assert_eq!(upper, vec![f64::INFINITY]);
    }

    #[test]
    fn confident_dimension_above_cutoff_is_classified() {
        // theta jauh di atas cutoff relatif SE -> met, arah above.
        let theta = ThetaVector::from_vec(vec![3.0]);
        let se = SEVector::from_vec(vec![0.3]);
        let (met, decisions) = evaluate(&theta, &se, &[0.0], 0.05);
        assert_eq!(met, vec![true]);
        assert_eq!(decisions, vec![ClassifyDecision::AboveCutoff]);
    }

    #[test]
    fn confident_dimension_below_cutoff_is_classified() {
        let theta = ThetaVector::from_vec(vec![-3.0]);
        let se = SEVector::from_vec(vec![0.3]);
        let (met, decisions) = evaluate(&theta, &se, &[0.0], 0.05);
        assert_eq!(met, vec![true]);
        assert_eq!(decisions, vec![ClassifyDecision::BelowCutoff]);
    }

    #[test]
    fn theta_close_to_cutoff_relative_to_se_is_no_decision() {
        let theta = ThetaVector::from_vec(vec![0.05]);
        let se = SEVector::from_vec(vec![1.0]);
        let (met, decisions) = evaluate(&theta, &se, &[0.0], 0.05);
        assert_eq!(met, vec![false]);
        assert_eq!(decisions, vec![ClassifyDecision::NoDecision]);
    }

    #[test]
    fn two_dimensions_one_confident_one_not() {
        let theta = ThetaVector::from_vec(vec![3.0, 0.05]);
        let se = SEVector::from_vec(vec![0.3, 1.0]);
        let (met, decisions) = evaluate(&theta, &se, &[0.0, 0.0], 0.05);
        assert_eq!(met, vec![true, false]);
        assert_eq!(
            decisions,
            vec![ClassifyDecision::AboveCutoff, ClassifyDecision::NoDecision]
        );
        assert!(!met.iter().all(|&m| m));
    }

    #[test]
    fn nan_se_is_treated_as_infinity_never_met() {
        let theta = ThetaVector::from_vec(vec![5.0]);
        let se = SEVector::from_vec(vec![f64::NAN]);
        let (met, decisions) = evaluate(&theta, &se, &[0.0], 0.05);
        assert_eq!(met, vec![false]);
        assert_eq!(decisions, vec![ClassifyDecision::NoDecision]);
    }

    #[test]
    fn cutoff_exactly_on_interval_bound_is_no_decision() {
        // alpha = 0.5 -> z = 0 persis, interval menyusut jadi titik theta:
        // cutoff tepat di batas (theta == cutoff) belum "di luar" interval.
        let theta = ThetaVector::from_vec(vec![1.0, 1.0, 1.0]);
        let se = SEVector::from_vec(vec![0.5, 0.5, 0.5]);
        let (met, decisions) = evaluate(&theta, &se, &[1.0, 0.999, 1.001], 0.5);
        assert_eq!(met, vec![false, true, true]);
        assert_eq!(
            decisions,
            vec![
                ClassifyDecision::NoDecision,
                ClassifyDecision::AboveCutoff,
                ClassifyDecision::BelowCutoff
            ]
        );
    }

    #[test]
    fn zero_se_classifies_unless_theta_equals_cutoff() {
        let theta = ThetaVector::from_vec(vec![0.2, -0.2, 0.0]);
        let se = SEVector::from_vec(vec![0.0, 0.0, 0.0]);
        let (_, decisions) = evaluate(&theta, &se, &[0.0, 0.0, 0.0], 0.05);
        assert_eq!(
            decisions,
            vec![
                ClassifyDecision::AboveCutoff,
                ClassifyDecision::BelowCutoff,
                ClassifyDecision::NoDecision
            ]
        );
    }

    #[test]
    fn non_finite_or_negative_se_is_explicit_no_decision() {
        let theta = ThetaVector::from_vec(vec![5.0, 5.0, 5.0]);
        let se = SEVector::from_vec(vec![f64::INFINITY, f64::NAN, -0.1]);
        let (met, decisions) = evaluate(&theta, &se, &[0.0, 0.0, 0.0], 0.05);
        assert_eq!(met, vec![false; 3]);
        assert_eq!(decisions, vec![ClassifyDecision::NoDecision; 3]);
    }
}
