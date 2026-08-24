#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"

if ! rustup target list --installed | grep -q '^wasm32-wasip1$'; then
  echo "Missing target wasm32-wasip1. Install with: rustup target add wasm32-wasip1" >&2
  exit 1
fi

cargo build -p grapheme-wasm --release --target wasm32-wasip1

WASM_PATH="$ROOT_DIR/target/wasm32-wasip1/release/grapheme-wasm.wasm"
if [[ ! -f "$WASM_PATH" && -n "${CARGO_TARGET_DIR:-}" ]]; then
  WASM_PATH="${CARGO_TARGET_DIR}/wasm32-wasip1/release/grapheme-wasm.wasm"
fi

if [[ ! -f "$WASM_PATH" ]]; then
  echo "Wasm artifact not found at $WASM_PATH" >&2
  exit 1
fi

echo "Built $WASM_PATH ($(wc -c < "$WASM_PATH") bytes)"
