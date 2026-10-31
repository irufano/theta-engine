# 04 — `min_sem`: Stopping Rule Berbasis Standard Error

> Landasan teori untuk mode `ClassifyType::None` (`src/stopping_rule/sem.rs`)
> — mode DEFAULT `StoppingConfig::classify`.

## 1. Rumus

```
met_i = SE_i < min_sem_i        -- SE_i = NaN (tak terhitung) diperlakukan Infinity
stop  = all(met_i)
```

Berhenti hanya bila **SEMUA** dimensi memenuhi (`all()`) — lih.
`docs/teori/04-stopping-rule-overview.md` §2 untuk motivasi semantik ini.

`sem::evaluate()` menerapkan `NaN -> Infinity` lewat pengecekan `is_nan()`
eksplisit: `met_i = !se_i.is_nan() && se_i < min_sem_i`. `SE` bisa `NaN`
saat Hessian estimasi (Fase 3, method `Map`/`Ml`/`Wle`) tidak invertible —
kasus ini secara aman diperlakukan sebagai "belum presisi", bukan panic
atau `false` yang salah arah.

## 2. `min_sem` sebagai skalar vs vektor

`StoppingConfig::min_sem: Vec<f64>` boleh berpanjang `1` (satu ambang,
di-broadcast ke seluruh `nfact` dimensi) ATAU persis `nfact` (ambang
berbeda per dimensi); panjang lain adalah error. Aturan broadcast ini
diterapkan fungsi privat `stopping_rule::broadcast()`, dipakai juga oleh
`delta_thetas` (lih. `docs/teori/04-stopping-rule-delta-thetas.md`).

Default `min_sem = vec![0.3]` — ambang yang cukup ketat
untuk model unidimensional/dimensi rendah, tapi lih.
`docs/teori/04-stopping-rule-overview.md` §3 untuk kenapa ambang ini
sering perlu dilonggarkan (atau digantikan `max_items` sebagai batas
praktis) pada `nfact` tinggi.

## Referensi

- Weiss, D. J., & Kingsbury, G. G. (1984). Application of computerized
  adaptive testing to educational problems. *Journal of Educational
  Measurement*, 21(4), 361–375.
- Reckase, M. D. (2009). *Multidimensional Item Response Theory*, bab 7.
  Springer.
