//! Uji sesi MCAT lewat API Rust biner (`session`), tanpa JS. Perilaku yang
//! diharapkan diturunkan dari kontrak engine (lih. `docs/panduan/` di inti),
//! bukan dari implementasi binding. Lapisan JS diuji di `wasm/tests/js/`.

use serde_json::{json, Value};
use theta_engine::theta::ThetaVector;
use theta_engine_wasm::session::{Engine, EngineError, EngineSpec, SessionState};

const NFACT: usize = 3;

/// Bank `n` item M2PL 3 dimensi; dimensi dominan bergilir `i % 3`.
fn items(n: usize) -> Vec<Value> {
    (0..n)
        .map(|i| {
            let mut a = vec![0.2; NFACT];
            a[i % NFACT] = 0.8 + (i % 5) as f64 * 0.2;
            json!({ "type": "dichotomous", "a": a, "d": ((i % 7) as f64 - 3.0) * 0.4 })
        })
        .collect()
}

fn base_spec(n: usize) -> Value {
    json!({
        "items": items(n),
        "group": { "gmeans": [0.0, 0.0, 0.0],
                   "gcov": [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]] },
        "selection": { "criterion": { "DRule": { "prior": true } }, "weights": null, "debug": false },
        "estimation": { "method": "Map", "quadpts": null, "debug": false },
        "stopping": { "min_items": 1, "max_items": 10, "max_time": null, "min_sem": [0.0],
                      "delta_thetas": [0.0], "classify": "None",
                      "allow_constrain_breaks": false, "debug": false },
        "exposure": null,
        "content_balancing": null
    })
}

fn engine(spec: Value) -> Result<Engine, EngineError> {
    Engine::new(
        serde_json::from_value::<EngineSpec>(spec)
            .map_err(|e| EngineError::InvalidInput(e.to_string()))?,
    )
}

fn set(spec: &mut Value, path: &[&str], value: Value) {
    let (last, parents) = path.split_last().unwrap();
    let mut node = &mut *spec;
    for key in parents {
        node = child(node, key);
    }
    *child(node, last) = value;
}

/// Anak `key` dari objek, atau elemen ke-`key` dari larik.
fn child<'a>(node: &'a mut Value, key: &str) -> &'a mut Value {
    match node {
        Value::Array(items) => &mut items[key.parse::<usize>().unwrap()],
        _ => &mut node[key],
    }
}

/// Jalankan sesi sampai berhenti; jawaban = `(index + langkah) % 2`.
fn run_session(engine: &Engine, seed0: u64) -> (SessionState, Vec<usize>) {
    let mut state = engine.start(None).unwrap();
    let mut served = vec![];
    for step in 0.. {
        let next = engine.next(&state, seed0 + step as u64).unwrap();
        served.push(next.index);
        let out = engine
            .submit(&next.state, next.index, (next.index + step) % 2, None)
            .unwrap();
        state = out.state;
        if out.stop.stop_now {
            break;
        }
    }
    (state, served)
}

fn invalid(result: Result<Engine, EngineError>) -> EngineError {
    match result {
        Ok(_) => panic!("spec seharusnya ditolak"),
        Err(e) => e,
    }
}

// ---- alur normal ----------------------------------------------------------

#[test]
fn session_runs_until_max_items_without_repeating_items() {
    let engine = engine(base_spec(30)).unwrap();
    let (state, served) = run_session(&engine, 1);

    assert_eq!(served.len(), 10);
    let mut unique = served.clone();
    unique.sort_unstable();
    unique.dedup();
    assert_eq!(unique.len(), 10, "item tidak boleh disajikan dua kali");

    let result = engine.finish(&state).unwrap();
    assert_eq!(result.items_answered, served);
    assert_eq!(result.thetas_history.len(), served.len());
    assert_eq!(result.column_names, ["Theta_1", "Theta_2", "Theta_3"]);
    assert!(
        !result.terminated_early,
        "berhenti karena max_items, bukan presisi"
    );
}

#[test]
fn same_state_and_seed_select_the_same_item() {
    let mut spec = base_spec(30);
    set(&mut spec, &["selection", "criterion"], json!("Random"));
    let engine = engine(spec).unwrap();
    let state = engine.start(None).unwrap();

    let a = engine.next(&state, 42).unwrap().index;
    let b = engine.next(&state, 42).unwrap().index;
    assert_eq!(a, b);

    let picks: std::collections::HashSet<_> = (0..40)
        .map(|seed| engine.next(&state, seed).unwrap().index)
        .collect();
    assert!(
        picks.len() > 1,
        "seed berbeda harus bisa memberi item berbeda pada kriteria Random"
    );
}

#[test]
fn start_uses_given_theta_and_rejects_wrong_dimension() {
    let engine = engine(base_spec(10)).unwrap();

    let state = engine
        .start(Some(ThetaVector::from_vec(vec![0.5, -1.0, 0.25])))
        .unwrap();
    assert_eq!(state.person.theta().as_slice(), &[0.5, -1.0, 0.25]);

    let err = engine
        .start(Some(ThetaVector::from_vec(vec![0.5, 1.0])))
        .unwrap_err();
    assert_eq!(err.code(), "DIMENSION_MISMATCH");
}

#[test]
fn next_fails_with_no_candidates_once_bank_is_exhausted() {
    let mut spec = base_spec(3);
    set(&mut spec, &["stopping", "max_items"], json!(100));
    let engine = engine(spec).unwrap();
    let mut state = engine.start(None).unwrap();
    for step in 0..3 {
        let next = engine.next(&state, step).unwrap();
        state = engine
            .submit(&next.state, next.index, 1, None)
            .unwrap()
            .state;
    }
    assert_eq!(
        engine.next(&state, 9).unwrap_err(),
        EngineError::NoCandidates
    );
}

#[test]
fn max_time_stops_session_using_elapsed_ms() {
    let mut spec = base_spec(10);
    set(
        &mut spec,
        &["stopping", "max_time"],
        json!({ "secs": 30, "nanos": 0 }),
    );
    let engine = engine(spec).unwrap();
    let state = engine.start(None).unwrap();

    let next = engine.next(&state, 0).unwrap();
    let fast = engine
        .submit(&next.state, next.index, 1, Some(29_000))
        .unwrap();
    assert!(!fast.stop.stop_now);

    let next = engine.next(&fast.state, 1).unwrap();
    let slow = engine
        .submit(&next.state, next.index, 1, Some(2_000))
        .unwrap();
    assert!(slow.stop.stop_now, "total 31 detik melewati batas 30 detik");
}

#[test]
fn content_balancing_keeps_dimensions_near_target_proportions() {
    let mut spec = base_spec(30);
    set(&mut spec, &["stopping", "max_items"], json!(15));
    let categories: Vec<_> = (0..30)
        .map(|i| ["verbal", "numerik", "spasial"][i % 3])
        .collect();
    set(
        &mut spec,
        &["content_balancing"],
        json!({ "category_per_item": categories,
                "target_prop": { "verbal": 1.0 / 3.0, "numerik": 1.0 / 3.0, "spasial": 1.0 / 3.0 } }),
    );
    let engine = engine(spec).unwrap();
    let (_, served) = run_session(&engine, 7);

    let mut counts = [0usize; 3];
    for i in served {
        counts[i % 3] += 1;
    }
    assert_eq!(counts, [5, 5, 5]);
}

#[test]
fn sympson_hetter_with_zero_exposure_never_serves_the_item_while_others_remain() {
    let mut spec = base_spec(12);
    let mut exposure = vec![1.0; 12];
    // Item 0 sengaja diberi peluang lolos 0: tidak boleh disajikan selama masih
    // ada kandidat lain.
    exposure[0] = 0.0;
    set(
        &mut spec,
        &["exposure"],
        json!({ "SympsonHetter": exposure }),
    );
    set(&mut spec, &["stopping", "max_items"], json!(8));
    let engine = engine(spec).unwrap();
    for seed in 0..10 {
        let (_, served) = run_session(&engine, seed * 100);
        assert!(
            !served.contains(&0),
            "seed {seed}: item 0 muncul {served:?}"
        );
    }
}

#[test]
fn randomesque_selection_is_reproducible_for_the_same_seed() {
    let mut spec = base_spec(20);
    set(
        &mut spec,
        &["exposure"],
        json!({ "Randomesque": vec![3; 20] }),
    );
    let engine = engine(spec).unwrap();
    let (_, first) = run_session(&engine, 11);
    let (_, second) = run_session(&engine, 11);
    assert_eq!(first, second);
}

// ---- spec ditolak saat konstruksi ------------------------------------------

#[test]
fn rejects_specs_that_core_would_only_fail_midway() {
    type Edit = fn(&mut Value);
    let cases: Vec<(&str, Edit, &str)> = vec![
        (
            "bank kosong",
            |s| set(s, &["items"], json!([])),
            "INVALID_INPUT",
        ),
        (
            "u <= g",
            |s| {
                set(
                    s,
                    &["items", "0"],
                    json!({ "type": "dichotomous", "a": [1.0, 1.0, 1.0], "d": 0.0, "g": 0.5, "u": 0.4 }),
                )
            },
            "INVALID_PARAMETER",
        ),
        (
            "boundary graded tidak menurun",
            |s| {
                set(
                    s,
                    &["items", "0"],
                    json!({ "type": "graded", "a": [1.0, 1.0, 1.0], "d": [0.0, 1.0] }),
                )
            },
            "INVALID_PARAMETER",
        ),
        (
            "nfact item tidak konsisten",
            |s| {
                set(
                    s,
                    &["items", "1"],
                    json!({ "type": "dichotomous", "a": [1.0, 1.0], "d": 0.0 }),
                )
            },
            "DIMENSION_MISMATCH",
        ),
        (
            "nfact grup beda dari item",
            |s| {
                set(
                    s,
                    &["group"],
                    json!({ "gmeans": [0.0, 0.0], "gcov": [[1.0, 0.0], [0.0, 1.0]] }),
                )
            },
            "DIMENSION_MISMATCH",
        ),
        (
            "gcov tidak simetris",
            |s| {
                set(
                    s,
                    &["group", "gcov"],
                    json!([[1.0, 0.5, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]),
                )
            },
            "INVALID_PARAMETER",
        ),
        (
            "gcov tidak definit positif",
            |s| {
                set(
                    s,
                    &["group", "gcov"],
                    json!([[1.0, 2.0, 0.0], [2.0, 1.0, 0.0], [0.0, 0.0, 1.0]]),
                )
            },
            "INVALID_PARAMETER",
        ),
        (
            "gcov memuat NaN-setara (non-hingga)",
            |s| {
                set(
                    s,
                    &["group", "gcov"],
                    json!([[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, null]]),
                )
            },
            "INVALID_INPUT",
        ),
        (
            "bobot seleksi salah panjang",
            |s| set(s, &["selection", "weights"], json!([1.0, 1.0])),
            "DIMENSION_MISMATCH",
        ),
        (
            "kriteria satu dimensi pada nfact 3",
            |s| set(s, &["selection", "criterion"], json!("Mlwi")),
            "INVALID_PARAMETER",
        ),
        (
            "exposure salah panjang",
            |s| set(s, &["exposure"], json!({ "SympsonHetter": [1.0, 1.0] })),
            "DIMENSION_MISMATCH",
        ),
        (
            "Sympson-Hetter di luar [0, 1]",
            |s| set(s, &["exposure"], json!({ "SympsonHetter": vec![1.5; 12] })),
            "INVALID_PARAMETER",
        ),
        (
            "Randomesque berisi 0",
            |s| set(s, &["exposure"], json!({ "Randomesque": vec![0; 12] })),
            "INVALID_PARAMETER",
        ),
        (
            "exposure dengan kriteria tanpa skor",
            |s| {
                set(s, &["selection", "criterion"], json!("Random"));
                set(s, &["exposure"], json!({ "SympsonHetter": vec![1.0; 12] }));
            },
            "INVALID_PARAMETER",
        ),
        (
            "content balancing dengan Seq",
            |s| {
                set(s, &["selection", "criterion"], json!("Seq"));
                set(
                    s,
                    &["content_balancing"],
                    json!({ "category_per_item": vec!["a"; 12], "target_prop": { "a": 1.0 } }),
                );
            },
            "CONTENT_BALANCING_INCOMPATIBLE_WITH_SEQ",
        ),
        (
            "kategori content balancing salah panjang",
            |s| {
                set(
                    s,
                    &["content_balancing"],
                    json!({ "category_per_item": ["a"], "target_prop": { "a": 1.0 } }),
                )
            },
            "DIMENSION_MISMATCH",
        ),
        (
            "target proporsi tidak berjumlah 1",
            |s| {
                set(
                    s,
                    &["content_balancing"],
                    json!({ "category_per_item": vec!["a"; 12], "target_prop": { "a": 0.5 } }),
                )
            },
            "INVALID_PARAMETER",
        ),
        (
            "min_sem salah panjang",
            |s| set(s, &["stopping", "min_sem"], json!([0.3, 0.3])),
            "INVALID_PARAMETER",
        ),
        (
            "cutoff klasifikasi salah panjang",
            |s| {
                set(
                    s,
                    &["stopping", "classify"],
                    json!({ "Ci": { "cutoffs": [0.0], "alpha": 0.05 } }),
                )
            },
            "DIMENSION_MISMATCH",
        ),
        (
            "field tak dikenal ditolak",
            |s| set(s, &["selecton"], json!({})),
            "INVALID_INPUT",
        ),
    ];

    for (name, edit, code) in cases {
        let mut spec = base_spec(12);
        edit(&mut spec);
        let err = invalid(engine(spec));
        assert_eq!(err.code(), code, "{name}: {err}");
    }
}

// ---- input tidak dipercaya pada submit dan state ----------------------------

#[test]
fn submit_rejects_out_of_range_index_response_and_repeated_item() {
    let engine = engine(base_spec(10)).unwrap();
    let state = engine.start(None).unwrap();

    for (index, response) in [(10, 0), (usize::MAX, 0), (0, 2)] {
        let err = engine.submit(&state, index, response, None).unwrap_err();
        assert_eq!(
            err.code(),
            "INVALID_INPUT",
            "index {index}, respons {response}"
        );
    }

    let after = engine.submit(&state, 4, 1, None).unwrap().state;
    let err = engine.submit(&after, 4, 0, None).unwrap_err();
    assert_eq!(
        err.code(),
        "INVALID_INPUT",
        "item yang sama tidak boleh dijawab dua kali"
    );
}

#[test]
fn rejects_session_state_that_does_not_match_the_bank() {
    let engine = engine(base_spec(10)).unwrap();
    let state = engine.start(None).unwrap();
    let answered = engine.submit(&state, 3, 1, None).unwrap().state;
    let json = serde_json::to_value(&answered).unwrap();

    type Edit = fn(&mut Value);
    let tampered: Vec<(&str, Edit)> = vec![
        ("valid_items terlalu pendek", |v| {
            v["valid_items"] = json!([true])
        }),
        ("item di luar bank", |v| {
            v["person"]["answered_items"] = json!([99])
        }),
        ("respons di luar kategori", |v| {
            v["person"]["responses"] = json!([5])
        }),
        ("item ganda", |v| {
            v["person"]["answered_items"] = json!([3, 3]);
            v["person"]["responses"] = json!([1, 1]);
        }),
        ("answered dan responses beda panjang", |v| {
            v["person"]["responses"] = json!([])
        }),
        ("theta beda dimensi", |v| {
            v["person"]["theta"] = json!([0.0])
        }),
    ];

    for (name, edit) in tampered {
        let mut value = json.clone();
        edit(&mut value);
        let state: SessionState = serde_json::from_value(value).unwrap();
        for result in [
            engine.next(&state, 0).map(|_| ()),
            engine.submit(&state, 0, 0, None).map(|_| ()),
            engine.finish(&state).map(|_| ()),
        ] {
            assert_eq!(result.unwrap_err().code(), "INVALID_INPUT", "{name}");
        }
    }

    let mut unknown = json.clone();
    unknown["extra"] = json!(1);
    assert!(serde_json::from_value::<SessionState>(unknown).is_err());
}

// ---- paritas dengan fixture emas ---------------------------------------------

/// Fixture emas dipakai juga oleh uji JS (`wasm/tests/js/engine.test.ts`) untuk
/// memastikan hasil WebAssembly sama dengan native. Perbarui dengan
/// `UPDATE_GOLDEN=1 cargo test -p theta-engine-wasm golden`.
#[test]
fn golden_session_matches_fixture() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/golden_session.json"
    );
    if std::env::var_os("UPDATE_GOLDEN").is_some() {
        std::fs::write(
            path,
            serde_json::to_string_pretty(&build_golden()).unwrap() + "\n",
        )
        .unwrap();
    }
    let golden: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();

    let engine = engine(golden["spec"].clone()).unwrap();
    let mut state = engine.start(None).unwrap();
    let mut served = vec![];
    for step in golden["steps"].as_array().unwrap() {
        let next = engine.next(&state, step["seed"].as_u64().unwrap()).unwrap();
        served.push(next.index);
        state = engine
            .submit(
                &next.state,
                next.index,
                step["response"].as_u64().unwrap() as usize,
                None,
            )
            .unwrap()
            .state;
    }
    let result = engine.finish(&state).unwrap();

    let expected = &golden["expected"];
    let expected_items: Vec<usize> = serde_json::from_value(expected["items"].clone()).unwrap();
    assert_eq!(served, expected_items, "urutan item berubah");
    let expected_thetas: Vec<f64> = serde_json::from_value(expected["thetas"].clone()).unwrap();
    let expected_se: Vec<f64> = serde_json::from_value(expected["se_thetas"].clone()).unwrap();
    for (got, want) in result.thetas.as_slice().iter().zip(&expected_thetas) {
        assert!((got - want).abs() < 1e-9, "theta {got} vs {want}");
    }
    for (got, want) in result.se_thetas.as_slice().iter().zip(&expected_se) {
        assert!((got - want).abs() < 1e-9, "se {got} vs {want}");
    }
}

fn build_golden() -> Value {
    let mut spec = base_spec(30);
    set(
        &mut spec,
        &["estimation", "method"],
        json!({ "Eap": { "qmc": false } }),
    );
    set(&mut spec, &["stopping", "max_items"], json!(12));
    let categories: Vec<_> = (0..30)
        .map(|i| ["verbal", "numerik", "spasial"][i % 3])
        .collect();
    set(
        &mut spec,
        &["content_balancing"],
        json!({ "category_per_item": categories,
                "target_prop": { "verbal": 1.0 / 3.0, "numerik": 1.0 / 3.0, "spasial": 1.0 / 3.0 } }),
    );
    set(
        &mut spec,
        &["exposure"],
        json!({ "SympsonHetter": (0..30).map(|i| if i % 4 == 0 { 0.5 } else { 1.0 }).collect::<Vec<_>>() }),
    );

    let engine = engine(spec.clone()).unwrap();
    let mut state = engine.start(None).unwrap();
    let (mut steps, mut served) = (vec![], vec![]);
    for step in 0..12u64 {
        let seed = 1000 + step * 17;
        let next = engine.next(&state, seed).unwrap();
        let response = (next.index + step as usize) % 2;
        steps.push(json!({ "seed": seed, "response": response }));
        served.push(next.index);
        state = engine
            .submit(&next.state, next.index, response, None)
            .unwrap()
            .state;
    }
    let result = engine.finish(&state).unwrap();
    json!({
        "spec": spec,
        "steps": steps,
        "expected": { "items": served, "thetas": result.thetas.as_slice(), "se_thetas": result.se_thetas.as_slice() }
    })
}
