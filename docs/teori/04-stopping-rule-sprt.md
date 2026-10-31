# 04 — Klasifikasi `Sprt` (Sequential Probability Ratio Test)

> Landasan teori untuk `ClassifyType::Sprt` (`src/stopping_rule/sprt.rs`).
> **Beda semantik fundamental** dari `Ci`
> (`docs/teori/04-stopping-rule-ci.md`): BUKAN "all dimensi", melainkan
> satu uji rasio log-likelihood sekuensial **gabungan** untuk seluruh
> vektor `theta`.

## 1. Rumus (Wald, 1945)

```
LL0     = log L(theta_lower)        -- log-likelihood item terjawab di hipotesis bawah
LL1     = log L(theta_upper)        -- log-likelihood item terjawab di hipotesis atas
llr     = LL1 - LL0                 -- log rasio likelihood
a       = log(beta / (1 - alpha))
b       = log((1 - beta) / alpha)

llr < a            -> BelowCutoff   (bukti kuat mendukung theta_lower)
llr > b            -> AboveCutoff   (bukti kuat mendukung theta_upper)
a <= llr <= b      -> belum ada keputusan, tes lanjut
```

`LL0`/`LL1` adalah log-likelihood **seluruh item terjawab** dievaluasi di
titik hipotesis `theta_lower`/`theta_upper` (bukan estimasi `theta`
berjalan) — `llr` adalah log rasio likelihood `L(upper)/L(lower)`.
Batas `[a, b]` diturunkan dari target tingkat kesalahan Tipe I (`alpha`) &
Tipe II (`beta`) yang diinginkan (mis. keduanya `0.05`) — semakin ketat
`alpha`/`beta`, semakin lebar `[a, b]`, semakin banyak bukti (item) yang
dibutuhkan sebelum keputusan diambil.

`sprt::evaluate()` menghitung `LL0`/`LL1` lewat
[`crate::ability_estimation::log_likelihood_at`] (Fase 3, sudah teruji) —
**tidak** menulis ulang loop probabilitas item (prinsip desain engine:
jangan duplikasi).

## 2. Keputusan gabungan, bukan per-dimensi

Beda krusial dari `Ci`: `theta_lower`/`theta_upper` adalah dua **titik**
`nfact`-dimensi (dua hipotesis vektor lengkap), dan `llr` adalah SATU
angka skalar (log rasio likelihood ANTARA kedua hipotesis titik itu) — bukan
`nfact` angka terpisah. Karenanya `StopDecision::classification` untuk mode
`Sprt` selalu berpanjang **1** (keputusan tunggal untuk seluruh vektor),
BUKAN `nfact` seperti mode `Ci`. Ini bukan penyederhanaan implementasi,
melainkan konsekuensi langsung dari uji Wald atas dua hipotesis titik.

## 3. Hipotesis sebagai satu titik `nfact`-dimensi

`lower`/`upper` di `ClassifyType::Sprt` bertipe `ThetaVector`: masing-masing
**satu** titik di ruang `nfact`-dimensi, bukan `nfact` titik 1-dimensi
terpisah. Perbedaan ini menentukan: menafsirkan vektor hipotesis sebagai
`nfact` observasi 1-dimensi akan menghitung likelihood di titik yang salah
dan membuat `LL0`/`LL1` tidak bermakna. `log_likelihood_at` menerima
`ThetaVector` multidimensional dengan orientasi yang benar sejak awal,
jadi ambiguitas itu tidak muncul.

Fixture `tests/fixtures/stopping_rule.json` (bagian `sprt_loglik`)
memverifikasi `LL0`/`LL1` pada model `nfact = 3` dengan
`lower = (-1, -1, -1)` dan `upper = (1, 1, 1)`, untuk beberapa jumlah item
terjawab.

## Referensi

- Wald, A. (1945). Sequential tests of statistical hypotheses. *Annals of
  Mathematical Statistics*, 16(2), 117–186.
- Eggen, T. J. H. M. (1999). Item selection in adaptive testing with the
  sequential probability ratio test. *Applied Psychological Measurement*,
  23(3), 249–261.
- van der Linden, W. J., & Glas, C. A. W. (Eds.) (2010). *Elements of
  Adaptive Testing*, bab stopping rule sekuensial. Springer.
