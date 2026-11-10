# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

Planned as **0.9.0**. `grapheme-artifact` is **0.5.0**. Not published.

### Added

- **Store tasks (RFC-0007 Part F, headless slice).** `task LoadFeed on Feed { apply { status: "pending" }; core.value(…); apply { status: "ready", posts: $current } }` declares a store-bound effect workflow. Tasks may call capabilities; each of the task's own `apply`/`transition` steps commits its own revision. `$state` is kept on a separate lane so a capability call that replaces `$current` does not erase store fields, and bare `$field` in `if` / `match` reads that lane. `StoreCommit.effects` lists effect receipts for capability calls (not pure `core.value` / `core.echo`), with `cause`. A failed task keeps earlier commits and returns those receipts, including a `failed` receipt when a call is refused. A mutation called from a task is one transaction. Mutations cannot call tasks. Views may bind events to tasks; UI glyphs may `expose` them. Mutations stay pure (no module ops). Concurrency policies, `@if_unchanged`, `time.sleep`, registries and subscriptions are not in this slice. New example `examples/feed-headless.gr`.
- **View `if` / `for` (RFC-0007 Part C.7).** `if $status == "ready" { … }` and `for $post in $posts key $post.id { … }` are view children. Compile-time IR uses `$if` / `$for`; the headless session expands them when rendering (false branches omitted; list rows keyed by the `key` expression). Item scope `$post` / `$post.field` works in props and bindings. Example `examples/feed-view.gr`. `VirtualList` is not in this slice.

### Fixed

- **Bare `if` / `match` after a capability read the store.** `$status` in a task lowers to `state.status`, so the branch uses the store lane rather than the capability result left in `$current`.
- **A mutation cannot call a task**, including through another mutation or an inline target. The task's capability calls no longer run on the mutation path.
- **A mutation called from a task commits once.** Applies inside that call share one revision and roll back together if the call fails. Earlier task applies stay committed.
- **A store field named `error` is not a step failure.** `__input.error` still fails a step when the input is a previous op's result.
- **Effect receipts match F.4 for this slice.** Capability ops only, with `cause` (`action`, `task`, `generation`). `generation` is null until fencing exists. Refused calls get a `failed` receipt, and the list is returned when the task fails.

### Changed

- **Crate versions.** Language crates are **0.9.0**. `grapheme-artifact` is **0.5.0** because `MirFunctionKind::Task` is a new public enum variant; 0.4.0 deserializers reject `"Task"`. Not published.

### Crate versions (this slice)

| Crate | Version |
| --- | --- |
| `grapheme-signatures` | 0.9.0 |
| `grapheme-stdlib` | 0.9.0 |
| `grapheme-aot-container` | 0.9.0 |
| `grapheme-runtime` | 0.9.0 |
| `grapheme-compiler` | 0.9.0 |
| `grapheme-sdk` | 0.9.0 |
| `grapheme-cli` | 0.9.0 |
| `grapheme-lsp` | 0.9.0 |
| `grapheme-wasm` | 0.9.0 |
| `grapheme-artifact` | 0.5.0 |

## [0.8.0] - 2026-11-10

**GraphemeUI language surface (RFC-0007 rollout steps 0–2 + Part H).** State stores, value expressions, views / View IR, and headless UI glyph mount — enough for a counter driven by `grapheme session` or an external renderer bridge. See `docs/internal/rfc/rfc-0007-graphemeui-views-v1.md`.

### Added

- **UI glyphs and headless app session (RFC-0007 Part H / I.2).** `glyph CounterApp on Counter { expose Increment; CounterView }` mounts a store-backed UI app. The body is one root element (a view reference or an inline tree); `expose` lists mutations, queries and views reachable outside the tree. `StoreSession::mount` / `GraphemeStoreSession::mount` (CLI: `grapheme session --mount [Glyph]`) returns `{glyph, store, revision, ir, actions}` including `expose:<Name>` actions for exposed mutations. `invoke_action("expose:Increment")` runs an exposed no-arg mutation. Workflow glyphs are unchanged. Example `examples/view-counter.gr` now includes `CounterApp`.
- **Views and View IR (RFC-0007 Parts C, D, I; rollout step 2).** `view CounterView on Counter { Column { value: Text("{$count}") increment: Button(label: "Increment") { on click: Increment } } }` declares an element tree over a store, using a new built-in catalog of `Column`, `Row`, `Text` and `Button`, named children, positional content, expression props and view references. Unknown elements, props and events, steps or module calls inside a view, and bindings whose target is not a mutation on the view's store or whose arguments don't match its parameters exactly are compile errors; an interactive element with no label or text gets a `view-accessibility` lint. `grapheme compile --emit view-ir` prints `{"$ir": "grapheme-ui/1", "views": [...]}` with stable node keys. `StoreSession::render(view)` / `GraphemeStoreSession::render` evaluate a view at the latest revision and return its action list; `invoke_action(id)` runs an action as one transaction and rejects ids whose binding epoch changed with `stale_action`. `grapheme session` gains `--render View` and `--action <id>`. New example `examples/view-counter.gr`. Styles, view params, slots, `if`/`for`, and `VirtualList` are not in this release. Programs without views compile to the same MIR as before.
- **Expression query bodies (RFC-0007 Part B.5).** A query body may be a single expression whose value is the result: `query ReadSelected on Panel -> Int { $selected }`. It lowers to a new pure `core.value(value: …)` op, so the run returns the value (a scalar, for example) rather than the input object; `StoreSession::query` and `grapheme session --query` return it directly. Under `--type-policy strict` the expression must fit the declared `-> T`; under `warn` a mismatch is an `expression-type` lint. Mutation bodies stay pipelines. Existing programs parse unchanged.
- **State stores (RFC-0007 Part A).** `state Counter { count: Int = 0, const step: Int = 2, transient pulse: Int = 0, derive double: Int = $count * 2 }` declares a store. `mutation Increment on Counter { apply { count: $count + $step } }` reads fields by bare name and writes them with lane-less `apply`; enum fields governed by a `state_machine` are written only by `transition`. Writing const, derive, lifecycle or unknown fields, derive cycles, defaults that read fields, and cross-store calls are compile errors. Store queries are read-only. Each mutation commits one revision with a runtime diff (transient fields included) and a history diff (transient fields removed); failures and illegal transitions commit nothing. New `grapheme session <file.gr> --invoke Name[:<json>] --query Name[:<json>]` command, `GraphemeEngine::store_session` and `grapheme_runtime::StoreSession`. `singleton`/`transient state`, registries, history/undo and hot reload are not in this release. Programs without stores compile to the same MIR as before.
- **Expression type checking and `.count` (RFC-0007 Part B).** Under `--type-policy strict`, ill-typed expressions are compile errors: `"a" - 1`, non-`Bool` `&&`/`||`/`!`/`if` operands, `if` branches of different types, a `set` field whose expression doesn't match its declared type, and enum comparisons against non-members (`$state.density == fast`). Under `warn` they are `expression-type` lints. Operands without a declared type are not checked. `$list.count` and `$text.count` read the length of a list or string.
- **Value expressions (RFC-0007 Part B).** Any value position now accepts `+ - * / %`, comparisons (`== != < <= > >=`), `&& || !`, parentheses and `if … then … else`: `set { count: $state.count + 1, mode: if $busy then pulsing else idle }`. `Int op Int` stays `Int` (`/` truncates); overflow, division by zero and type mismatches fail the step with `EXPRESSION_ERROR`. Stage A and Stage B evaluate the same `{"$expr": …}` MIR form. No new commands, flags or environment variables. Existing programs parse unchanged; parse errors after a value now say "expected operator" where they used to list literal rules.

### Fixed

- **`transition $state.<field>` writes the root field.** The step lowered to `core.set_fields` with a literal `state.status` key, so the field kept its old value and a typed output (`-> T`) rejected it with "field 'state.status' is not declared". It now writes `status`. `examples/mutation-state-machine-apply.gr` (tutorial 02) compiles again under both type policies and ends with `status: approved`; `examples/core-validate-schema.gr` now updates `status` instead of adding a `state.status` key. (RFC-0007 Part J.3)
- **Partial literal patches under `-> T`.** `set`, `core.set_fields` and `core.merge` patches in a query or mutation `on S -> T` no longer have to restate required fields of `T` that the input type `S` declares; the runtime already carried them over. (RFC-0007 Part J.1)
- **`inc_field` / `dec_field` keep integers.** An integer field stays an integer (`5` → `6`, previously `6.0`). Float fields and the missing-field default (`1.0`) are unchanged. (RFC-0007 Part J.2)

### Changed

- **`grapheme-artifact` 0.4.0** — publishes the shared `expr` evaluator and View IR / UI glyph MIR types that RFC-0007 added after 0.3.0. Downstream crates that depended on path `0.3.0` must take `0.4.0` for crates.io builds.

### Crate versions (this release)

Language crates ship at **0.8.0**. `grapheme-artifact` moves to **0.4.0** (expr + View IR MIR surface). The VS Code extension is **0.8.0** (LSP binary ships with the new language surface). The release tag is **v0.8.0**.

| Crate | Version |
| --- | --- |
| `grapheme-signatures` | 0.8.0 |
| `grapheme-stdlib` | 0.8.0 |
| `grapheme-aot-container` | 0.8.0 |
| `grapheme-runtime` | 0.8.0 |
| `grapheme-compiler` | 0.8.0 |
| `grapheme-sdk` | 0.8.0 |
| `grapheme-cli` | 0.8.0 |
| `grapheme-lsp` | 0.8.0 |
| `grapheme-wasm` | 0.8.0 |
| VS Code extension | 0.8.0 |
| `grapheme-artifact` | 0.4.0 |

## [0.7.2] - 2026-10-23

**SQLite link coexistence and TLS/tokio security floors.** The `sql` capability stays on sqlx, moved to 0.9 (#27) so one Cargo build can also link rusqlite 0.37 (`libsqlite3-sys` 0.35). Web search no longer pulls the unfixed `reqwest` 0.11 / `rustls` 0.21 line.

### Fixed

- **`libsqlite3-sys` links conflict** (#27) — `grapheme-stdlib`'s `sql` feature (pulled in by the default `host` profile, and therefore by `grapheme-sdk` / `grapheme-cli` host and full builds) depended on sqlx 0.8.6. That release's `sqlx-sqlite` requires `libsqlite3-sys ^0.30.1`. Cargo allows only one crate with `links = "sqlite3"` per build, so a consumer on rusqlite 0.37 (`libsqlite3-sys ^0.35.0`, as used by locus-sqlite-adapter 0.1.0) could not compile the host stack. sqlx **0.9.0** declares `libsqlite3-sys >=0.30.1, <0.38.0`, which includes 0.35. `sql.query`, `sql.execute`, `sql.health`, and `sql.transaction` behave the same. sqlx 0.9 only treats `&'static str` as SQL-safe, so workflow SQL is passed through `AssertSqlSafe` (parameters stay bound).

### Security

- **`websearch` 0.1.1 removed.** That crate pinned `reqwest` 0.11.27 and `rustls` 0.21, which pull `rustls-webpki` 0.101.x and `webpki-roots` 0.25.x. Those lines have no fix (RUSTSEC-2026-0381, RUSTSEC-2026-0382) and are gone from the tree. `web.*` and `websearch.*` now call DuckDuckGo, Google Custom Search, Exa (`xaviv`), Tavily, and Brave through the workspace `reqwest` 0.13 crate with rustls. Ops, credential env vars, and result fields (`url`, `title`, `snippet`, `domain`, `provider`, plus `published_date` / `raw` where the old crate emitted them) stay the same. Brave was already a direct API call. `websearch`'s other providers (ArXiv, SerpAPI, SearXNG) were not part of Grapheme's public catalog and are not reimplemented.
- **Dependency floors** so a downstream resolver (Stasis) cannot select the vulnerable releases:
  - `tokio` **1.53.3+** (RUSTSEC-2026-0386) on `grapheme-stdlib`, `grapheme-runtime`, and `grapheme-lsp`.
  - `rustls-webpki` **0.103.16+** (RUSTSEC-2026-0382) on the stdlib features that pull rustls (`http` / `web` / `email` / `sql` / `surreal`) and on `grapheme-runtime`'s `wasix-runtime` feature.
  - `webpki-roots` **1.0.10+** (RUSTSEC-2026-0381) on those same stdlib features. The `webpki-roots` 0.26 crate remains as the upstream shim over 1.x (`tokio-tungstenite`).
  - `rustls` **0.23.46+** (RUSTSEC-2026-0285, RUSTSEC-2026-0346, RUSTSEC-2026-0348, RUSTSEC-2026-0349) on those same stdlib features and on `grapheme-runtime`'s `wasix-runtime` feature.
  - `webpki-root-certs` **1.0.11+** (RUSTSEC-2026-0383) on the stdlib `web` feature and on `wasix-runtime`. This is the wasm root store used by `rustls-platform-verifier`, a different crate from `webpki-roots`.
- `plugins/http-rs` (ureq) uses the same `webpki-roots`, `rustls-webpki`, and `rustls` floors.
- Lockfiles move the other reachable advisories and yanked crates: `h2` 0.4.20, `quinn-proto` 0.11.19, `ammonia` 4.2.4, `crossbeam-epoch` 0.9.21, `rkyv` 0.8.18 (0.7 removed via `rust_decimal` 1.43.0), `ctutils` 0.4.5, `chacha20` 0.10.2, `enumset` 1.1.15, `spin` 0.9.9, plus the warning-level `anyhow`, `event-listener`, `lru`, and `memmap2` 0.9 updates. `rsa` 0.9.10, `ruzstd` 0.8.3, and `memmap2` 0.6.2 stay because their parents do not allow the patched release.

### Changed

- **Toolchain** — Rust **1.94+** / edition 2024. sqlx 0.9's MSRV is 1.94. The combined `runtime-tokio-rustls` feature was removed upstream; the `sql` feature now enables `runtime-tokio` + `tls-rustls` (still rustls with ring) and the bundled `sqlite` driver.
- `@timeout` no longer calls `Instant::now()` on `wasm32-unknown-unknown` (that target panics without a JS clock); step budget still applies. WASI/native clocks are unchanged. This replaces downstream patches that swapped in `web_time::Instant`.
- CLI AOT build-manifest golden no longer pins rustc-dependent `aot_id` hashes (redacts to `aot-<derived>` and checks the `aot-<16 hex>` shape). This was already failing on main after 0.7.1.

### Added

- **Runtime-in-Wasm (RFC-0006)** — `grapheme-wasm` WASI crate embeds `RuntimeEngine` + compiler + the wasm-safe stdlib profile so Grapheme can *run inside* Wasm (not only host Wasm plugins or Stage B workflow containers). Wasix stays host-only.
- CI compile gates for `grapheme-runtime` / `grapheme-compiler` / slim SDK on `wasm32-wasip1` and `wasm32-unknown-unknown`, plus `grapheme-wasm` WASI check.

### Crate versions (this release)

Language crates ship at **0.7.2**. `grapheme-artifact` stays **0.3.0**. The VS Code extension stays **0.7.1** (LSP behavior is unchanged). The release tag is **v0.7.2**. Not published from this change.

| Crate | Version |
| --- | --- |
| `grapheme-signatures` | 0.7.2 |
| `grapheme-stdlib` | 0.7.2 |
| `grapheme-aot-container` | 0.7.2 |
| `grapheme-runtime` | 0.7.2 |
| `grapheme-compiler` | 0.7.2 |
| `grapheme-sdk` | 0.7.2 |
| `grapheme-cli` | 0.7.2 |
| `grapheme-lsp` | 0.7.2 |
| `grapheme-wasm` | 0.7.2 |
| VS Code extension | 0.7.1 |
| `grapheme-artifact` | 0.3.0 |

## [0.7.1] - 2026-08-22

**Slim embedded SDK release** — upstreams the host-integration seams needed by
embedded products and adds a dependency-light core profile for iOS and Wasm.

### Added

- **Slim SDK profile** — `grapheme-sdk` can be built with
  `default-features = false, features = ["slim"]`; it keeps the core/json
  compiler and runtime while excluding the host capability stack, AOT
  container, and Wasix runtime.
- **Host module registration** — embedders can register MirV1 host manifests
  with `ModuleRegistry::register_host_module` and configure the registry from
  `GraphemeEngineBuilder`.
- **Per-call initial state** — `execute_source_with_initial_state` seeds
  `state.current` without mutating the engine's reusable runtime options.

### Changed

- **Optional Stage B** — AOT/Stage B support is behind the `stage-b` feature;
  default SDK/compiler/runtime builds retain the existing host + Stage B
  behavior, while slim builds omit it.
- **Target coverage** — the slim SDK is checked for `aarch64-apple-ios`, both
  iOS simulator architectures, and `wasm32-unknown-unknown`.

### Crate versions (this release)

All workspace language crates ship at **0.7.1** except `grapheme-artifact`,
which remains at **0.3.0**. The release tag is **v0.7.1**.

| Crate | Version |
| --- | --- |
| `grapheme-signatures` | 0.7.1 |
| `grapheme-stdlib` | 0.7.1 |
| `grapheme-aot-container` | 0.7.1 |
| `grapheme-runtime` | 0.7.1 |
| `grapheme-compiler` | 0.7.1 |
| `grapheme-sdk` | 0.7.1 |
| `grapheme-cli` | 0.7.1 |
| `grapheme-lsp` | 0.7.1 |
| VS Code extension | 0.7.1 |
| `grapheme-artifact` | 0.3.0 |

## [0.7.0] - 2026-08-21

**Language + Stage B release** — executable parameters / tagged variables (RFC-0004) and a Wasm-compilable Stage B AOT path (RFC-0005). See `docs/internal/rfc/rfc-0004-params-and-tagged-vars-v1.md` and `docs/internal/rfc/rfc-0005-wasm-compilable-stdlib-v1.md`.

### Added

- **Executable parameters (RFC-0004 Phase 1)** — GraphQL-style `$param` lists on `query` / `mutation` / `iterator` / `subscription`, lowered through HIR → MIR → runtime locals with CLI/SDK entrypoint arg binding.
- **Tag schemas + scoped `using` (RFC-0004 Phase 2a)** — tagged variable schemas and block-scoped `using` for ambient auth/session-style handles.
- **Wasm-safe stdlib profile (RFC-0005)** — `grapheme-stdlib` feature split: default `host` vs `wasm` / `transforms` so Stage B containers compile for `wasm32-wasip1` without TLS/DB stacks.
- **`grapheme-aot-container`** — slim MIR walker WASI binary for Stage B workflows; local ops for `core` / `json` / `csv` / `yaml` / `html`; host-only ops surface as `grapheme.runtime.host.v1::call.capability` stubs.
- **Stage B host fulfillment** — multi-round in-process walker + `CapabilityHost` fulfillment (`call.capability`, `state.read` / `state.write`) via `host_fulfillments`.
- **Default Stage B emission** — `compile_to_aot_stage_b` / SDK `compile_source_to_aot_stage_b_default` + `scripts/build-aot-container.sh`.
- **Wasix multi-round Stage B** — optional sandbox path (`prefer_stage_b_wasix` / `GRAPHEME_PREFER_STAGE_B_WASIX`) using the same fulfillment contract.
- **Stage A ↔ Stage B parity fixtures** — outcome + `state.current` parity for core transforms and host-capability paths.

### Changed

- **Toolchain** — Rust edition **2024** / **rust-version 1.92+** required.
- **Strict Stage B** — in-process container walker is the primary path; Wasix is opt-in; parity fallback still available when not strict.
- **CI** — conformance gates for Wasm stdlib check, Stage B container, parity tests, and Wasix multi-round (builds container artifact first).
- **Docs / examples** — onboarding covers params, tags/`using`, and Stage B; canonical demos `examples/params-call-bind.gr` and `examples/tag-using-scope.gr`; author extract `docs/internal/language/params-and-tags-v1.md`.

### Crate versions (this release)

All workspace language crates ship at **0.7.0** except `grapheme-artifact`, which advances to **0.3.0** for the expanded MIR contract. The release tag is **v0.7.0**.

| Crate | Version |
| --- | --- |
| `grapheme-signatures` | 0.7.0 |
| `grapheme-stdlib` | 0.7.0 |
| `grapheme-aot-container` | 0.7.0 |
| `grapheme-runtime` | 0.7.0 |
| `grapheme-compiler` | 0.7.0 |
| `grapheme-sdk` | 0.7.0 |
| `grapheme-cli` | 0.7.0 |
| `grapheme-lsp` | 0.7.0 |
| VS Code extension | 0.7.0 |
| `grapheme-artifact` | 0.3.0 |

### Notes

- Last published Git tag before this cut was **v0.4.0**; Cargo already carried 0.5.x/0.6.x trains without matching tags. **v0.7.0** is the aligned language + Stage B cut.
- `grapheme-artifact` 0.3.0 carries the breaking MIR additions for executable parameters and scoped `using` instructions required by the 0.7.0 crates.
- Build the Stage B wasm asset before Wasix sandbox runs: `./scripts/build-aot-container.sh`.
- RFC-0004 Phases 3+ (tag-typed parameters as the full call-edge model, `uses` sugar) remain follow-ups beyond this release.

## [0.6.1] - 2026-06-03

### Fixed

- **`grapheme-stdlib` publish tarball** — `src/data/` was excluded by a overly broad `.gitignore` rule (`data/`); fixed to `/data/` and included `data/mod.rs` + `data/frame.rs` in the crate.

## [0.6.0] - 2026-06-03

**Extensible platform release** — Wasm discovery, hotload, typed host envelopes, and opt-in capability modules. See `docs/internal/roadmaps/release-0.6.0-extensible-platform.md`.

### Added

- **Opt-in capability modules** (`data`, `pdf`, `image`, `plot`, `media`) behind Cargo feature flags on `grapheme-stdlib`, `grapheme-sdk`, `grapheme-signatures`, `grapheme-runtime`, and `grapheme-compiler`.
- **Host result envelope** (`{ data, meta, error }`) via `grapheme-stdlib::envelope` — capability results normalize to this shape with legacy flat dual-read.
- **Dynamic Wasm module discovery** — `grapheme-runtime::discover_wasm_modules`, manifest sidecar spec (`docs/internal/runtime/wasm-module-manifest-v1.md`, `schemas/wasm-module-manifest.schema.json`).
- **`grapheme modules scan|activate|rollback|status`** — discover Wasm modules, persist generations in `.grapheme/modules/hotload.json`, rollback with in-flight pinning metadata.
- **Hotload store (v1)** — `.grapheme/modules/hotload.json` hydrates CLI runs and SDK sessions via `with_default_hotload_store()`.
- **Reference Wasm plugins** — `plugins/pdf-rs`, `plugins/image-rs`, `plugins/plot-rs` with manifests under `modules/*.module.json`.
- **`data` module (Polars, native)** — `read_csv`, `filter`, `group_by`, `aggregate(op=count)`, `schema`, `to_json` on `grapheme.data.frame/v1` handles.
- **`media` module (ffmpeg native)** — `media.probe` (ffprobe JSON) and `media.transcode` (ffmpeg CLI bridge).
- **LSP envelope hints** — `$current` / `$current.data.*` completion and hover infer payload fields from the prior pipeline op; signature help lists envelope + data output fields.
- **Examples** — `data-read-csv.gr`, `data-filter.gr`, `pdf-generate.gr`, `image-metadata.gr`, `plot-line.gr`, `media-probe.gr`, `platform-release-060.gr`.

### Changed

- **`grapheme-cli`** — `default = ["full"]`; ships all capability features (v0.6.0).
- **`grapheme-sdk`** — `default = []`; embedders opt in with `full` or per-module features (`data`, `pdf`, …).
- **`grapheme-lsp`** — ships with `full` signatures for editor discovery (v0.6.0).
- **Auto-bind on run** — discovered modules from `[modules].scan` (or `modules/` + `plugins/` defaults) bind automatically when referenced in a workflow.

### Crate versions (this release)

All workspace crates ship at **0.6.0** except `grapheme-artifact` (unchanged at 0.2.0). The release tag is **v0.6.0**.

| Crate | Version |
| --- | --- |
| `grapheme-signatures` | 0.6.1 |
| `grapheme-stdlib` | 0.6.1 |
| `grapheme-runtime` | 0.6.1 |
| `grapheme-compiler` | 0.6.1 |
| `grapheme-sdk` | 0.6.1 |
| `grapheme-cli` | 0.6.1 |
| `grapheme-lsp` | 0.6.1 |
| VS Code extension | 0.6.0 |
| `grapheme-artifact` | 0.2.0 (unchanged) |

### Notes

- **`pdf` / `image` / `plot`** resolve to Wasm plugins when activated or auto-bound; stdlib scaffolds remain fallbacks for Mir-only paths.
- **`media`** requires `ffmpeg` and `ffprobe` on `PATH` for probe/transcode ops.
- Legacy `.grapheme/modules/bindings.json` is still read when no hotload store exists.

## [0.5.0] - 2026-06-03

User-facing release tag. Ships stdlib expansion and matching library crates.

### Added

- **`email` module** (host-backed, lettre): `email.smtp`, `email.gmail`, `email.providers`, `email.capabilities` for TLS/auth SMTP delivery with provider presets and env-based credentials (`SMTP_*`, `GMAIL_*`, `EMAIL_FROM`).
- **Web search providers** on `web.*` and `websearch.*`:
  - `google` — `GOOGLE_API_KEY` + `GOOGLE_CX`
  - `xaviv` — semantic search via Exa (`XAVIV_API_KEY` or `EXA_API_KEY`)
  - `tavily` — `TAVILY_API_KEY`
  - `brave` — native Brave Search API (`BRAVE_API_KEY`)
- Examples: `examples/email-smtp.gr`, `examples/web-tavily.gr`, `examples/web-brave.gr`.

### Changed

- `web.providers` / `web.capabilities` catalog reflects all wired providers (no breaking changes to existing ops).
- Policy guard applies recipient-domain rules to `email.smtp` and `email.gmail` (same as `smtp.send_mail`).
- SDK module search guidance includes `email`.

### Crate versions (this release)

| Crate | Version |
| --- | --- |
| `grapheme-signatures` | 0.4.0 |
| `grapheme-stdlib` | 0.4.0 |
| `grapheme-runtime` | 0.4.0 |
| `grapheme-compiler` | 0.4.0 |
| `grapheme-sdk` | 0.4.0 |
| `grapheme-cli` | 0.5.0 |
| `grapheme-lsp` | 0.5.0 |
| VS Code extension | 0.5.0 |
| `grapheme-artifact` | 0.2.0 (unchanged) |

### Notes

- **`smtp.send_mail`** remains available; the `email` module is additive.
- **Brave** uses a direct API integration because `websearch` 0.1.1 still stubs `BraveProvider`.

## [0.4.1] - 2026-05-24

### Changed

- CLI patch release on the 0.4.x tooling line.

## [0.4.0] - 2026-05-24

### Changed

- Major tooling and crate alignment: CLI/LSP/VS Code extension 0.4.x, compiler/runtime/signatures/stdlib 0.3.x → 0.4.x train (see git tag history for full scope).
- Parser and documentation updates for newline handling and release packaging.

## [0.3.0] - 2026-05

### Added

- Extended web provider routing examples, HTTP/websearch modules, and real-world scenario pack foundations.
