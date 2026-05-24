#!/usr/bin/env bash
set -euo pipefail

# Getting-started command path sanity checks.
cargo run -- parse examples/hello-world.gr > /tmp/grapheme-docs-parse.out

RUN_OUT="$(cargo run -- run examples/hello-world.gr --json)"
echo "$RUN_OUT" | rg '"outcome": "succeeded"' >/dev/null
echo "$RUN_OUT" | rg '"message": "LETS GO\?!!!!!"' >/dev/null

echo "Docs smoke checks passed."
