//! Logika sesi MCAT murni Rust, tanpa tipe `wasm-bindgen`, supaya bisa diuji
//! di target native (`cargo test -p theta-engine-wasm`). Lapisan `lib.rs`
//! hanya menerjemahkan antara `JsValue` dan tipe di sini.
//!
//! Satu [`Engine`] mewakili satu definisi tes (bank item, parameter grup, dan
//! konfigurasi algoritma). Ia **tidak** menyimpan state peserta: state sesi
//! ([`SessionState`]) masuk sebagai argumen dan keluar sebagai hasil, jadi
//! pemanggil (backend) bebas menyimpannya di database dan memakai ulang satu
//! `Engine` untuk banyak sesi.
//!
//! Urutan satu langkah mengikuti `examples/09_configurable_pipeline_with_validation.rs`
//! di crate inti: kandidat -> content balancing -> seleksi (dengan atau tanpa
//! exposure control) -> catat jawaban -> estimasi ulang -> aturan berhenti.

use std::collections::HashMap;
use std::fmt;
use std::time::Duration;

use rand::rngs::StdRng;
use rand::SeedableRng;
use serde::{Deserialize, Serialize};
use theta_engine::irt::{Dichotomous, GradedResponse};
use theta_engine::theta::{InfoMatrix, ThetaVector};
use theta_engine::{
    evaluate_stopping_rule, narrow_candidates, rank_candidates, select_next_item,
    select_with_exposure, ClassifyType, ContentBalancingConfig, EstimationConfig,
    ExposureControlConfig, ExposureMode, GroupParams, InitialThetaConfig, ItemBank, ItemIndex,
    ItemMeta, McatError, McatResult, PersonState, SelectionConfig, StopDecision, StoppingConfig,
    ThetaStart,
};

/// Galat dari binding. `code` menjadi properti `code` pada `Error` JS.
#[derive(Debug, PartialEq)]
pub enum EngineError {
    /// Galat dari inti (`McatError`).
    Mcat(McatError),
    /// Input dari pemanggil tidak valid: bentuk, rentang, atau tidak cocok
    /// dengan bank.
    InvalidInput(String),
    /// Tidak ada item tersisa untuk disajikan.
    NoCandidates,
}

impl EngineError {
    /// Kode stabil untuk dipakai pemanggil, tidak bergantung pada teks pesan.
    pub fn code(&self) -> &'static str {
        match self {
            EngineError::Mcat(McatError::DimensionMismatch { .. }) => "DIMENSION_MISMATCH",
            EngineError::Mcat(McatError::InvalidParameter(_)) => "INVALID_PARAMETER",
            EngineError::Mcat(McatError::ContentBalancingIncompatibleWithSeq) => {
                "CONTENT_BALANCING_INCOMPATIBLE_WITH_SEQ"
            }
            EngineError::InvalidInput(_) => "INVALID_INPUT",
            EngineError::NoCandidates => "NO_CANDIDATES",
        }
    }

    pub(crate) fn invalid(message: impl Into<String>) -> Self {
        EngineError::InvalidInput(message.into())
    }
}

impl fmt::Display for EngineError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EngineError::Mcat(e) => write!(f, "{e}"),
            EngineError::InvalidInput(m) => write!(f, "input tidak valid: {m}"),
            EngineError::NoCandidates => write!(f, "tidak ada item tersisa untuk disajikan"),
        }
    }
}

impl From<McatError> for EngineError {
    fn from(e: McatError) -> Self {
        EngineError::Mcat(e)
    }
}

/// Parameter satu item. Dibangun lewat konstruktor inti sehingga validasinya
/// (asimtot, urutan boundary) ikut berlaku.
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum ItemSpec {
    /// Item dikotomus M2PL/M3PL/M4PL. `g` default 0, `u` default 1.
    Dichotomous {
        a: Vec<f64>,
        d: f64,
        #[serde(default)]
        g: f64,
        #[serde(default = "one")]
        u: f64,
    },
    /// Item politomus graded response; `d` menurun.
    Graded { a: Vec<f64>, d: Vec<f64> },
}

fn one() -> f64 {
    1.0
}

/// Parameter populasi `N(gmeans, gcov)`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GroupSpec {
    pub gmeans: ThetaVector,
    pub gcov: InfoMatrix,
}

/// Content balancing: kategori tiap item (urutan sama dengan `items`) dan
/// target proporsi per kategori.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContentBalancingSpec {
    pub category_per_item: Vec<String>,
    pub target_prop: HashMap<String, f64>,
}

/// Definisi satu tes.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EngineSpec {
    pub items: Vec<ItemSpec>,
    pub group: GroupSpec,
    pub selection: SelectionConfig,
    pub estimation: EstimationConfig,
    pub stopping: StoppingConfig,
    #[serde(default)]
    pub exposure: Option<ExposureMode>,
    #[serde(default)]
    pub content_balancing: Option<ContentBalancingSpec>,
}

/// State satu sesi peserta. Buram bagi pemanggil: simpan apa adanya dan
/// kembalikan ke `Engine` yang sama (atau yang dibangun dari spec yang sama).
///
/// Hanya kembalikan state yang dulu dikeluarkan engine. `Engine` memeriksa
/// kesesuaian bentuknya dengan bank, tapi bukan keaslian nilainya.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SessionState {
    pub person: PersonState,
    /// Tanda item yang masih boleh disajikan (state Sympson-Hetter), satu
    /// entri per item bank. Bertahan sepanjang sesi.
    pub valid_items: Vec<bool>,
}

/// Hasil [`Engine::next`].
#[derive(Debug, Serialize)]
pub struct NextItem {
    /// Posisi item di `items` pada spec.
    pub index: ItemIndex,
    pub state: SessionState,
}

/// Hasil [`Engine::submit`].
#[derive(Debug, Serialize)]
pub struct SubmitResult {
    pub state: SessionState,
    pub stop: StopDecision,
}

/// Mesin untuk satu definisi tes. Lihat dokumentasi modul.
pub struct Engine {
    bank: ItemBank,
    group: GroupParams,
    selection: SelectionConfig,
    estimation: EstimationConfig,
    stopping: StoppingConfig,
    exposure: Option<ExposureControlConfig>,
    content_balancing: Option<ContentBalancingConfig>,
}

impl Engine {
    /// Bangun dan validasi seluruh definisi tes. Kombinasi yang pasti gagal di
    /// tengah sesi ditolak di sini.
    pub fn new(spec: EngineSpec) -> Result<Self, EngineError> {
        let bank = build_bank(spec.items)?;
        let n_items = bank.len();
        let nfact = bank
            .nfact()
            .expect("bank tidak kosong (divalidasi build_bank)");

        let group = GroupParams::new(spec.group.gmeans, spec.group.gcov)?;
        if group.nfact() != nfact {
            return Err(McatError::DimensionMismatch {
                expected: nfact,
                actual: group.nfact(),
                context: "Engine::new: group vs item",
            }
            .into());
        }
        check_covariance(&group)?;

        let selection = SelectionConfig::new(
            spec.selection.criterion,
            spec.selection.weights,
            nfact,
            false,
        )?;

        let content_balancing = spec
            .content_balancing
            .map(|cb| build_content_balancing(cb, n_items, &selection))
            .transpose()?;

        let exposure = spec
            .exposure
            .map(|mode| build_exposure(mode, n_items))
            .transpose()?;

        check_stopping(&spec.stopping, nfact)?;

        let engine = Self {
            bank,
            group,
            selection,
            estimation: EstimationConfig {
                debug: false,
                ..spec.estimation
            },
            stopping: StoppingConfig {
                debug: false,
                ..spec.stopping
            },
            exposure,
            content_balancing,
        };
        engine.probe_selection()?;
        Ok(engine)
    }

    /// Jumlah item di bank.
    pub fn n_items(&self) -> usize {
        self.bank.len()
    }

    /// Jumlah dimensi.
    pub fn nfact(&self) -> usize {
        self.group.nfact()
    }

    /// Sesi baru. `theta` (opsional) menimpa titik awal nol, mis. estimasi
    /// akhir Sesi 1; SE awal tetap dari `gcov`.
    pub fn start(&self, theta: Option<ThetaVector>) -> Result<SessionState, EngineError> {
        let config = InitialThetaConfig {
            start: theta.map_or(ThetaStart::Zero, ThetaStart::Fixed),
            ..Default::default()
        };
        let (theta0, se0, _weights, _) =
            theta_engine::initial_theta::initialize(&config, &self.group, 0)?;
        Ok(SessionState {
            person: PersonState::from_theta_and_se(theta0, se0),
            valid_items: vec![true; self.n_items()],
        })
    }

    /// Pilih item berikutnya. `seed` menentukan seluruh keacakan langkah ini
    /// (Random, exposure control) sehingga hasilnya bisa diulang.
    pub fn next(&self, state: &SessionState, seed: u64) -> Result<NextItem, EngineError> {
        self.check_state(state)?;
        let person = &state.person;
        let mut valid_items = state.valid_items.clone();

        let mut answered = vec![false; self.n_items()];
        for &i in person.answered_items() {
            answered[i] = true;
        }
        let candidates: Vec<ItemIndex> = (0..self.n_items())
            .filter(|&i| valid_items[i] && !answered[i])
            .collect();
        if candidates.is_empty() {
            return Err(EngineError::NoCandidates);
        }

        let candidates = match &self.content_balancing {
            Some(cb) => narrow_candidates(cb, person.answered_items(), &candidates)?.0,
            None => candidates,
        };

        let mut rng = StdRng::seed_from_u64(seed);
        let chosen = match &self.exposure {
            Some(exposure) => {
                let scores = rank_candidates(
                    &self.selection,
                    &candidates,
                    person,
                    &self.bank,
                    &self.group,
                )?;
                select_with_exposure(
                    exposure,
                    &scores,
                    person.n_answered(),
                    &mut valid_items,
                    &mut rng,
                )?
                .0
            }
            None => {
                select_next_item(
                    &self.selection,
                    &candidates,
                    person,
                    &self.bank,
                    &self.group,
                    &mut rng,
                )?
                .0
            }
        };

        Ok(NextItem {
            index: chosen,
            state: SessionState {
                person: person.clone(),
                valid_items,
            },
        })
    }

    /// Catat jawaban `response` (indeks kategori, mulai 0) untuk item `index`,
    /// estimasi ulang `theta`, lalu evaluasi aturan berhenti. `elapsed_ms`
    /// (waktu pengerjaan item) hanya berpengaruh pada `max_time`.
    pub fn submit(
        &self,
        state: &SessionState,
        index: usize,
        response: usize,
        elapsed_ms: Option<u64>,
    ) -> Result<SubmitResult, EngineError> {
        self.check_state(state)?;
        let item = self.bank.item(index).ok_or_else(|| {
            EngineError::invalid(format!(
                "index item {index} di luar bank ({} item)",
                self.n_items()
            ))
        })?;
        if state.person.answered_items().contains(&index) {
            return Err(EngineError::invalid(format!(
                "item {index} sudah dijawab dalam sesi ini"
            )));
        }
        if response >= item.n_categories() {
            return Err(EngineError::invalid(format!(
                "respons {response} di luar {} kategori item {index}",
                item.n_categories()
            )));
        }

        let mut person = state.person.clone();
        person.record_answer(index, response);
        if let Some(ms) = elapsed_ms {
            person.record_item_time(Duration::from_millis(ms));
        }
        let ((theta, se), _) = theta_engine::ability_estimation::estimate(
            &self.estimation,
            &person,
            &self.bank,
            &self.group,
        )?;
        person.set_estimate(theta, se);

        let (stop, _) = evaluate_stopping_rule(&self.stopping, &person, &self.bank, false, None)?;
        Ok(SubmitResult {
            state: SessionState {
                person,
                valid_items: state.valid_items.clone(),
            },
            stop,
        })
    }

    /// Hasil akhir sesi. Aturan berhenti dievaluasi ulang pada state ini,
    /// sehingga `terminated_early` dan klasifikasi konsisten dengan keputusan
    /// `submit` terakhir.
    pub fn finish(&self, state: &SessionState) -> Result<McatResult, EngineError> {
        self.check_state(state)?;
        let (stop, _) =
            evaluate_stopping_rule(&self.stopping, &state.person, &self.bank, false, None)?;
        Ok(McatResult::from_person(&state.person, &stop))
    }

    /// Pastikan bentuk state cocok dengan bank ini. Menolak state dari bank
    /// lain atau yang rusak, supaya inti tidak sampai trap di tengah hitungan.
    fn check_state(&self, state: &SessionState) -> Result<(), EngineError> {
        let nfact = self.nfact();
        let person = &state.person;

        if state.valid_items.len() != self.n_items() {
            return Err(EngineError::invalid(format!(
                "valid_items berisi {} entri, bank {} item",
                state.valid_items.len(),
                self.n_items()
            )));
        }
        let vectors_ok = person.theta().nfact() == nfact
            && person.se().nfact() == nfact
            && person.previous_theta().is_none_or(|t| t.nfact() == nfact)
            && person.thetas_history().iter().all(|t| t.nfact() == nfact)
            && person
                .thetas_se_history()
                .iter()
                .all(|s| s.nfact() == nfact);
        if !vectors_ok {
            return Err(EngineError::invalid(format!(
                "theta/SE pada state tidak berdimensi {nfact}"
            )));
        }
        if person.answered_items().len() != person.responses().len() {
            return Err(EngineError::invalid(
                "answered_items dan responses beda panjang",
            ));
        }
        if person.thetas_history().len() != person.thetas_se_history().len() {
            return Err(EngineError::invalid(
                "thetas_history dan thetas_se_history beda panjang",
            ));
        }
        let mut seen = vec![false; self.n_items()];
        for (&index, &response) in person.answered_items().iter().zip(person.responses()) {
            let item = self.bank.item(index).ok_or_else(|| {
                EngineError::invalid(format!("state memuat item {index} di luar bank"))
            })?;
            if std::mem::replace(&mut seen[index], true) {
                return Err(EngineError::invalid(format!(
                    "state memuat item {index} dua kali"
                )));
            }
            if response >= item.n_categories() {
                return Err(EngineError::invalid(format!(
                    "state memuat respons {response} di luar kategori item {index}"
                )));
            }
        }
        Ok(())
    }

    /// Jalankan satu seleksi percobaan dari state awal supaya kombinasi
    /// kriteria x `nfact` x exposure yang tak didukung inti gagal saat
    /// konstruksi, bukan pada peserta pertama.
    fn probe_selection(&self) -> Result<(), EngineError> {
        let person = self.start(None)?.person;
        let mut rng = StdRng::seed_from_u64(0);
        select_next_item(
            &self.selection,
            &[0],
            &person,
            &self.bank,
            &self.group,
            &mut rng,
        )?;
        if self.exposure.is_some() {
            rank_candidates(&self.selection, &[0], &person, &self.bank, &self.group)?;
        }
        Ok(())
    }
}

fn build_bank(items: Vec<ItemSpec>) -> Result<ItemBank, EngineError> {
    if items.is_empty() {
        return Err(EngineError::invalid("items kosong"));
    }
    let mut bank = ItemBank::new();
    for (index, item) in items.into_iter().enumerate() {
        let model: Box<dyn theta_engine::irt::ItemResponseModel> = match item {
            ItemSpec::Dichotomous { a, d, g, u } => Box::new(Dichotomous::new(a, d, g, u)?),
            ItemSpec::Graded { a, d } => Box::new(GradedResponse::new(a, d)?),
        };
        bank.push(model, ItemMeta::new(index))?;
    }
    Ok(bank)
}

/// `gcov` harus simetris dan definit positif (dipakai sebagai prior dan untuk
/// titik QMC); tanpa ini galat baru muncul di tengah estimasi.
fn check_covariance(group: &GroupParams) -> Result<(), EngineError> {
    let gcov = group.gcov();
    let n = gcov.nfact();
    for i in 0..n {
        for j in 0..n {
            let (x, y) = (gcov.get(i, j), gcov.get(j, i));
            if !x.is_finite() || (x - y).abs() > 1e-9 * (1.0 + x.abs().max(y.abs())) {
                return Err(McatError::InvalidParameter(
                    "group.gcov harus simetris dan berisi bilangan hingga".into(),
                )
                .into());
            }
        }
    }
    if gcov.min_eigenvalue() <= 0.0 {
        return Err(McatError::InvalidParameter("group.gcov harus definit positif".into()).into());
    }
    Ok(())
}

fn build_content_balancing(
    spec: ContentBalancingSpec,
    n_items: usize,
    selection: &SelectionConfig,
) -> Result<ContentBalancingConfig, EngineError> {
    if spec.category_per_item.len() != n_items {
        return Err(McatError::DimensionMismatch {
            expected: n_items,
            actual: spec.category_per_item.len(),
            context: "content_balancing.category_per_item",
        }
        .into());
    }
    let config = ContentBalancingConfig::new(
        spec.category_per_item.into_iter().map(Into::into).collect(),
        spec.target_prop
            .into_iter()
            .map(|(name, p)| (name.into(), p))
            .collect(),
        false,
    )?;
    config.validate_with_criterion(selection.criterion)?;
    Ok(config)
}

fn build_exposure(
    mode: ExposureMode,
    n_items: usize,
) -> Result<ExposureControlConfig, EngineError> {
    let valid = match &mode {
        ExposureMode::SympsonHetter(v) => v.iter().all(|p| (0.0..=1.0).contains(p)),
        ExposureMode::Randomesque(v) => v.iter().all(|&n| n >= 1),
    };
    if !valid {
        return Err(McatError::InvalidParameter(
            "exposure: Sympson-Hetter butuh nilai di [0, 1], Randomesque butuh bilangan bulat >= 1"
                .into(),
        )
        .into());
    }
    Ok(ExposureControlConfig::new(mode, n_items, false)?)
}

/// Panjang vektor aturan berhenti yang baru dicek `evaluate` setelah
/// `min_items`; di sini dicek di muka.
fn check_stopping(config: &StoppingConfig, nfact: usize) -> Result<(), EngineError> {
    for (name, values) in [
        ("min_sem", &config.min_sem),
        ("delta_thetas", &config.delta_thetas),
    ] {
        if values.len() != 1 && values.len() != nfact {
            return Err(McatError::InvalidParameter(format!(
                "stopping.{name}: panjang harus 1 atau {nfact}, didapat {}",
                values.len()
            ))
            .into());
        }
    }
    match &config.classify {
        ClassifyType::None => {}
        ClassifyType::Ci { cutoffs, .. } => {
            if cutoffs.len() != nfact {
                return Err(McatError::DimensionMismatch {
                    expected: nfact,
                    actual: cutoffs.len(),
                    context: "stopping.classify Ci.cutoffs",
                }
                .into());
            }
        }
        ClassifyType::Sprt { lower, upper, .. } => {
            lower.validate_nfact(nfact, "stopping.classify Sprt.lower")?;
            upper.validate_nfact(nfact, "stopping.classify Sprt.upper")?;
        }
    }
    Ok(())
}
