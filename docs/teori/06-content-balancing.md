# 06 — Content Balancing

> Landasan teori untuk `src/content_balancing/` (Fase 6).

## 1. Tujuan dan posisi di pipeline

Content balancing menjaga proporsi kategori konten item yang disajikan
(mis. "Aljabar" vs "Geometri") mendekati target yang ditentukan pengguna,
beroperasi **sebelum** exposure control (Fase 5) dalam pipeline seleksi
item. Pendekatannya mengikuti Kingsbury & Zara (1989): di tiap langkah,
batasi kandidat ke kategori yang paling tertinggal dari targetnya.
Veldkamp & van der Linden (2002) membahas constraint konten serupa untuk
CAT multidimensional.

## 2. Target proporsi WAJIB menjumlah persis 1

```
|sum(target_prop) - 1| <= 1e-6     -- selain itu: error
```

Target proporsi tidak fleksibel: `ContentBalancingConfig::new`
(`src/content_balancing/mod.rs`) menolak target yang tidak menjumlah `1.0`
(toleransi `1e-6`), juga target bernilai negatif. Validasi ini atas
**seluruh** target yang diberikan pemanggil, termasuk target untuk
kategori yang TIDAK muncul di bank — supaya target selalu bisa dibaca
sebagai distribusi proporsi yang utuh, bukan bobot relatif yang maknanya
bergantung pada isi bank. Sebaliknya, setiap kategori yang muncul di
`category_per_item` wajib punya entri target.

## 3. Kategori terurut alfabetis, proporsi empiris mulai dari nol

`ContentBalancingConfig` menyusun `category_order`: himpunan kategori dari
`category_per_item`, dideduplikasi dan terurut alfabetis. Proporsi empiris
dihitung atas SEMUA kategori di `category_order`:

```
empirical[c] = (jumlah item terjawab berkategori c) / n_answered
empirical[c] = 0                     bila n_answered = 0 (bukan 0/0)
```

Kategori yang belum pernah terjawab tetap terhitung `0.0` sepanjang sesi
belum ada item terjawab sama sekali di kategori itu, bukan hilang dari
perhitungan. `narrow_candidates()` (`src/content_balancing/mod.rs`)
selalu memakai `category_order` sebagai basis perhitungan proporsi
empiris, bukan kategori yang benar-benar muncul di `answered_items` saja.

## 4. Pemilihan kategori prioritas

```
deficit[c] = target[c] - empirical[c]
prioritas  = argmax_c deficit[c]          -- tie: pertama di category_order
pool_baru  = kandidat berkategori prioritas
bila pool_baru kosong: pool_baru = pool penuh (fallback)
```

Kategori dengan `target - empirical` **terbesar** (paling kurang jatah)
jadi prioritas. Pada seri, kategori yang muncul **pertama** di
`category_order` (alfabetis, lih. §3) menang — tie-break yang sama dengan
pola `argmax` yang sudah ada di `item_selection::argmax`
(`src/item_selection/mod.rs`, `>=` menjaga entri pertama pada seri).

Bila kategori prioritas TIDAK punya kandidat tersisa, pool kandidat tetap
UTUH, bukan kosong atau error. Ini fallback ke pool penuh (`fallback_used`
di `ContentBalancingDebug`): sesi tetap bisa berjalan walau satu kategori
sudah habis, dengan konsekuensi proporsi akhir menyimpang dari target.

## 5. Hanya `Criterion::Seq` yang dilarang

`Seq` menyajikan item dalam urutan index yang tetap — mempersempit pool
berdasar kategori akan mengubah urutan itu, bertentangan dengan arti
kriteria non-adaptif. `Criterion::Random` justru **didukung**: content
balancing berlaku sama seperti kriteria berskor, "sempitkan ke kategori
prioritas, fallback ke pool penuh bila kategori itu habis", lalu pilih
uniform random dari pool hasil sempitan.
`ContentBalancingConfig::validate_with_criterion()`
(`src/content_balancing/mod.rs`) menerapkan larangan yang tepat: hanya
`Criterion::Seq` yang ditolak
(`McatError::ContentBalancingIncompatibleWithSeq`).

## 6. Urutan kerja & keputusan desain `narrow_candidates`

Secara konseptual, untuk kriteria berskor: skor SELURUH kandidat belum
dijawab dihitung dulu lewat fungsi kriteria (Fase 2), BARU disempitkan
berdasar kategori, BARU exposure control (Fase 5) jalan di atas hasil
sempitan.

Karena skor tiap kandidat (`DRule`/`TRule`/dst.) dihitung independen
terhadap kandidat lain, menyempitkan pool `ItemIndex` **sebelum** memanggil
`item_selection::rank_candidates` menghasilkan nilai numerik identik dengan
menyempitkan **sesudah** ranking — dibuktikan lewat replay fixture
`tests/fixtures/content_balancing.json` di
`tests/integration_content_balancing.rs` (kecocokan EKSAK item demi item,
bukan cuma keanggotaan set: skenario fixture deterministik penuh, kriteria
`DRule` tanpa exposure control, tie-break "pertama" konsisten di kedua
sisi). `content_balancing::narrow_candidates()` karena itu menerima
`candidate_pool: &[ItemIndex]` MENTAH (belum diranking), bukan
`candidate_scores` hasil `rank_candidates` — skor hanya dihitung untuk
kandidat yang lolos penyempitan.

`ContentBalancingConfig` sengaja tetap **decoupled dari `ItemBank`** (pola
sama seperti `exposure_control::ExposureControlConfig`, yang menyimpan
`Vec<f64>` sendiri, bukan bergantung ke `ItemBank`) — `category_per_item`
dikonstruksi eksplisit oleh pemanggil. `ItemMeta.content: Option<String>`
(`src/bank.rs`) sudah disiapkan sebelumnya untuk fase ini (komentar
eksplisit "dipakai content balancing, Fase 6"); orkestrator sesi eksternal
adalah titik yang tepat untuk mengisi `category_per_item` dari
`bank.meta(i).content`, menjaga `content_balancing` tetap bisa diuji tanpa
`ItemBank` sungguhan.

## Peringatan interaksi dengan exposure control (Fase 5)

Bila kombinasi filter content + exposure control sangat ketat, kandidat
tersisa bisa menyusut drastis di beberapa langkah — `select_with_exposure`
(Fase 5) mengembalikan `Err` eksplisit bila `candidate_scores` kosong,
bukan panic. `narrow_candidates()` sendiri tidak pernah menghasilkan pool
kosong (fallback ke pool penuh, §4) — risiko kehabisan kandidat murni
tanggung jawab exposure control di langkah setelahnya.

## Referensi

- Kingsbury, G. G., & Zara, A. R. (1989). Procedures for selecting items
  for computerized adaptive tests. *Applied Measurement in Education*,
  2(4), 359–375.
- Veldkamp, B. P., & van der Linden, W. J. (2002). Multidimensional
  adaptive testing with constraints on test content. *Psychometrika*,
  67(4), 575–588.
