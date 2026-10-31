# 03 — MAP (Maximum A Posteriori)

> Landasan teori untuk `Method::Map` (`src/ability_estimation/map.rs`),
> **default** `EstimationConfig::method` (alasannya di §4).

## 1. Rumus

MAP memaksimalkan log-posterior langsung (bukan integrasi kuadratur seperti
EAP):

```
theta_hat = argmax_theta [ log L(theta) + log dmvnorm(theta; gmeans, gcov) ]
```

di mana `log L(theta) = sum_i log P_{k_i}(theta)` atas item terjawab
(`i`), `dmvnorm(theta; gmeans, gcov)` densitas normal multivariat prior
populasi.

`SE` dari akar diagonal invers Hessian log-posterior negatif di titik
optimum (observed information):

```
SE_j = sqrt( [ (-Hessian)^-1 ]_jj )
```

## 2. Optimisasi: Newton-Raphson finite-difference

`ability_estimation::optim::maximize()` mencari optimum lewat Newton-Raphson
dengan gradien & Hessian **finite-difference** (central difference),
backtracking line search, dan clamp `theta` ke `[-20, 20]`
(`optim::MAX_THETA`). Objektif cukup dievaluasi sebagai skalar — tidak
perlu gradien analitik per model item — jadi optimizer yang sama dipakai
ulang oleh `Ml` dan `Wle`. Yang dijamin adalah konvergensi ke optimum
fungsi objektif, bukan jalur iterasi tertentu. Clamp `[-20, 20]` menjaga
iterasi tetap finite (tidak meledak ke `infinity`/`NaN`) bila objektif
terus naik di satu arah.

## 3. Efek prior: sempit vs lebar

`gcov` (kovarians prior) mengontrol seberapa kuat `theta_hat` "ditarik" ke
arah `gmeans`:

- **Prior sempit** (`gcov` kecil, informasi prior kuat) — `theta_hat`
  mendekati `gmeans`, hampir tidak peduli data respons kecuali sangat
  banyak/ekstrem.
- **Prior lebar** (`gcov` besar, informasi prior lemah) — `theta_hat`
  mendekati estimasi ML murni (§ `docs/teori/03-ability-estimation-ml.md`),
  data respons mendominasi.

Formal: kontribusi prior ke skor (gradien log-posterior) adalah
`-inverse(gcov) * (theta - gmeans)` — makin kecil `gcov`, makin besar
`inverse(gcov)`, makin kuat "gaya tarik" ke `gmeans` relatif terhadap skor
likelihood.

## 4. MAP vs ML pada respons ekstrem

Beda penting dari `Ml` (`docs/teori/03-ability-estimation-ml.md`): karena
`log dmvnorm(theta; gmeans, gcov)` adalah fungsi cekung (concave) yang
menurun tanpa batas seiring `|theta|` menjauh dari `gmeans`, `MAP` **selalu**
punya maksimum berhingga — bahkan untuk pola respons yang membuat `ML`
divergen ke `±infinity` (semua item terjawab benar/salah), `MAP` tetap
memberi estimasi finite (ditarik prior). Ini alasan engine memakai `MAP`
sebagai default, bukan `ML`.

## Referensi

- Reckase, M. D. (2009). *Multidimensional Item Response Theory*, bab 5.
  Springer.
