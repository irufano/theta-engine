# 03 — `Fixed`: Identity (Tanpa Estimasi Ulang)

> Landasan teori untuk `Method::Fixed` (`src/ability_estimation/fixed.rs`).

## 1. Semantik

`Fixed` **tidak** mengestimasi apa pun — mengembalikan `(person.theta(),
person.se())` persis apa adanya, tanpa memanggil optimisasi atau kuadratur
sama sekali:

```
theta_baru = theta_terakhir
SE_baru    = SE_terakhir
```

Ini beda mendasar dari method lain di Fase 3, yang semuanya mengestimasi
ulang dan mengubah `theta`/SE berdasar respons baru.

## 2. Kapan dipakai

`Fixed` dipakai untuk **kuesioner non-adaptif**: seluruh item disajikan
dalam urutan tetap (biasanya dipasangkan dengan `Criterion::Seq`, Fase 2),
dan `theta` peserta hanya dihitung **sekali** di akhir (lewat method lain,
mis. `EAP`/`MAP`, di luar siklus item-per-item) — bukan diestimasi ulang
setiap item dijawab. Menjaga skor akhir tidak "terpengaruh urutan
penyajian" secara adaptif, sesuai untuk skenario di mana adaptivitas item
selection sendiri tidak dipakai (`nfact > 1` dengan kriteria `Seq`, lih.
`docs/teori/02-item-selection-seq.md`).

## 3. Kenapa bukan sekadar "tidak memanggil `estimate()`"

Alternatif naif: caller cukup TIDAK memanggil `ability_estimation::estimate()`
sama sekali untuk kuesioner non-adaptif. Tapi `Fixed` sebagai varian
`Method` eksplisit tetap berguna: memungkinkan `EstimationConfig` yang
SAMA (satu titik konfigurasi) dipakai konsisten di seluruh siklus CAT
(termasuk sesi campuran adaptif/non-adaptif, atau logging/debug yang
seragam lewat `AbilityEstimationDebug` — walau untuk `Fixed` seluruh
fieldnya `None`, lih. `docs/panduan/03-ability-estimation-fixed.md`), tanpa
percabangan `if method == fixed { skip } else { estimate(...) }` tersebar
di kode caller.

## Referensi

- van der Linden, W. J., & Glas, C. A. W. (Eds.) (2010). *Elements of
  Adaptive Testing*. Springer.
