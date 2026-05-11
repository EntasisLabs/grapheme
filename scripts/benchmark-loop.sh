#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

EXAMPLE="examples/fixtures/v1-loop-max-fixed.gr"
TIMING_MODE="${GRAPHEME_BENCH_TIMING:-0}"

run_case() {
  local label="$1"
  shift

  echo "== $label =="
  if command -v /usr/bin/time >/dev/null 2>&1; then
    if [[ "$TIMING_MODE" == "1" ]]; then
      env GRAPHEME_RUNTIME_TIMING=1 /usr/bin/time -f "elapsed=%E rss_kb=%M" cargo run -- run "$EXAMPLE" --native-modules "$@" >/tmp/grapheme-bench.out
    else
      /usr/bin/time -f "elapsed=%E rss_kb=%M" cargo run -- run "$EXAMPLE" --native-modules "$@" >/tmp/grapheme-bench.out
    fi
  else
    # Fallback when GNU time is unavailable; capture elapsed seconds only.
    local started
    local ended
    local elapsed
    started="$(date +%s)"
    if [[ "$TIMING_MODE" == "1" ]]; then
      GRAPHEME_RUNTIME_TIMING=1 cargo run -- run "$EXAMPLE" --native-modules "$@" >/tmp/grapheme-bench.out
    else
      cargo run -- run "$EXAMPLE" --native-modules "$@" >/tmp/grapheme-bench.out
    fi
    ended="$(date +%s)"
    elapsed="$((ended - started))"
    echo "elapsed_s=$elapsed"
  fi

  local lines
  lines="$(wc -l < /tmp/grapheme-bench.out | tr -d ' ')"
  echo "output_lines=$lines"
  echo
}

run_case "lean" --trace-profile lean --trace-steps 128 --trace-projection minimal
run_case "debug" --trace-profile debug

echo "Benchmark complete."
if [[ "$TIMING_MODE" == "1" ]]; then
  echo "Timing diagnostics were enabled via GRAPHEME_RUNTIME_TIMING=1."
fi
