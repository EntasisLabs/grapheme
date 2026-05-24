# LLM-Friendliness Checklist

Purpose: make Grapheme pipelines more predictable for autonomous agents while preserving composability.

## Status Legend

- `[x]` shipped
- `[ ]` planned

## Checklist

- [x] Non-mutating debug passthrough op (`core.tap`)
  - Pain point: `core.echo` rewrites `$current`, which can accidentally break downstream steps.
  - Change: add `core.tap(message?: String)` that preserves `__input` as the operation output.
  - Backward compatibility: additive op; no behavior change to existing `core.echo` programs.
  - Example:

```gr
http.get(url: "https://example.com")
|> core.tap(message: "fetched")
|> core.get_path(path: "body")
```

- [x] Flow/data split helper set (`core.pack_state_data`, `core.get_state`, `core.get_data`)
  - Pain point: flow transitions and host payloads compete for the same root shape.
  - Change: add helper ops that standardize a two-lane envelope for control and payload.
  - Backward compatibility: additive helpers; existing pipelines remain valid.
  - Example:

```gr
websearch.research_materials(query: "rust async runtime patterns")
|> core.pack_state_data(state: { phase: "collecting" })
|> core.get_data
|> core.get_path(path: "sources")
```

- [ ] Flow/data split contract (`current.state` + `current.data` convention)
  - Pain point: flow transitions and host payloads compete for the same root shape.
  - Change: define and document canonical split, plus helper transforms.
  - Backward compatibility: initially documented convention + optional helper ops.

- [ ] Host return envelope normalization (`{ data, meta, error }`)
  - Pain point: mixed return shapes force brittle path access and recovery logic.
  - Change: normalize host module outputs under a stable envelope.
  - Backward compatibility: dual-read compatibility layer during migration window.

- [ ] Match-branch guard ergonomics
  - Pain point: inline `if` in `match` branches is not accepted in common forms and causes agent retries.
  - Change: add grammar support for branch guards or compact conditional targets.
  - Backward compatibility: additive grammar extension.

- [x] Shape-clobber lint prototype (advisory warning)
  - Pain point: accidental object-to-scalar/object-to-envelope transitions are hard to detect early.
  - Change: verifier emits warning when `core.echo` is followed by a step that reads non-`message` `$current.<field>` values.
  - Backward compatibility: warning-only prototype; no compile failure.
  - Delivery: warnings are exposed as structured `lint_warnings` in CLI `run --json` output.

- [ ] Shape-clobber diagnostics profile
  - Pain point: accidental object-to-scalar/object-to-envelope transitions are hard to detect early.
  - Change: lints/warnings when step output shape diverges from observed/expected workflow shape.
  - Backward compatibility: opt-in profile first (`--lint-profile llm`).

## Next Slice

- Expand helper-set usage in additional state-machine showcase examples.
- Move advisory lint into an opt-in profile flag (`--lint-profile llm`) with structured output.
