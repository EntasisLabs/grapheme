#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd -- "$SCRIPT_DIR/.." && pwd)"
cd "$REPO_ROOT"

CANONICAL_GRAMMAR="crates/grapheme-compiler/src/grapheme.pest"
LEGACY_DUPLICATE="src/grapheme.pest"
ROOT_PARSER="src/parser.rs"
EXPECTED_ATTR='#[grammar = "../crates/grapheme-compiler/src/grapheme.pest"]'

if [[ ! -f "$CANONICAL_GRAMMAR" ]]; then
  echo "Missing canonical grammar: $CANONICAL_GRAMMAR"
  exit 1
fi

if [[ -f "$LEGACY_DUPLICATE" ]]; then
  echo "Duplicate grammar source must not exist: $LEGACY_DUPLICATE"
  echo "Use only: $CANONICAL_GRAMMAR"
  exit 1
fi

if command -v rg >/dev/null 2>&1; then
  MATCH_CMD=(rg -F "$EXPECTED_ATTR" "$ROOT_PARSER")
else
  MATCH_CMD=(grep -F "$EXPECTED_ATTR" "$ROOT_PARSER")
fi

if ! "${MATCH_CMD[@]}" >/dev/null; then
  echo "Root parser must reference canonical grammar via:"
  echo "  $EXPECTED_ATTR"
  echo "File: $ROOT_PARSER"
  exit 1
fi

echo "Single-source grammar contract check passed."
