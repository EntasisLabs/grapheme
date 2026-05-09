# Grapheme Runtime Module and Capability Spec (V1)

## Purpose

Define the core module set and capability taxonomy for Grapheme runtime so each module can map to a concrete `.wasm` plugin boundary.

## Why `wasmer-wasix`

`wasmer-wasix` is a strong fit for Grapheme runtime plugins because it gives:

1. WASI/WASIX process-style execution.
2. Host FS and network feature toggles.
3. Sandboxed capability boundaries at runtime.
4. Clean evolution path from MIR interpreter to Wasm plugin execution.

Current crate integration status:
- Feature-gated dependency enabled in runtime crate via `wasix-runtime`.
- Initial backend stub is present to keep the integration boundary stable.

## Core V1 Modules

1. `core`
- Primary role: pure transformations and utility.
- Typical ops: `echo`, `map`, `filter`, `merge`, `validate_schema`.

2. `io`
- Primary role: controlled filesystem and stream operations.
- Typical ops: `read_text`, `write_text`, `list_dir`.

3. `http`
- Primary role: web/API interactions.
- Typical ops: `get`, `post`.

4. `tcp`
- Primary role: low-level socket integrations.
- Typical ops: `connect`, `send`, `receive`.

5. `smtp`
- Primary role: outbound messaging.
- Typical ops: `send_mail`.

6. `memory`
- Primary role: context continuity.
- Typical ops: `load_context`, `store_context`, `summarize_context`.

7. `runtime`
- Primary role: control-flow and checkpointing.
- Typical ops: `retry_with_backoff`, `checkpoint_state`, `emit_event`.

8. `secrets`
- Primary role: secure credential operations.
- Typical ops: `get_secret_handle`, `sign_request`.

9. `policy`
- Primary role: admission, compliance, and guardrails.
- Typical ops: `check_capability`, `check_data_egress`, `require_approval`.

## Capability Naming Pattern

Use resource-scoped capabilities:

`module.action.scope`

Examples:
- `http.get.allowed_domain`
- `io.read.workspace`
- `smtp.send.notifications`
- `memory.namespace.access`
- `policy.enforce`

## ABI Strategy

Two ABIs are modeled for V1:

1. `mir_v1`
- Used by pure/internal modules running on MIR interpreter path.

2. `wasix_v1`
- Used by modules compiled to Wasm and executed through Wasmer WASIX.

## Plugin Manifest Model

Runtime now includes a typed manifest model:

- Module id/version
- ABI kind
- Exported operations
- Required capabilities
- Resource limits

The CLI command below emits the baseline core manifests:

```bash
grapheme modules
```

## Next Runtime Milestones

1. Implement real `WasixBackend` loader and module cache.
2. Add policy admission step before plugin invocation.
3. Add capability-aware host import layer for FS/network/secrets.
4. Add conformance tests for each core module manifest.
