# 04 — Klasifikasi `Ci` (Confidence Interval)

> Landasan teori untuk `ClassifyType::Ci` (`src/stopping_rule/classify_ci.rs`).

## 1. Uji interval kepercayaan

Untuk tiap dimensi `i`, dari estimasi `theta_i` dan galat baku terkini
`SE_i` dibentuk interval kepercayaan

```
[ L_i , U_i ] = [ theta_i - z·SE_i ,  theta_i + z·SE_i ]
z             = z_{1-alpha}                -- kuantil normal standar peringkat 1-alpha
```

Dimensi `i` **terklasifikasi** bila `cutoff_i` berada di luar interval itu:

```
L_i > cutoff_i   -> AboveCutoff   (seluruh interval di atas cutoff)
U_i < cutoff_i   -> BelowCutoff   (seluruh interval di bawah cutoff)
selain itu       -> NoDecision    (cutoff di dalam interval, atau tepat di batasnya)
stop             = semua dimensi terklasifikasi
```

Ini klasifikasi berbasis interval kepercayaan klasik untuk tes mastery
adaptif (Weiss & Kingsbury, 1984; Eggen & Straetmans, 2000): peserta
diklasifikasikan begitu cutoff tidak lagi masuk akal berada dalam rentang
nilai yang konsisten dengan data. Pertidaksamaan bersifat **tegas** —
cutoff yang tepat berimpit dengan `L_i` atau `U_i` belum dianggap di luar
interval. `alpha` adalah ekor satu sisi: `alpha = (1 - tingkat_kepercayaan) / 2`,
jadi `alpha = 0.05` setara interval kepercayaan 90% (`z = 1.645`).
Kuantil normal simetris, sehingga `z_{1-alpha} = |Φ⁻¹(alpha)|`.

Secara aljabar, `L_i > cutoff_i` setara `(theta_i - cutoff_i) / SE_i > z`
dan `U_i < cutoff_i` setara `(cutoff_i - theta_i) / SE_i > z`: jarak
`theta_i` dari cutoff dalam satuan `SE_i` harus melebihi `z`.

## 2. Keputusan **per-dimensi**, independen dari gerbang `all()`

`classify_ci::evaluate()` sengaja menghitung **kedua** nilai (status `met`
DAN label `ClassifyDecision`) untuk **setiap** dimensi secara independen
dari gerbang `all()` — mengekspos status "satu dimensi sudah yakin, satu
belum" secara real-time (`NoDecision` untuk dimensi yang belum `met`, tanpa
menunggu seluruh dimensi selesai). Label akhir saat semua dimensi `met`
tidak berubah karenanya — status per-langkah hanyalah informasi tambahan,
berguna untuk observability (lih. `examples/04_stopping_rule_ci.rs`).

Stop hanya terjadi bila **SEMUA** dimensi `met` (`all()`) — persis semantik
`docs/teori/04-stopping-rule-overview.md` §2.

## 3. `SE` tidak terhingga atau tak terdefinisi: cabang "tanpa keputusan"

`SE_i` bernilai `NaN` (Hessian estimasi tidak invertible), `Infinity`,
atau negatif membuat interval tidak informatif (tak terbatas atau
tak terdefinisi). `classify_ci::evaluate()` menangani ini lewat cabang
**eksplisit** sebelum interval dihitung: dimensi itu langsung `NoDecision`
dan tidak pernah terklasifikasi — bukan `NaN` yang akan gagal senyap di
perbandingan. (`SE_i = 0` sah: interval menyusut menjadi titik `theta_i`,
dan dimensi terklasifikasi kecuali `theta_i` persis sama dengan `cutoff_i`.)
Sikap ini sejalan dengan `min_sem` (`docs/teori/04-stopping-rule-min-sem.md` §1).

## Referensi

- Weiss, D. J., & Kingsbury, G. G. (1984). Application of computerized
  adaptive testing to educational problems. *Journal of Educational
  Measurement*, 21(4), 361–375.
- Eggen, T. J. H. M., & Straetmans, G. J. J. M. (2000). Computerized
  adaptive testing for classifying examinees into three categories.
  *Educational and Psychological Measurement*, 60(5), 713–734.
- van der Linden, W. J., & Glas, C. A. W. (Eds.) (2010). *Elements of
  Adaptive Testing*, bab stopping rule berbasis klasifikasi. Springer.
- Reckase, M. D. (2009). *Multidimensional Item Response Theory*, bab 7.
  Springer.
