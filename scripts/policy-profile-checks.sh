#!/usr/bin/env bash
set -euo pipefail

pass() {
  printf "[PASS] %s\n" "$1"
}

fail() {
  printf "[FAIL] %s\n" "$1" >&2
  exit 1
}

run_and_capture() {
  local label="$1"
  shift
  local out
  if ! out=$("$@" 2>&1); then
    printf "%s\n" "$out" >&2
    fail "$label"
  fi
  printf "%s" "$out"
}

echo "== Policy Profile Checks =="

# 1) Unbounded loop syntax, bounded by step budget policy.
out1=$(run_and_capture \
  "loop_unbounded_syntax_bounded_policy execution" \
  env GRAPHEME_RUNTIME_MAX_STEPS=10 cargo run -- run examples/v1-loop-unbounded-budgeted.aql --native-modules --json)

echo "$out1" | grep -q '"outcome": "fatal_failure"' || fail "loop_unbounded_syntax_bounded_policy expected fatal_failure"
echo "$out1" | grep -q '"code": "STEP_BUDGET_EXCEEDED"' || fail "loop_unbounded_syntax_bounded_policy expected STEP_BUDGET_EXCEEDED"
pass "loop_unbounded_syntax_bounded_policy"

# 2) Unbounded recursion syntax, bounded by call-depth policy.
out2=$(run_and_capture \
  "recursion_unbounded_syntax_bounded_policy execution" \
  env GRAPHEME_RUNTIME_MAX_CALL_DEPTH=4 cargo run -- run examples/v1-recursive-policy-bounded.aql --native-modules --json)

echo "$out2" | grep -q '"outcome": "fatal_failure"' || fail "recursion_unbounded_syntax_bounded_policy expected fatal_failure"
echo "$out2" | grep -q '"code": "MAX_CALL_DEPTH_EXCEEDED"' || fail "recursion_unbounded_syntax_bounded_policy expected MAX_CALL_DEPTH_EXCEEDED"
pass "recursion_unbounded_syntax_bounded_policy"

# 3) Unbounded recursion syntax + unbounded call-depth policy, but program terminates via branch return.
out3=$(run_and_capture \
  "recursion_unbounded_syntax_unbounded_policy_terminates execution" \
  env GRAPHEME_RUNTIME_MAX_CALL_DEPTH=none cargo run -- run examples/v1-flow-branch-recursive-unbounded.aql --native-modules --json)

echo "$out3" | grep -q '"outcome": "succeeded"' || fail "recursion_unbounded_syntax_unbounded_policy_terminates expected succeeded"
pass "recursion_unbounded_syntax_unbounded_policy_terminates"

# 4) Bounded loop syntax under default policy should succeed.
out4=$(run_and_capture \
  "loop_bounded_syntax_default_policy execution" \
  cargo run -- run examples/v1-loop-max-fixed.aql --native-modules --json)

echo "$out4" | grep -q '"outcome": "succeeded"' || fail "loop_bounded_syntax_default_policy expected succeeded"
pass "loop_bounded_syntax_default_policy"

# 5) While-like counter should terminate under unbounded runtime policy.
out5=$(run_and_capture \
  "while_counter_unbounded_profile_terminates execution" \
  env GRAPHEME_RUNTIME_MAX_STEPS=none GRAPHEME_RUNTIME_MAX_CALL_DEPTH=none \
    cargo run -- run examples/v1-while-counter.aql --native-modules --json)

echo "$out5" | grep -q '"outcome": "succeeded"' || fail "while_counter_unbounded_profile_terminates expected succeeded"
pass "while_counter_unbounded_profile_terminates"

# 6) Partial function should fail deterministically under bounded step budget.
out6=$(run_and_capture \
  "partial_function_bounded_policy_fails execution" \
  env GRAPHEME_RUNTIME_MAX_STEPS=12 GRAPHEME_RUNTIME_MAX_CALL_DEPTH=none \
    cargo run -- run examples/v1-partial-diverge.aql --native-modules --json)

echo "$out6" | grep -q '"outcome": "fatal_failure"' || fail "partial_function_bounded_policy_fails expected fatal_failure"
echo "$out6" | grep -q '"code": "STEP_BUDGET_EXCEEDED"' || fail "partial_function_bounded_policy_fails expected STEP_BUDGET_EXCEEDED"
pass "partial_function_bounded_policy_fails"

# 7) 2-counter Minsky-style transfer should succeed under unbounded profile.
out7=$(run_and_capture \
  "minsky_transfer_unbounded_profile_succeeds execution" \
  env GRAPHEME_RUNTIME_MAX_STEPS=none GRAPHEME_RUNTIME_MAX_CALL_DEPTH=none \
    cargo run -- run examples/v1-minsky-transfer.aql --native-modules --json)

echo "$out7" | grep -q '"outcome": "succeeded"' || fail "minsky_transfer_unbounded_profile_succeeds expected succeeded"
pass "minsky_transfer_unbounded_profile_succeeds"

# 8) Same transfer should fail deterministically under bounded step budget.
out8=$(run_and_capture \
  "minsky_transfer_bounded_policy_fails execution" \
  env GRAPHEME_RUNTIME_MAX_STEPS=5 GRAPHEME_RUNTIME_MAX_CALL_DEPTH=none \
    cargo run -- run examples/v1-minsky-transfer.aql --native-modules --json)

echo "$out8" | grep -q '"outcome": "fatal_failure"' || fail "minsky_transfer_bounded_policy_fails expected fatal_failure"
echo "$out8" | grep -q '"code": "STEP_BUDGET_EXCEEDED"' || fail "minsky_transfer_bounded_policy_fails expected STEP_BUDGET_EXCEEDED"
pass "minsky_transfer_bounded_policy_fails"

# 9) Branching 2-counter program should succeed under unbounded profile.
out9=$(run_and_capture \
  "minsky_branching_unbounded_profile_succeeds execution" \
  env GRAPHEME_RUNTIME_MAX_STEPS=none GRAPHEME_RUNTIME_MAX_CALL_DEPTH=none \
    cargo run -- run examples/v1-minsky-branching.aql --native-modules --json)

echo "$out9" | grep -q '"outcome": "succeeded"' || fail "minsky_branching_unbounded_profile_succeeds expected succeeded"
pass "minsky_branching_unbounded_profile_succeeds"

# 10) Same branching program should fail deterministically under bounded step budget.
out10=$(run_and_capture \
  "minsky_branching_bounded_policy_fails execution" \
  env GRAPHEME_RUNTIME_MAX_STEPS=6 GRAPHEME_RUNTIME_MAX_CALL_DEPTH=none \
    cargo run -- run examples/v1-minsky-branching.aql --native-modules --json)

echo "$out10" | grep -q '"outcome": "fatal_failure"' || fail "minsky_branching_bounded_policy_fails expected fatal_failure"
echo "$out10" | grep -q '"code": "STEP_BUDGET_EXCEEDED"' || fail "minsky_branching_bounded_policy_fails expected STEP_BUDGET_EXCEEDED"
pass "minsky_branching_bounded_policy_fails"

echo "All policy profile checks passed."
