# 04 — Klasifikasi `CI` (Confidence Interval)

## Quickstart

```rust
use theta_engine::stopping_rule::{evaluate, ClassifyDecision, ClassifyType, StoppingConfig};

let config = StoppingConfig {
    min_items: 5,
    max_items: bank.len(),
    classify: ClassifyType::Ci {
        cutoffs: vec![0.0, 0.0], // satu cutoff per-dimensi
        alpha: 0.05,             // interval kepercayaan dua sisi 90%
    },
    ..Default::default()
};
let (decision, _) = evaluate(&config, &person, &bank, false, None)?;
for (i, d) in decision.classification.iter().enumerate() {
    let label = match d {
        ClassifyDecision::AboveCutoff => "above cutoff",
        ClassifyDecision::BelowCutoff => "below cutoff",
        ClassifyDecision::NoDecision => "no decision",
    };
    println!("dimensi {i}: {label}");
}
```

## Kapan pakai

- Keputusan lulus/tidak-lulus (mastery testing) relatif satu titik cutoff
  tetap PER DIMENSI (mis. "kompeten" bila `theta_i > 0`).
- Butuh tingkat kepercayaan eksplisit yang bisa diatur (`alpha`; tingkat
  kepercayaan interval = `1 - 2*alpha`) — bukan sekadar "presisi umum"
  seperti `min_sem`.

## Cara kerja singkat

Untuk tiap dimensi dibentuk interval `theta_i ± z·SE_i` (`z = z_{1-alpha}`,
mis. `1.645` untuk `alpha = 0.05`). Bila seluruh interval berada di atas
`cutoff_i` → `AboveCutoff`; seluruhnya di bawah → `BelowCutoff`; bila
cutoff masih di dalam interval (atau tepat di batasnya) → `NoDecision`.
`SE` yang `NaN`/tak terhingga selalu `NoDecision`. Rumus dan rujukan:
`docs/teori/04-stopping-rule-ci.md` §1 dan §3.

## Memilih `cutoffs` & `alpha`

`cutoffs` biasanya `0.0` (skala theta standar `N(0,1)`, cutoff di rata-rata
populasi) kecuali ada kalibrasi standard-setting eksplisit (mis. cutoff
kelulusan hasil studi Angoff). `alpha` lebih kecil (mis. `0.01`) menuntut
bukti lebih kuat sebelum "yakin" (interval kepercayaan lebih lebar,
tingkat kepercayaan lebih tinggi) — tes cenderung lebih panjang; `alpha` lebih
besar (mis. `0.1`) sebaliknya.

## Membaca `decision.classification` per langkah

`classification[i]` selalu terisi (bukan `Option`) untuk SETIAP dimensi
begitu `n_answered >= min_items` — termasuk `NoDecision` SEBELUM semua
dimensi selesai. Ini memudahkan menampilkan progres klasifikasi real-time
(mis. UI CAT yang menunjukkan status tiap subskala saat tes berjalan):

```rust
// Sebelum semua dimensi yakin: classification == [AboveCutoff, NoDecision]
// Setelah semua dimensi yakin: classification == [AboveCutoff, BelowCutoff], stop_now = true
```

`decision.stop_now` tetap menunggu **SEMUA** dimensi (`all()`,
lih. `docs/teori/04-stopping-rule-ci.md` §2) — dimensi yang sudah
`AboveCutoff`/`BelowCutoff` tidak memaksa berhenti sendirian.

Contoh lengkap (dim1 cepat 'above cutoff', dim2 lambat -- tes menunggu
keduanya): `cargo run --example 04_stopping_rule_ci`.
