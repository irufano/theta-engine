# 00 — Fondasi Model IRT Multidimensional

> Landasan teori untuk `src/theta.rs`, `src/irt/`, dan `src/bank.rs` (Fase 0).

## 1. Vektor kemampuan `theta` dan struktur data

Pada IRT multidimensional, kemampuan peserta bukan skalar tunggal
melainkan vektor `theta = (theta_1, ..., theta_m)` di ruang laten `m` dimensi
(`nfact = m`). Ketidakpastian estimasi direpresentasikan sebagai matriks
kovarians `nfact × nfact`; standard error (SE) per-dimensi yang biasa
dilaporkan adalah akar diagonal matriks tersebut — **bukan** seluruh
matriks, karena off-diagonal menyatakan kovariansi antar-dimensi, bukan
presisi satu dimensi (Reckase, 2009, bab 3).

`theta-engine` merepresentasikan ini sebagai tiga newtype tipis di atas
`nalgebra`:

- `ThetaVector` — vektor `theta`, panjang `nfact`.
- `SEVector` — vektor SE per-dimensi, panjang `nfact`.
- `InfoMatrix` — matriks persegi `nfact × nfact`, dipakai baik untuk
  matriks informasi Fisher maupun matriks kovarians populasi (`gcov`).

## 2. Model item dikotomus multidimensional (M2PL/M3PL/M4PL)

Untuk item dengan dua kategori respons (`0`/`1`), probabilitas menjawab
benar pada `theta` adalah:

```
z(theta)     = d + a_1*theta_1 + ... + a_m*theta_m
P*(theta)    = 1 / (1 + exp(-z(theta)))                  (kurva logistik "murni")
P(u=1|theta) = g + (u - g) * P*(theta)
P(u=0|theta) = 1 - P(u=1|theta)
```

di mana `a = (a_1, ..., a_m)` adalah vektor **slope/discrimination**
per-dimensi, `d` adalah **intercept** (terkait kesulitan), `g` adalah
**lower asymptote/guessing**, dan `u` adalah **upper asymptote**. Model ini
mencakup tiga kasus khusus lewat parameterisasi yang sama:

| Model | `g` | `u` |
|---|---|---|
| M2PL | `0` | `1` |
| M3PL | bebas `[0,1)` | `1` |
| M4PL | bebas `[0,1)` | bebas `(g,1]` |

Bentuk kompensatori multidimensional ini mengikuti Reckase (2009, bab 4),
sedangkan upper asymptote `u` (model 4PL) berasal dari Barton & Lord (1981).

`theta-engine` juga menyediakan **arah kesulitan multidimensional** lewat
`mdisc()` dan `mdiff()`: `MDISC = ||a||` (norma Euclidean vektor slope)
menyatakan daya beda gabungan lintas dimensi, dan `MDIFF = -d / MDISC`
menyatakan kesulitan pada arah slope tersebut (Reckase, 1985; 2009).

- `Dichotomous::mdisc()` / `mdiff()` mengembalikan satu nilai. `g` dan `u`
  tidak ikut dihitung: MDISC/MDIFF didefinisikan dari slope dan intercept
  saja (Reckase, 1985).
- `GradedResponse::mdisc()` mengembalikan satu nilai, sedangkan `mdiff()`
  mengembalikan satu nilai per boundary (`-d_k / MDISC`, searah `d`).
- Bila semua slope nol, `MDISC = 0` dan `MDIFF = 0` (bukan `NaN`/`Inf`):
  tanpa slope tidak ada arah kesulitan. Ini sama dengan perilaku
  `mdiff()` di frontend, yang memakai nilai engine sebagai rujukan.

## 3. Graded response model (GRM) multidimensional

Untuk item politomus terurut (`K` kategori, `K > 2`), GRM mendefinisikan
`K - 1` **kurva batas kumulatif** (boundary curves):

```
Pstar_0(theta)     = 1
Pstar_k(theta)     = 1 / (1 + exp(-(d_k + a·theta)))   untuk k = 1..(K-1)
Pstar_K(theta)     = 0
P_k(theta)         = Pstar_{k-1}(theta) - Pstar_k(theta)   untuk k = 1..K
```

`Pstar_k` diinterpretasikan sebagai `P(respons >= kategori k+1 | theta)`.
Supaya `P_k >= 0` untuk semua `k`, boundary `d_1 > d_2 > ... > d_{K-1}`
**harus** terurut menurun ketat — `theta-engine` memvalidasi ini saat
konstruksi (`GradedResponse::new`, error `McatError::InvalidParameter`
bila tidak terurut), alih-alih diam-diam menghasilkan probabilitas nol
untuk parameter tak-valid — sesuai prinsip desain engine: validasi ketat &
error eksplisit.

Fungsi logistik dihitung dengan bentuk stabil yang dipecah menurut tanda
logit (`1 / (1 + exp(-z))` untuk `z >= 0`, `exp(z) / (1 + exp(z))` untuk
`z < 0`), sehingga `exp()` tidak pernah overflow dan logit **tidak** perlu
dipotong, berapa pun `theta`-nya. Selisih `P_k = Pstar_{k-1} - Pstar_k`
secara matematis tidak negatif, tetapi pembulatan floating point bisa
membuatnya `0` atau sedikit negatif di ekor kurva. Karena itu `P_k`
dilantai di bilangan `f64` positif terkecil (`f64::MIN_POSITIVE`, tanpa batas
atas karena batas atas tidak berpengaruh pada `f64`). Lantai ini hanya
menjaga `ln P_k` tetap finite untuk perhitungan likelihood, dan tidak
dipakai untuk hal lain. Bila kedua kurva berada di sisi atas
(`Pstar_k > 0.5`), selisih dihitung dari komplemennya
(`Qstar_k - Qstar_{k-1}` dengan `Qstar = 1 - Pstar`, identik secara
matematis) agar kategori kecil tidak hilang oleh pengurangan dua bilangan
yang dekat ke 1.

Informasi Fisher (pembagian oleh `P_k`) memakai aturan kontribusi nol: di
ekor kurva logistik gradien `dP_k/dtheta` berorde `P_k` itu sendiri, sehingga
`grad · grad^T / P_k` berorde `P_k` dan menuju `0` ketika `P_k -> 0`. Suku
sebuah kategori karena itu dianggap `0` (limit) bila gradiennya persis `0`
atau `P_k` tidak di atas lantai, alih-alih membagi nilai yang nol atau
hasil pembulatan. Aturan yang sama berlaku untuk model dikotomus saat `P`
atau `1 - P` persis `0` (mis. M2PL pada `|z|` sangat besar).

Rujukan: Muraki & Carlson (1995) untuk perluasan multidimensional GRM;
Samejima untuk formulasi boundary-curve aslinya (unidimensional).

## 4. Informasi Fisher multidimensional

Untuk item dengan kategori respons `1..K`, matriks informasi Fisher
`nfact × nfact` pada `theta` didefinisikan sebagai jumlah "outer product of
gradients" per kategori, dibagi probabilitas kategori tersebut:

```
I(theta) = sum_{k=1}^{K} [ grad(P_k)(theta) · grad(P_k)(theta)^T ] / P_k(theta)
```

Ini adalah bentuk umum informasi Fisher untuk keluarga eksponensial, dan
untuk M2PL/M3PL/M4PL
mereduksi ke bentuk tertutup yang dikenal (Reckase, 2009, bab 4):

```
I_2PL(theta) = P(theta) * (1 - P(theta)) * a * a^T
```

dengan `P(theta) = P(u=1|theta)`. Untuk M3PL/M4PL, faktor skalarnya
berubah menjadi `((u-g) * P*(theta) * (1-P*(theta)))^2 / (P(theta)(1-P(theta)))`,
tapi struktur `a * a^T`-nya sama. `theta-engine` menguji kedua bentuk
tertutup ini sebagai *sanity check* unit test (kasus 1 dimensi) sebelum
diverifikasi lintas-dimensi lewat fixture (lih. §5).

Matriks informasi inilah yang menjadi dasar seluruh kriteria seleksi item
D/T/A/E/W-rule di Fase 2 (`DRule` = determinan, `TRule` = trace, `ARule` =
negatif trace invers, `ERule` = eigenvalue minimum, `WRule` = bentuk
kuadratik berbobot) — lih. `InfoMatrix::{determinant, trace, trace_weighted,
inverse, min_eigenvalue}` di `src/theta.rs`.

## 5. Validasi numerik

`theta-engine` adalah implementasi langsung dari formula di atas.
Kesesuaian numeriknya diuji `tests/integration_irt.rs` terhadap fixture
`tests/fixtures/irt_probability_information.json`: probabilitas kategori,
matriks informasi Fisher, MDISC, dan MDIFF untuk item M2PL/M3PL 2D–3D dan
graded response 2D di beberapa titik `theta`, dengan toleransi `1e-6`.
Nilai acuan fixture dihitung di luar engine, bukan dari kode Rust;
`tests/fixtures/compute_expected_fixture.py` menghitung ulang nilai yang
sama langsung dari rumus §2–§4 sebagai cek silang independen.

## Referensi

- Reckase, M. D. (2009). *Multidimensional Item Response Theory*. Springer.
- Reckase, M. D. (1985). The difficulty of test items that measure more
  than one ability. *Applied Psychological Measurement*, 9(4), 401–412.
- Barton, M. A., & Lord, F. M. (1981). *An upper asymptote for the
  three-parameter logistic item-response model* (ETS Research Report
  RR-81-20). Educational Testing Service.
- Muraki, E., & Carlson, J. E. (1995). Full-information factor analysis for
  polytomous item responses. *Applied Psychological Measurement*, 19(1),
  73–90.
