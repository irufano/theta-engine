/** Parameter satu item. `g` default 0 dan `u` default 1 untuk `dichotomous`. */
export type ItemSpec =
  | { type: "dichotomous"; a: number[]; d: number; g?: number; u?: number }
  | { type: "graded"; a: number[]; d: number[] };

/** Prior populasi `N(gmeans, gcov)`; `gcov` simetris dan definit positif. */
export interface GroupSpec {
  gmeans: number[];
  gcov: number[][];
}

/** Bentuk serde enum Rust: varian tanpa data berupa string, lainnya objek satu kunci. */
export type Criterion =
  | "Seq"
  | "Random"
  | "Mepv"
  | "Mlwi"
  | "Mpwi"
  | "Mei"
  | "Ikl"
  | { Kl: { delta: number } }
  | { KlN: { delta: number } }
  | { DRule: { prior: boolean } }
  | { TRule: { prior: boolean } }
  | { ARule: { prior: boolean } }
  | { ERule: { prior: boolean } }
  | { WRule: { prior: boolean } };

export interface SelectionConfig {
  criterion: Criterion;
  /** Bobot per dimensi untuk T/A/W-rule; `null` berarti semua 1. */
  weights: number[] | null;
  debug: boolean;
}

export type Method =
  | "Map"
  | "Ml"
  | "Wle"
  | "EapSum"
  | "Fixed"
  | { Eap: { qmc: boolean } };

export interface EstimationConfig {
  method: Method;
  quadpts: number | null;
  debug: boolean;
}

export type ClassifyType =
  | "None"
  | { Ci: { cutoffs: number[]; alpha: number } }
  | { Sprt: { lower: number[]; upper: number[]; alpha: number; beta: number } };

export interface StoppingConfig {
  min_items: number;
  /** Wajib; jangan lebih besar dari jumlah item bank. */
  max_items: number;
  /** Total waktu tes; `null` berarti tak terbatas. */
  max_time: { secs: number; nanos: number } | null;
  /** Panjang 1 (disebar ke semua dimensi) atau `nfact`. */
  min_sem: number[];
  /** Panjang 1 atau `nfact`. */
  delta_thetas: number[];
  classify: ClassifyType;
  allow_constrain_breaks: boolean;
  debug: boolean;
}

/** Sympson-Hetter: peluang lolos per item di [0, 1]. Randomesque: top-n per posisi penyajian (bilangan bulat >= 1). */
export type ExposureMode =
  | { SympsonHetter: number[] }
  | { Randomesque: number[] };

export interface ContentBalancingSpec {
  /** Kategori tiap item, urutan sama dengan `items`. */
  category_per_item: string[];
  /** Proporsi target per kategori, jumlahnya 1. */
  target_prop: Record<string, number>;
}

/** Definisi satu tes. Urutan `items` menjadi indeks item di seluruh API. */
export interface EngineSpec {
  items: ItemSpec[];
  group: GroupSpec;
  selection: SelectionConfig;
  estimation: EstimationConfig;
  stopping: StoppingConfig;
  exposure?: ExposureMode | null;
  content_balancing?: ContentBalancingSpec | null;
}

/**
 * State sesi. Simpan apa adanya (JSON) dan kembalikan ke engine; jangan
 * pernah menerimanya dari klien. Waktu pengerjaan di dalamnya berbentuk
 * `{ secs, nanos }`.
 */
export interface SessionState {
  person: Record<string, unknown>;
  valid_items: boolean[];
}

export interface NextItem {
  /** Posisi item di `EngineSpec.items`. */
  index: number;
  state: SessionState;
}

export type ClassifyDecision = "AboveCutoff" | "BelowCutoff" | "NoDecision";

export interface StopDecision {
  stop_now: boolean;
  met_sem: boolean[];
  met_delta_thetas: boolean[];
  met_classify: boolean[] | null;
  classification: ClassifyDecision[];
  terminated_early: boolean;
}

export interface SubmitResult {
  state: SessionState;
  stop: StopDecision;
}

export interface McatResult {
  thetas: number[];
  se_thetas: number[];
  thetas_history: number[][];
  thetas_se_history: number[][];
  classification: ClassifyDecision[];
  column_names: string[];
  items_answered: number[];
  responses: Array<number | null>;
  terminated_early: boolean;
}

/** `Error` dengan `name === "McatError"` dan `code` stabil. */
export type McatErrorCode =
  | "DIMENSION_MISMATCH"
  | "INVALID_PARAMETER"
  | "CONTENT_BALANCING_INCOMPATIBLE_WITH_SEQ"
  | "INVALID_INPUT"
  | "NO_CANDIDATES";
