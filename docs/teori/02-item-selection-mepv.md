# 02 — `Mepv`: Minimum Expected Posterior Variance

> Landasan teori untuk `src/item_selection/criteria/mepv.rs` (Fase 2).
> Khusus `nfact = 1`.

## Definisi

Pilih item yang, setelah dijawab, diharapkan membuat posterior `theta` paling
sempit (Owen, 1975; van der Linden, 1998). Untuk kandidat `j` dengan kategori
respons `k = 0..K-1` dan bobot posterior saat ini `w_g` di grid:

```
P(u_j = k)  = sum_g  w_g * P_jk(theta_g)            peluang prediktif jawaban k
Var_k       = Var(theta | data, u_j = k)            varians posterior bila jawaban k
EPV_j       = sum_k  P(u_j = k) * Var_k
```

Engine mengembalikan **`-EPV_j`** sebagai nilai kriteria, supaya semua
kriteria tetap dipilih lewat argmax (konvensi yang sama dengan `ARule`).

## Intuisi

Berbeda dari kriteria informasi, `Mepv` tidak memakai informasi Fisher sama
sekali. Ia mensimulasikan setiap kemungkinan jawaban, membentuk posterior
barunya, dan mengukur lebarnya. Jawaban yang tidak mungkin terjadi diberi
bobot kecil lewat `P(u_j = k)`.

## Sifat yang dijamin

- **Batas atas.** Menurut hukum varians total,
  `E[Var(theta | u)] <= Var(theta)`: varians yang diharapkan sesudah satu
  item tidak pernah melebihi varians posterior saat ini. Jadi
  `-Var(saat ini) <= skor <= 0`. Test
  `mepv_score_never_exceeds_negative_current_posterior_variance` mengunci
  kedua batas pada sesi sungguhan.
- Item tanpa diskriminasi (`a = 0`) tidak mengubah posterior, sehingga
  skornya tepat `-Var(saat ini)`: skor terendah yang mungkin.
- Skor naik (EPV turun) bila diskriminasi item naik.

## Perhitungan di engine

- Posterior memakai prior grup (`GroupParams`), grid 61 titik pada
  `[-6, 6]`, aturan trapesium: lih. `docs/teori/02-item-selection-mlwi.md`
  untuk detail grid dan normalisasi.
- Untuk tiap kategori, bobot gabungan `w_g * P_jk(theta_g)` dinormalisasi
  menjadi posterior baru; variansnya dihitung dari momen pertama dan kedua
  (dilantai nol terhadap galat pembulatan).
- Kategori dengan peluang prediktif nol dilewati.
- Berlaku untuk item dikotomus maupun graded (jumlah atas semua kategori).
- Tidak bergantung pada `person.theta()`.

## Batasan

- `nfact` harus 1; selain itu `McatError::InvalidParameter`.
- Biaya `O(kandidat x kategori x 61)` per langkah: momen posterior baru
  dihitung ulang untuk tiap kategori tiap kandidat.

## Referensi

- Owen, R. J. (1975). A Bayesian sequential procedure for quantal response
  in the context of adaptive mental testing. *Journal of the American
  Statistical Association*, 70(350), 351–356.
- van der Linden, W. J. (1998). Bayesian item selection criteria for
  adaptive testing. *Psychometrika*, 63(2), 201–216.
