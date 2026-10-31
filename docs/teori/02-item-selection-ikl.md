# 02 — `Ikl`: Integrated Kullback-Leibler

> Landasan teori untuk `src/item_selection/criteria/ikl.rs` (Fase 2).
> Khusus `nfact = 1`.

## Definisi

Divergensi KL antara distribusi respons item kandidat di estimasi `theta_hat`
dan di setiap `theta` lain, dirata-ratakan dengan bobot likelihood:

```
KL_j(theta_hat || theta) = sum_k P_jk(theta_hat) * ln( P_jk(theta_hat) / P_jk(theta) )
IKL_j                    = integral KL_j(theta_hat || theta) * L(theta) d theta
```

Item dengan `IKL_j` terbesar dipilih. Item itu adalah item yang distribusi
responsnya di `theta_hat` paling mudah dibedakan dari distribusi di nilai
`theta` lain yang masih didukung data, yaitu item yang paling mampu
"menyingkirkan" alternatif terhadap estimasi saat ini.

## Posisi terhadap literatur dan terhadap `Kl`/`KlN`

- Indeks KL dan gagasan informasi global berasal dari Chang & Ying (1996),
  yang mengintegralkan `KL` atas interval sempit
  `[theta_hat - delta_n, theta_hat + delta_n]` dengan `delta_n = c / sqrt(n)`.
  `Ikl` **bukan** indeks itu: integralnya atas seluruh grid `[-6, 6]`
  dengan bobot likelihood, mengikuti gagasan pembobotan likelihood
  `Mlwi` (Veerkamp & Berger, 1997) yang diterapkan pada KL.
- `Kl`/`KlN` (lih. `docs/teori/02-item-selection-kl.md`) hanya
  membandingkan dua titik ujung `theta +- delta` dan butuh parameter `delta`.
  `Ikl` tidak punya parameter itu dan menimbang setiap `theta` sesuai
  dukungan data.

## Perhitungan di engine

- `theta_hat` diambil dari `person.theta()` (estimasi apa pun yang dipasang
  pemanggil lewat `set_estimate`/`set_theta`). Ini satu-satunya kriteria 1D
  yang bergantung pada estimasi titik; sisanya hanya memakai jawaban dan
  prior.
- Bobot hanya likelihood (prior datar), grid 61 titik, aturan trapesium:
  lih. `docs/teori/02-item-selection-mlwi.md`.
- Peluang kategori dilantai pada `f64::MIN_POSITIVE` sebelum `ln`, dan
  kategori dengan `P(theta_hat) = 0` tidak berkontribusi (limit
  `0 * ln 0 = 0`).
- Berlaku untuk item dikotomus maupun graded.

## Sifat

- `KL >= 0` (pertidaksamaan Gibbs), jadi skor tak negatif.
- Item tanpa diskriminasi bernilai tepat `0`: distribusi responsnya sama di
  semua `theta`.
- Skor naik bila diskriminasi item naik.
- Pada `n_answered = 0` likelihood datar dan `theta_hat` adalah `theta`
  awal dari pemanggil, jadi skor tetap terdefinisi (beda dari `KlN`, yang
  nol untuk semua kandidat di langkah pertama).

## Batasan

- `nfact` harus 1; selain itu `McatError::InvalidParameter`.
- Skor sangat bergantung pada `theta_hat`; bila estimasi terbaru belum
  dipasang ke `PersonState` skornya memakai estimasi lama.

## Referensi

- Chang, H.-H., & Ying, Z. (1996). A global information approach to
  computerized adaptive testing. *Applied Psychological Measurement*,
  20(3), 213–229. (Asal indeks KL; bukan sumber rumus persis di sini.)
- Veerkamp, W. J. J., & Berger, M. P. F. (1997). Some new item selection
  criteria for adaptive testing. *Journal of Educational and Behavioral
  Statistics*, 22(2), 203–226. (Pembobotan likelihood.)
