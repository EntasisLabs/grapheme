#!/usr/bin/env bash
set -euo pipefail

# Aggregates exported telemetry JSON reports into a weekly markdown summary.
# Usage:
#   scripts/telemetry-weekly-rollup.sh [reports_dir] [--out path]

REPORTS_DIR="${1:-.grapheme/telemetry/reports}"
OUT_PATH=""

if [[ $# -ge 2 ]]; then
  if [[ "${2}" == "--out" ]]; then
    if [[ $# -lt 3 ]]; then
      echo "error: --out requires a path" >&2
      exit 1
    fi
    OUT_PATH="${3}"
  else
    echo "error: unknown flag '${2}'" >&2
    exit 1
  fi
fi

if [[ ! -d "${REPORTS_DIR}" ]]; then
  echo "error: reports directory not found: ${REPORTS_DIR}" >&2
  exit 1
fi

REPORT="$(python3 - "${REPORTS_DIR}" <<'PY'
import json
import pathlib
import statistics
import sys

reports_dir = pathlib.Path(sys.argv[1])
files = sorted(reports_dir.glob("*.json"))

totals = {
    "report_count": 0,
    "event_count": 0,
    "command_result_count": 0,
    "success_count": 0,
    "failure_count": 0,
    "ttfs_start_count": 0,
    "ttfs_success_count": 0,
    "ttfs_failure_count": 0,
}

durations = []
command_counts = {}
error_counts = {}
failure_stage_counts = {}

def add_count(bucket, key, value):
    bucket[key] = bucket.get(key, 0) + int(value)

for f in files:
    try:
        data = json.loads(f.read_text(encoding="utf-8"))
    except Exception:
        continue

    summary = data.get("summary", data)
    totals["report_count"] += 1
    for key in [
        "event_count",
        "command_result_count",
        "success_count",
        "failure_count",
        "ttfs_start_count",
        "ttfs_success_count",
        "ttfs_failure_count",
    ]:
        totals[key] += int(summary.get(key, 0) or 0)

    avg_ms = summary.get("avg_duration_ms")
    if isinstance(avg_ms, (int, float)):
        durations.append(float(avg_ms))

    for k, v in (summary.get("command_counts") or {}).items():
        add_count(command_counts, k, v)

    for k, v in (summary.get("failure_stage_counts") or {}).items():
        add_count(failure_stage_counts, k, v)

    for row in (summary.get("top_error_classes") or []):
        name = row.get("error_class")
        count = row.get("count")
        if name is None or count is None:
            continue
        add_count(error_counts, name, count)

if totals["report_count"] == 0:
    print("# Weekly Telemetry Rollup\n\nNo valid JSON telemetry reports found.")
    sys.exit(0)

success_rate = 0.0
if totals["command_result_count"]:
    success_rate = totals["success_count"] / totals["command_result_count"]

ttfs_success_rate = 0.0
if totals["ttfs_start_count"]:
    ttfs_success_rate = totals["ttfs_success_count"] / totals["ttfs_start_count"]

avg_duration = statistics.mean(durations) if durations else None

def top_rows(d, limit=5):
    return sorted(d.items(), key=lambda kv: (-kv[1], kv[0]))[:limit]

lines = []
lines.append("# Weekly Telemetry Rollup")
lines.append("")
lines.append("## Coverage")
lines.append("")
lines.append(f"- reports: {totals['report_count']}")
lines.append(f"- events: {totals['event_count']}")
lines.append(f"- command_results: {totals['command_result_count']}")
lines.append(f"- success_rate: {success_rate:.2%}")
if avg_duration is None:
    lines.append("- avg_duration_ms: n/a")
else:
    lines.append(f"- avg_duration_ms: {avg_duration:.2f}")
lines.append("")
lines.append("## TTFS Funnel")
lines.append("")
lines.append(f"- starts: {totals['ttfs_start_count']}")
lines.append(f"- successes: {totals['ttfs_success_count']}")
lines.append(f"- failures: {totals['ttfs_failure_count']}")
lines.append(f"- success_rate: {ttfs_success_rate:.2%}")
lines.append("")

if failure_stage_counts:
    lines.append("## Top Failure Stages")
    lines.append("")
    for name, count in top_rows(failure_stage_counts):
        lines.append(f"- {name}: {count}")
    lines.append("")

if error_counts:
    lines.append("## Top Error Classes")
    lines.append("")
    for name, count in top_rows(error_counts):
        lines.append(f"- {name}: {count}")
    lines.append("")

if command_counts:
    lines.append("## Top Commands")
    lines.append("")
    for name, count in top_rows(command_counts):
        lines.append(f"- {name}: {count}")
    lines.append("")

print("\n".join(lines))
PY
)"

if [[ -n "${OUT_PATH}" ]]; then
  mkdir -p "$(dirname "${OUT_PATH}")"
  printf "%s\n" "${REPORT}" > "${OUT_PATH}"
  echo "Wrote weekly telemetry rollup to ${OUT_PATH}"
else
  printf "%s\n" "${REPORT}"
fi