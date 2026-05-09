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
grapheme run examples/hello-world.aql --bind core=plugins/core.wasm
```

Current repository demo command:

```bash
grapheme run examples/hello-world.aql --bind core=plugins/core-echo.wat
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
