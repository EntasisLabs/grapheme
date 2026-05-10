#!/usr/bin/env bash
set -euo pipefail

echo "== V1 Computability Gate =="

# Primary conformance matrix (bounded/unbounded profile behavior).
bash scripts/policy-profile-checks.sh

echo "Computability gate passed."
