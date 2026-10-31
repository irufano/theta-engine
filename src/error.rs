//! Error terpusat yang dipakai lintas fase `theta-engine`.

use thiserror::Error;

/// Error yang bisa terjadi saat konstruksi atau evaluasi model/konfigurasi
/// `theta-engine`. Dipakai sebagai pengganti `panic!` — setiap batasan
/// konfigurasi/model dinyatakan sebagai varian di sini, dicek saat konstruksi, bukan saat runtime jauh di dalam
/// perhitungan.
#[derive(Debug, Error, PartialEq)]
pub enum McatError {
    /// Dimensi vektor/matriks tidak cocok dengan `nfact` yang diharapkan.
    #[error("dimensi tidak cocok: diharapkan {expected}, didapat {actual} ({context})")]
    DimensionMismatch {
        /// Dimensi yang diharapkan.
        expected: usize,
        /// Dimensi yang sebenarnya didapat.
        actual: usize,
        /// Konteks tekstual (nama fungsi/operasi) tempat mismatch terjadi.
        context: &'static str,
    },

    /// Parameter model tidak valid (mis. `u <= g`, boundary graded tidak terurut).
    #[error("parameter tidak valid: {0}")]
    InvalidParameter(String),

    /// Content balancing digabung dengan `Criterion::Seq` — kombinasi ini
    /// sengaja dilarang (lih.
    /// `ContentBalancingConfig::validate_with_criterion`).
    #[error("content balancing tidak kompatibel dengan Criterion::Seq")]
    ContentBalancingIncompatibleWithSeq,
}
