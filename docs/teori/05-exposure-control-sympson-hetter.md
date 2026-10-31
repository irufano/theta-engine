# 05 — Sympson–Hetter: Seleksi Terima/Tolak Per-Item

> Landasan teori untuk `src/exposure_control/sympson_hetter.rs`.

## 1. Definisi metode

Misalkan `k_i = exposure[i]` adalah parameter eksposur item `i`,
`k_i ∈ [0, 1]`, diindeks per **ID item** (bukan posisi — lih.
`05-exposure-control-overview.md` §3). Per langkah, kandidat berskor
diurutkan **sekali** menurut skor kriteria, dari yang terbaik (tie: indeks
item yang lebih kecil lebih dulu, supaya hasil deterministik). Daftar
berperingkat itu lalu ditelusuri dari atas. Untuk tiap kandidat `i`
digambar `u ~ Uniform[0, 1)`, dan kandidat **diterima bila dan hanya bila
`u < k_i`**. Kandidat pertama yang diterima disajikan.

Karena `P(u < x) = x` untuk `u` kontinu seragam,

```
P(administrasi i | i sedang diuji) = k_i
```

— inilah mekanisme inti Sympson & Hetter (1985; lih. juga van der Linden
& Glas, 2010, bab kontrol eksposur): `k_i` BUKAN batas kuota keras,
melainkan **peluang lolos setiap kali item itu menjadi kandidat yang
diuji**. `k_i = 1` selalu diterima; `k_i = 0` tidak pernah (`u < 0`
mustahil). Untuk daftar berperingkat `i_1, i_2, …, i_m`, peluang item
peringkat `j` disajikan lewat draw adalah

```
P_j = k_{i_j} · Π_{l < j} (1 − k_{i_l})
```

dan peluang semua kandidat ditolak adalah `Π_{l ≤ m} (1 − k_{i_l})`.

## 2. Aturan penyisihan dan fallback semua-ditolak

1. **Kandidat yang ditolak disisihkan** untuk peserta ini: ditandai
   `valid_items[item] = false` (invalid untuk SISA sesi peserta, lih. §3),
   lalu penelusuran lanjut ke kandidat peringkat berikutnya.
2. **Bila SEMUA kandidat ditolak** (peluangnya `Π(1 − k)`), engine
   menyajikan kandidat berperingkat teratas (skor tertinggi), melaporkan
   `Outcome::forced_accept = true`, dan **tidak** menandainya invalid —
   item itu tetap boleh muncul lagi di langkah-langkah berikutnya. Kandidat
   lain yang sudah ditolak pada panggilan itu tetap ditandai invalid.
   Tidak ada bilangan acak yang digambar untuk pilihan paksa ini.

Fallback pada butir 2 adalah **keputusan desain engine ini**, bukan bagian
definisi baku Sympson–Hetter. Alasannya: (a) tes harus selalu bisa
berlanjut — tidak boleh macet atau berakhir error hanya karena seluruh
`k_i` kandidat kebetulan kecil; (b) di antara semua pilihan paksa yang
mungkin, item paling informatif adalah yang paling kecil kerugiannya bagi
ketepatan pengukuran, sehingga pilihan paksa jatuh pada peringkat teratas.
Konsekuensinya, peluang peringkat teratas sedikit di atas `k` miliknya
sendiri (`P_1 + Π(1 − k)`), selisih yang kecil kecuali seluruh `k` rendah.

`sympson_hetter::select()` (Rust) menjalankan aturan ini apa adanya: satu
entri `attempts` per kandidat yang di-draw (urut peringkat); pada fallback
semua entri `false`, `chosen` sama dengan item entri pertama, dan
`Outcome::forced_accept` menandai kasus itu.

## 3. Persistensi `valid_items` — bukan sekadar satu langkah

Detail non-obvious: penandaan `valid_items[item] = false` di satu langkah
**tetap berlaku** pada langkah-langkah berikutnya untuk peserta yang sama —
item yang ditolak sekali TIDAK PERNAH dicoba lagi sepanjang SISA sesi
peserta itu, bukan cuma "dilewati untuk giliran ini". Tanpa persistensi,
item yang ditolak bisa langsung jadi kandidat terbaik lagi di langkah
berikutnya dan mendapat kesempatan draw ulang, sehingga peluang lolos
efektifnya lebih besar dari `exposure[item]`.

`select_with_exposure()` menerapkan ini lewat parameter eksplisit
`valid_items: &mut [bool]` yang WAJIB diteruskan pemanggil sebagai slice
yang SAMA di setiap langkah sesi peserta yang sama (lih.
`05-exposure-control-overview.md` §6 untuk alasan ini bukan field
`PersonState`).

## 4. Properti matematis: `exposure=0.0` tidak pernah lolos lewat draw

Konsekuensi langsung §1: bila `exposure[item] = 0.0`, syarat `u < 0.0`
tidak pernah benar untuk `u ∈ [0, 1)`. Ini properti **EXACT**, bukan
"sangat jarang" — dibuktikan lewat simulasi 100+ sesi di
`tests/integration_exposure_control.rs::sh_simulation_zero_exposure_item_never_selected_and_start_item_always_selected`
(item exposure=0.0 di fixture `sh_simulation`: 0 dari 1800 penyajian
selama 300 peserta × 6 item; Rust: 0 dari simulasi independennya
sendiri).

Satu pengecualian: fallback §2 butir 2 — bila item exposure=0.0 itu
kebetulan berperingkat teratas dan SEMUA kandidat ditolak, ia tetap
disajikan. Ini bukan kontradiksi: pilihan paksa adalah keputusan "tidak
ada pilihan lain", bukan celah pada uji `u < exposure[item]` itu sendiri.

## 5. Item pertama (start item) sepenuhnya bypass exposure control

Bila sesi memakai item pertama yang ditetapkan (start item), item itu
disajikan langsung oleh pemanggil sebelum ada item dijawab, tanpa melewati
kriteria seleksi maupun `select_with_exposure()`. Praktis: start item akan
selalu terpilih di langkah pertama SETIAP sesi, terlepas dari nilai
`exposure[start_item]` — properti yang juga diverifikasi test simulasi di
§4.

## Referensi

- van der Linden, W. J., & Glas, C. A. W. (Eds.) (2010). *Elements of
  Adaptive Testing*, bab kontrol eksposur item. Springer.
- Sympson, J. B., & Hetter, R. D. (1985). Controlling item-exposure rates
  in computerized adaptive testing. *Proceedings of the 27th Annual
  Meeting of the Military Testing Association*, 973–977.
- Georgiadou, E., Triantafillou, E., & Economides, A. (2007). A review of
  item exposure control strategies for computerized adaptive testing
  developed on the item response theory framework. *International
  Journal on E-Learning*, 6(1).
