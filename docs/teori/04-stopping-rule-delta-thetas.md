# 04 — `delta_thetas`: Stopping Rule Berbasis Stabilitas `theta`

> Landasan teori untuk `src/stopping_rule/delta_theta.rs`. **Dievaluasi
> independen** dari `StoppingConfig::classify` (`None`/`Ci`/`Sprt`) — lih.
> §2.

## 1. Rumus

```
met_i = |theta_i - previous_theta_i| < delta_thetas_i
stop  = all(met_i)
```

`previous_theta` adalah estimasi `theta` pada langkah SEBELUM langkah
terakhir (bukan riwayat penuh — hanya dua titik terakhir yang relevan).
Berhenti hanya bila **SEMUA** dimensi stabil bersamaan (`all()`).

`delta_theta::evaluate()` menerima `previous: Option<&ThetaVector>` —
`None` (belum ada estimasi ulang sebelumnya, mis. sebelum item pertama
di-re-estimasi) menghasilkan seluruh dimensi `false` (belum bisa
dievaluasi, bukan `true` "kebetulan" sama dengan diri sendiri).
`PersonState::previous_theta()` (field baru Fase 4) menyimpan nilai ini
otomatis setiap kali `set_theta`/`set_estimate` dipanggil.

## 2. Kenapa independen dari mode klasifikasi

Berbeda dari `min_sem` (yang HANYA aktif saat `classify == None`) atau
`Ci`/`Sprt` (mode klasifikasi eksplisit), `delta_thetas` selalu dievaluasi
begitu `n_answered >= min_items`, TANPA gerbang mode klasifikasi.
Konsekuensinya: sesi dengan mode `Ci` AKTIF pun tetap bisa berhenti lewat
`delta_thetas` semata (bila `theta` sudah stabil tapi belum ada dimensi yang
cukup yakin terklasifikasi) — dua jalur berhenti independen berjalan paralel
setiap langkah.

## 3. Default nonaktif

`StoppingConfig::delta_thetas` default `vec![0.0]` (di-broadcast ke
seluruh `nfact` dimensi) — karena `|Δtheta| < 0` mustahil terpenuhi (nilai
absolut selalu `>= 0`), kriteria ini efektif **nonaktif** kecuali pengguna
eksplisit mengatur ambang positif.

## Referensi

- van der Linden, W. J., & Glas, C. A. W. (Eds.) (2010). *Elements of
  Adaptive Testing*, bab stopping rule multidimensional. Springer.
