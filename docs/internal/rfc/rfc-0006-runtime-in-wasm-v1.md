# RFC-0006: Runtime-in-Wasm v1

Status: accepted-for-implementation
Authors: runtime + compiler
Created: 2026-08-24
Target release window: after RFC-0005 Stage B (Track 4)

## Summary

Ship the **Grapheme runtime itself** as a Wasm module so hosts can **run Grapheme in Wasm** (browser, WASI, edge), rather than only using Wasm as a plugin/AOT-container backend.

This is a different axis from existing Track 4 work:

| Path | What is Wasm? | Who interprets MIR? |
| --- | --- | --- |
| Wasix plugins | capability modules | native `grapheme-runtime` hosts Wasm |
| Stage B AOT container | one workflow walker | slim in-container walker (`grapheme-aot-container`) |
| **Runtime-in-Wasm (this RFC)** | the runtime engine | full `RuntimeEngine` (+ optional compiler) lives inside Wasm |

## Probe results (current tree)

These `cargo check` recipes already succeed. Compiling the crates is **not** the blocker; **productizing an entrypoint** is.

```bash
# Runtime interpreter + policy (no Wasix)
cargo check -p grapheme-runtime --target wasm32-wasip1
cargo check -p grapheme-runtime --no-default-features --target wasm32-unknown-unknown

# Compiler (no Stage B container bytes)
cargo check -p grapheme-compiler --no-default-features --target wasm32-wasip1
cargo check -p grapheme-compiler --no-default-features --target wasm32-unknown-unknown

# Slim SDK (core/json compile + execute)
cargo check -p grapheme-sdk --no-default-features --features slim --target wasm32-wasip1
cargo check -p grapheme-sdk --no-default-features --features slim --target wasm32-unknown-unknown
```

Hard fail (by design):

```bash
cargo check -p grapheme-runtime --features wasix-runtime --target wasm32-wasip1
# aws-lc-sys / Wasmer / tokio host stack cannot and should not target Wasm
```

Wasix is a **native host** that *runs* Wasm plugins. Nesting it inside Wasm is wasm-in-wasm and pulls TLS/Cranelift. Runtime-in-Wasm must keep `wasix-runtime` off.

## Motivation

Embedders want Grapheme on Wasm hosts:

- browser playgrounds / VS Code web
- WASI CLI (`wasmtime grapheme-wasm.wasm < request.json`)
- edge workers that cannot ship a native `grapheme` binary

Today the only in-Wasm execute surface is `grapheme-aot-container`: a slim MIR walker for Stage B workflow *containers*. It does not expose `RuntimeEngine` policy, traces, or source compilation.

## Goals

1. Provide a WASI binary (`grapheme-wasm`) that links `grapheme-compiler` + `grapheme-runtime` + wasm-safe stdlib and executes source or artifacts.
2. Keep capability policy and MIR interpretation identical to the native interpreter path for wasm-safe ops (`core` / `json` / `csv` / `yaml` / `html`).
3. Gate runtime-in-Wasm compilability in CI (runtime, compiler, slim SDK, new crate).
4. Document the layering vs Stage B containers vs Wasix plugins.
5. Do not panic on `wasm32-unknown-unknown` merely because `@timeout` uses `Instant::now()`.

## Non-Goals (v1)

1. Compiling `wasix-runtime` / Wasmer into Wasm.
2. Browser JS glue (`wasm-bindgen`) and `cdylib` exports — WASI stdin/stdout first; unknown-unknown lib check only.
3. Fulfilling host-only ops (`http`, `sql`, …) via JS/WASI imports. v1 dispatches the wasm stdlib profile locally and fails other ops the same way slim SDK does.
4. Hotload / filesystem module discovery inside Wasm (those APIs stay host-side).
5. `no_std`.
6. Replacing Stage B AOT containers. Workflow-in-Wasm and runtime-in-Wasm coexist.

## Feature / crate design

New crate `grapheme-wasm`:

```toml
grapheme-compiler = { default-features = false }          # no Stage B
grapheme-runtime  = { default-features = false }          # no Stage B / Wasix
grapheme-stdlib   = { default-features = false, features = ["wasm"] }
grapheme-artifact = { ... }
```

WASI `_start` contract (stdin JSON → stdout JSON), matching the Stage B container shape so hosts can reuse envelopes:

```json
{
  "source": "query HelloWorld { set { message: \"hi\" } |> core.echo() }",
  "artifact": null,
  "initial_current": {},
  "args": null,
  "entrypoint": null
}
```

`source` XOR `artifact` is required. Wasix `{ module, op, args }` wrapping is accepted for stdin (same as `grapheme-aot-container`).

## Host boundary

Inside Wasm:

- **Local:** stdlib `wasm` profile (`core`, `json`, `csv`, `yaml`, `html`).
- **Denied / host-owned:** sockets, TLS, DB, filesystem plugins, Wasix.

v1 returns `HostCallError::Fatal` for ops outside the wasm profile (same as `grapheme-sdk` slim `StdlibHost`). A later slice can grow multi-round `host_fulfillments` like Stage B.

Filesystem-backed runtime APIs (`discover_wasm_modules`, hotload store, temp-file Stage B Wasix) are not invoked by `grapheme-wasm`. They remain in the crate so native builds keep them; calling them on a Wasm host without WASI FS will fail at runtime, which is acceptable.

## Clock / timeout

`RuntimeEngine` starts `Instant::now()` on every function for `@timeout`. That is valid on `wasm32-wasip1` (WASI clocks) and native. On `wasm32-unknown-unknown`, `Instant::now()` panics without a JS time import.

v1: skip wall-clock timeout enforcement on `wasm32-unknown-unknown`; step-budget still applies. Browser glue can restore clocks via `web-time` / `wasm-bindgen` later.

## Acceptance criteria

1. `cargo check -p grapheme-runtime --target wasm32-wasip1` (default features, no `wasix-runtime`) succeeds.
2. `cargo check -p grapheme-runtime --no-default-features --target wasm32-unknown-unknown` succeeds.
3. Same for `grapheme-compiler --no-default-features` on both targets.
4. Slim SDK check on both targets.
5. `cargo check -p grapheme-wasm --target wasm32-wasip1` succeeds.
6. Native `cargo test -p grapheme-wasm` executes `examples/hello-world.gr` through the in-Wasm execute API.
7. CI runs the checks above.
8. Docs distinguish runtime-in-Wasm vs Stage B vs Wasix.

## Risks

1. **Binary size** — compiler + pest + runtime is larger than the Stage B walker. Accept for v1; size work (`opt-level = "z"`, LTO) is follow-up.
2. **Timeout gap on unknown-unknown** — documented; step budget remains.
3. **Feature drift** — someone enabling `wasix-runtime` on a Wasm target will fail loudly (good).
4. **Two walkers** — `grapheme-aot-container` and `RuntimeEngine` must stay outcome-parity for core transforms (already covered by Stage A/B fixtures on the native host).

## Alternatives considered

1. **Only document that crates already check** — insufficient; no way to actually run in Wasm.
2. **Reuse `grapheme-aot-container` as the runtime** — it is a slim walker without policy traces, compiler, or full MIR coverage. Wrong product.
3. **`cdylib` + wasm-bindgen first** — higher JS surface area; WASI is already the repo's Wasm ABI (`wasm32-wasip1`).
4. **Compile full stdlib/host into Wasm** — rejected in RFC-0005; still rejected.

## Decision

Land `crates/grapheme-wasm` as a WASI compile+execute engine over the existing wasm-safe crate graph, add CI compile gates for runtime/compiler/slim SDK, and keep Wasix host-only.
