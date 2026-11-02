#!/usr/bin/env bash
# Bangun paket npm `@dayatech/theta-engine-wasm` ke wasm/pkg.
#
# Prasyarat: target `wasm32-unknown-unknown` dan `wasm-bindgen-cli` dengan versi
# SAMA PERSIS dengan `wasm-bindgen` di Cargo.lock (lih. wasm/README.md).
# `wasm-opt` (binaryen) opsional: dipakai bila ada.
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
out="$root/wasm/pkg"
cd "$root"

cargo build -p theta-engine-wasm --target wasm32-unknown-unknown --profile wasm-release --locked

rm -rf "$out"
wasm-bindgen --target nodejs --out-dir "$out" --typescript \
  target/wasm32-unknown-unknown/wasm-release/theta_engine_wasm.wasm

if command -v wasm-opt >/dev/null 2>&1; then
  wasm-opt -O3 --all-features "$out/theta_engine_wasm_bg.wasm" -o "$out/theta_engine_wasm_bg.wasm"
else
  echo "wasm-opt tidak ditemukan: melewati optimasi binaryen" >&2
fi

id="$(cargo pkgid -p theta-engine-wasm)"
version="${id##*[#@]}"

cat >"$out/package.json" <<EOF
{
  "name": "@dayatech/theta-engine-wasm",
  "version": "$version",
  "description": "Mesin MCAT theta-engine untuk Node/Bun (WebAssembly)",
  "license": "Apache-2.0",
  "repository": "github:irufano/theta-engine",
  "main": "theta_engine_wasm.js",
  "types": "theta_engine_wasm.d.ts",
  "files": [
    "theta_engine_wasm.js",
    "theta_engine_wasm.d.ts",
    "theta_engine_wasm_bg.wasm",
    "theta_engine_wasm_bg.wasm.d.ts",
    "README.md"
  ]
}
EOF

cp "$root/wasm/README.md" "$out/README.md"
# wasm-bindgen menulis .gitignore berisi "*" yang membuat `npm pack` melewati
# semua file; daftar `files` di atas sudah cukup.
rm -f "$out/.gitignore"
echo "paket siap: $out ($version)"
