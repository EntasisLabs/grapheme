#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
OUT_DIR="$ROOT_DIR/crates/grapheme-aot-container/assets"
OUT_WASM="$OUT_DIR/grapheme-aot-container.wasm"

if ! rustup target list --installed | grep -q '^wasm32-wasip1$'; then
  echo "Missing target wasm32-wasip1. Install with: rustup target add wasm32-wasip1" >&2
  exit 1
fi

mkdir -p "$OUT_DIR"
cargo build -p grapheme-aot-container --release --target wasm32-wasip1

WASM_PATH="$ROOT_DIR/target/wasm32-wasip1/release/grapheme-aot-container.wasm"
if [[ ! -f "$WASM_PATH" && -n "${CARGO_TARGET_DIR:-}" ]]; then
  WASM_PATH="${CARGO_TARGET_DIR}/wasm32-wasip1/release/grapheme-aot-container.wasm"
fi

if [[ ! -f "$WASM_PATH" ]]; then
  echo "Wasm artifact not found at $WASM_PATH" >&2
  exit 1
fi

cp "$WASM_PATH" "$OUT_WASM"
echo "Built $OUT_WASM ($(wc -c < "$OUT_WASM") bytes)"
