#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
PLUGIN_DIR="$ROOT_DIR/plugins/core-echo-rs"
OUT_DIR="$ROOT_DIR/plugins"
OUT_FILE="$OUT_DIR/core-echo-rs.wasm"

if ! rustup target list --installed | grep -q '^wasm32-wasip1$'; then
  echo "Missing target wasm32-wasip1. Install with: rustup target add wasm32-wasip1" >&2
  exit 1
fi

cargo build --manifest-path "$PLUGIN_DIR/Cargo.toml" --release --target wasm32-wasip1
cp "$PLUGIN_DIR/target/wasm32-wasip1/release/core-echo-plugin.wasm" "$OUT_FILE"

echo "Built $OUT_FILE"
