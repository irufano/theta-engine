# 02 — `Mei`: Maximum Expected Information

> Landasan teori untuk `src/item_selection/criteria/mei.rs` (Fase 2).
> Khusus `nfact = 1`.

## Definisi

Informasi Fisher item kandidat dievaluasi di estimasi `theta` **sesudah**
item itu dijawab, lalu dirata-ratakan atas kemungkinan jawaban. Untuk
kandidat `j` dengan kategori respons `k = 0..K-1` dan bobot posterior saat
ini `w_g`:

```
P(u_j = k)   = sum_g  w_g * P_jk(theta_g)            peluang prediktif jawaban k
theta_hat_k  = E[theta | data, u_j = k]              EAP bila jawaban k
MEI_j        = sum_k  P(u_j = k) * I_j(theta_hat_k)
```

Gagasan dasarnya dari van der Linden (1998). Rumus di atas adalah definisi
yang dipakai engine ini; bila membandingkan dengan literatur, periksa bentuk
persis yang dipakai di sana (mis. apakah yang dievaluasi informasi item
kandidat saja atau informasi tes).

## Beda dengan `Mpwi`

`Mpwi` merata-ratakan `I_j` atas posterior **saat ini**. `Mei` mengakui bahwa
jawaban item `j` sendiri menggeser estimasi: bila `j` sangat sulit, jawaban
benar menggeser `theta_hat` naik ke arah tempat `I_j` besar, sedangkan jawaban
salah menggesernya turun ke tempat `I_j` kecil. `Mei` menimbang kedua
akibat itu dengan peluangnya masing-masing, dan hanya mengevaluasi `I_j` di
satu titik per kategori (EAP baru), bukan di seluruh grid.

## Sifat

- Item tanpa diskriminasi bernilai tepat `0`.
- Pada item simetris (`a` bebas, `b = gmeans`) dan prior simetris, jawaban
  benar dan salah menggeser estimasi ke `+x` dan `-x` dengan peluang sama,
  dan `I(+x) = I(-x)` untuk 2PL tanpa tebakan, sehingga `MEI = I(x)`. Test
  `symmetric_item_and_prior_give_mirrored_updates_with_equal_information`
  memverifikasi kasus ini.
- Dengan tebakan `c > 0` informasi tidak simetris; rumusnya tetap berlaku
  tanpa penyederhanaan.

## Perhitungan di engine

- Posterior memakai prior grup, grid 61 titik, aturan trapesium: lih.
  `docs/teori/02-item-selection-mlwi.md`.
- EAP baru dihitung dari momen pertama bobot gabungan `w_g * P_jk(theta_g)`.
- `I_j` dievaluasi memakai informasi Fisher item itu sendiri
  (`ItemResponseModel::information`) di titik `theta_hat_k`.
- Tidak bergantung pada `person.theta()`.

## Batasan

- `nfact` harus 1; selain itu `McatError::InvalidParameter`.

## Referensi

- van der Linden, W. J. (1998). Bayesian item selection criteria for
  adaptive testing. *Psychometrika*, 63(2), 201–216.
