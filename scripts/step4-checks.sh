#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

echo "[1/4] Runtime conformance tests"
cargo test -p grapheme-runtime

echo "[2/4] Compiler negative fixture (invalid merge)"
if cargo run -- compile examples/v1-loop-invalid-merge-value.aql --emit mir >/tmp/grapheme-invalid-merge.out 2>/tmp/grapheme-invalid-merge.err; then
  echo "expected invalid merge fixture to fail, but compile succeeded" >&2
  exit 1
fi
if ! grep -qi "@loop merge must be one of replace|append|reduce|none" /tmp/grapheme-invalid-merge.err; then
  echo "expected invalid merge error message not found" >&2
  cat /tmp/grapheme-invalid-merge.err >&2
  exit 1
fi

echo "[3/4] Compiler crate unit tests"
cargo test -p grapheme-compiler

echo "[4/4] Loop benchmark"
bash scripts/benchmark-loop.sh

echo "Step 4 checks complete."
