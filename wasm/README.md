# @dayatech/theta-engine-wasm

Binding WebAssembly untuk `theta-engine`, dibuat untuk backend Node/Bun. Mesin
adaptif hanya berjalan di server: jangan memuat paket ini di klien, karena
klien tidak boleh menerima parameter item.

## Pemakaian

```ts
import { Engine, engineVersion } from "@dayatech/theta-engine-wasm";

// Satu Engine per versi bank; pakai ulang untuk semua sesi dan simpan di cache.
const engine = new Engine(spec); // spec: EngineSpec (lih. theta_engine_wasm.d.ts)

let state = engine.start(null); // atau engine.start([0.4, -0.2, 0.1]) dari Sesi 1
const { index, state: served } = engine.next(state, seed);
// elapsedMs: null bila waktu pengerjaan tidak dilacak
const { state: next, stop } = engine.submit(served, index, response, elapsedMs);
if (stop.stop_now) {
  const result = engine.finish(next);
}
```

- `index` adalah posisi item di `spec.items`. Pemetaan ke id item (UUID) milik
  pemanggil.
- `state` simpan sebagai JSON dan kembalikan ke engine. Jangan pernah menerima
  `state` dari klien: engine memeriksa bentuknya terhadap bank, bukan
  keasliannya.
- `seed` bertipe `bigint`. Turunkan dari sesi dan jumlah jawaban (mis. hash dari
  id sesi + `n_answered`) supaya pemilihan item bisa diulang saat audit.
- Hasil hanya dijamin identik pada versi artefak yang sama (`engineVersion()`),
  dengan selisih numerik kecil antara native dan WebAssembly. Catat versinya
  bersama tiap pemilihan item.
- Galat berupa `Error` dengan `name === "McatError"` dan `code`:
  `DIMENSION_MISMATCH`, `INVALID_PARAMETER`,
  `CONTENT_BALANCING_INCOMPATIBLE_WITH_SEQ`, `INVALID_INPUT`, `NO_CANDIDATES`.
- Metode estimasi `Ml` dan `Wle` menghasilkan `theta` tak hingga atau `NaN`
  pada pola jawaban ekstrem (perilaku inti, mis. semua benar pada jawaban
  pertama). Nilai itu hilang menjadi `null` di JSON dan `next`/`submit`
  berikutnya menolaknya sebagai `INVALID_INPUT`. Untuk sesi yang state-nya
  disimpan sebagai JSON, pakai `Map` atau `Eap`.
- Eksekusi sinkron di thread pemanggil. Ukur lama `next`/`submit` untuk bank dan
  `nfact` Anda sebelum memanggilnya langsung dari handler request.
- Panggil `engine.free()` bila `Engine` tidak dipakai lagi.

## Membangun

```sh
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --version <versi wasm-bindgen di Cargo.lock> --locked
wasm/build.sh            # menghasilkan wasm/pkg
```

Versi `wasm-bindgen-cli` harus sama persis dengan crate `wasm-bindgen` di
`Cargo.lock`; bila berbeda, `wasm-bindgen` menolak dengan pesan skema tidak
cocok. `wasm-opt` (binaryen) dipakai otomatis bila terpasang.

## Pengujian

```sh
cargo test -p theta-engine-wasm        # logika sesi, native
wasm/build.sh && (cd wasm/tests/js && bun test)   # WebAssembly sungguhan
```

`wasm/tests/fixtures/golden_session.json` dibaca kedua jalur: urutan item harus
sama persis dan `theta`/SE dalam toleransi `1e-9`. Setelah perubahan algoritma
yang disengaja, perbarui dengan
`UPDATE_GOLDEN=1 cargo test -p theta-engine-wasm golden` dan baca selisihnya.
