# grapheme-aot-container

Stage B AOT workflow container (RFC-0005 step 2).

WASI binary that:

1. Accepts an execute request on stdin (direct JSON or Wasix `{ module, op, args }` envelope)
2. Walks MIR with a slim interpreter
3. Dispatches Wasm-safe stdlib ops locally (`core`, `json`, `csv`, `yaml`, `html`)
4. Surfaces host-only ops as `grapheme.runtime.host.v1::call.capability` stubs

## Build

```bash
# Native walker / tests
cargo test -p grapheme-aot-container

# WASI artifact used for Stage B AOT emission
bash scripts/build-aot-container.sh
# -> crates/grapheme-aot-container/assets/grapheme-aot-container.wasm
```

`default_workflow_wasm()` prefers the built artifact and falls back to a minimal placeholder module for metadata-only envelopes.

## Execute request

```json
{
  "entrypoint": "HelloWorld",
  "mir": { "functions": [], "capabilities": [] },
  "initial_current": {},
  "args": null,
  "allowed_imports": [
    "grapheme.runtime.host.v1::state.read",
    "grapheme.runtime.host.v1::state.write",
    "grapheme.runtime.host.v1::call.capability"
  ]
}
```

Wasix hosts wrap the same payload under `args`.

## Next

- Broader MIR→Wasm lowering beyond the interpreter-in-Wasm container
- Expand local Wasm module set beyond `core` / `json` / `csv` / `yaml` / `html`
