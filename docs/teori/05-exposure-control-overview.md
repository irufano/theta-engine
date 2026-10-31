# 05 — Exposure Control: Overview

> Landasan teori untuk `src/exposure_control/` (Fase 5). Lih. dokumen per
> mode untuk rumus & referensi rinci tiap mekanisme.

## 1. Mode yang didukung

Exposure control membatasi seberapa sering item terbaik menurut kriteria
seleksi benar-benar disajikan, supaya item populer tidak overexposed
(Georgiadou dkk., 2007). Dua mekanisme klasik didukung, ditambah keadaan
tanpa pembatasan:

- **Tanpa exposure control** (default) — selalu ambil kandidat berskor
  tertinggi. Di Rust, ini direpresentasikan dengan **tidak memanggil**
  `exposure_control::select_with_exposure()` sama sekali (bukan varian enum
  `ExposureMode::None`) — pola yang sama seperti `Criterion` Fase 2
  (ketiadaan pemanggilan, bukan varian khusus, untuk "fitur nonaktif").
- **`ExposureMode::SympsonHetter`** (Sympson & Hetter, 1985) — lih.
  `05-exposure-control-sympson-hetter.md`.
- **`ExposureMode::Randomesque`** (McBride & Martin, 1983) — lih.
  `05-exposure-control-randomesque.md`.

## 2. Deteksi mode otomatis (`ExposureMode::from_values`)

`ExposureMode::from_values()` (`src/exposure_control/mod.rs`) menentukan
mode dari satu vektor nilai `exposure`:

```
semua nilai di [0, 1]  -> SympsonHetter(nilai apa adanya)
selain itu             -> Randomesque, dengan syarat SETIAP nilai v memenuhi
                          trunc(v) == v dan trunc(v) >= 1; bila tidak -> error
```

Poin non-obvious: begitu **satu saja** elemen vektor `> 1`, SELURUH vektor
beralih ke mode `Randomesque` — termasuk elemen yang aslinya `<= 1`. Elemen
itu lalu wajib bilangan bulat `>= 1` juga (praktis berarti harus persis
`1`, karena integer `>=1` dan `<=1` sekaligus hanya `1`). Pengecekan memakai
`f64::trunc()` (pemotongan ke arah nol, bukan pembulatan) — `2.9` jadi `2`,
lalu gagal validasi `2 == 2.9`. Vektor yang seluruhnya `1` jatuh ke
`SympsonHetter` (semua nilai di `[0, 1]`), dan hasilnya setara: peluang
lolos 1 untuk semua item sama dengan tanpa pembatasan.

Fixture `tests/fixtures/exposure_control.json` (bagian `mode_detection`)
memuat beberapa vektor contoh beserta mode dan nilai yang diharapkan;
`tests/integration_exposure_control.rs` membandingkannya EKSAK
(deterministik, tanpa RNG sama sekali, beda dari dua skenario lain di
fixture yang sama).

## 3. Pengindeksan berbeda antar mode — titik paling mudah salah

- **`SympsonHetter`**: `exposure[item]` — diindeks per **ID item**.
  "Berapa peluang item spesifik ini boleh lolos, di posisi manapun dalam
  tes?"
- **`Randomesque`**: `exposure[position_in_test]` — diindeks per **posisi
  penyajian** (`position_in_test` = jumlah item terjawab, berbasis nol:
  elemen ke-`k` dipakai saat `k` item sudah terjawab). "Untuk giliran ke-n,
  ambil acak dari berapa kandidat teratas?"

Keduanya sama-sama vektor sepanjang `n_items`, tapi maknanya sama sekali
berbeda — menukar keduanya adalah bug diam-diam (tidak ada error tipe yang
akan menangkapnya, karena keduanya `Vec<f64>`/`Vec<u32>` panjang sama).

## 4. `Seq`/`Random` tidak pernah masuk exposure control

Kriteria `Seq` dan `Random` tidak menghasilkan skor kandidat: keduanya
memilih item langsung (index terkecil / uniform random), sebelum ada
ranking apa pun. Exposure control bekerja di atas ranking skor dari
kriteria berbasis skor (`Kl`/`KlN`/`DRule`/`TRule`/`ARule`/`ERule`/`WRule`,
dengan maupun tanpa prior), jadi tidak punya makna untuk kedua kriteria
itu.

[`crate::item_selection::rank_candidates`] (fungsi baru Fase 5, diekstrak
dari `select_next_item` Fase 2 — lih. §5) karenanya mengembalikan `Err`
untuk `Criterion::Seq`/`Criterion::Random`, bukan `Vec` kosong. Ini
memaksa pemanggil menyadari exposure control tidak berlaku untuk kedua
kriteria itu, alih-alih diam-diam menerima input yang tidak bermakna.

## 5. Reuse: skor kriteria dari Fase 2, bukan duplikasi

`item_selection::rank_candidates()` (diekstrak dari `select_next_item`,
perilaku & nilai numerik identik, sudah dibuktikan lewat suite test Fase 2
yang tetap hijau setelah refactor) adalah SATU-SATUNYA sumber skor kandidat
yang dikonsumsi `select_with_exposure()`. Sesuai prinsip desain engine
(jangan duplikasi), exposure control murni mekanisme *di atas* ranking
yang sudah ada, tidak mengimplementasikan ulang rumus kriteria apa pun.

## 6. Status "item tidak valid untuk sesi ini" — keputusan desain

Mode `SympsonHetter` menandai item yang ditolak sebagai tidak valid, dan
status itu **persisten** sepanjang sisa sesi peserta itu (bukan cuma satu
langkah — lih. `05-exposure-control-sympson-hetter.md` §3).

`PersonState` (Fase 2, sudah selesai & teruji) **sengaja tidak** ditambahi
field ini. Mengikuti preseden Fase 4 (`allow_constrain_breaks` jadi
parameter `bool` yang diteruskan pemanggil, bukan field `PersonState`,
karena tanggung jawab lintas-fase yang belum ada orchestrator-nya):
`select_with_exposure()` menerima `valid_items: &mut [bool]` sebagai
parameter eksplisit, dimiliki & disiapkan pemanggil. Ini juga memudahkan
mekanisme lain yang perlu menandai item tidak valid untuk sisa sesi (mis.
constraint administrasi item yang belum diimplementasikan) berbagi
struktur yang sama tanpa desain ulang `PersonState` nanti.

## Peta dokumen per-mode

| Mode | Teori | Panduan |
|---|---|---|
| `SympsonHetter` | `05-exposure-control-sympson-hetter.md` | `05-exposure-control-sympson-hetter.md` |
| `Randomesque` | `05-exposure-control-randomesque.md` | `05-exposure-control-randomesque.md` |

## Referensi

- Sympson, J. B., & Hetter, R. D. (1985). Controlling item-exposure rates
  in computerized adaptive testing. *Proceedings of the 27th Annual
  Meeting of the Military Testing Association*, 973–977.
- McBride, J. R., & Martin, J. T. (1983). Reliability and validity of
  adaptive ability tests in a military setting. In *New Horizons in
  Testing* (pp. 223–236). Academic Press.
- Georgiadou, E., Triantafillou, E., & Economides, A. (2007). A review of
  item exposure control strategies for computerized adaptive testing
  developed on the item response theory framework. *International
  Journal on E-Learning*, 6(1).
