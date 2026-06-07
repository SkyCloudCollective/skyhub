#!/usr/bin/env bash
# Build the dsp-wasm bridge and stage it for the web studio.
#
# Output is a self-contained .wasm (no wasm-bindgen, no JS glue): the studio's
# AudioWorklet instantiates it directly. The file is a build artifact (gitignored)
# — CI and local dev regenerate it from the Rust crates.
set -euo pipefail
cd "$(dirname "$0")/.."

# shellcheck disable=SC1090
[ -f "$HOME/.cargo/env" ] && . "$HOME/.cargo/env"

TARGET=wasm32-unknown-unknown
OUT=apps/web/static/dsp
SRC=target/$TARGET/release/dsp_wasm.wasm

echo "▶ building dsp-wasm ($TARGET, release)…"
cargo build -p dsp-wasm --target "$TARGET" --release

mkdir -p "$OUT"
cp "$SRC" "$OUT/dsp_wasm.wasm"

# Optional size pass if wasm-opt is on PATH (not required).
if command -v wasm-opt >/dev/null 2>&1; then
  echo "▶ wasm-opt -Oz…"
  wasm-opt -Oz "$OUT/dsp_wasm.wasm" -o "$OUT/dsp_wasm.wasm"
fi

echo "▶ headless ABI smoke test…"
node tools/wasm-smoke.mjs "$OUT/dsp_wasm.wasm"

printf '✓ staged %s (%s bytes)\n' "$OUT/dsp_wasm.wasm" "$(stat -c%s "$OUT/dsp_wasm.wasm")"
