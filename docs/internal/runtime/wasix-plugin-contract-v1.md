# Grapheme WASIX Plugin Contract (V1 Draft)

## Goal

Define a stable contract for mapping Grapheme module operations to `.wasm` plugin files executed through the `wasix-runtime` backend.

## Runtime Routing Rules

1. MIR call includes `module`, `op`, `capability`, and `args`.
2. Runtime resolves module/op through `ModuleRegistry`.
3. If module ABI is `mir_v1`, runtime uses host call path.
4. If module ABI is `wasix_v1`, runtime requires a bound wasm path.

CLI binding example:

```bash
grapheme run examples/hello-world.gr --bind core=plugins/core.wasm
```

Current repository demo command:

```bash
grapheme run examples/hello-world.gr --bind core=plugins/core-echo.wat
```

## Module Binding

Bindings are keyed by module id, lowercased.

- `core` -> `plugins/core.wasm`
- `http` -> `plugins/http.wasm`

The runtime checks:

1. file exists/readable
2. wasm module compiles
3. module is WASI/WASIX compatible

## Current V1 Execution State

Current backend behavior for `wasix_v1`:

1. Validates and compiles the target wasm module.
2. Returns structured routing output with module, op, and args.

Planned next step:

1. Invoke exported operation entrypoint and return plugin result payload.

## AOT Stage B Container Boundary (Scaffold)

Stage B now introduces a workflow container metadata envelope (`grapheme.aot.stage_b.v1`) on top of Stage A artifacts.

Current scaffold contract:

1. Stage B payload carries `workflow_wasm` metadata: byte length, sha256, entry export, allowed imports.
2. Host interface id must match runtime contract (`grapheme.runtime.host.v1`).
3. Allowed imports must remain inside host boundary namespace:

- `grapheme.runtime.host.v1::state.read`
- `grapheme.runtime.host.v1::state.write`
- `grapheme.runtime.host.v1::call.capability`

Workflow container runtime (`grapheme-aot-container`):

1. WASI bin walks MIR from stdin (`args.mir` + `args.entrypoint` under the Wasix envelope).
2. Wasm-safe stdlib ops (`core`/`json`/`csv`/`yaml`/`html`) run locally inside the container.
3. Other capability ops emit a structured `host_call_required` stub for `call.capability`.
4. Host runtimes fulfill stubs across rounds via `host_fulfillments` (in-process walker is the default Stage B path; Wasix remains an optional sandbox).
5. Default Stage B emission loads `assets/grapheme-aot-container.wasm` (build with `scripts/build-aot-container.sh`).

Optional inline container transport:

1. `workflow_wasm.inline_wasm_hex` may carry full wasm bytes inline.
2. When present, runtime validates both `byte_len` and `sha256` against decoded bytes before execution.

Runtime scaffold behavior today:

1. Stage B execution routes through runtime `execute_aot` Stage B branch.
2. Runtime emits `aot.stage_b.container_routed` lifecycle event with entry export/hash/import metadata.
3. With `wasix-runtime` enabled and valid inline bytes present, runtime attempts direct container invocation.
4. If container execution is unavailable or fails in scaffold mode, runtime falls back to Stage A parity execution path.

Strict mode option:

1. `RuntimeOptions.strict_stage_b_container_execution = true` disables Stage B parity fallback.
2. In strict mode, unavailable direct container runtime path is treated as an artifact compatibility error.
3. Runtime defaults to strict mode in release builds, and can be overridden via `GRAPHEME_STRICT_STAGE_B=true|false`.

## Recommended Export Pattern

V1 recommended export strategy for plugins:

1. expose `_start` for WASI compatibility
2. accept JSON request via stdin
3. write JSON response via stdout

Request envelope:

```json
{
  "module": "core",
  "op": "echo",
  "args": { "message": "Hello World" }
}
```

Response envelope:

```json
{
  "ok": true,
  "result": { "message": "Hello World" }
}
```

## Security Considerations

1. Capability checks occur before backend invocation.
2. Resource limits must be enforced per module manifest.
3. Domain/path/secret policy checks remain host-controlled.
4. Multi-tenant deployments should isolate module binding scopes.
