# 04 — `max_items`/`max_time`: Batas Keras Independen

> Landasan teori untuk `src/stopping_rule/max_items_time.rs`. Lih.
> `docs/teori/04-stopping-rule-overview.md` §3-4 untuk konteks & motivasi
> lintas-mekanisme.

## 1. Rumus

```
stop = n_answered >= max_items
stop = total_time() >= max_time        -- hanya bila max_time = Some(..)
```

Kedua kondisi ini dicek **tanpa syarat** `n_answered >= min_items` —
berbeda dari kriteria presisi (`min_sem`/`delta_thetas`/klasifikasi, lih.
`docs/teori/04-stopping-rule-overview.md` §2) yang hanya dievaluasi setelah
`min_items` terpenuhi. `max_items_time::max_items_reached()`/
`max_time_reached()` menerapkan kedua kondisi ini sebagai fungsi murni
terpisah, dipanggil `stopping_rule::evaluate()` di luar gerbang
`min_items`.

`max_items_time::max_items_reached()` memakai `>=` (bukan `==`) — hasil
identik untuk penggunaan wajar (jumlah item terjawab tidak pernah
melompati `max_items` dalam loop administrasi normal), tapi lebih robust
terhadap pemanggil yang memeriksa kondisi ini setelah `n_answered` sudah
melampaui `max_items` karena alasan lain.

## 2. `max_items`: bukan cuma jaring pengaman

`StopDecision::terminated_early == false` saat berhenti lewat `max_items`
(beda dari berhenti lewat presisi, yang `terminated_early == true`) —
distingsi ini penting secara operasional: laporan hasil tes (Fase 7) bisa
membedakan "berhenti karena YAKIN" vs "berhenti karena KEHABISAN item",
yang punya implikasi berbeda untuk interpretasi presisi hasil akhir.

Lih. `docs/teori/04-stopping-rule-overview.md` §3 untuk kenapa `max_items`
sering jadi kondisi PRAKTIS (bukan cuma teoretis) pada model `nfact` tinggi.

## 3. `max_time`: satuan & akumulasi

`PersonState::item_times: Vec<Duration>` (field baru Fase 4) menyimpan
waktu per-item, dicatat lewat `record_item_time()`; `total_time()`
menjumlahkannya untuk dibandingkan dengan `max_time`. Tidak pernah memanggil
`record_item_time()` membuat `total_time()` selalu nol, artinya `max_time`
efektif tidak pernah tercapai — konsisten dengan default
`max_time = None` (tanpa batas) saat fitur pengukuran waktu tidak dipakai
sama sekali (mis. simulasi tanpa antarmuka peserta).

## Referensi

- Weiss, D. J., & Kingsbury, G. G. (1984). Application of computerized
  adaptive testing to educational problems. *Journal of Educational
  Measurement*, 21(4), 361–375.
