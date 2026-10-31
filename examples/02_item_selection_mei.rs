//! Contoh Fase 2: kriteria `Mei` -- Maximum Expected Information: informasi Fisher item kandidat dievaluasi
//! di estimasi `theta` *sesudah* item itu dijawab, lalu dirata-ratakan atas
//! kemungkinan jawaban.
//!
//! Khusus `nfact = 1`. Contoh ini menjalankan lima langkah CAT satu dimensi
//! dengan jawaban skrip (benar, benar, salah, benar, benar), estimasi ulang
//! `theta` lewat EAP di tiap langkah, dan mencetak skor `Mei` seluruh
//! kandidat. Kolom "DRule" menunjukkan item yang akan dipilih kriteria
//! informasi di satu titik (setara Maximum Fisher Information pada
//! `nfact = 1`) sebagai pembanding.
//!
//! Jalankan dengan:
//! ```sh
//! cargo run --example 02_item_selection_mei
//! ```

use rand::rngs::StdRng;
use rand::SeedableRng;
use theta_engine::ability_estimation::estimate;
use theta_engine::irt::Dichotomous;
use theta_engine::{
    select_next_item, Criterion, EstimationConfig, GroupParams, ItemBank, ItemMeta, Method,
    PersonState, SelectionConfig,
};

/// Sembilan item 2PL satu dimensi: `b` dari -2 sampai 2 (langkah 0.5),
/// diskriminasi `a` bergantian 1.0 / 1.4 / 1.8.
fn build_bank() -> ItemBank {
    let mut bank = ItemBank::new();
    for i in 0..9 {
        let b = -2.0 + 0.5 * i as f64;
        let a = [1.0, 1.4, 1.8][i % 3];
        bank.push(
            Box::new(Dichotomous::m2pl(vec![a], -a * b).unwrap()),
            ItemMeta::new(i),
        )
        .unwrap();
    }
    bank
}

fn main() {
    let bank = build_bank();
    let group = GroupParams::standard_normal(1);
    let config = SelectionConfig {
        criterion: Criterion::Mei,
        debug: true,
        ..Default::default()
    };
    let reference = SelectionConfig {
        criterion: Criterion::DRule { prior: true },
        ..Default::default()
    };
    let estimation = EstimationConfig {
        method: Method::Eap { qmc: false },
        ..Default::default()
    };

    let scripted_responses = [1, 1, 0, 1, 1];
    let mut person = PersonState::new(1);
    let mut rng = StdRng::seed_from_u64(0);

    println!("== Mei ==");
    for (step, &response) in scripted_responses.iter().enumerate() {
        let candidates: Vec<usize> = (0..bank.len())
            .filter(|idx| !person.answered_items().contains(idx))
            .collect();
        let (chosen, debug) =
            select_next_item(&config, &candidates, &person, &bank, &group, &mut rng).unwrap();
        let (drule_choice, _) =
            select_next_item(&reference, &candidates, &person, &bank, &group, &mut rng).unwrap();
        let debug = debug.expect("debug=true harus menghasilkan Some");

        println!(
            "  langkah {step} (theta_hat = {:.3}): Mei memilih item ke-{}, DRule memilih item ke-{}",
            person.theta()[0],
            chosen + 1,
            drule_choice + 1
        );
        let mut scores = debug.candidate_scores.clone();
        scores.sort_by_key(|(idx, _)| *idx);
        for (idx, value) in &scores {
            let marker = if *idx == chosen { "  <- dipilih" } else { "" };
            println!("    item ke-{}: {value:.6}{marker}", idx + 1);
        }

        person.record_answer(chosen, response);
        let ((theta, se), _) = estimate(&estimation, &person, &bank, &group).unwrap();
        println!(
            "    jawaban {response} -> theta_hat = {:.3}, SE = {:.3}",
            theta[0], se[0]
        );
        person.set_estimate(theta, se);
    }
}
