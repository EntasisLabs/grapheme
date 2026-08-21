# grapheme-aot-container

Stage B AOT workflow container (RFC-0005 step 2).

WASI binary that:

1. Accepts an execute request on stdin (direct JSON or Wasix `{ module, op, args }` envelope)
2. Walks MIR with a slim interpreter
3. Dispatches Wasm-safe stdlib ops locally (`core`, `json`, `csv`, `yaml`, `html`)
4. Surfaces host-only ops as `grapheme.runtime.host.v1::call.capability` stubs

## Build

```bash
cargo check -p grapheme-aot-container
cargo build -p grapheme-aot-container --release --target wasm32-wasip1
```

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

- Link real host imports in the Wasix Stage B path (RFC step 3)
- Compiler emission of this container instead of placeholder bytes (RFC step 4)
