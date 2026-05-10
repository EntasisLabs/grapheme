# Policy Profile Matrix

This document defines a private conformance matrix for runtime policy profiles.

Goal:

- Keep language semantics unbounded where intended.
- Prove runtime policy can bound execution deterministically.
- Keep examples safe to run during local development.

## Profiles

- bounded: explicit runtime caps via env vars.
- unbounded: runtime caps disabled (`none`) for selected controls.

## Matrix

1. loop_unbounded_syntax_bounded_policy
- Program: `examples/v1-loop-unbounded-budgeted.aql`
- Env: `GRAPHEME_RUNTIME_MAX_STEPS=10`
- Expected: fatal failure with `STEP_BUDGET_EXCEEDED`

2. recursion_unbounded_syntax_bounded_policy
- Program: `examples/v1-recursive-policy-bounded.aql`
- Env: `GRAPHEME_RUNTIME_MAX_CALL_DEPTH=4`
- Expected: fatal failure with `MAX_CALL_DEPTH_EXCEEDED`

3. recursion_unbounded_syntax_unbounded_policy_terminates
- Program: `examples/v1-flow-branch-recursive-unbounded.aql`
- Env: `GRAPHEME_RUNTIME_MAX_CALL_DEPTH=none`
- Expected: succeeded

4. loop_bounded_syntax_default_policy
- Program: `examples/v1-loop-max-fixed.aql`
- Env: default runtime policy
- Expected: succeeded

5. while_counter_unbounded_profile_terminates
- Program: `examples/v1-while-counter.aql`
- Env: `GRAPHEME_RUNTIME_MAX_STEPS=none`, `GRAPHEME_RUNTIME_MAX_CALL_DEPTH=none`
- Expected: succeeded

6. partial_function_bounded_policy_fails
- Program: `examples/v1-partial-diverge.aql`
- Env: `GRAPHEME_RUNTIME_MAX_STEPS=12`, `GRAPHEME_RUNTIME_MAX_CALL_DEPTH=none`
- Expected: fatal failure with `STEP_BUDGET_EXCEEDED`

7. minsky_transfer_unbounded_profile_succeeds
- Program: `examples/v1-minsky-transfer.aql`
- Env: `GRAPHEME_RUNTIME_MAX_STEPS=none`, `GRAPHEME_RUNTIME_MAX_CALL_DEPTH=none`
- Expected: succeeded

8. minsky_transfer_bounded_policy_fails
- Program: `examples/v1-minsky-transfer.aql`
- Env: `GRAPHEME_RUNTIME_MAX_STEPS=5`, `GRAPHEME_RUNTIME_MAX_CALL_DEPTH=none`
- Expected: fatal failure with `STEP_BUDGET_EXCEEDED`

9. minsky_branching_unbounded_profile_succeeds
- Program: `examples/v1-minsky-branching.aql`
- Env: `GRAPHEME_RUNTIME_MAX_STEPS=none`, `GRAPHEME_RUNTIME_MAX_CALL_DEPTH=none`
- Expected: succeeded

10. minsky_branching_bounded_policy_fails
- Program: `examples/v1-minsky-branching.aql`
- Env: `GRAPHEME_RUNTIME_MAX_STEPS=6`, `GRAPHEME_RUNTIME_MAX_CALL_DEPTH=none`
- Expected: fatal failure with `STEP_BUDGET_EXCEEDED`

## One-command Validation

Run:

```bash
bash scripts/policy-profile-checks.sh
```

The script exits non-zero if any expectation fails.
