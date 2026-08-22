# Language Tour

This page gives a conceptual map of Grapheme syntax and workflow structure.

## Mental Model

A Grapheme program defines one or more executable units (for example, queries or mutations) that compose module operations into a stateful flow.

Think in three layers:

1. intent (what outcome you want),
2. flow (how data moves and branches),
3. capabilities (which modules perform side effects).

## Building Blocks

- Imports: declare module capabilities.
- Executables: define named workflow entry points (`query`, `mutation`, `iterator`, …).
- **Parameters (0.7.0):** named `$param` lists with defaults; bind via `call` or CLI `--args-json`.
- **Tags / `using` (0.7.0):** ambient tagged bindings activated for a scoped block.
- Operations: call module functions.
- State transitions: evolve structured data through steps.
- Control flow: branch and iterate with explicit intent.
- **Stage B AOT (0.7.0):** compile/run workflows through the Wasm container path (`grapheme build` defaults to `stage_b`).

## Authoring Style

Prefer:

- explicit data shaping,
- small composable steps,
- stable module ops unless experimental behavior is required.

Avoid:

- hidden side-effect assumptions,
- monolithic workflow blobs,
- overloading one executable with unrelated jobs.

## Safety Model

Grapheme is designed to run with policy boundaries around side effects.

In practice, that means your workflow logic and operational permissions stay separate:

- source defines intent,
- runtime policy defines allowed external actions.

## Learn by Running

Use these examples to see core language patterns:

- `examples/hello-world.gr`
- `examples/params-call-bind.gr` (executable parameters)
- `examples/tag-using-scope.gr` (tags + scoped `using`)
- `examples/core-merge.gr`
- `examples/core-filter.gr`
- `examples/resilience-composition.gr`
- `examples/mutation-state-machine-apply.gr`

Author extract: `docs/internal/language/params-and-tags-v1.md`.

Then progress to scenario playbooks in `playbooks.md`.
