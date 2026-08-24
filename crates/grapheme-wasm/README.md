# grapheme-wasm

Runtime-in-Wasm engine (RFC-0006).

WASI binary that embeds the **Grapheme runtime itself** (plus compiler) so hosts can run Grapheme *in* Wasm — not a Stage B workflow container, and not a Wasix plugin runner.

1. Accepts source or an artifact envelope on stdin
2. Compiles source with `grapheme-compiler` (no Stage B)
3. Executes via `RuntimeEngine` with the wasm-safe stdlib profile (`core` / `json` / `csv` / `yaml` / `html`)
4. Host-only ops fail with a fatal capability error

## Build

```bash
# Native tests
cargo test -p grapheme-wasm

# WASI artifact
bash scripts/build-runtime-wasm.sh
# -> target/wasm32-wasip1/release/grapheme-wasm.wasm
```

## Execute request

```json
{
  "source": "import core from \"grapheme/core\"\n\nquery HelloWorld {\n  set { message: \"hi\" }\n  |> core.echo(message: $state.message)\n}",
  "initial_current": {},
  "args": null,
  "entrypoint": null
}
```

Or pass a precompiled `artifact` envelope instead of `source`. Wasix hosts may wrap the same payload under `args`.

```bash
wasmtime run target/wasm32-wasip1/release/grapheme-wasm.wasm < request.json
```

## Next

- JS / `wasm-bindgen` `cdylib` exports for browsers
- Multi-round host fulfillment for `http` / `sql` / … (same contract as Stage B)
- Restore `@timeout` clocks on `wasm32-unknown-unknown` via `web-time`
