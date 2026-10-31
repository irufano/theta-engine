# 07 — Hasil Bersih Akhir untuk Theta

> Landasan teori untuk `src/result.rs` (Fase 7).

## 1. Isi hasil akhir dan `terminated_early`

`McatResult::from_person` merangkum `PersonState` (state internal sesi)
jadi hasil akhir yang dikembalikan ke pemanggil di akhir sesi: `thetas`,
`se_thetas`, `thetas_history`, `thetas_se_history`, `classification`, dan
`terminated_early`.

`McatResult::terminated_early` diteruskan dari
`StopDecision::terminated_early` (Fase 4). Field itu adalah SINYAL yang
berguna untuk interpretasi: `true` hanya bila berhenti dipicu kriteria
presisi (`min_sem`/`delta_thetas`/klasifikasi), bukan
`max_items`/`max_time`/`CustomStop` — membedakan "berhenti karena YAKIN"
dari "berhenti karena KEHABISAN item/waktu" (lih.
`docs/teori/04-stopping-rule-max-items-time.md` §2).

`tests/integration_final_result.rs` memverifikasi bahwa untuk skenario
yang SUNGGUH berhenti lebih awal lewat klasifikasi `Ci` (fixture
`tests/fixtures/final_result.json`: 4 item terjawab dari `max_items = 30`),
`terminated_early` bernilai `true`.

## 2. Reuse eksplisit: `ClassifyDecision` & `info_thetas` BUKAN dibangun ulang

Hasil akhir tidak membangun enum klasifikasi baru maupun accessor matriks
informasi terpisah, karena keduanya sudah ada:

- `stopping_rule::ClassifyDecision` (Fase 4, `src/stopping_rule/mod.rs`)
  sudah persis enum yang dibutuhkan (`AboveCutoff`/`BelowCutoff`/
  `NoDecision`) -- dipakai ulang langsung sebagai
  `crate::stopping_rule::ClassifyDecision`.
- `PersonState::info_thetas(&self, bank: &ItemBank)` (Fase 2,
  `src/item_selection/mod.rs`) sudah persis accessor "matriks informasi
  kumulatif, dihitung sesuai permintaan, bukan bagian hasil baku".
  `McatResult` sengaja TIDAK menyimpan field `info_thetas` -- pengguna yang
  butuh kovarians estimasi manual memanggil `person.info_thetas(&bank)`
  selama `PersonState`/`ItemBank` masih hidup, SEBELUM
  `McatResult::from_person` dipanggil (yang mengonsumsi `&PersonState`
  secara referensi, bukan memindahkan kepemilikannya -- `person` tetap bisa
  dipakai setelahnya).

Ini prinsip desain engine (jangan duplikasi) yang sudah diterapkan
berulang di Fase 2 (`argmax` dipakai ulang exposure control), Fase 4
(`log_likelihood_at` dipakai ulang SPRT), dan Fase 6 (`ItemMeta.content`
disiapkan sebelumnya).

## 3. `thetas_history`/`thetas_se_history`: `N_item`, bukan `N_item + 1`

Konvensi yang umum adalah menyisipkan baris `theta`/SE PRA-TES (sebelum
item manapun dijawab) sebagai baris PERTAMA riwayat, lalu menambah satu
baris per item terjawab — panjang akhirnya `N_item + 1`.

`PersonState::thetas_history()`/`thetas_se_history()`
(`src/item_selection/mod.rs`, Fase 7) SENGAJA tidak menyertakan baris
pra-tes itu -- kosong sampai re-estimasi PERTAMA terjadi (dipanggil dari
`set_theta`/`set_estimate`), sehingga panjangnya persis `N_item`, sejalan
1:1 dengan `answered_items()`/`responses()`. Alasan:

1. **Konsisten dengan `items_answered`/`responses`** -- ketiganya jadi
   sama panjang, indeks `k` di ketiga array itu semua merujuk item
   terjawab ke-`k` yang SAMA. Dengan baris pra-tes, `thetas_history[k+1]`
   yang berkorespondensi dengan `items_answered[k]` — off-by-one yang
   gampang keliru dipakai konsumen API.
2. **Judul fase ini secara eksplisit "Hasil BERSIH Akhir"** -- baris
   pra-tes (theta nol/SE prior, sebelum ada informasi apa pun) bukan
   bagian dari "riwayat estimasi", melainkan titik AWAL sebelum estimasi
   ada. Konsumen yang butuh titik awal itu tetap bisa membangunnya sendiri
   dari `initial_theta::initialize()` (Fase 1) -- informasi itu tidak
   hilang, cuma tidak digandakan ke dalam array riwayat.

Fixture `tests/fixtures/final_result.json` menyimpan riwayat dengan
baris pra-tes (5 baris untuk 4 item terjawab);
`tests/integration_final_result.rs` membuang baris pertama itu sebelum
membandingkan, didokumentasikan eksplisit di komentar test.

## 4. `classification` untuk mode `Sprt`: selalu panjang `nfact`

`StopDecision::classification` (Fase 4) panjangnya BERVARIASI sesuai
`ClassifyType`: kosong (`None`), `nfact` (`Ci`, per-dimensi), atau `1`
(`Sprt` -- satu keputusan gabungan, lih.
`docs/teori/04-stopping-rule-sprt.md` §2).

`McatResult::classification` (spesifikasi desain fase ini: vektor
panjang `nfact`, bukan satu keputusan global) SELALU panjang `nfact`,
supaya konsumen hasil akhir mendapat bentuk yang seragam apa pun mode
klasifikasinya:

- `None` (kosong) -> `vec![NoDecision; nfact]`.
- `Ci` (sudah `nfact`) -> diteruskan apa adanya.
- `Sprt` (panjang 1) -> *broadcast* ke seluruh dimensi. SPRT inheren SATU
  uji hipotesis gabungan (Wald, 1945) atas seluruh vektor `theta`
  sekaligus, bukan `nfact` uji independen -- keputusan yang sama secara
  substantif berlaku untuk tiap dimensi (tidak ada informasi per-dimensi
  yang hilang dengan broadcasting, karena SPRT memang tidak pernah
  menghitung keputusan per-dimensi terpisah).

`normalize_classification()` (`src/result.rs`) mengimplementasikan
normalisasi ini, diuji lewat ketiga skenario di
`result::tests::classification_*`.

## Referensi

- Segall, D. O. (1996). Multidimensional adaptive testing. *Psychometrika*,
  61(2), 331–354.
- Wald, A. (1945). Sequential tests of statistical hypotheses. *Annals of
  Mathematical Statistics*, 16(2), 117–186. (dasar SPRT, relevan untuk
  keputusan `classification` gabungan §4)
