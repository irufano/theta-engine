# 05 — Randomesque: Top-*n* Random

> Landasan teori untuk `src/exposure_control/randomesque.rs`.

## 1. Rumus

Per langkah, `exposure` adalah nilai vektor untuk posisi penyajian saat
ini dan `crit` adalah skor kandidat. Parameter eksposur `n = exposure`
berarti: urutkan kandidat menurut `crit` dari yang terbaik, lalu pilih
SATU secara seragam acak di antara `n` kandidat teratas. Untuk `n = 1`
himpunan itu hanya berisi kandidat terbaik, sehingga metode ini sama
dengan memilih skor tertinggi biasa (tie: yang pertama muncul). Untuk
`n > 1` tie dalam peringkat diputus acak seragam, bukan menurut urutan tetap.

`exposure` di sini adalah `exposure[position_in_test]` — diindeks per
**posisi penyajian** (`position_in_test` = jumlah item terjawab, jadi
menentukan giliran ke berapa saat ini; lih.
`05-exposure-control-overview.md` §3), BUKAN per ID item seperti
`SympsonHetter`.

- **`exposure == 1`**: deterministik, identik dengan tanpa exposure
  control sama sekali (`argmax(crit)`, tie pertama menang).
- **`exposure > 1`**: (a) beri tiap kandidat **peringkat descending**
  berdasarkan `crit`, dengan tie-break **random** (tie mendapat urutan
  seragam-acak, bukan urutan tetap berdasar index); (b) ambil `exposure`
  kandidat berperingkat teratas; (c) pilih SATU secara **uniform random**
  di antara himpunan itu (bukan berbobot peringkat — kandidat rank-1 dan
  rank-`n` dalam himpunan punya peluang terpilih yang SAMA). Inilah ide
  randomesque McBride & Martin (1983): item terbaik tidak selalu terpilih,
  sehingga beban exposure terbagi ke beberapa item yang hampir sama baik.

## 2. Tie-break random di Rust

Tie-break random berarti tiap kandidat yang tied mendapat peringkat unik
yang diacak secara seragam. `randomesque::select()` mewujudkannya tanpa
fungsi ranking khusus: **shuffle acak** seluruh kandidat lebih dulu
(`Vec::shuffle`), BARU *stable sort* descending berdasarkan skor. Karena
`sort_by` Rust stabil, kandidat yang tied mempertahankan urutan relatif
dari hasil shuffle — yang sudah seragam-acak. Hasilnya: peringkat acak di
antara yang tied, dengan biaya satu shuffle dan satu sort.

## 3. Bila `exposure > jumlah kandidat tersisa`

`randomesque::select()` memangkas lewat
`n = exposure_for_position.min(candidates.len())` — pemilihan otomatis
jadi uniform random atas SELURUH kandidat tersisa, bukan error atau panic.
Ini perilaku yang wajar di akhir tes: "ambil acak dari `n` teratas" ketika
yang tersisa kurang dari `n` berarti ambil acak dari semua yang ada.

## 4. Beda fundamental dari Sympson–Hetter

| | Sympson–Hetter | Randomesque |
|---|---|---|
| Indeks vektor `exposure` | per ID item | per posisi penyajian |
| Mekanisme | terima/tolak probabilistik (loop) | pilih uniform dari top-*n* |
| Efek "tidak pernah lolos" | `exposure[item]=0.0` (lih. `05-exposure-control-sympson-hetter.md` §4) | tidak ada — semua kandidat di luar top-*n* MUNGKIN saja masuk top-*n* di posisi lain/sesi lain (peringkat kriteria berubah seiring info kumulatif) |
| State lintas-langkah | ya (`valid_items` persisten) | tidak — tiap langkah independen, tidak ada state yang dibawa |

Randomesque TIDAK punya konsep "item dilarang" — ia murni membatasi
KANDIDAT MANA yang boleh bersaing di satu posisi tertentu, sehingga item
yang "kalah" di suatu posisi tetap bisa jadi top-*n* di posisi lain begitu
kandidat lain sudah terjawab & peringkat kriteria berubah.

## Referensi

- McBride, J. R., & Martin, J. T. (1983). Reliability and validity of
  adaptive ability tests in a military setting. In *New Horizons in
  Testing* (pp. 223–236). Academic Press.
- Georgiadou, E., Triantafillou, E., & Economides, A. (2007). A review of
  item exposure control strategies for computerized adaptive testing
  developed on the item response theory framework. *International
  Journal on E-Learning*, 6(1).
