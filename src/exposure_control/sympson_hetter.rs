//! Seleksi Sympson–Hetter (`SH`) — tiap kandidat diterima dengan peluang
//! sebesar parameter eksposurnya; `exposure` diindeks per **ID item**
//! (bukan posisi penyajian — beda dari [`super::randomesque`], lih. catatan
//! modul `super`).
//!
//! **Definisi metode** (Sympson & Hetter, 1985; van der Linden & Glas,
//! 2010, bab kontrol eksposur). Misalkan `k_i = exposure[i]` adalah
//! parameter eksposur item `i`. Pada tiap langkah, kandidat diurutkan
//! sekali menurut skor kriteria (terbaik dulu; seri: indeks item yang lebih
//! kecil lebih dulu, sehingga hasil deterministik), lalu daftar itu ditelusuri
//! dari atas. Untuk tiap kandidat digambar `u ~ Uniform[0, 1)`; kandidat
//! diterima bila dan hanya bila `u < k_i`. Jadi `P(administrasi i | i dipilih
//! kriteria) = k_i`: `k_i = 1` selalu diterima, `k_i = 0` tidak pernah.
//!
//! **Kandidat yang ditolak** disisihkan untuk peserta ini: ditandai
//! `valid_items[item] = false` — **persisten** untuk sisa sesi peserta
//! (pemanggil meneruskan slice yang sama di langkah-langkah berikutnya),
//! bukan cuma satu langkah. Item yang akhirnya disajikan tidak pernah
//! ditandai invalid.
//!
//! **Fallback semua-ditolak (keputusan desain engine ini, bukan bagian
//! definisi baku metode)**: bila setiap kandidat gagal draw, engine menyajikan
//! kandidat berperingkat teratas (skor tertinggi), melaporkan
//! `forced_accept = true`, dan **tidak** menandainya invalid. Alasannya:
//! tes harus selalu bisa berlanjut (tidak boleh macet atau error hanya
//! karena semua `k_i` kebetulan kecil), dan item paling informatif adalah
//! pilihan paksa yang paling kecil kerugiannya bagi ketepatan pengukuran.
//! Pilihan paksa tidak memakai bilangan acak (tidak ada draw tambahan). Kandidat
//! lain yang sudah ditolak pada panggilan itu tetap ditandai invalid.

use std::cmp::Ordering;

use rand::Rng;

use crate::error::McatError;
use crate::item_selection::ItemIndex;

/// Hasil satu pemanggilan [`select`].
#[derive(Debug)]
pub(crate) struct Outcome {
    pub chosen: ItemIndex,
    /// `(item, diterima)` satu entri per kandidat yang di-draw, urut
    /// peringkat (terbaik dulu). Pada jalur normal entri terakhir adalah
    /// `chosen` dengan `true`. Pada fallback semua-ditolak seluruh entri
    /// `false` (tidak ada entri untuk pilihan paksa, karena tidak ada draw)
    /// dan `chosen` sama dengan item pada entri pertama. Kosong bila
    /// `collect_debug == false` — tidak ada alokasi tambahan (prinsip desain
    /// #7).
    pub attempts: Vec<(ItemIndex, bool)>,
    /// `true` bila semua kandidat gagal draw dan `chosen` adalah kandidat
    /// berperingkat teratas yang disajikan lewat fallback, bukan lolos draw
    /// uniform normal.
    pub forced_accept: bool,
}

/// Urutan peringkat: skor menurun, skor `NaN` paling akhir, seri diputus
/// indeks item yang lebih kecil.
fn rank_order(a: &(ItemIndex, f64), b: &(ItemIndex, f64)) -> Ordering {
    a.1.is_nan()
        .cmp(&b.1.is_nan())
        .then_with(|| b.1.partial_cmp(&a.1).unwrap_or(Ordering::Equal))
        .then_with(|| a.0.cmp(&b.0))
}

/// `exposure` diindeks per ID item (panjang `n_items`, sama seperti
/// `valid_items`); `valid_items` dimutasi in-place untuk item yang ditolak
/// (bukan sekadar dibaca — lih. catatan modul di atas soal persistensi
/// antar-langkah). Error bila `candidate_scores` kosong (kehabisan item
/// untuk disajikan); selain itu SH selalu menghasilkan satu item, karena
/// fallback semua-ditolak menyajikan kandidat berperingkat teratas.
pub(crate) fn select(
    exposure: &[f64],
    candidate_scores: &[(ItemIndex, f64)],
    valid_items: &mut [bool],
    rng: &mut impl Rng,
    collect_debug: bool,
) -> Result<Outcome, McatError> {
    if candidate_scores.is_empty() {
        return Err(McatError::InvalidParameter(
            "sympson_hetter::select: candidate_scores kosong -- kehabisan item untuk disajikan"
                .into(),
        ));
    }

    let mut ranked: Vec<(ItemIndex, f64)> = candidate_scores.to_vec();
    ranked.sort_by(rank_order);

    let top = ranked[0].0;
    let top_was_valid = valid_items[top];
    let mut attempts = Vec::new();
    for &(item, _) in &ranked {
        let u: f64 = rng.gen_range(0.0..1.0);
        let accepted = u < exposure[item];
        if collect_debug {
            attempts.push((item, accepted));
        }
        if accepted {
            return Ok(Outcome {
                chosen: item,
                attempts,
                forced_accept: false,
            });
        }
        valid_items[item] = false;
    }

    // Semua kandidat ditolak: sajikan peringkat teratas dan kembalikan
    // status validnya seperti sebelum panggilan, yaitu tidak ditandai invalid
    // (lih. catatan modul soal alasan desain).
    valid_items[top] = top_was_valid;
    Ok(Outcome {
        chosen: top,
        attempts,
        forced_accept: true,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    #[test]
    fn errors_on_empty_candidates() {
        let mut valid = vec![true; 3];
        let mut rng = StdRng::seed_from_u64(0);
        let err = select(&[0.5, 0.5, 0.5], &[], &mut valid, &mut rng, false).unwrap_err();
        assert!(matches!(err, McatError::InvalidParameter(_)));
    }

    #[test]
    fn zero_exposure_item_never_passes_across_many_trials() {
        // exposure[0] = 0.0 -> `u < 0.0` mustahil untuk u >= 0, jadi item 0
        // selalu ditolak & jatuh ke kandidat berikut (di sini item 1,
        // exposure=1.0, selalu lolos begitu jadi giliran dicoba).
        let exposure = vec![0.0, 1.0];
        let scores = [(0usize, 10.0), (1usize, 1.0)];
        let mut rng = StdRng::seed_from_u64(1);
        for _ in 0..200 {
            let mut valid = vec![true; 2];
            let outcome = select(&exposure, &scores, &mut valid, &mut rng, false).unwrap();
            assert_eq!(outcome.chosen, 1);
        }
    }

    #[test]
    fn full_exposure_item_always_passes_immediately() {
        let exposure = vec![1.0, 1.0];
        let scores = [(0usize, 10.0), (1usize, 1.0)];
        let mut rng = StdRng::seed_from_u64(2);
        for _ in 0..50 {
            let mut valid = vec![true; 2];
            let outcome = select(&exposure, &scores, &mut valid, &mut rng, true).unwrap();
            assert_eq!(outcome.chosen, 0);
            assert!(!outcome.forced_accept);
            assert_eq!(outcome.attempts, vec![(0, true)]);
        }
    }

    #[test]
    fn forced_accept_when_only_one_candidate_remains() {
        // Kandidat tunggal, exposure=0.0 -> draw normal selalu gagal, tapi
        // fallback semua-ditolak harus tetap memilihnya (bukan error).
        let exposure = vec![0.0];
        let scores = [(0usize, 5.0)];
        let mut rng = StdRng::seed_from_u64(3);
        for _ in 0..50 {
            let mut valid = vec![true; 1];
            let outcome = select(&exposure, &scores, &mut valid, &mut rng, true).unwrap();
            assert_eq!(outcome.chosen, 0);
            assert!(outcome.forced_accept);
            // Pilihan paksa tidak menandai item invalid (lih. catatan modul).
            assert!(valid[0]);
        }
    }

    #[test]
    fn all_rejected_administers_top_ranked_without_marking_it_invalid() {
        // Perilaku baru (fallback desain engine): semua k=0 -> semua kandidat
        // ditolak -> item skor tertinggi (item 2) disajikan, bukan kandidat
        // berperingkat terakhir seperti pada loop lama.
        let exposure = vec![0.0, 0.0, 0.0];
        let scores = [(0usize, 1.0), (1usize, 2.0), (2usize, 9.0)];
        let mut valid = vec![true; 3];
        let mut rng = StdRng::seed_from_u64(10);
        let outcome = select(&exposure, &scores, &mut valid, &mut rng, true).unwrap();
        assert_eq!(outcome.chosen, 2);
        assert!(outcome.forced_accept);
        assert!(valid[2], "item pilihan paksa tidak boleh ditandai invalid");
        // Satu entri per kandidat yang di-draw, semuanya ditolak, urut peringkat.
        assert_eq!(outcome.attempts, vec![(2, false), (1, false), (0, false)]);
    }

    #[test]
    fn all_rejected_marks_the_other_candidates_invalid() {
        let exposure = vec![0.0; 4];
        let scores = [(0usize, 3.0), (1usize, 7.0), (2usize, 5.0), (3usize, 1.0)];
        let mut valid = vec![true; 4];
        let mut rng = StdRng::seed_from_u64(11);
        let outcome = select(&exposure, &scores, &mut valid, &mut rng, false).unwrap();
        assert_eq!(outcome.chosen, 1);
        assert!(outcome.forced_accept);
        assert_eq!(valid, vec![false, true, false, false]);
    }

    #[test]
    fn zero_exposure_everywhere_administers_top_ranked_across_many_trials() {
        let exposure = vec![0.0; 3];
        let scores = [(0usize, 0.5), (1usize, 4.0), (2usize, 2.0)];
        let mut rng = StdRng::seed_from_u64(12);
        for _ in 0..100 {
            let mut valid = vec![true; 3];
            let outcome = select(&exposure, &scores, &mut valid, &mut rng, false).unwrap();
            assert_eq!(outcome.chosen, 1);
            assert!(outcome.forced_accept);
        }
    }

    #[test]
    fn forced_choice_draws_no_extra_random_number() {
        // Tiga kandidat semua k=0 -> tepat 3 draw. Bila pilihan paksa ikut
        // menggambar bilangan, RNG berikutnya akan bergeser dari rujukan.
        let exposure = vec![0.0; 3];
        let scores = [(0usize, 1.0), (1usize, 2.0), (2usize, 3.0)];
        let mut valid = vec![true; 3];
        let mut rng = StdRng::seed_from_u64(13);
        select(&exposure, &scores, &mut valid, &mut rng, false).unwrap();
        let next_after_select: f64 = rng.gen_range(0.0..1.0);

        let mut reference = StdRng::seed_from_u64(13);
        for _ in 0..3 {
            let _: f64 = reference.gen_range(0.0..1.0);
        }
        let expected: f64 = reference.gen_range(0.0..1.0);
        assert_eq!(next_after_select, expected);
    }

    #[test]
    fn ties_are_broken_by_lower_item_index_regardless_of_input_order() {
        // Skor seri antara item 5 dan 2: yang berindeks lebih kecil (2)
        // dicoba lebih dulu, apa pun urutan masukan.
        let exposure = vec![1.0; 6];
        let scores = [(5usize, 1.0), (2usize, 1.0), (4usize, 0.1)];
        let mut valid = vec![true; 6];
        let mut rng = StdRng::seed_from_u64(14);
        let outcome = select(&exposure, &scores, &mut valid, &mut rng, true).unwrap();
        assert_eq!(outcome.chosen, 2);
        assert_eq!(outcome.attempts, vec![(2, true)]);
    }

    #[test]
    fn same_seed_gives_same_outcome_and_validity() {
        let exposure = vec![0.3, 0.5, 0.2, 0.7];
        let scores = [(0usize, 4.0), (1usize, 3.0), (2usize, 2.0), (3usize, 1.0)];
        for seed in 0..20u64 {
            let run = |seed: u64| {
                let mut valid = vec![true; 4];
                let mut rng = StdRng::seed_from_u64(seed);
                let o = select(&exposure, &scores, &mut valid, &mut rng, true).unwrap();
                (o.chosen, o.attempts, o.forced_accept, valid)
            };
            assert_eq!(run(seed), run(seed));
        }
    }

    #[test]
    fn selection_frequency_matches_product_formula_over_ranked_list() {
        // Daftar peringkat tetap: item 0 (k=0.5) > item 1 (k=0.4) > item 2
        // (k=0.8). Peluang item peringkat j terpilih lewat draw:
        //   P_j = k_j * prod_{l<j} (1 - k_l)
        // Peluang semua ditolak = 0.5 * 0.6 * 0.2 = 0.06 dan jatuh ke item 0.
        //   item 0: 0.50 + 0.06 = 0.56; item 1: 0.5*0.4 = 0.20; item 2: 0.5*0.6*0.8 = 0.24
        // N = 20000 -> simpangan baku maksimum sqrt(0.56*0.44/20000) ~ 0.0035;
        // toleransi 0.02 ~ 5.7 sigma, aman terhadap kebetulan seed.
        let exposure = vec![0.5, 0.4, 0.8];
        let scores = [(0usize, 3.0), (1usize, 2.0), (2usize, 1.0)];
        let mut rng = StdRng::seed_from_u64(2024);
        let n = 20_000;
        let mut counts = [0usize; 3];
        let mut forced = 0usize;
        for _ in 0..n {
            let mut valid = vec![true; 3];
            let o = select(&exposure, &scores, &mut valid, &mut rng, false).unwrap();
            counts[o.chosen] += 1;
            forced += usize::from(o.forced_accept);
        }
        let freq = |c: usize| c as f64 / n as f64;
        assert!((freq(counts[0]) - 0.56).abs() < 0.02, "{:?}", counts);
        assert!((freq(counts[1]) - 0.20).abs() < 0.02, "{:?}", counts);
        assert!((freq(counts[2]) - 0.24).abs() < 0.02, "{:?}", counts);
        assert!((freq(forced) - 0.06).abs() < 0.02, "forced={forced}");
    }

    #[test]
    fn rejected_items_are_marked_invalid_and_never_chosen_again_this_call() {
        // item0 exposure=0.0 (selalu ditolak jika masih ada kandidat lain),
        // item1 exposure=1.0 (selalu lolos). Karena item0 selalu skor
        // tertinggi, ia akan dicoba duluan, ditolak, ditandai invalid, lalu
        // item1 dicoba & lolos.
        let exposure = vec![0.0, 1.0];
        let scores = [(0usize, 100.0), (1usize, 1.0)];
        let mut valid = vec![true; 2];
        let mut rng = StdRng::seed_from_u64(4);
        let outcome = select(&exposure, &scores, &mut valid, &mut rng, true).unwrap();
        assert_eq!(outcome.chosen, 1);
        assert!(!valid[0], "item yang ditolak harus ditandai invalid");
        assert!(valid[1]);
        assert_eq!(outcome.attempts, vec![(0, false), (1, true)]);
    }

    #[test]
    fn invalidation_persists_only_in_the_given_slice_not_globally() {
        // Dua "sesi" (slice valid_items terpisah) dipakai bergantian --
        // penolakan di satu sesi tidak boleh memengaruhi sesi lain, karena
        // isolasi state sepenuhnya tanggung jawab slice yang diberikan
        // pemanggil (bukan struct global).
        let exposure = vec![0.0, 1.0];
        let scores = [(0usize, 100.0), (1usize, 1.0)];
        let mut valid_session_a = vec![true; 2];
        let valid_session_b = [true; 2];
        let mut rng = StdRng::seed_from_u64(5);

        select(&exposure, &scores, &mut valid_session_a, &mut rng, false).unwrap();
        assert!(!valid_session_a[0]);
        assert!(valid_session_b[0], "sesi lain tidak boleh ikut termutasi");
    }

    #[test]
    fn debug_false_collects_no_attempts() {
        let exposure = vec![0.0, 1.0];
        let scores = [(0usize, 100.0), (1usize, 1.0)];
        let mut valid = vec![true; 2];
        let mut rng = StdRng::seed_from_u64(6);
        let outcome = select(&exposure, &scores, &mut valid, &mut rng, false).unwrap();
        assert!(outcome.attempts.is_empty());
    }
}
