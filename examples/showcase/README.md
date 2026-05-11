# Showcase Programs

These examples stress language/runtime features beyond basic hello-world flows.

## Files

- `fibonacci-threshold-loop.gr`
  - Iterative Fibonacci progression with arithmetic state transitions and branch-based early return by index.
- `fibonacci-threshold-typed.gr`
  - Same Fibonacci threshold flow with first-class `struct` and typed executable signatures.
- `fibonacci-threshold-namespaced.gr`
  - Uses `import types` and `Namespace::Type` signatures/initializers for cross-file struct reuse.
- `types-domain.gr`
  - Shared type declarations for namespaced examples.
- `poll-until-ready.gr`
  - Loop-until control flow with branch handlers and state mutation.
- `queue-triage-each.gr`
  - `@loop(each)` over object arrays, branch dispatch, and append merge mode.
- `transform-router.gr`
  - Native transform chain (`yaml.to_json` -> `json.parse`) plus control-flow routing.

Run any showcase:

```bash
cargo run -- run examples/showcase/<file>.gr --native-modules
```

Optional step-level trace:

```bash
cargo run -- run examples/showcase/<file>.gr --native-modules --stream-steps
```
