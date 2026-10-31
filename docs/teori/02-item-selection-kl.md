# 02 — `Kl`/`KlN`: Pendekatan KL Pointwise Engine

> Landasan teori untuk `src/item_selection/criteria/kl.rs` (Fase 2).

## Definisi

Untuk setiap item kandidat, `Kl` menghitung divergensi Kullback-Leibler
antara distribusi probabilitas kategori respons item itu pada dua titik
yang berjarak `2*delta`:

```
p0 = probabilitas(item, theta - delta)
p1 = probabilitas(item, theta + delta)
skor = sum_k p1_k * (ln(p1_k) - ln(p0_k))
```

`theta` adalah estimasi saat ini, dan `delta` adalah satu skalar yang
ditambahkan ke (dan dikurangkan dari) **setiap** dimensi. Item dengan skor
terbesar dipilih. Intuisinya: item yang distribusi responsnya paling
berubah antara dua titik di sekitar estimasi adalah item yang paling mampu
membedakan peserta yang sedikit di atas dari yang sedikit di bawah estimasi
itu.

## Apa ini, dan apa bukan

Ini pendekatan pointwise milik engine ini sendiri. Ia **bukan** indeks KL
global Chang & Ying (1996). Indeks itu mengintegralkan
`KL(theta_hat || theta)` atas interval
`[theta_hat - delta_n, theta_hat + delta_n]` dengan `delta_n = c / sqrt(n)`,
sehingga intervalnya **menyempit** saat `n` bertambah. Perbedaan engine ini:

- **Dua titik ujung, bukan integral.** Skor hanya membandingkan distribusi
  di `theta - delta` dan `theta + delta`, tanpa integrasi atas interval.
  Hasilnya murah dan tidak butuh grid kuadratur, sehingga tetap praktis
  untuk `nfact > 1`. Harganya: skor tidak merepresentasikan rata-rata
  divergensi di sepanjang interval seperti indeks aslinya.
- **Satu `delta` untuk semua dimensi.** Menambahkan skalar yang sama ke
  tiap dimensi adalah penyederhanaan multidimensi yang dipilih engine, bukan
  rumus dari sebuah makalah. Perluasan KL multidimensi yang dirumuskan
  secara tersendiri ada di Wang, Chang & Boughton (2011); engine ini tidak
  mengimplementasikannya.
- **`KlN` melebarkan interval.** Lihat bagian berikut.

## `KlN`: interval yang melebar (konvensi engine)

`KlN` memakai skor yang sama, tetapi `delta` dikalikan akar jumlah item
terjawab:

```
delta_efektif = delta * sqrt(n_answered)
```

`n_answered` adalah jumlah item yang sudah dijawab **sebelum** langkah
seleksi ini. Interval jadi **melebar** seiring tes berjalan. Itu kebalikan
dari `delta_n = c / sqrt(n)` pada Chang & Ying (1996), dan tidak punya dasar
dari literatur: ini konvensi sengaja engine ini, dan satu-satunya dasarnya
adalah bahwa perilakunya terdefinisi dan terdokumentasi.

Konsekuensinya langsung dari rumus: pada `n_answered = 0`,
`delta_efektif = 0`, kedua titik berimpit, dan skor **semua** kandidat persis
nol. Pilihan item pertama lalu hanya ditentukan aturan seri (kandidat
pertama dengan skor tertinggi), bukan oleh informasi item. Perilaku ini
dikunci oleh test `kln_uses_wider_effective_delta_as_n_answered_grows`.

## Kenapa tidak butuh matriks informasi kumulatif

Beda dari D/T/A/E/W-rule, `Kl`/`KlN` **tidak** memakai `info_thetas`
(matriks kumulatif dari item terjawab). Skornya murni fungsi probabilitas
**item kandidat itu sendiri** pada `theta ± delta`. Ini konsekuensi
definisinya: yang dibandingkan adalah dua titik, bukan presisi estimasi
kumulatif.

## Referensi terkait

- Chang, H.-H., & Ying, Z. (1996). A global information approach to
  computerized adaptive testing. *Applied Psychological Measurement*,
  20(3), 213–229. (Indeks KL global; rujukan terkait, bukan sumber rumus
  di sini.)
- Wang, C., Chang, H.-H., & Boughton, K. A. (2011). Kullback-Leibler
  information and its applications in multi-dimensional adaptive testing.
  *Psychometrika*, 76(1), 13–39. (Perluasan KL multidimensi; rujukan
  terkait, bukan sumber rumus di sini.)
