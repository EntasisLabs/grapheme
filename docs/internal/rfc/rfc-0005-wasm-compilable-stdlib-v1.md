# RFC-0005: Wasm-Compilable Stdlib Layering v1

Status: accepted-for-implementation
Authors: runtime + stdlib + compiler
Created: 2026-08-21
Target release window: Track 4 (AOT to Wasm) Stage B unblocking

## Summary

Split `grapheme-stdlib` into a **Wasm-safe core** and **host-only capability layers** so Stage B workflow containers and other `wasm32-wasip1` builds can compile without pulling TLS/DB/native stacks (`sqlx`, `surrealdb`, `lettre`/`aws-lc-sys`, etc.).

Today the stdlib feature matrix only gates `data` / `pdf` / `image` / `plot` / `media`. Core host modules (`http`, `sql`, `surreal`, `email`, …) are always compiled, which makes any Wasm path for the crate impractical.

## Motivation

Track 4 Stage B needs a real workflow Wasm container. Two execution shapes are on the table:

1. **Interpreter-in-Wasm** — embed a slim MIR executor + pure transforms inside the container; capability ops become `grapheme.runtime.host.v1::*` imports.
2. **Lowered Wasm** — emit control flow as Wasm and keep the same host import boundary for capabilities.

Both shapes fail if the only available stdlib build graph includes host-native crates that do not (and should not) target WASI.

Observed blocker (current tree):

```text
cargo check -p grapheme-stdlib --target wasm32-wasip1
→ aws-lc-sys / lettre / sqlx / surrealdb / tokio host stack fails
```

Proven Wasm-safe today (standalone probe): `serde_json`, `csv`, `serde_yaml`, `html-to-markdown-rs`.

## Goals

1. Provide a Cargo feature profile that compiles cleanly for `wasm32-wasip1`.
2. Keep current CLI/SDK host behavior (http/sql/email/etc. remain available without new opt-in for embedders).
3. Document which modules belong **inside** a Stage B container vs **host imports** vs **Wasix plugins**.
4. Add a conformance gate so Wasm compilability does not regress.

## Non-Goals

1. Compiling `sql` / `surreal` / `email` / `tcp` / `smtp` / `media` / Polars `data` into workflow Wasm in v1.
2. Replacing existing `plugins/*-rs` Wasix modules.
3. Full MIR→Wasm lowering (Stage B emission); this RFC only clears the stdlib prerequisite.
4. `no_std` support.

## Module Classification

| Layer | Modules | Wasm32-WASIP1 | Stage B role |
| --- | --- | --- | --- |
| **always** | `core`, `json`, `envelope`, `capability` | yes | in-container |
| **transforms** | `csv`, `yaml`, `html` | yes | in-container |
| **host-net** | `http`, `web`, `websearch`, `research`, `tcp`, `smtp`, `email` | no | host import or Wasix plugin |
| **host-db** | `sql`, `surreal` | no | host import |
| **host-native** | `data` (Polars), `media` (ffmpeg) | no | host-only |
| **plugin scaffolds** | `pdf`, `image`, `plot` | stubs only | prefer Wasix sidecar |

Rule of thumb: if it needs sockets, TLS, a DB client, or a host process, it stays outside the workflow container.

## Feature Design

```toml
[features]
default = ["host"]
# Wasm-safe transforms (csv/yaml/html) + always-on core/json
wasm = ["transforms"]
transforms = ["dep:csv", "dep:serde_yaml", "dep:html-to-markdown-rs"]
host = ["transforms", "http", "web", "net", "email", "sql", "surreal"]
http = ["dep:ehttp"]
web = ["dep:websearch", "http", "dep:tokio"]
net = []          # tcp + smtp (std::net)
email = ["dep:lettre"]
sql = ["dep:sqlx", "dep:tokio"]
surreal = ["dep:surrealdb", "dep:tokio"]
data = ["dep:polars"]
pdf = []
image = []
plot = []
media = []
full = ["host", "data", "pdf", "image", "plot", "media"]
```

Compatibility notes:

1. `default = ["host"]` preserves prior “everything but capability modules” behavior for direct stdlib consumers.
2. `grapheme-sdk` keeps `default-features = false` on stdlib but must list `features = ["host"]` so embedders still get network/DB modules without enabling `full`.
3. `grapheme-cli` / `full` continue to enable the complete stack.

Build recipes:

```bash
# Wasm-safe profile (Stage B / container work)
cargo check -p grapheme-stdlib --no-default-features --features wasm --target wasm32-wasip1

# Host defaults (unchanged product path)
cargo check -p grapheme-stdlib
cargo check -p grapheme-stdlib --features full
```

## Stage B Integration Path

Sequence after this layering lands:

1. **Stdlib Wasm gate** (this RFC) — green `wasm32-wasip1` check for `--features wasm`. **Done.**
2. **Container crate sketch** — thin `cdylib`/`bin` that links `grapheme-stdlib` with `wasm` only, plus a MIR walk loop; capability calls call host imports. **Done** (`crates/grapheme-aot-container`).
3. **Host import surface** — stabilize `grapheme.runtime.host.v1::{state.*, call.capability, ...}` beyond the current Stage B scaffold allowlist; fulfill stubs from the host Wasix path. **Done** (in-process multi-round fulfillment via `CapabilityHost`; `state.read`/`state.write`/`call.capability`).
4. **Emitter** — compiler replaces Stage B “bytes provided by caller” scaffold with real container emission for reference workflows (`core`/`json`/`csv`/`yaml`/`html` only). **Done** (`compile_to_aot_stage_b` / `*_default` + `scripts/build-aot-container.sh`).
5. **Parity** — same fixtures vs interpreted Stage A path; strict Stage B mode already rejects fallback. **Done** (outcome + `state.current` parity fixtures; Wasix multi-round uses the same `host_fulfillments` contract when `prefer_stage_b_wasix` is set).

Security boundary stays unchanged: policy admission remains on the host side of every capability import.

## Acceptance Criteria

1. `cargo check -p grapheme-stdlib --no-default-features --features wasm --target wasm32-wasip1` succeeds.
2. Host `cargo test -p grapheme-stdlib --features full` still passes.
3. SDK/CLI default host modules (`http`, `sql`, …) remain available without requiring new public feature names for existing embedders.
4. Docs (`sdk-feature-flags.md`, Track 4 roadmap) describe the Wasm profile and module placement rules.
5. CI runs the Wasm stdlib check on PRs.

## Risks

1. **Feature drift** — registry/`is_registered_op` must stay behind the same `cfg(feature = …)` gates as modules.
2. **SDK surprise** — forgetting `features = ["host"]` on the SDK→stdlib edge would silently drop network/DB ops; covered by SDK integration tests.
3. **ehttp on Wasm** — not enabled in the `wasm` profile even if the crate can target browser/WASI; Stage B must not smuggle network into the container in v1.

## Alternatives Considered

1. **Target `cfg` only** (`cfg(not(target_arch = "wasm32"))` deps) — works for WASI builds but does not give a slim native profile and is harder to document as an intentional product surface.
2. **New `grapheme-stdlib-wasm` crate** — clearer boundary, higher duplication/registry drift cost.
3. **Compile full stdlib to Wasm via WASIX** — rejected; TLS/DB stacks are the wrong trust and size boundary for workflow containers.

## Decision

Implement feature layering in `grapheme-stdlib` first, wire SDK/CLI compatibility, add CI + docs, then proceed with Stage B container emission against the `wasm` profile only.
