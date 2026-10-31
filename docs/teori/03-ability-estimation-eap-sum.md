# 03 — EAP Berbasis Summed Score (`EapSum`)

> Landasan teori untuk `Method::EapSum` (`src/ability_estimation/eap_sum.rs`).

## 1. Beda dari EAP biasa

EAP biasa (`docs/teori/03-ability-estimation-eap.md`) memakai likelihood
**pola respons lengkap** (kategori spesifik tiap item). `EapSum` hanya
peduli **jumlah skor total** (`sum_score = sum` kategori respons seluruh
item terjawab) — dua peserta dengan pola respons berbeda tapi skor total
sama akan mendapat estimasi `theta` yang **identik** di bawah `EapSum`.
Berguna untuk pelaporan berbasis skor mentah (mis. tabel konversi skor ->
kemampuan), atau saat urutan/identitas item spesifik dianggap tidak
relevan untuk estimasi akhir.

## 2. Rekursi Lord & Wingersky (1984)

Inti algoritma: hitung distribusi `P(total_score = s | theta)` untuk
**tiap** kemungkinan skor `s` (dari `0` sampai jumlah kategori maksimum
seluruh item terjawab), di **tiap** titik grid `theta_g` — lewat konvolusi
berurutan distribusi kategori tiap item:

```
dist_0 = [1]                              -- P(skor=0)=1 sebelum item manapun
dist_k = konvolusi(dist_{k-1}, P_item_k)  -- tambahkan kontribusi item ke-k

dist_k[s] = sum_{j} dist_{k-1}[s-j] * P_item_k(kategori=j)
```

Setelah seluruh item terjawab dikonvolusi, `dist_n[s] = P(total_score=s |
theta_g)` — inilah `score_lik[g][s]` di `eap_sum.rs`.

Kompleksitas: `O(n_items * max_score^2)` per titik grid — jauh lebih murah
dari menghitung likelihood tiap pola respons individual (`2^n_items` untuk
item dikotomus), karena distribusi skor dikonvolusi bertahap, bukan
di-enumerasi.

## 3. EAP per kategori skor

Setelah `score_lik[g][s]` didapat untuk seluruh grid `g`, EAP untuk kategori skor
`s` memakai formula **identik** dengan EAP biasa (§ `docs/teori/
03-ability-estimation-eap.md` §1), tapi `L(theta_g)` diganti `score_lik[g][s]`:

```
theta_hat(s) = sum_g theta_g * score_lik[g][s] * W(theta_g)
               -----------------------------------------
                    sum_g score_lik[g][s] * W(theta_g)
```

`W(theta_g)` = `dmvnorm(theta_g; gmeans, gcov)` (grid EAP standar, sama
seperti §2 dokumen EAP — `EapSum` selalu grid-based, tidak ada varian QMC
untuk method ini).

Karena grid & bobot sama untuk **semua** kategori skor, tabel skor->theta
lengkap (`0..=max_score`) dihitung sekaligus dalam satu panggilan
(`eap_sum::estimate`) — inilah alasan `ability_estimation::eap_sum_score_table()`
disediakan sebagai fungsi terpisah dari `estimate()` (yang hanya
mengembalikan baris relevan untuk skor peserta saat ini).

## 4. Keputusan desain: item terjawab saja, bukan seluruh bank

Item yang belum dijawab tidak punya skor, jadi tidak boleh ikut rekursi
Lord & Wingersky: menyertakannya sebagai "kategori tambahan" akan
menggeser distribusi skor total dan mengubah arti `s`. Konsekuensinya,
estimasi dari model berisi HANYA item terjawab sama persis dengan estimasi
dari seluruh bank yang mengabaikan item belum dijawab.

`ability_estimation::eap_sum::estimate()` karenanya beroperasi langsung
atas `person.answered_items()` (bukan seluruh `ItemBank` + penanda item
belum dijawab) — pilihan yang benar sekaligus natural untuk `PersonState`,
yang memang cuma melacak item yang sudah dijawab. Kasus `2d_partial_3of6`
di `tests/fixtures/ability_estimation.json` (3 dari 6 item terjawab)
menguji perilaku ini.

## Referensi

- Lord, F. M., & Wingersky, M. S. (1984). Comparison of IRT true-score and
  equipercentile observed-score "equatings". *Applied Psychological
  Measurement*, 8(4), 453–461.
- Thissen, D., Pommerich, M., Billeaud, K., & Williams, V. S. (1995). Item
  response theory for scores on tests including polytomous items with
  ordered responses. *Applied Psychological Measurement*, 19(1), 39–49.
