#!/usr/bin/env bash
set -euo pipefail

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

if ! rg -F "$EXPECTED_ATTR" "$ROOT_PARSER" >/dev/null; then
  echo "Root parser must reference canonical grammar via:"
  echo "  $EXPECTED_ATTR"
  echo "File: $ROOT_PARSER"
  exit 1
fi

echo "Single-source grammar contract check passed."
