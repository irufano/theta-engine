# 02 — `DRule`: D-Optimality

> Landasan teori untuk `src/item_selection/criteria/d_rule.rs` (Fase 2).

## Definisi

`DRule` memaksimalkan **determinan** matriks informasi Fisher kumulatif:

```
skor(kandidat) = det(total_info)
total_info = info_thetas(item terjawab) + info(kandidat, theta)
```

Determinan matriks informasi berbanding terbalik dengan **volume** elipsoid
kepercayaan asimtotik `theta` (D-optimality dalam teori desain eksperimen
klasik) — memaksimalkan determinan meminimalkan volume elipsoid itu, yakni
memaksimalkan presisi **gabungan** di seluruh dimensi laten sekaligus
(bukan per-dimensi independen seperti `TRule`).

`DRule { prior: true }` menambahkan `inverse(gcov)` (presisi prior populasi) ke
`total_info` sebelum menghitung determinan — lih.
`docs/teori/02-item-selection-overview.md` §4.

## Rank-deficiency di awal tes

Info Fisher dari **satu** item dikotomus di `nfact > 1` selalu berbentuk
hasil kali luar (*outer product*) satu vektor gradien terhadap dirinya
sendiri — matriks ini selalu **rank 1**. Konsekuensinya: `total_info`
(info `n` item terjawab + info kandidat) paling banyak berpangkat `n + 1`.
Selama `n + 1 < nfact`, `det(total_info) = 0` **persis** untuk semua
kandidat — `DRule` tanpa prior tidak bisa membedakan kandidat sampai
`n + 1 >= nfact`, dan itu pun hanya bila arah info item saling bebas
linear. Pada rentang ini urutan kandidat hanya ditentukan sisa pembulatan
floating point, jadi tidak ada pemilihan "terbaik menurut kriteria" yang
bermakna. Varian `prior: true` tidak kena masalah ini (`inverse(gcov)`
selalu full rank).

Test yang menyangkut rentang ini tidak boleh mengandalkan urutan kandidat:
`tests/integration_exposure_control.rs` memeriksa bahwa semua skor ~0 dan
item acuan adalah kandidat sah, sedangkan pemeriksaan top-n baru berlaku
setelah `n + 1 >= nfact`.

## Referensi

- Segall, D. O. (1996). Multidimensional adaptive testing.
  *Psychometrika*, 61(2), 331–354.
- Mulder, J., & van der Linden, W. J. (2009). Multidimensional adaptive
  testing with optimal design criteria for item selection.
  *Psychometrika*, 74(2), 273–296.
