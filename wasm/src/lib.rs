//! Binding WebAssembly untuk `theta-engine`, dibangun dengan `wasm-bindgen`
//! (target `nodejs`, dipakai backend Bun/Node). Mesin adaptif hanya berjalan
//! di server: klien tidak pernah menerima parameter item (lih. DR-004 di
//! `engaugemetrics-specs`).
//!
//! Seluruh logika ada di [`session`]; modul ini hanya mengubah `JsValue`
//! menjadi tipe Rust dan sebaliknya. Masukan dibaca lewat JSON
//! (`JSON.stringify` lalu `serde_json`), bukan `serde-wasm-bindgen`, karena yang
//! kedua mengabaikan field tak dikenal sehingga `deny_unknown_fields` tidak
//! berlaku dan salah ketik pada spec bisa mematikan fitur tanpa pesan. Akibatnya
//! `NaN`/`Infinity` pada masukan ditolak (menjadi `null`). Keluaran lewat
//! `serde-wasm-bindgen` agar bentuknya objek biasa.

pub mod session;

use serde::Serialize;
use session::{Engine as Inner, EngineError, EngineSpec, SessionState};
use wasm_bindgen::prelude::*;

#[wasm_bindgen(typescript_custom_section)]
const TYPES: &str = include_str!("types.d.ts");

/// Versi inti + binding (satu versi untuk keduanya, lih. `[workspace.package]`).
/// Catat bersama tiap pemilihan item: hasil hanya dijamin bisa diulang dengan
/// artefak versi yang sama.
#[wasm_bindgen(js_name = engineVersion)]
pub fn engine_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// Mesin untuk satu definisi tes. Bangun sekali per versi bank dan pakai
/// ulang untuk semua sesi. Panggil `free()` bila tidak dipakai lagi.
#[wasm_bindgen]
pub struct Engine {
    inner: Inner,
}

#[wasm_bindgen]
impl Engine {
    /// Bangun dan validasi mesin dari `EngineSpec`. Melempar `McatError`
    /// bila spec tidak valid.
    #[wasm_bindgen(constructor)]
    pub fn new(
        #[wasm_bindgen(unchecked_param_type = "EngineSpec")] spec: JsValue,
    ) -> Result<Engine, JsValue> {
        let spec: EngineSpec = decode(spec)?;
        Ok(Engine {
            inner: Inner::new(spec).map_err(to_js_error)?,
        })
    }

    /// Jumlah item di bank.
    #[wasm_bindgen(getter, js_name = nItems)]
    pub fn n_items(&self) -> usize {
        self.inner.n_items()
    }

    /// Jumlah dimensi.
    #[wasm_bindgen(getter)]
    pub fn nfact(&self) -> usize {
        self.inner.nfact()
    }

    /// Sesi baru. `theta` opsional menimpa titik awal nol.
    #[wasm_bindgen(unchecked_return_type = "SessionState")]
    pub fn start(
        &self,
        #[wasm_bindgen(unchecked_param_type = "number[] | null")] theta: Option<JsValue>,
    ) -> Result<JsValue, JsValue> {
        let theta: Option<Vec<f64>> = decode(theta.unwrap_or(JsValue::NULL))?;
        let state = self
            .inner
            .start(theta.map(theta_engine::theta::ThetaVector::from_vec))
            .map_err(to_js_error)?;
        encode(&state)
    }

    /// Pilih item berikutnya. Seed yang sama pada state yang sama memberi item
    /// yang sama.
    #[wasm_bindgen(unchecked_return_type = "NextItem")]
    pub fn next(
        &self,
        #[wasm_bindgen(unchecked_param_type = "SessionState")] state: JsValue,
        seed: u64,
    ) -> Result<JsValue, JsValue> {
        let state: SessionState = decode(state)?;
        encode(&self.inner.next(&state, seed).map_err(to_js_error)?)
    }

    /// Catat jawaban, estimasi ulang `theta`, dan evaluasi aturan berhenti.
    /// `index` dan `response` harus bilangan bulat >= 0.
    #[wasm_bindgen(unchecked_return_type = "SubmitResult")]
    pub fn submit(
        &self,
        #[wasm_bindgen(unchecked_param_type = "SessionState")] state: JsValue,
        index: f64,
        response: f64,
        #[wasm_bindgen(unchecked_param_type = "number | null", js_name = elapsedMs)]
        elapsed_ms: Option<JsValue>,
    ) -> Result<JsValue, JsValue> {
        let state: SessionState = decode(state)?;
        let index = whole_number(index, "index")?;
        let response = whole_number(response, "response")?;
        let elapsed_ms: Option<f64> = decode(elapsed_ms.unwrap_or(JsValue::NULL))?;
        let elapsed_ms = elapsed_ms
            .map(|ms| whole_number(ms, "elapsedMs").map(|ms| ms as u64))
            .transpose()?;
        encode(
            &self
                .inner
                .submit(&state, index, response, elapsed_ms)
                .map_err(to_js_error)?,
        )
    }

    /// Hasil akhir sesi.
    #[wasm_bindgen(unchecked_return_type = "McatResult")]
    pub fn finish(
        &self,
        #[wasm_bindgen(unchecked_param_type = "SessionState")] state: JsValue,
    ) -> Result<JsValue, JsValue> {
        let state: SessionState = decode(state)?;
        encode(&self.inner.finish(&state).map_err(to_js_error)?)
    }
}

fn decode<T: serde::de::DeserializeOwned>(value: JsValue) -> Result<T, JsValue> {
    let json = js_sys::JSON::stringify(&value)
        .ok()
        .and_then(|s| s.as_string())
        .ok_or_else(|| to_js_error(EngineError::invalid("nilai tidak bisa dibaca sebagai JSON")))?;
    serde_json::from_str(&json).map_err(|e| to_js_error(EngineError::invalid(e.to_string())))
}

fn encode<T: Serialize>(value: &T) -> Result<JsValue, JsValue> {
    // `null` (bukan `undefined`) untuk `Option::None` dan objek biasa untuk
    // map: bentuk yang sama dengan JSON yang disimpan backend.
    let serializer = serde_wasm_bindgen::Serializer::new()
        .serialize_missing_as_null(true)
        .serialize_maps_as_objects(true);
    value
        .serialize(&serializer)
        .map_err(|e| to_js_error(EngineError::invalid(e.to_string())))
}

/// `wasm-bindgen` mengubah `NaN`, pecahan, dan negatif menjadi bilangan bulat
/// diam-diam saat argumen bertipe bilangan bulat; terima `f64` lalu tolak
/// eksplisit.
fn whole_number(value: f64, name: &str) -> Result<usize, JsValue> {
    if value.is_finite() && value >= 0.0 && value.fract() == 0.0 && value <= 9_007_199_254_740_991.0
    {
        Ok(value as usize)
    } else {
        Err(to_js_error(EngineError::invalid(format!(
            "{name} harus bilangan bulat >= 0, didapat {value}"
        ))))
    }
}

/// `Error` JS dengan `name = "McatError"` dan `code` stabil.
fn to_js_error(error: EngineError) -> JsValue {
    let js = js_sys::Error::new(&error.to_string());
    js.set_name("McatError");
    // Gagal menyetel `code` tidak boleh menutupi galat aslinya.
    let _ = js_sys::Reflect::set(&js, &"code".into(), &error.code().into());
    js.into()
}
