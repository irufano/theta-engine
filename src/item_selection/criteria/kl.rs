//! `KL`/`KLn` — skor divergensi Kullback-Leibler pointwise milik engine ini.
//!
//! Skor satu item kandidat memakai **probabilitas kategorinya sendiri** pada
//! dua titik, `theta - delta` dan `theta + delta`, lalu menjumlahkan
//! `p1_k * (ln p1_k - ln p0_k)` atas kategori `k`. Kriteria ini tidak
//! menyentuh matriks informasi kumulatif.
//!
//! Ini **bukan** indeks KL global Chang & Ying (1996), yang mengintegralkan
//! `KL(theta_hat || theta)` atas interval `[theta_hat - delta_n,
//! theta_hat + delta_n]` dengan `delta_n = c / sqrt(n)` (menyempit seiring
//! `n` bertambah). Perbedaannya:
//!
//! - Engine ini hanya membandingkan **dua titik ujung** interval, tanpa
//!   integrasi. Hasilnya pendekatan pointwise yang murah dan tidak bergantung
//!   pada grid kuadratur, jadi tetap praktis untuk `nfact > 1`.
//! - `delta` adalah satu skalar yang ditambahkan ke **setiap dimensi**
//!   sekaligus. Ini penyederhanaan multidimensi yang dipilih engine, bukan
//!   rumus dari sebuah makalah (Wang, Chang & Boughton, 2011, memberi
//!   perluasan KL multidimensi yang berbeda dan lebih lengkap).
//! - `KLn` **melebarkan** interval: `delta_n = delta * sqrt(n_answered)`.
//!   Itu kebalikan dari `delta_n` yang menyempit pada Chang & Ying, dan
//!   merupakan konvensi sengaja engine ini, tanpa dasar dari literatur.
//!   Akibatnya, saat `n_answered = 0` kedua titik berimpit dan skor semua
//!   kandidat persis nol.
//!
//! Chang & Ying (1996) dan Wang, Chang & Boughton (2011) dicantumkan hanya
//! sebagai literatur terkait.

use crate::error::McatError;
use crate::irt::ItemResponseModel;
use crate::theta::ThetaVector;

/// Probabilitas kategori item pada `theta` yang digeser `shift` di setiap
/// dimensi.
fn probabilities_shifted(
    item: &dyn ItemResponseModel,
    theta: &ThetaVector,
    shift: f64,
) -> Result<Vec<f64>, McatError> {
    let moved = ThetaVector::from_vec(theta.as_slice().iter().map(|t| t + shift).collect());
    item.probabilities(&moved)
}

/// Skor KL untuk satu item kandidat: divergensi `KL(p_atas || p_bawah)` antara
/// distribusi kategori di `theta + delta` dan `theta - delta` (`delta` skalar,
/// sama di semua dimensi):
/// `sum_k p_atas_k * (ln p_atas_k - ln p_bawah_k)`.
pub(crate) fn symmetric_interval_kl(
    item: &dyn ItemResponseModel,
    theta: &ThetaVector,
    delta: f64,
) -> Result<f64, McatError> {
    let below = probabilities_shifted(item, theta, -delta)?;
    let above = probabilities_shifted(item, theta, delta)?;
    Ok(above
        .iter()
        .zip(&below)
        .fold(0.0, |acc, (hi, lo)| acc + hi * (hi.ln() - lo.ln())))
}

/// Setengah lebar interval efektif untuk `KLn`: `delta * sqrt(n_answered)`.
/// Interval melebar seiring jumlah item terjawab, dan nol saat belum ada
/// item terjawab.
pub(crate) fn widened_delta(delta: f64, n_answered: usize) -> f64 {
    delta * (n_answered as f64).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::irt::Dichotomous;
    use approx::assert_relative_eq;

    #[test]
    fn score_is_zero_at_zero_delta_and_nonnegative_otherwise() {
        // KL(P||P) = 0 di delta=0; untuk delta != 0 nilainya adalah
        // divergensi KL sungguhan (KL(p1 || p0)), yang selalu >= 0
        // (Gibbs' inequality) -- catatan: KL TIDAK simetris terhadap tanda
        // delta secara umum (membalik delta menukar arah KL(p1||p0) jadi
        // KL(p0||p1), yang berbeda kecuali pada titik simetri item).
        let item = Dichotomous::m2pl(vec![1.2, 0.8], 0.1).unwrap();
        let theta = ThetaVector::from_vec(vec![0.3, -0.2]);
        assert_relative_eq!(
            symmetric_interval_kl(&item, &theta, 0.0).unwrap(),
            0.0,
            epsilon = 1e-12
        );
        assert!(symmetric_interval_kl(&item, &theta, 0.15).unwrap() >= 0.0);
        assert!(symmetric_interval_kl(&item, &theta, -0.15).unwrap() >= 0.0);
    }

    #[test]
    fn widened_delta_grows_with_n_answered() {
        let base = 0.1;
        assert_relative_eq!(widened_delta(base, 0), 0.0);
        assert_relative_eq!(widened_delta(base, 1), 0.1);
        assert_relative_eq!(widened_delta(base, 4), 0.2);
        assert_relative_eq!(widened_delta(base, 9), 0.3);
        assert!(widened_delta(base, 9) > widened_delta(base, 4));
    }

    #[test]
    fn larger_delta_yields_larger_kl_for_discriminating_item() {
        let item = Dichotomous::m2pl(vec![1.5, 0.0], 0.0).unwrap();
        let theta = ThetaVector::from_vec(vec![0.0, 0.0]);
        let small = symmetric_interval_kl(&item, &theta, 0.05).unwrap();
        let large = symmetric_interval_kl(&item, &theta, 0.3).unwrap();
        assert!(large > small);
    }
}
