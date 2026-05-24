#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd -- "$SCRIPT_DIR/.." && pwd)"
cd "$REPO_ROOT"

# Getting-started command path sanity checks.
cargo run -- parse examples/hello-world.gr > /tmp/grapheme-docs-parse.out

RUN_OUT="$(cargo run -- run examples/hello-world.gr --json)"
if command -v rg >/dev/null 2>&1; then
	echo "$RUN_OUT" | rg '"outcome": "succeeded"' >/dev/null
	echo "$RUN_OUT" | rg '"message": "LETS GO\?!!!!!"' >/dev/null
else
	echo "$RUN_OUT" | grep -E '"outcome": "succeeded"' >/dev/null
	echo "$RUN_OUT" | grep -E '"message": "LETS GO\?!!!!!"' >/dev/null
fi

echo "Docs smoke checks passed."
