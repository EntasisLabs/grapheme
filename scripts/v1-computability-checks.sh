#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd -- "$SCRIPT_DIR/.." && pwd)"
cd "$REPO_ROOT"

echo "== V1 Computability Gate =="

# Primary conformance matrix (bounded/unbounded profile behavior).
bash "$SCRIPT_DIR/policy-profile-checks.sh"

echo "Computability gate passed."
