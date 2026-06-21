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
  local crate_dir
  crate_dir="$(dirname "$manifest")"

  cargo build --manifest-path "$manifest" --release --target wasm32-wasip1

  local wasm_path="$crate_dir/target/wasm32-wasip1/release/${binary}.wasm"
  if [[ ! -f "$wasm_path" && -n "${CARGO_TARGET_DIR:-}" ]]; then
    wasm_path="${CARGO_TARGET_DIR}/wasm32-wasip1/release/${binary}.wasm"
  fi
  if [[ ! -f "$wasm_path" && -f "$ROOT_DIR/target/wasm32-wasip1/release/${binary}.wasm" ]]; then
    wasm_path="$ROOT_DIR/target/wasm32-wasip1/release/${binary}.wasm"
  fi

  if [[ ! -f "$wasm_path" ]]; then
    echo "Wasm artifact not found for ${binary}; expected under ${crate_dir}/target or workspace target" >&2
    exit 1
  fi

  cp "$wasm_path" "$output"
  echo "Built $output"
}

build_plugin "$ROOT_DIR/plugins/core-echo-rs/Cargo.toml" "core-echo-plugin" "$ROOT_DIR/plugins/core-echo-rs.wasm"
build_plugin "$ROOT_DIR/plugins/io-rs/Cargo.toml" "io-plugin" "$ROOT_DIR/plugins/io-rs.wasm"
build_plugin "$ROOT_DIR/plugins/pdf-rs/Cargo.toml" "pdf-plugin" "$ROOT_DIR/plugins/pdf-rs.wasm"
cp "$ROOT_DIR/plugins/pdf-rs.wasm" "$ROOT_DIR/modules/pdf.wasm"
echo "Copied modules/pdf.wasm"
build_plugin "$ROOT_DIR/plugins/image-rs/Cargo.toml" "image-plugin" "$ROOT_DIR/plugins/image-rs.wasm"
cp "$ROOT_DIR/plugins/image-rs.wasm" "$ROOT_DIR/modules/image.wasm"
echo "Copied modules/image.wasm"
build_plugin "$ROOT_DIR/plugins/plot-rs/Cargo.toml" "plot-plugin" "$ROOT_DIR/plugins/plot-rs.wasm"
cp "$ROOT_DIR/plugins/plot-rs.wasm" "$ROOT_DIR/modules/plot.wasm"
echo "Copied modules/plot.wasm"
