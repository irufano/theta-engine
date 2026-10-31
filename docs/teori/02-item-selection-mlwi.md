# 02 — `Mlwi`: Maximum Likelihood Weighted Information

> Landasan teori untuk `src/item_selection/criteria/mlwi.rs` (Fase 2).
> Khusus `nfact = 1`.

## Definisi

Untuk item kandidat `j`, `Mlwi` merata-ratakan informasi Fisher item itu atas
`theta` dengan bobot likelihood respons yang sudah terkumpul:

```
MLWI_j = integral I_j(theta) * L(theta) d theta
```

`L(theta)` adalah likelihood dari seluruh item yang sudah dijawab
(`person.answered_items()` + `person.responses()`), dan `I_j(theta)` informasi
Fisher item `j` di `theta`. Item dengan `MLWI_j` terbesar dipilih (Veerkamp &
Berger, 1997).

## Kenapa bukan `I_j(theta_hat)`

Kriteria informasi di satu titik menganggap estimasi `theta_hat` sudah benar.
Di awal tes `theta_hat` sangat tidak pasti, sehingga item yang paling
informatif *di titik itu* bisa meleset. `Mlwi` menilai item atas seluruh
rentang `theta` yang masih didukung data: likelihood yang lebar (sedikit
jawaban) berarti penilaian yang lebar, likelihood yang menyempit di sekitar
`theta_hat` berarti nilai yang mendekati `I_j(theta_hat)`. Perilaku limit ini
dikunci oleh test
`with_sharp_posterior_all_criteria_agree_with_point_information_criterion`.

## Perhitungan di engine

- Integral dihitung dengan aturan trapesium atas grid `[-6, 6]` yang sama
  dengan grid EAP (`default_quadpts(1)` = 61 titik). Titik ujung berbobot
  setengah: likelihood murni tidak mendekati nol di ujung `+6`, dan jumlah
  polos akan menggelembungkan normalisasinya.
- Bobot dinormalisasi menjumlah 1 (dihitung di ruang log lalu dikurangi
  maksimumnya, jadi tidak underflow pada tes panjang). Nilai kriteria yang
  dilaporkan di `ItemSelectionDebug::candidate_scores` karenanya adalah
  informasi **rata-rata berbobot**, bukan integral mentah; konstanta
  pengali yang sama untuk semua kandidat tidak mengubah urutan.
- **Tidak memakai prior.** Bobot hanya likelihood. Untuk bobot yang
  memasukkan prior grup, pakai `Mpwi`.
- Tidak bergantung pada `person.theta()`: bobot dibangun dari jawaban, bukan
  dari estimasi titik.

## Sifat

- Item tanpa diskriminasi (`a = 0`) bernilai tepat `0`.
- Tanpa jawaban, likelihood datar, sehingga skor tiap item sama dengan rata-rata
  `I_j` atas `[-6, 6]`. Item dengan `a` sama tetapi `b` berbeda hampir seri
  (selisih hanya dari pemotongan di ujung grid). Ini akibat langsung dari
  definisi; untuk item pertama gunakan `Mpwi`/`Mepv`/`Mei`/`Ikl` (yang
  memakai prior atau `theta` awal), atau item awal yang ditetapkan
  pemanggil.

## Batasan

- `nfact` harus 1. Pada `nfact > 1`, `select_next_item` dan `rank_candidates`
  mengembalikan `McatError::InvalidParameter` yang menyebut `Mlwi`.
- Biaya `O(kandidat x 61)` evaluasi informasi item per langkah, ditambah
  satu evaluasi likelihood per titik grid yang tumbuh linear dengan jumlah
  item terjawab.

## Referensi

- Veerkamp, W. J. J., & Berger, M. P. F. (1997). Some new item selection
  criteria for adaptive testing. *Journal of Educational and Behavioral
  Statistics*, 22(2), 203–226.
