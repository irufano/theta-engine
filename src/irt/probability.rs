//! Fungsi probabilitas dasar dipakai lintas tipe item.

use nalgebra::DMatrix;

/// Fungsi logistik standar `1 / (1 + exp(-z))` yang stabil secara numerik
/// untuk `z` berapa pun, tanpa memotong (clamp) nilai logit.
///
/// Rumus dipecah menurut tanda `z` supaya `exp()` selalu dipanggil dengan
/// argumen `<= 0`:
///
/// - `z >= 0`: `1 / (1 + exp(-z))`
/// - `z < 0`: `exp(z) / (1 + exp(z))`
///
/// Dengan begitu `exp()` tidak pernah overflow ke `Inf`, dan sisi kecil
/// kurva (`z` sangat negatif) dihitung langsung sebagai `exp(z)` yang
/// presisi relatifnya terjaga sampai underflow. Untuk `|z|` sangat besar
/// hasilnya menyatu ke `0` atau `1` karena `exp(-|z|)` underflow ke `0`,
/// sehingga selalu berada di `[0, 1]`.
/// `z` non-finite (`NaN`) tetap merambat sebagai `NaN`.
pub(crate) fn logistic(z: f64) -> f64 {
    if z >= 0.0 {
        1.0 / (1.0 + (-z).exp())
    } else {
        let e = z.exp();
        e / (1.0 + e)
    }
}

/// Hasil kali `logistic(z) * logistic(-z)` = `P*(1 - P*)`, turunan kurva
/// logistik terhadap `z`. Dihitung dari dua sisi logistik yang masing-masing
/// akurat, bukan dari `1 - logistic(z)`, karena pengurangan itu membulat
/// ke `0` begitu `z > ~36` padahal nilai sebenarnya masih positif.
pub(crate) fn logistic_derivative(z: f64) -> f64 {
    logistic(z) * logistic(-z)
}

/// Tambahkan satu suku informasi Fisher `grad · grad^T / p` ke `m`, dengan
/// aturan kontribusi nol untuk suku yang tak terdefinisi secara numerik.
///
/// Suku kategori dengan probabilitas `p` dan gradien `grad = dP/dtheta`
/// berkontribusi `grad · grad^T / p`. Pada model logistik gradien selalu
/// berorde `p` itu sendiri di ekor kurva (mis. `dP/dtheta = a * P * Q` untuk
/// M2PL dengan `Q <= 1`), sehingga `|grad|^2 / p = O(p) -> 0` ketika
/// `p -> 0`: limit kontribusinya adalah `0`. Karena itu suku dilewati (nol)
/// bila:
///
/// - `p <= f64::MIN_POSITIVE` (nol, negatif akibat pembulatan, atau sudah di
///   lantai probabilitas), karena pembagian oleh nilai ini hanya
///   menghasilkan `Inf`/`NaN` atau pembulatan murni, dan
/// - hasil kali gradien `grad[i] * grad[j]` persis `0` (mis. `0 / 0`
///   untuk `p` yang juga `0`; untuk `p > 0` hasilnya memang `0`).
///
/// Di luar kasus itu pembagian biasa dilakukan, sehingga hasil di rentang
/// normal tidak berubah.
pub(crate) fn add_fisher_term(m: &mut DMatrix<f64>, grad: &[f64], p: f64) {
    // Perbandingan `<=` sengaja dipakai: `NaN` (mis. dari `theta` NaN) tidak
    // lolos sebagai "kecil", jadi ia merambat ke matriks, bukan disembunyikan
    // sebagai kontribusi nol.
    if p <= f64::MIN_POSITIVE {
        return;
    }
    for (i, gi) in grad.iter().enumerate() {
        for (j, gj) in grad.iter().enumerate() {
            let outer = gi * gj;
            if outer != 0.0 {
                m[(i, j)] += outer / p;
            }
        }
    }
}

/// Logit linear multidimensional `z = d + a · theta` (+ intercept opsional
/// dari `theta` terobservasi lain bila diperlukan fase selanjutnya).
pub(crate) fn linear_predictor(a: &[f64], d: f64, theta: &[f64]) -> f64 {
    debug_assert_eq!(a.len(), theta.len());
    d + a.iter().zip(theta).map(|(aj, tj)| aj * tj).sum::<f64>()
}

/// `MDISC = ||a||` (norma Euclidean slope; Reckase, 2009).
pub(crate) fn mdisc(a: &[f64]) -> f64 {
    a.iter().map(|aj| aj * aj).sum::<f64>().sqrt()
}

/// `MDIFF = -d / MDISC` untuk satu intercept/boundary. Bernilai
/// `0.0` bila `mdisc == 0` (tidak ada slope yang membedakan, jadi tidak
/// ada arah kesulitan), bukan `NaN`/`Inf` dari pembagian dengan nol. Hasil
/// nol selalu `+0.0` (bukan `-0.0` dari `-d` saat `d = 0`) supaya tidak
/// tampil sebagai "-0.00" di UI.
pub(crate) fn mdiff(d: f64, mdisc: f64) -> f64 {
    if mdisc == 0.0 {
        0.0
    } else {
        -d / mdisc + 0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn logistic_at_zero_is_half() {
        assert_relative_eq!(logistic(0.0), 0.5);
    }

    #[test]
    fn logistic_saturates_for_extreme_z() {
        assert!(logistic(1000.0) > 1.0 - 1e-12);
        assert!(logistic(-1000.0) < 1e-12);
    }

    #[test]
    fn linear_predictor_matches_dot_product_plus_intercept() {
        let a = [1.0, -2.0, 0.5];
        let theta = [0.5, 1.0, -1.0];
        let z = linear_predictor(&a, 0.25, &theta);
        assert_relative_eq!(z, 0.25 + (1.0 * 0.5 + -2.0 * 1.0 - 0.5));
    }

    /// Rumus lama: logit diklem ke `[-35, 35]`, lalu `1 / (1 + exp(-z))`.
    fn naive_clamped_logistic(z: f64) -> f64 {
        let z = z.clamp(-35.0, 35.0);
        1.0 / (1.0 + (-z).exp())
    }

    #[test]
    fn stable_logistic_equals_naive_formula_in_normal_range() {
        let mut z = -35.0;
        while z <= 35.0 {
            let diff = (logistic(z) - naive_clamped_logistic(z)).abs();
            assert!(diff < 1e-15, "z={z}: selisih {diff}");
            z += 0.125;
        }
    }

    #[test]
    fn logistic_is_finite_and_in_unit_interval_for_extreme_z() {
        for z in [50.0, -50.0, 400.0, -400.0, 1e4, -1e4, 1e300, -1e300] {
            let p = logistic(z);
            assert!(p.is_finite() && (0.0..=1.0).contains(&p), "z={z}: p={p}");
        }
    }

    #[test]
    fn logistic_tails_underflow_to_exact_zero_and_one() {
        assert_eq!(logistic(1e4), 1.0);
        assert_eq!(logistic(-1e4), 0.0);
    }

    #[test]
    fn logistic_keeps_relative_precision_of_small_tail() {
        // logistic(-50) = exp(-50) / (1 + exp(-50)) ~ exp(-50).
        let expected = (-50.0_f64).exp();
        assert_relative_eq!(logistic(-50.0), expected, max_relative = 1e-15);
    }

    #[test]
    fn logistic_is_symmetric_around_zero() {
        for z in [0.3, 2.0, 10.0, 36.0, 400.0] {
            assert_relative_eq!(logistic(z) + logistic(-z), 1.0, epsilon = 1e-15);
        }
    }

    #[test]
    fn logistic_propagates_nan() {
        assert!(logistic(f64::NAN).is_nan());
    }

    #[test]
    fn logistic_derivative_is_finite_and_non_negative_for_extreme_z() {
        for z in [0.0, 50.0, -50.0, 400.0, -400.0, 1e4, -1e4] {
            let w = logistic_derivative(z);
            assert!(w.is_finite() && w >= 0.0, "z={z}: w={w}");
        }
        assert_relative_eq!(logistic_derivative(0.0), 0.25);
    }

    #[test]
    fn logistic_derivative_stays_positive_where_one_minus_p_rounds_to_zero() {
        // 1 - logistic(40) membulat ke 0, tetapi turunan sebenarnya ~ exp(-40).
        assert_relative_eq!(
            logistic_derivative(40.0),
            (-40.0_f64).exp(),
            max_relative = 1e-12
        );
    }

    #[test]
    fn fisher_term_adds_outer_product_over_probability() {
        let mut m = DMatrix::zeros(2, 2);
        add_fisher_term(&mut m, &[2.0, 3.0], 0.5);
        assert_relative_eq!(m[(0, 0)], 8.0);
        assert_relative_eq!(m[(0, 1)], 12.0);
        assert_relative_eq!(m[(1, 0)], 12.0);
        assert_relative_eq!(m[(1, 1)], 18.0);
    }

    #[test]
    fn fisher_term_is_zero_when_probability_is_zero_or_negative() {
        for p in [0.0, -1e-17, f64::MIN_POSITIVE] {
            let mut m = DMatrix::zeros(2, 2);
            add_fisher_term(&mut m, &[1.0, 1.0], p);
            assert_eq!(m, DMatrix::zeros(2, 2), "p={p}");
        }
    }

    #[test]
    fn fisher_term_is_zero_when_gradient_is_exactly_zero() {
        let mut m = DMatrix::zeros(2, 2);
        add_fisher_term(&mut m, &[0.0, 0.0], 0.3);
        assert_eq!(m, DMatrix::zeros(2, 2));
    }

    #[test]
    fn fisher_term_propagates_nan_probability() {
        let mut m = DMatrix::zeros(1, 1);
        add_fisher_term(&mut m, &[1.0], f64::NAN);
        assert!(m[(0, 0)].is_nan());
    }
}
