# 02 — `Mpwi`: Maximum Posterior Weighted Information

> Landasan teori untuk `src/item_selection/criteria/mpwi.rs` (Fase 2).
> Khusus `nfact = 1`.

## Definisi

Sama seperti `Mlwi`, tetapi bobotnya posterior, yaitu likelihood dikali prior
grup:

```
MPWI_j = integral I_j(theta) * L(theta) * g(theta) d theta
```

`g(theta)` adalah kerapatan `N(gmeans, gcov)` dari `GroupParams` (van der
Linden, 1998). Item dengan `MPWI_j` terbesar dipilih.

## Peran prior

Prior menahan kriteria agar tidak terseret ke ekor `theta` yang tidak masuk
akal saat jawaban masih sedikit. Dengan prior `N(0, 1)` dan tanpa jawaban,
bobotnya adalah prior itu sendiri, sehingga item dengan informasi memuncak
di dekat `gmeans` menang (dikunci test
`bayesian_criteria_pick_item_centered_on_prior_mean_before_any_answer`).
Makin banyak item terjawab, likelihood makin mendominasi dan `Mpwi` makin
mendekati `Mlwi`. Bila prior informatif dan jawaban pertama bertentangan
dengannya, urutan kandidat bisa berbeda antara kedua kriteria (dikunci test
`differs_from_mlwi_when_prior_is_informative` di modul `mpwi`).

## Perhitungan di engine

- Grid, aturan trapesium, dan normalisasi sama dengan `Mlwi`: lih.
  `docs/teori/02-item-selection-mlwi.md`.
- Log-kerapatan prior dihitung dengan fungsi densitas yang sama dengan EAP,
  sehingga bobot posterior di sini identik dengan bobot posterior EAP
  (selain bobot ujung trapesium).
- Tidak bergantung pada `person.theta()`.

## Sifat

- Skor tak negatif; `0` untuk item tanpa diskriminasi.
- Menjumlah ke rata-rata `I_j` di bawah posterior saat ini, jadi satuannya
  informasi Fisher.

## Batasan

- `nfact` harus 1; selain itu `McatError::InvalidParameter`.
- Prior diambil dari `GroupParams` yang diberikan ke `select_next_item`.
  Prior yang sangat sempit dan salah menggeser pilihan item; bila prior
  tidak dipercaya, pakai `Mlwi`.

## Referensi

- van der Linden, W. J. (1998). Bayesian item selection criteria for
  adaptive testing. *Psychometrika*, 63(2), 201–216.
