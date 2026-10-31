# 03 — ML (Maximum Likelihood)

> Landasan teori untuk `Method::Ml` (`src/ability_estimation/ml.rs`).

## 1. Rumus

ML memaksimalkan log-likelihood murni, tanpa suku prior:

```
theta_hat = argmax_theta [ log L(theta) ]
          = argmax_theta [ sum_i log P_{k_i}(theta) ]
```

atas item terjawab `i` dengan kategori respons teramati `k_i`. Sama seperti
`Map` (`docs/teori/03-ability-estimation-map.md`), dioptimasi lewat
Newton-Raphson finite-difference (`ability_estimation::optim::maximize`),
dengan objektif yang sama minus suku `log(prior)`.

## 2. Potensi divergensi pada respons ekstrem

Tanpa prior, `log L(theta)` untuk 2PL/3PL/4PL monoton (mendekati batas
asimtotik) seiring `theta -> +infinity` (probabilitas benar mendekati batas
atas) atau `theta -> -infinity` (probabilitas benar mendekati batas bawah).
Kalau **seluruh** item terjawab punya kategori respons yang sama (semua
benar, atau semua salah), `log L(theta)` terus naik tanpa pernah mencapai
maksimum berhingga — estimasi ML divergen.

`ability_estimation::ml::estimate()` melaporkan divergensi ini secara
eksplisit: saat SE hasil optimisasi `> 30` (konstanta
`SE_DIVERGENCE_THRESHOLD` di `ml.rs`) atau Hessian tidak invertible, `theta`
dilaporkan sebagai `±infinity` (tanda dari nilai mentah hasil optimisasi)
dan `SE` sebagai `NaN` — bukan angka besar yang menyesatkan seolah presisi
(SE 30+ berarti interval kepercayaan praktis tak berhingga):

```
bila SE_j > 30 atau Hessian tidak invertible:
    theta_j = sign(theta_j_mentah) * infinity
    SE_j    = NaN
```

### Ambang `SE > 30` adalah heuristik engine ini

Aturan `SE > 30` **bukan** hasil makalah atau teorema; ia konvensi milik
engine ini untuk mengubah gejala numerik divergensi menjadi sinyal yang
tegas. Mekanismenya:

1. Pada pola seragam, `log L(theta)` monoton, jadi tidak ada maksimum
   berhingga. Optimizer berhenti di titik finite yang jauh, tempat
   probabilitas respons jenuh (`P` mendekati 0 atau 1).
2. SE berasal dari `sqrt(diag(inverse(-H)))`, dengan `-H` informasi observasi
   di titik itu. Di wilayah jenuh, suku `P(1-P)` tiap item mendekati 0,
   sehingga `-H` hampir singular dan inversnya (kovarians asimtotik)
   meledak: SE menjadi sangat besar, atau Hessian sama sekali tidak
   invertible (SE `NaN`, juga dianggap divergen).
3. Nilai 30 dipertahankan karena jauh di atas SE ML yang sah pada skala IRT
   standar (orde satuan) dan jauh di bawah SE pola divergen, sehingga ada
   margin lebar di kedua sisi dan test (`2d_all_correct`, pola semua-benar
   dan semua-salah di `ml.rs`) tidak sensitif terhadap nilai persisnya.

Nilai ini konvensi yang boleh disetel (mis. 10 atau 100 akan memberi hasil
sama pada kasus-kasus di atas), bukan konstanta matematis. Mengubahnya
berarti mengubah kapan sebuah SE besar dilaporkan sebagai `±infinity`;
estimasi finite dengan SE di sekitar ambang (pola hampir ekstrem) adalah
daerah abu-abu yang memang tidak punya jawaban benar tunggal.

Kasus `2d_all_correct` di `tests/fixtures/ability_estimation.json` (pola
respons semua-benar) menguji perilaku ini.

## 3. Yang BUKAN cakupan `Method::Ml`

Sesi CAT operasional biasanya memasang fallback otomatis: kalau method
`ML` DAN seluruh respons terjawab identik (kurang dari dua kategori
berbeda), estimasi diganti jadi `MAP` — mencegah peserta pernah melihat
`theta = Inf` di tengah sesi.

**Ini logika sesi CAT, bukan tanggung jawab
`ability_estimation::estimate()`.** `Method::Ml` di Fase 3 sengaja mentah
(divergen apa adanya) — keputusan KAPAN beralih ke `MAP` sebagai
pengamanan adalah kebijakan orkestrasi level lebih tinggi, terpisah dari
definisi matematis estimator ML itu sendiri. Lih.
`docs/teori/03-ability-estimation-overview.md` §5.

## Referensi

- Reckase, M. D. (2009). *Multidimensional Item Response Theory*, bab 5.
  Springer.
