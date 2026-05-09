#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"

if ! rustup target list --installed | grep -q '^wasm32-wasip1$'; then
  echo "Missing target wasm32-wasip1. Install with: rustup target add wasm32-wasip1" >&2
  exit 1
fi

build_plugin() {
  local manifest="$1"
  local binary="$2"
  local output="$3"

  cargo build --manifest-path "$manifest" --release --target wasm32-wasip1
  cp "$(dirname "$manifest")/target/wasm32-wasip1/release/$binary.wasm" "$output"
  echo "Built $output"
}

build_plugin "$ROOT_DIR/plugins/core-echo-rs/Cargo.toml" "core-echo-plugin" "$ROOT_DIR/plugins/core-echo-rs.wasm"
build_plugin "$ROOT_DIR/plugins/io-rs/Cargo.toml" "io-plugin" "$ROOT_DIR/plugins/io-rs.wasm"
