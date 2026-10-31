//! Mode klasifikasi `SPRT` (Sequential Probability Ratio Test, Wald 1945) —
//! **beda semantik** dari `Ci`: bukan "all dimensi", melainkan satu uji
//! rasio log-likelihood sekuensial antara dua hipotesis vektor `theta_lower`
//! vs `theta_upper`, dibandingkan terhadap batas `[a, b]` yang diturunkan
//! dari `alpha`/`beta`. Keputusan bersifat **gabungan** (satu label untuk
//! seluruh vektor `theta`, bukan per-dimensi) -- `classification` berisi
//! satu elemen saja untuk mode ini.
//!
//! ```text
//! LL0  = log_likelihood(theta = lower)   // atas seluruh item terjawab
//! LL1  = log_likelihood(theta = upper)
//! LLR  = LL1 - LL0
//! a    = ln(beta / (1 - alpha))
//! b    = ln((1 - beta) / alpha)
//! LLR < a -> stop, BelowCutoff
//! LLR > b -> stop, AboveCutoff
//! selain itu -> lanjut, NoDecision
//! ```
//!
//! **Catatan bentuk `theta`**: `lower`/`upper` dievaluasi sebagai SATU
//! titik `nfact`-dimensi (matriks `1 x nfact`), BUKAN `nfact` titik
//! 1-dimensi terpisah (kolom `nfact x 1`) -- tafsiran kedua membuat uji
//! tidak valid untuk `nfact > 1`. Fixture referensi SPRT crate ini juga
//! disusun dengan `theta` berbentuk `1 x nfact` -- lih.
//! `docs/teori/04-stopping-rule-sprt.md`. `log_likelihood_at` sudah
//! bekerja benar untuk `ThetaVector` multidimensional.

use crate::bank::ItemBank;
use crate::error::McatError;
use crate::item_selection::PersonState;
use crate::stopping_rule::ClassifyDecision;
use crate::theta::ThetaVector;

/// Hasil evaluasi satu langkah SPRT.
pub(crate) struct SprtOutcome {
    pub log_likelihood_ratio: f64,
    pub bounds: (f64, f64),
    pub decision: ClassifyDecision,
    pub stop_now: bool,
}

/// Batas `[a, b]` Wald SPRT dari `alpha`/`beta`: `a = ln(beta / (1 - alpha))`,
/// `b = ln((1 - beta) / alpha)`.
pub(crate) fn bounds(alpha: f64, beta: f64) -> (f64, f64) {
    let a = (beta / (1.0 - alpha)).ln();
    let b = ((1.0 - beta) / alpha).ln();
    (a, b)
}

/// `LL0`/`LL1` dihitung lewat `ability_estimation::log_likelihood_at` (Fase
/// 3, sudah teruji) atas seluruh item yang sudah dijawab `person` -- TIDAK
/// menulis ulang loop probabilitas item.
pub(crate) fn evaluate(
    person: &PersonState,
    bank: &ItemBank,
    lower: &ThetaVector,
    upper: &ThetaVector,
    alpha: f64,
    beta: f64,
) -> Result<SprtOutcome, McatError> {
    let ll0 = crate::ability_estimation::log_likelihood_at(person, bank, lower)?;
    let ll1 = crate::ability_estimation::log_likelihood_at(person, bank, upper)?;
    let ratio = ll1 - ll0;
    let (a, b) = bounds(alpha, beta);
    let decision = if ratio < a {
        ClassifyDecision::BelowCutoff
    } else if ratio > b {
        ClassifyDecision::AboveCutoff
    } else {
        ClassifyDecision::NoDecision
    };
    let stop_now = !matches!(decision, ClassifyDecision::NoDecision);
    Ok(SprtOutcome {
        log_likelihood_ratio: ratio,
        bounds: (a, b),
        decision,
        stop_now,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bank::ItemMeta;
    use crate::irt::Dichotomous;
    use approx::assert_relative_eq;

    fn bounds_reference(alpha: f64, beta: f64) -> (f64, f64) {
        ((beta / (1.0 - alpha)).ln(), ((1.0 - beta) / alpha).ln())
    }

    #[test]
    fn bounds_match_wald_formula() {
        let (a, b) = bounds(0.05, 0.05);
        let (ra, rb) = bounds_reference(0.05, 0.05);
        assert_relative_eq!(a, ra, epsilon = 1e-12);
        assert_relative_eq!(b, rb, epsilon = 1e-12);
        // alpha=beta=0.05 -> a = ln(0.05/0.95) ~ -2.944, b = ln(0.95/0.05) ~ 2.944
        assert_relative_eq!(a, -2.9444389791664403, epsilon = 1e-9);
        assert_relative_eq!(b, 2.9444389791664403, epsilon = 1e-9);
    }

    fn bank_2d_all_correct_favors_upper() -> (ItemBank, PersonState) {
        let mut bank = ItemBank::new();
        for i in 0..5 {
            bank.push(
                Box::new(Dichotomous::m2pl(vec![1.5, 1.5], 0.0).unwrap()),
                ItemMeta::new(i),
            )
            .unwrap();
        }
        let mut person = PersonState::new(2);
        for i in 0..5 {
            person.record_answer(i, 1); // seluruh jawaban benar (kategori atas)
        }
        (bank, person)
    }

    #[test]
    fn responses_consistent_with_upper_hypothesis_decide_above_cutoff() {
        let (bank, person) = bank_2d_all_correct_favors_upper();
        let lower = ThetaVector::from_vec(vec![-1.5, -1.5]);
        let upper = ThetaVector::from_vec(vec![1.5, 1.5]);
        let outcome = evaluate(&person, &bank, &lower, &upper, 0.05, 0.05).unwrap();
        assert_eq!(outcome.decision, ClassifyDecision::AboveCutoff);
        assert!(outcome.stop_now);
        assert!(outcome.log_likelihood_ratio > outcome.bounds.1);
    }

    #[test]
    fn responses_consistent_with_lower_hypothesis_decide_below_cutoff() {
        let mut bank = ItemBank::new();
        for i in 0..5 {
            bank.push(
                Box::new(Dichotomous::m2pl(vec![1.5, 1.5], 0.0).unwrap()),
                ItemMeta::new(i),
            )
            .unwrap();
        }
        let mut person = PersonState::new(2);
        for i in 0..5 {
            person.record_answer(i, 0); // seluruh jawaban salah (kategori bawah)
        }
        let lower = ThetaVector::from_vec(vec![-1.5, -1.5]);
        let upper = ThetaVector::from_vec(vec![1.5, 1.5]);
        let outcome = evaluate(&person, &bank, &lower, &upper, 0.05, 0.05).unwrap();
        assert_eq!(outcome.decision, ClassifyDecision::BelowCutoff);
        assert!(outcome.stop_now);
        assert!(outcome.log_likelihood_ratio < outcome.bounds.0);
    }

    #[test]
    fn ambiguous_response_pattern_before_any_answer_is_no_decision() {
        let mut bank = ItemBank::new();
        bank.push(
            Box::new(Dichotomous::m2pl(vec![1.5, 1.5], 0.0).unwrap()),
            ItemMeta::new(0),
        )
        .unwrap();
        let person = PersonState::new(2); // belum ada item terjawab -> LL0=LL1=0
        let lower = ThetaVector::from_vec(vec![-1.5, -1.5]);
        let upper = ThetaVector::from_vec(vec![1.5, 1.5]);
        let outcome = evaluate(&person, &bank, &lower, &upper, 0.05, 0.05).unwrap();
        assert_eq!(outcome.decision, ClassifyDecision::NoDecision);
        assert!(!outcome.stop_now);
        assert_relative_eq!(outcome.log_likelihood_ratio, 0.0, epsilon = 1e-12);
    }
}
