// Uji artefak WebAssembly sungguhan (hasil `wasm/build.sh`) lewat Bun, runtime
// backend. Logika sesi sudah diuji native di `wasm/tests/session.rs`; di sini
// yang diuji adalah batas JS <-> wasm: paritas numerik, kontrak galat,
// serialisasi state, dan validasi argumen.
import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { Engine, engineVersion } from "../../pkg/theta_engine_wasm.js";
import type { EngineSpec, SessionState } from "../../pkg/theta_engine_wasm";

const golden = JSON.parse(
  readFileSync(new URL("../fixtures/golden_session.json", import.meta.url), "utf8"),
) as {
  spec: EngineSpec;
  steps: { seed: number; response: number }[];
  expected: { items: number[]; thetas: number[]; se_thetas: number[] };
};

const clone = <T>(value: T): T => JSON.parse(JSON.stringify(value));

function expectMcatError(fn: () => unknown, code: string) {
  try {
    fn();
  } catch (error) {
    expect(error).toBeInstanceOf(Error);
    expect((error as Error).name).toBe("McatError");
    expect((error as { code: string }).code).toBe(code);
    return;
  }
  throw new Error(`diharapkan galat ${code}, tetapi tidak ada yang dilempar`);
}

describe("paritas dengan native", () => {
  test("sesi emas menghasilkan urutan item sama dan theta/SE dalam 1e-9", () => {
    const engine = new Engine(golden.spec);
    let state = engine.start(null);
    const served: number[] = [];
    for (const step of golden.steps) {
      const next = engine.next(state, BigInt(step.seed));
      served.push(next.index);
      state = engine.submit(next.state, next.index, step.response, null).state;
    }
    const result = engine.finish(state);

    expect(served).toEqual(golden.expected.items);
    result.thetas.forEach((v, i) => expect(Math.abs(v - golden.expected.thetas[i]!)).toBeLessThan(1e-9));
    result.se_thetas.forEach((v, i) => expect(Math.abs(v - golden.expected.se_thetas[i]!)).toBeLessThan(1e-9));
    engine.free();
  });

  test("engineVersion sama dengan versi paket", () => {
    const pkg = JSON.parse(readFileSync(new URL("../../pkg/package.json", import.meta.url), "utf8"));
    expect(engineVersion()).toBe(pkg.version);
  });
});

describe("state sesi", () => {
  test("state bisa disimpan sebagai JSON di tengah sesi tanpa mengubah hasil", () => {
    const engine = new Engine(golden.spec);
    let direct = engine.start(null);
    let viaJson = engine.start(null);
    for (const step of golden.steps.slice(0, 6)) {
      const a = engine.next(direct, BigInt(step.seed));
      const b = engine.next(clone(viaJson), BigInt(step.seed));
      expect(b.index).toBe(a.index);
      direct = engine.submit(a.state, a.index, step.response, 1500).state;
      viaJson = clone(engine.submit(clone(b.state), b.index, step.response, 1500).state);
    }
    expect(viaJson).toEqual(direct);
    expect(engine.finish(viaJson)).toEqual(engine.finish(direct));
  });

  test("start dengan theta dari Sesi 1 memakainya sebagai titik awal", () => {
    const engine = new Engine(golden.spec);
    const state = engine.start([0.4, -0.2, 0.1]);
    expect(state.person.theta).toEqual([0.4, -0.2, 0.1]);
  });

  test("state yang tidak cocok dengan bank ditolak, bukan membuat wasm trap", () => {
    const engine = new Engine(golden.spec);
    const state = clone(engine.start(null)) as SessionState;
    (state.person as { answered_items: number[] }).answered_items = [9999];
    (state.person as { responses: number[] }).responses = [1];
    expectMcatError(() => engine.next(state, 1n), "INVALID_INPUT");

    expectMcatError(() => engine.next({ person: {}, valid_items: [] } as never, 1n), "INVALID_INPUT");
    expectMcatError(() => engine.next(null as never, 1n), "INVALID_INPUT");
  });
});

describe("argumen dari JS", () => {
  const engine = new Engine(golden.spec);
  const state = engine.start(null);

  test.each([
    ["NaN", Number.NaN],
    ["pecahan", 1.5],
    ["negatif", -1],
    ["Infinity", Number.POSITIVE_INFINITY],
    ["di luar bank", 1000],
  ])("index %s ditolak", (_name, index) => {
    expectMcatError(() => engine.submit(state, index, 0, null), "INVALID_INPUT");
  });

  test("respons di luar kategori dan elapsedMs tidak valid ditolak", () => {
    expectMcatError(() => engine.submit(state, 0, 2, null), "INVALID_INPUT");
    expectMcatError(() => engine.submit(state, 0, 0.5, null), "INVALID_INPUT");
    expectMcatError(() => engine.submit(state, 0, 0, -5), "INVALID_INPUT");
  });

  test("seed harus bigint: number tidak dibulatkan diam-diam", () => {
    expect(() => engine.next(state, 5 as never)).toThrow();
  });
});

describe("spec dari JS", () => {
  test("melempar McatError dengan kode stabil", () => {
    const bad = clone(golden.spec);
    bad.items = [];
    expectMcatError(() => new Engine(bad), "INVALID_INPUT");

    const mismatch = clone(golden.spec);
    mismatch.group = { gmeans: [0, 0], gcov: [[1, 0], [0, 1]] };
    expectMcatError(() => new Engine(mismatch), "DIMENSION_MISMATCH");

    expectMcatError(() => new Engine(null as never), "INVALID_INPUT");
  });

  test("gcov berisi NaN ditolak", () => {
    const bad = clone(golden.spec);
    bad.group = { gmeans: [0, 0, 0], gcov: [[1, 0, 0], [0, 1, 0], [0, 0, Number.NaN]] };
    expectMcatError(() => new Engine(bad), "INVALID_INPUT");
  });

  test("field tak dikenal ditolak agar salah ketik tidak diam-diam diabaikan", () => {
    const bad = { ...clone(golden.spec), selecton: {} };
    expectMcatError(() => new Engine(bad as never), "INVALID_INPUT");
  });
});

describe("daur hidup", () => {
  test("engine yang sudah di-free tidak bisa dipakai", () => {
    const engine = new Engine(golden.spec);
    engine.free();
    expect(() => engine.start(null)).toThrow();
  });
});
