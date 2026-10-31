# 02 — `ARule`: Minimum (Weighted) Error-Variance

> Landasan teori untuk `src/item_selection/criteria/a_rule.rs` (Fase 2).

## Definisi

`ARule` meminimalkan `tr(W I^-1)`, **jumlah berbobot varians asimtotik**
per dimensi (van der Linden, 1999), dengan `I` = matriks informasi Fisher
kumulatif dan `W = diag(weights)`. `I^-1` adalah pendekatan matriks
kovarians asimtotik estimator `theta_hat`, jadi diagonalnya adalah varians
asimtotik per dimensi. Nilai kriteria disajikan **sudah dinegasikan** agar
konsisten dengan konvensi argmax dipakai seluruh kriteria lain:

```
skor(kandidat) = -sum_i weights[i] * pseudo_inverse(total_info)[i,i]
```

Memaksimalkan skor ini sama dengan meminimalkan varians asimtotik
berbobot.

## Pseudo-invers, bukan invers biasa (penyimpangan sengaja)

Makalah merumuskan kriteria dengan invers biasa. Engine **sengaja
menyimpang**: ia memakai pseudo-invers dengan ambang singular value
`1e-10` (`PSEUDO_INVERSE_EPS`), supaya matriks berpangkat kurang di awal
tes tetap menghasilkan skor. Perilaku untuk matriks berpangkat penuh tidak
berubah.

Info Fisher dari **satu** item dikotomus di `nfact > 1` selalu **rank 1**
(lih. juga `docs/teori/02-item-selection-d-rule.md`) — sebelum cukup item
terjawab, `total_info` bisa **singular**, dan invers biasa tidak
terdefinisi. `theta-engine` memakai **pseudo-invers Moore-Penrose** (via
SVD, `InfoMatrix::pseudo_inverse`) yang tetap memberi nilai berhingga untuk
matriks rank-defisien: singular value di bawah ambang diperlakukan nol,
sehingga arah yang belum terinformasi menyumbang nol ke pseudo-invers,
bukan varians tak-hingga. Akibatnya skor di awal tes tidak mencerminkan
ketidakpastian arah itu; ini harga dari pilihan tersebut.

Pilihan ini membuat `ARule` tetap terdefinisi di langkah-langkah awal tes:
invers biasa akan gagal atau menghasilkan `Inf` untuk kandidat dengan info
rank-defisien, sehingga argmax tidak bermakna. Nilai kriteria di langkah
0 (sebelum ada item dijawab, info tiap kandidat rank-defisien) diuji
terhadap fixture `tests/fixtures/item_selection.json`
(`tests/integration_item_selection.rs`).

`ARule { prior: true }` menambahkan `inverse(gcov)` ke `total_info` sebelum
inversi — selalu full rank, jadi tidak butuh jalur pseudo-invers untuk kasus
degenerate (meski `pseudo_inverse()` tetap dipakai seragam, karena untuk
matriks full-rank hasilnya identik dengan invers biasa).

## Referensi

- van der Linden, W. J. (1999). Multidimensional adaptive testing with a
  minimum error-variance criterion. *Journal of Educational and
  Behavioral Statistics*, 24(4), 398–412.
