# Showcase Programs

These examples stress language/runtime features beyond basic hello-world flows.

## Files

- `fibonacci-threshold-loop.aql`
  - Iterative Fibonacci progression with arithmetic state transitions and branch-based early return by index.
- `fibonacci-threshold-typed.aql`
  - Same Fibonacci threshold flow with first-class `struct` and typed executable signatures.
- `poll-until-ready.aql`
  - Loop-until control flow with branch handlers and state mutation.
- `queue-triage-each.aql`
  - `@loop(each)` over object arrays, branch dispatch, and append merge mode.
- `transform-router.aql`
  - Native transform chain (`yaml.to_json` -> `json.parse`) plus control-flow routing.

Run any showcase:

```bash
cargo run -- run examples/showcase/<file>.aql --native-modules
```

Optional step-level trace:

```bash
cargo run -- run examples/showcase/<file>.aql --native-modules --stream-steps
```
