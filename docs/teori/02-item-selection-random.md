# 02 — `Random`: Baseline Exposure-Neutral

> Landasan teori untuk `src/item_selection/criteria/random.rs` (Fase 2).

## Definisi

`Random` memilih satu item secara **uniform random** di antara seluruh
kandidat (item belum dijawab) — setiap kandidat punya peluang terpilih
yang sama, tidak melibatkan model IRT atau matriks informasi sama sekali.

## Kenapa dibutuhkan

Kriteria adaptif berbasis informasi (D/T/A/E/W-rule, KL) secara sistematis
lebih sering memilih item dengan parameter diskriminasi tinggi — kalau
tidak dikendalikan, ini menyebabkan **item overexposure** (item populer
dipakai jauh lebih sering dari item lain, risiko keamanan bank soal) dan
**underexposure** item lain (nyaris tidak pernah dipakai, investasi
pengembangan item terbuang). `Random` adalah kasus ekstrem sebaliknya:
exposure benar-benar merata di antara seluruh bank, dengan mengorbankan
seluruh presisi adaptif.

Dalam praktik, `Random` jarang dipakai murni sebagai kriteria produksi —
lebih sering sebagai:

- **Baseline** exposure-neutral untuk membandingkan tingkat overexposure
  kriteria adaptif lain.
- **Komponen** dalam skema exposure control yang lebih canggih (mis.
  *randomesque*: pilih uniform random di antara *beberapa item terbaik*,
  bukan seluruh bank — lih. `docs/teori/05-exposure-control-randomesque.md`,
  Fase 5).

## Kenapa valid untuk `nfact > 1`

Sama seperti `Seq`: tidak melibatkan model IRT, validitasnya untuk
multidimensional trivial.

## Referensi

- Georgiadou, E., Triantafillou, E., & Economides, A. (2007). A review of
  item exposure control strategies for computerized adaptive testing
  developed on the item response theory framework. *International
  Journal on E-Learning*, 6(1).
