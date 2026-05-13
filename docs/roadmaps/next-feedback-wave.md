# Next Feedback Wave

This roadmap captures the latest language/tooling feedback and proposed rollout order.

## Scope From Feedback

1. Rename/align reusable unit keyword from `iterator` to `node`.
2. Allow more primitive-oriented ergonomics similar to struct usage.
3. Expand standard transformation/utilities for mapping, string ops, and general-purpose workflows.
4. Add composition sugar for resilience policies.
5. Add intent annotation that is visible in trace output.
6. Add cargo-like module discovery commands.

## Current Status

Implemented in this wave:

1. `node` keyword support as a non-breaking alias for `iterator`.
2. CLI modules discovery subcommands:
   - `grapheme modules search <query>`
   - `grapheme modules info <module>`
   - `grapheme modules types <module>`
   - `grapheme modules examples <module>`
3. `@resilient` directive sugar lowering to `@loop`, `@retry`, `@timeout`.

## Proposed Phases

### Phase 1: Compatibility and Discoverability (Done)

- Add `node` syntax alias with no behavior change.
- Extend `modules` CLI UX for search/info/types/examples.

### Phase 2: Resilience Composition Sugar (Done)

Target syntax (draft):

```grapheme
@resilient(
  loop: { max: 32 },
  retry: { max: 2, backoff_ms: 50, on_fail: Fallback },
  timeout: { ms: 2500, on_timeout: TimeoutPath }
)
```

Lowering plan:

- Expand `@resilient(...)` into `@loop(...)`, `@retry(...)`, `@timeout(...)` at compile time.
- Preserve existing verifier/runtime semantics and errors.
- Reject mixing conflicting nested and top-level directive values.

### Phase 3: Intent Annotation + Trace Surfacing

Target syntax (draft):

```grapheme
@intent(message: "why this workflow exists")
```

Behavior plan:

- Directive accepted on executable definitions and optionally on pipeline steps.
- Propagate intent string into MIR metadata.
- Emit intent in runtime trace entries and stream output.

### Phase 4: Primitive-Centric Authoring Ergonomics

Candidate items:

- Clarify and extend scalar/list signatures for `query`/`mutation`/`node`.
- Add explicit literal initialization guidance where struct init is not required.
- Add typed examples for scalar/list-first workflows.

### Phase 5: Standard Library Expansion

Priority utility additions:

- Vector/list: `map`, `flat_map`, `reduce`, `find`, `sort_by`, `group_by`.
- String: `split`, `join`, `replace`, `trim`, `lower`, `upper`, `contains`.
- Object/data: `keys`, `values`, `has`, `get_path`, `set_path`.

Delivery pattern:

- Add op manifests + verifier arg checks.
- Add native module implementations and examples.
- Add module docs and `grapheme modules examples <module>` mappings.

## Acceptance Gates

For each phase:

1. Compiler tests for parse/lower/verify behavior.
2. Runtime tests for new execution semantics.
3. At least one curated showcase or cookbook example.
4. Docs update in language contract and CLI reference.
