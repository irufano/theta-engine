//! Integration test kriteria seleksi item khusus `nfact = 1`: `Mepv`,
//! `Mlwi`, `Mpwi`, `Mei`, `Ikl`.
//!
//! Berbeda dari `integration_item_selection.rs` (nilai acuan beku), test di
//! sini diturunkan dari sifat matematis tiap kriteria dan dari perilaku
//! sesi CAT sungguhan lewat API publik:
//! - semua kriteria menolak `nfact > 1` dengan error, bukan panic;
//! - pada posterior yang tajam, kelimanya memilih item yang sama dengan
//!   kriteria informasi di satu titik (`DRule` pada `nfact = 1`);
//! - sesi CAT penuh (seleksi -> respons -> EAP) menurunkan SE lebih cepat
//!   daripada urutan tetap, dan tidak pernah menyajikan item dua kali;
//! - nilai kriteria bisa dipakai exposure control (urutan skor tersedia);
//! - item politomus (graded) didukung: skor terdefinisi dan item berdaya
//!   beda tinggi menang.

use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use theta_engine::ability_estimation::estimate;
use theta_engine::irt::{Dichotomous, GradedResponse};
use theta_engine::{
    rank_candidates, select_next_item, Criterion, EstimationConfig, GroupParams, ItemBank,
    ItemMeta, McatError, Method, PersonState, SelectionConfig,
};

const UNIDIMENSIONAL_CRITERIA: [(&str, Criterion); 5] = [
    ("Mepv", Criterion::Mepv),
    ("Mlwi", Criterion::Mlwi),
    ("Mpwi", Criterion::Mpwi),
    ("Mei", Criterion::Mei),
    ("Ikl", Criterion::Ikl),
];

/// Bank 2PL satu dimensi: `n` item, `b` merata di [-3, 3], `a` bergantian
/// 0.8 / 1.2 / 1.6 supaya diskriminasi bervariasi.
fn bank_1d(n: usize) -> ItemBank {
    let mut bank = ItemBank::new();
    for i in 0..n {
        let b = -3.0 + 6.0 * i as f64 / (n - 1) as f64;
        let a = [0.8, 1.2, 1.6][i % 3];
        bank.push(
            Box::new(Dichotomous::m2pl(vec![a], -a * b).unwrap()),
            ItemMeta::new(i),
        )
        .unwrap();
    }
    bank
}

fn config(criterion: Criterion) -> SelectionConfig {
    SelectionConfig {
        criterion,
        ..Default::default()
    }
}

fn eap() -> EstimationConfig {
    EstimationConfig {
        method: Method::Eap { qmc: false },
        ..Default::default()
    }
}

/// Satu sesi CAT: pilih -> jawab (disimulasikan dari `true_theta`) -> EAP.
/// Mengembalikan `(person, urutan item)`.
fn run_session(
    criterion: Criterion,
    bank: &ItemBank,
    true_theta: f64,
    n_items: usize,
    seed: u64,
) -> (PersonState, Vec<usize>) {
    let group = GroupParams::standard_normal(1);
    let mut rng = StdRng::seed_from_u64(seed);
    let mut person = PersonState::new(1);
    let mut order = Vec::new();
    for _ in 0..n_items {
        let candidates: Vec<usize> = (0..bank.len())
            .filter(|i| !person.answered_items().contains(i))
            .collect();
        let (chosen, _) = select_next_item(
            &config(criterion),
            &candidates,
            &person,
            bank,
            &group,
            &mut rng,
        )
        .unwrap();
        let p_correct = bank
            .item(chosen)
            .unwrap()
            .probabilities(&theta_engine::ThetaVector::from_vec(vec![true_theta]))
            .unwrap()[1];
        let response = usize::from(rng.gen::<f64>() < p_correct);
        person.record_answer(chosen, response);
        order.push(chosen);
        let ((theta, se), _) = estimate(&eap(), &person, bank, &group).unwrap();
        person.set_estimate(theta, se);
    }
    (person, order)
}

#[test]
fn every_unidimensional_criterion_rejects_multidimensional_person() {
    let mut bank = ItemBank::new();
    bank.push(
        Box::new(Dichotomous::m2pl(vec![1.0, 0.5], 0.0).unwrap()),
        ItemMeta::new(0),
    )
    .unwrap();
    let person = PersonState::new(2);
    let group = GroupParams::standard_normal(2);
    let mut rng = StdRng::seed_from_u64(0);

    for (name, criterion) in UNIDIMENSIONAL_CRITERIA {
        let err = select_next_item(&config(criterion), &[0], &person, &bank, &group, &mut rng)
            .unwrap_err();
        assert!(
            matches!(&err, McatError::InvalidParameter(m) if m.contains(name) && m.contains("nfact = 1")),
            "{name}: error tidak menyebut kriteria dan nfact = 1: {err}"
        );
        assert!(rank_candidates(&config(criterion), &[0], &person, &bank, &group).is_err());
    }
}

#[test]
fn every_unidimensional_criterion_rejects_empty_candidates_and_out_of_bank_index() {
    let bank = bank_1d(5);
    let person = PersonState::new(1);
    let group = GroupParams::standard_normal(1);
    let mut rng = StdRng::seed_from_u64(0);

    for (name, criterion) in UNIDIMENSIONAL_CRITERIA {
        assert!(
            select_next_item(&config(criterion), &[], &person, &bank, &group, &mut rng).is_err(),
            "{name}: candidates kosong harus error"
        );
        assert!(
            select_next_item(&config(criterion), &[99], &person, &bank, &group, &mut rng).is_err(),
            "{name}: index di luar bank harus error"
        );
    }
}

#[test]
fn bayesian_criteria_pick_item_centered_on_prior_mean_before_any_answer() {
    // Item identik kecuali `b`, prior N(0, 1) simetris: tanpa jawaban,
    // posterior = prior, jadi item dengan b = 0 harus menang.
    let mut bank = ItemBank::new();
    for (i, b) in [-2.0_f64, -1.0, 0.0, 1.0, 2.0].into_iter().enumerate() {
        bank.push(
            Box::new(Dichotomous::m2pl(vec![1.5], -1.5 * b).unwrap()),
            ItemMeta::new(i),
        )
        .unwrap();
    }
    let person = PersonState::new(1);
    let group = GroupParams::standard_normal(1);
    let mut rng = StdRng::seed_from_u64(0);
    let candidates: Vec<usize> = (0..bank.len()).collect();

    for (name, criterion) in [
        ("Mepv", Criterion::Mepv),
        ("Mpwi", Criterion::Mpwi),
        ("Mei", Criterion::Mei),
        ("Ikl", Criterion::Ikl),
    ] {
        let (chosen, _) = select_next_item(
            &config(criterion),
            &candidates,
            &person,
            &bank,
            &group,
            &mut rng,
        )
        .unwrap();
        assert_eq!(chosen, 2, "{name} harus memilih item b = 0");
    }
}

#[test]
fn with_sharp_posterior_all_criteria_agree_with_point_information_criterion() {
    // 40 jawaban bergantian pada item b = 0 memusatkan posterior di sekitar
    // theta = 0 dengan SE kecil. Dengan posterior setajam itu, merata-ratakan
    // atas theta hampir sama dengan mengevaluasi di theta_hat, sehingga
    // kelimanya harus memilih item yang sama dengan DRule (informasi di
    // satu titik pada nfact = 1).
    let mut bank = ItemBank::new();
    bank.push(
        Box::new(Dichotomous::m2pl(vec![1.5], 0.0).unwrap()),
        ItemMeta::new(0),
    )
    .unwrap();
    // Kandidat: yang paling informatif di theta = 0 adalah a tinggi dan b = 0.
    for (i, (a, b)) in [(1.0, 1.5), (1.0, -1.5), (1.0, 0.0), (2.0, 0.0), (1.5, 2.5)]
        .into_iter()
        .enumerate()
    {
        bank.push(
            Box::new(Dichotomous::m2pl(vec![a], -a * b).unwrap()),
            ItemMeta::new(i + 1),
        )
        .unwrap();
    }
    let group = GroupParams::standard_normal(1);
    let mut person = PersonState::new(1);
    for i in 0..40 {
        person.record_answer(0, i % 2);
    }
    let ((theta, se), _) = estimate(&eap(), &person, &bank, &group).unwrap();
    assert!(se[0] < 0.25, "posterior harus tajam, SE = {}", se[0]);
    person.set_estimate(theta, se);

    let candidates: Vec<usize> = (1..bank.len()).collect();
    let mut rng = StdRng::seed_from_u64(0);
    let (reference, _) = select_next_item(
        &config(Criterion::DRule { prior: false }),
        &candidates,
        &person,
        &bank,
        &group,
        &mut rng,
    )
    .unwrap();
    assert_eq!(reference, 4, "acuan: item a = 2, b = 0");

    for (name, criterion) in UNIDIMENSIONAL_CRITERIA {
        let (chosen, _) = select_next_item(
            &config(criterion),
            &candidates,
            &person,
            &bank,
            &group,
            &mut rng,
        )
        .unwrap();
        assert_eq!(chosen, reference, "{name} harus sepakat dengan DRule");
    }
}

#[test]
fn full_adaptive_session_never_repeats_an_item_and_is_deterministic() {
    let bank = bank_1d(60);
    for (name, criterion) in UNIDIMENSIONAL_CRITERIA {
        let (_, first) = run_session(criterion, &bank, 1.0, 15, 7);
        let (_, second) = run_session(criterion, &bank, 1.0, 15, 7);
        assert_eq!(first, second, "{name}: seed sama harus urutan sama");
        let mut sorted = first.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(
            sorted.len(),
            first.len(),
            "{name}: ada item tersaji dua kali"
        );
    }
}

#[test]
fn adaptive_selection_reaches_lower_se_than_fixed_order_for_high_ability_examinee() {
    // Urutan tetap (`Seq`) menyajikan item dari b = -3 naik, jadi 15 item
    // pertamanya hampir semuanya terlalu mudah untuk theta = 1.5. Kriteria
    // adaptif harus mencapai SE lebih kecil dengan jumlah item yang sama.
    let bank = bank_1d(60);
    let true_theta = 1.5;
    let (seq_person, _) = run_session(Criterion::Seq, &bank, true_theta, 15, 3);
    let seq_se = seq_person.se()[0];

    for (name, criterion) in UNIDIMENSIONAL_CRITERIA {
        let (person, _) = run_session(criterion, &bank, true_theta, 15, 3);
        let se = person.se()[0];
        assert!(
            se < seq_se,
            "{name}: SE adaptif {se:.3} tidak lebih kecil dari urutan tetap {seq_se:.3}"
        );
        assert!(
            (person.theta()[0] - true_theta).abs() < 0.8,
            "{name}: theta_hat {:.3} terlalu jauh dari {true_theta}",
            person.theta()[0]
        );
    }
}

#[test]
fn rank_candidates_exposes_a_score_per_candidate_for_exposure_control() {
    let bank = bank_1d(10);
    let person = PersonState::new(1);
    let group = GroupParams::standard_normal(1);
    let candidates: Vec<usize> = (0..bank.len()).collect();

    for (name, criterion) in UNIDIMENSIONAL_CRITERIA {
        let scores =
            rank_candidates(&config(criterion), &candidates, &person, &bank, &group).unwrap();
        assert_eq!(scores.len(), candidates.len(), "{name}");
        assert!(scores.iter().all(|(_, s)| s.is_finite()), "{name}");
    }
}

#[test]
fn debug_trace_lists_every_candidate_score_and_matches_chosen_item() {
    let bank = bank_1d(10);
    let person = PersonState::new(1);
    let group = GroupParams::standard_normal(1);
    let candidates: Vec<usize> = (0..bank.len()).collect();
    let mut rng = StdRng::seed_from_u64(0);

    for (name, criterion) in UNIDIMENSIONAL_CRITERIA {
        let cfg = SelectionConfig {
            criterion,
            debug: true,
            ..Default::default()
        };
        let (chosen, debug) =
            select_next_item(&cfg, &candidates, &person, &bank, &group, &mut rng).unwrap();
        let debug = debug.expect("debug = true harus menghasilkan Some");
        assert_eq!(debug.chosen, chosen, "{name}");
        assert_eq!(debug.candidate_scores.len(), candidates.len(), "{name}");
        let best = debug
            .candidate_scores
            .iter()
            .copied()
            .fold(f64::NEG_INFINITY, |m, (_, s)| m.max(s));
        let chosen_score = debug
            .candidate_scores
            .iter()
            .find(|(i, _)| *i == chosen)
            .unwrap()
            .1;
        assert_eq!(chosen_score, best, "{name}: item terpilih bukan argmax");
        assert!(
            debug.info_before.is_none() && debug.info_after.is_none(),
            "{name}"
        );
    }
}

#[test]
fn mepv_score_never_exceeds_negative_current_posterior_variance() {
    // Hukum varians total: varians posterior yang diharapkan setelah satu
    // item tidak pernah melebihi varians posterior saat ini. Skor = -EPV,
    // jadi skor tidak pernah lebih kecil dari -Var(saat ini) dan tidak
    // pernah positif.
    let bank = bank_1d(20);
    let group = GroupParams::standard_normal(1);
    let mut person = PersonState::new(1);
    person.record_answer(10, 1);
    person.record_answer(11, 0);
    let ((theta, se), _) = estimate(&eap(), &person, &bank, &group).unwrap();
    person.set_estimate(theta, se);
    let current_variance = person.se()[0].powi(2);

    let candidates: Vec<usize> = (0..bank.len())
        .filter(|i| !person.answered_items().contains(i))
        .collect();
    let scores = rank_candidates(
        &config(Criterion::Mepv),
        &candidates,
        &person,
        &bank,
        &group,
    )
    .unwrap();
    for (idx, s) in scores {
        assert!(s <= 1e-12, "item {idx}: skor {s} positif");
        assert!(
            s >= -current_variance - 1e-6,
            "item {idx}: EPV {} melebihi varians saat ini {current_variance}",
            -s
        );
    }
}

#[test]
fn unidimensional_criteria_support_graded_items() {
    // Dua item 4 kategori dengan boundary sama; hanya slope yang beda. Tanpa
    // jawaban dan dengan prior N(0, 1), item berslope tinggi harus menang di
    // semua kriteria.
    let mut bank = ItemBank::new();
    for (i, a) in [0.6, 2.0].into_iter().enumerate() {
        bank.push(
            Box::new(GradedResponse::new(vec![a], vec![1.0, 0.0, -1.0]).unwrap()),
            ItemMeta::new(i),
        )
        .unwrap();
    }
    let person = PersonState::new(1);
    let group = GroupParams::standard_normal(1);
    let mut rng = StdRng::seed_from_u64(0);

    for (name, criterion) in UNIDIMENSIONAL_CRITERIA {
        let scores = rank_candidates(&config(criterion), &[0, 1], &person, &bank, &group).unwrap();
        assert!(scores.iter().all(|(_, s)| s.is_finite()), "{name}");
        let (chosen, _) = select_next_item(
            &config(criterion),
            &[0, 1],
            &person,
            &bank,
            &group,
            &mut rng,
        )
        .unwrap();
        assert_eq!(chosen, 1, "{name}: item slope tinggi harus menang");
    }
}
