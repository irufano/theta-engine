# 03 — EAP (Expected A Posteriori)

> Landasan teori untuk `Method::Eap { qmc }` (`src/ability_estimation/eap.rs`).
> Lih. `docs/teori/03-ability-estimation-overview.md` untuk konteks
> integrasi dimensi tinggi & tabel `quadpts` default.

## 1. Rumus dasar (Bock & Aitkin, 1981)

EAP adalah rata-rata posterior `theta` — mean distribusi posterior
`p(theta | respons)`, didekati lewat kuadratur numerik atas titik `theta_g`
(`g = 1..N`):

```
L(theta_g)  = prod_i P_{k_i}(theta_g)          -- likelihood pola respons di titik g
              (i = item terjawab, k_i = kategori respons yang diobservasi)

W(theta_g)  = prior(theta_g)                    -- bobot titik g (lih. §2)

theta_hat   = sum_g theta_g * L(theta_g) * W(theta_g)
              -----------------------------------------
                   sum_g L(theta_g) * W(theta_g)

SE_j        = sqrt( sum_g (theta_g[j] - theta_hat[j])^2 * L(theta_g) * W(theta_g)
                     ----------------------------------------------------
                          sum_g L(theta_g) * W(theta_g) )
```

`SE` di sini hanya akar diagonal matriks kovarians posterior (varians per
dimensi); off-diagonal (kovarians antar-dimensi) dihitung tapi tidak
diekspos sebagai `SE` (sama seperti `SEVector::from_covariance_diagonal` di
Fase 0/1, lih. `docs/teori/00-fondasi-irt-multidimensional.md` §1).

Item yang **belum dijawab** tidak berkontribusi ke `L(theta_g)` — hanya
item di `person.answered_items()`/`person.responses()` yang ikut dihitung.

## 2. Dua mode integrasi: grid vs QMC

### Grid-based (`qmc: false`)

`theta_g` = cartesian product `quadpts` titik per dimensi pada `[-6, 6]`
(`quadrature::build_grid`). Bobot:

```
W(theta_g) = dmvnorm(theta_g; gmeans, gcov)
```

(densitas normal multivariat prior populasi, dinormalisasi implisit lewat
pembagian oleh `sum_g L*W` di rumus §1 — tidak perlu dinormalisasi eksplisit
ke jumlah 1 lebih dulu, konstanta skala apa pun pada `W` hilang di rasio).

### QMC-based (`qmc: true`)

Titik dibangun dari leaped-Halton sequence yang ditransformasi invers CDF
normal (`quadrature::build_qmc_points`, lih.
`docs/teori/03-ability-estimation-overview.md` §3 untuk detail algoritma)
menjadi awan kuasi-`N(0, I)` `z_g`. Awan itu lalu disesuaikan dengan prior
grup (`quadrature::transform_to_prior`):

```
Sigma = gcov = L L^T                 -- faktor Cholesky (L segitiga bawah)
theta_g = gmeans + L z_g             -- awan kuasi-N(gmeans, gcov)
W(theta_g) = 1                       -- bobot seragam
```

**Mengapa `mu + L z`.** Bila `z ~ N(0, I)`, maka `theta = mu + L z` adalah
transformasi linear dari `z`, sehingga tetap normal dengan rerata
`E[theta] = mu + L E[z] = mu` dan kovarians
`Cov[theta] = L Cov[z] L^T = L I L^T = Sigma`. Jadi `theta_g` adalah (kuasi-)
sampel dari prior grup itu sendiri.

**Mengapa bobotnya seragam.** Integral EAP adalah
`E_prior[theta * L(theta)] / E_prior[L(theta)]`. Bila titik sudah
disampel dari prior, kerapatan prior terkandung dalam sebaran titiknya, dan
rata-rata sampel dari `L(theta_g)` yang tidak dibobot sudah mendekati
`E_prior[L]`. Menimbang lagi dengan `dmvnorm(theta_g; gmeans, gcov)` akan
menghitung prior dua kali (importance sampling dengan proposal = prior
memiliki rasio prior/proposal sama dengan 1). Mean dan SE posterior kemudian
dihitung dari bobot `L(theta_g)` saja, sama seperti §1.

```
qmc: true   ->  theta_g = gmeans + L z_g (z_g kuasi-N(0,I))   W(theta_g) = 1
qmc: false  ->  theta_g = titik grid [-6, 6]^nfact            W(theta_g) = dmvnorm(theta_g; gmeans, gcov)
```

Pada `gmeans = 0` dan `gcov = I`, `L = I` sehingga titiknya tidak berubah
sama sekali. `gcov` yang tidak simetris atau tidak positif definit (Cholesky
tidak ada) menghasilkan `McatError::InvalidParameter`, bukan panic.

**Catatan.** Versi sebelumnya memakai awan kuasi-`N(0, I)` apa adanya untuk
semua grup, sehingga EAP QMC mengabaikan `gmeans`/`gcov`; untuk grup
non-standar hasilnya salah (bias bisa mencapai orde 1 satuan theta), sedangkan
mode grid benar. Perilaku itu adalah bug, bukan keputusan desain, dan sudah
diperbaiki: pada grup non-standar QMC kini sesuai grid dengan selisih orde
`1e-3` pada 20.000 titik (test `qmc_applies_group_means_and_covariance_like_grid_*`
di `eap.rs`).

**Keunggulan QMC.** QMC baru betul-betul unggul di `nfact` tinggi, di mana
grid dipaksa memakai `quadpts` sangat kecil (tabel overview §2) sementara QMC
bisa memakai jumlah titik total berapa pun (tidak terikat `quadpts^nfact`).

## 3. Kenapa dua algoritma satu modul

Keputusan desain engine (paralel dengan `Kl`/`KlN` di Fase 2): grid
& QMC adalah **satu algoritma** (EAP) dengan **mode integrasi** berbeda,
bukan dua algoritma estimasi yang beda secara konsep — keduanya
mengestimasi rumus posterior mean yang identik (§1), hanya beda cara
mendekati integralnya secara numerik.

## Referensi

- Bock, R. D., & Aitkin, M. (1981). Marginal maximum likelihood estimation
  of item parameters: Application of an EM algorithm. *Psychometrika*,
  46(4), 443–459.
- Bock, R. D., & Mislevy, R. J. (1982). Adaptive EAP estimation of ability
  in a microcomputer environment. *Applied Psychological Measurement*,
  6(4), 431–444.
- Kocis, L., & Whiten, W. J. (1997). Computational investigations of
  low-discrepancy sequences. *ACM Transactions on Mathematical Software*,
  23(2), 266–294. (Sumber konstruksi Halton "leaped" yang dipakai
  `quadrature::build_qmc_points`.)
