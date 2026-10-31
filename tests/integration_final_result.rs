//! Integration test Fase 7: membandingkan `McatResult` Rust vs fixture
//! `tests/fixtures/final_result.json`.
//!
//! Fixture menyimpan nilai acuan beku satu sesi CAT end-to-end (skenario
//! identik `ci_session` Fase 4: classify CI 3-dimensi, `min_items=3`) yang
//! BENAR-BENAR berhenti lebih awal (item ke-4 dari 30) dengan klasifikasi
//! 3-dimensi CAMPURAN.
//!
//! Metodologi **replay** (pola sama seperti Fase 5/6): `theta`/SE tiap
//! langkah diinjeksi LANGSUNG dari `thetas_history`/`thetas_se_history`
//! fixture (bukan diestimasi ulang lewat Fase 3 -- itu sudah diverifikasi
//! terpisah di `tests/integration_ability_estimation.rs`), lalu
//! `stopping_rule::evaluate()` (Fase 4) dipanggil di langkah TERAKHIR untuk
//! menghasilkan `StopDecision` sungguhan, dan `McatResult::from_person`
//! (Fase 7) dirakit dari situ -- membuktikan pipa Fase 4 -> Fase 7 bekerja
//! benar dengan input nyata, bukan `StopDecision` buatan tangan.
//!
//! **Beda bentuk `thetas_history` yang disengaja** (lih. `src/result.rs`):
//! fixture menyimpan baris theta PRA-TES sebagai baris pertama (`nrow =
//! N_item + 1`); `McatResult::thetas_history`/`thetas_se_history` Rust
//! TIDAK menyertakan baris itu (`nrow = N_item`). Test ini men-strip baris
//! pertama fixture sebelum membandingkan.
//!
//! **Catatan penting soal `terminated_early`**: fixture tidak memuat nilai
//! acuan untuk field ini (`result.terminated_early` kosong, `[]`) -- tidak
//! ada nilai literal untuk dibandingkan. `McatResult::terminated_early`
//! diteruskan dari `StopDecision::terminated_early` Rust (Fase 4, logikanya
//! sendiri SUDAH diverifikasi lewat unit/integration test Fase 4 terpisah)
//! -- test ini hanya memverifikasi bahwa untuk skenario yang SUNGGUH
//! berhenti lebih awal karena presisi (bukan `max_items`),
//! `StopDecision::terminated_early` Rust bernilai `true`, selaras dengan
//! sesi acuan yang berhenti di item ke-4 dari 30 karena presisi.

use serde_json::Value;
use std::fs;
use theta_engine::irt::Dichotomous;
use theta_engine::{
    evaluate_stopping_rule, ClassifyDecision, ClassifyType, ItemBank, ItemMeta, McatResult,
    PersonState, SEVector, StoppingConfig, ThetaVector,
};

fn load_fixture() -> Value {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/final_result.json"
    );
    let raw = fs::read_to_string(path).expect("fixture JSON harus ada");
    serde_json::from_str(&raw).expect("fixture harus JSON valid")
}

fn f64_array(v: &Value) -> Vec<f64> {
    v.as_array()
        .unwrap()
        .iter()
        .map(|x| x.as_f64().unwrap())
        .collect()
}

fn f64_matrix(v: &Value) -> Vec<Vec<f64>> {
    v.as_array().unwrap().iter().map(f64_array).collect()
}

fn classify_decision_from_str(s: &str) -> ClassifyDecision {
    match s {
        "above cutoff" => ClassifyDecision::AboveCutoff,
        "below cutoff" => ClassifyDecision::BelowCutoff,
        "no decision" => ClassifyDecision::NoDecision,
        other => panic!("string klasifikasi tak dikenal: {other}"),
    }
}

fn assert_slice_close(actual: &[f64], expected: &[f64], tol: f64, context: &str) {
    assert_eq!(actual.len(), expected.len(), "{context}: panjang beda");
    for i in 0..expected.len() {
        assert!(
            (actual[i] - expected[i]).abs() < tol,
            "{context}: dim {i} actual={} expected={}",
            actual[i],
            expected[i]
        );
    }
}

#[test]
fn mcat_result_matches_reference_final_result_fixture_after_ci_early_stop() {
    let fixture = load_fixture();
    let tol = fixture["tolerance"].as_f64().unwrap();
    let nfact = fixture["nfact"].as_u64().unwrap() as usize;
    let n_items = fixture["n_items"].as_u64().unwrap() as usize;

    // Bank + populasi persis parameter fixture.
    let a1 = f64_array(&fixture["item_params"]["a1"]);
    let a2 = f64_array(&fixture["item_params"]["a2"]);
    let a3 = f64_array(&fixture["item_params"]["a3"]);
    let d = f64_array(&fixture["item_params"]["d"]);
    let mut bank = ItemBank::new();
    for i in 0..n_items {
        bank.push(
            Box::new(Dichotomous::m2pl(vec![a1[i], a2[i], a3[i]], d[i]).unwrap()),
            ItemMeta::new(i),
        )
        .unwrap();
    }
    let local_pattern: Vec<i64> = fixture["local_pattern"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_i64().unwrap())
        .collect();
    let items_answered_1indexed: Vec<usize> = fixture["result"]["items_answered"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_u64().unwrap() as usize)
        .collect();
    let thetas_history_expected = f64_matrix(&fixture["result"]["thetas_history"]);
    let thetas_se_history_expected = f64_matrix(&fixture["result"]["thetas_se_history"]);
    assert_eq!(
        thetas_history_expected.len(),
        items_answered_1indexed.len() + 1
    );
    assert_eq!(
        thetas_se_history_expected.len(),
        items_answered_1indexed.len() + 1
    );

    // Replay: injeksi theta/SE LANGSUNG dari fixture tiap langkah (pola sama
    // seperti integration test Fase 5/6) -- respons diambil dari
    // `local_pattern` pada posisi item yang diadministrasikan.
    let mut person = PersonState::from_theta_and_se(
        ThetaVector::zeros(nfact),
        SEVector::from_vec(thetas_se_history_expected[0].clone()),
    );
    for (step, &item_1indexed) in items_answered_1indexed.iter().enumerate() {
        let item_idx0 = item_1indexed - 1;
        let response = local_pattern[item_idx0] as usize;
        person.record_answer(item_idx0, response);
        person.set_estimate(
            ThetaVector::from_vec(thetas_history_expected[step + 1].clone()),
            SEVector::from_vec(thetas_se_history_expected[step + 1].clone()),
        );
    }

    // Fase 4: evaluasi stopping rule sungguhan di state akhir, config persis
    // desain sesi fixture (classify CI cutoffs=(0,0,0), min_items=3).
    //
    // **Catatan penting**: parameter fixture `classify_ci` (di sini `0.05`)
    // BUKAN langsung alpha yang dipakai kuantil normal baku -- desain sesi acuan
    // mentransformasinya menjadi `classify_alpha = (1 - classify_ci) / 2`
    // sebelum dipakai. `StoppingConfig::classify::Ci.alpha` Rust (Fase 4)
    // memakai `alpha` LANGSUNG sebagai argumen kuantil normal baku (nilai
    // PASCA-transformasi, lih. `src/stopping_rule/classify_ci.rs`), jadi
    // transformasi yang sama harus diterapkan di sini sebelum diteruskan.
    let cutoffs = f64_array(&fixture["classify_cutoffs"]);
    let classify_ci = fixture["classify_ci"].as_f64().unwrap();
    let classify_alpha = (1.0 - classify_ci) / 2.0;
    let min_items = fixture["min_items"].as_u64().unwrap() as usize;
    let stopping_config = StoppingConfig {
        min_items,
        max_items: n_items,
        classify: ClassifyType::Ci {
            cutoffs,
            alpha: classify_alpha,
        },
        ..Default::default()
    };
    let (stop_decision, _) =
        evaluate_stopping_rule(&stopping_config, &person, &bank, false, None).unwrap();
    assert!(
        stop_decision.stop_now,
        "sesi acuan berhenti di item ke-4 dari 30 karena presisi -- Rust harus setuju"
    );
    assert!(
        stop_decision.terminated_early,
        "berhenti karena klasifikasi presisi (bukan max_items) -- lih. catatan modul soal \
         `terminated_early` yang tidak punya nilai acuan di fixture"
    );

    let classification_expected: Vec<ClassifyDecision> = fixture["result"]["classification"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| classify_decision_from_str(v.as_str().unwrap()))
        .collect();
    assert_eq!(stop_decision.classification, classification_expected);

    // Fase 7: rakit McatResult & bandingkan SELURUH field terhadap fixture.
    let result = McatResult::from_person(&person, &stop_decision);

    assert_slice_close(
        result.thetas.as_slice(),
        &f64_array(&fixture["result"]["thetas"]),
        tol,
        "thetas",
    );
    assert_slice_close(
        result.se_thetas.as_slice(),
        &f64_array(&fixture["result"]["se_thetas"]),
        tol,
        "se_thetas",
    );

    // `thetas_history` fixture punya baris pra-tes ekstra (baris ke-0) --
    // di-strip sebelum dibandingkan (lih. catatan di atas modul).
    let expected_history = &thetas_history_expected[1..];
    let expected_se_history = &thetas_se_history_expected[1..];
    assert_eq!(result.thetas_history.len(), expected_history.len());
    assert_eq!(result.thetas_se_history.len(), expected_se_history.len());
    for (i, (row, expected_row)) in result
        .thetas_history
        .iter()
        .zip(expected_history.iter())
        .enumerate()
    {
        assert_slice_close(
            row.as_slice(),
            expected_row,
            tol,
            &format!("thetas_history[{i}]"),
        );
    }
    for (i, (row, expected_row)) in result
        .thetas_se_history
        .iter()
        .zip(expected_se_history.iter())
        .enumerate()
    {
        assert_slice_close(
            row.as_slice(),
            expected_row,
            tol,
            &format!("thetas_se_history[{i}]"),
        );
    }

    assert_eq!(result.classification, classification_expected);

    let expected_column_names: Vec<String> = fixture["result"]["column_names"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();
    assert_eq!(result.column_names, expected_column_names);
    assert_eq!(result.column_names, McatResult::column_names(nfact));

    let expected_items_answered: Vec<usize> =
        items_answered_1indexed.iter().map(|&i| i - 1).collect();
    assert_eq!(result.items_answered, expected_items_answered);

    let expected_responses: Vec<Option<i32>> = items_answered_1indexed
        .iter()
        .map(|&i1| Some(local_pattern[i1 - 1] as i32))
        .collect();
    assert_eq!(result.responses, expected_responses);

    assert!(result.terminated_early);
}
