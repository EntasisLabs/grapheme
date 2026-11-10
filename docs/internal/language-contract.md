# Language Contract

This document defines the current behavior contract for Grapheme Lang in this repository.

Status: prototype contract for current implementation, not a final spec.

## Purpose

Grapheme Lang is a workflow-oriented DSL for capability calls over a shared AgentState.

It is compiled and executed via this pipeline:

1. parse source to AST
2. lower AST to HIR
3. verify HIR (shape and selected type checks)
4. lower HIR to MIR
5. package MIR in an artifact envelope
6. execute artifact entrypoint in runtime

## Program Shape

Top-level definitions currently supported:

- import
- query
- mutation
- subscription
- iterator
- node (alias of iterator)
- schema
- module proposal

Planned (not yet implemented):

- fragment

Only executable definitions are lowered to MIR functions:

- glyph
- query
- mutation
- subscription
- iterator
- node (lowered as iterator)

## Entrypoint and Execution Scope

Artifact execution runs exactly one MIR function: the artifact entrypoint.

- If an explicit entrypoint is provided, that function is used.
- Otherwise, if a `glyph` exists, that glyph name is used as the entrypoint.
- Otherwise, the first MIR function is used.

Glyph constraints:

- A file may declare at most one `glyph`.
- `glyph` is intended as Program-style composition root (similar to `fn main()` orchestration).
- Pipelines inside a glyph execute exactly like query pipelines.

Implication: definitions after the selected entrypoint are not executed in that run.

## Pipeline Semantics

A pipeline is an ordered list of call steps.

- Steps execute left-to-right.
- Each step output becomes current AgentState.
- Runtime injects previous current state into step args as `__input`.

Important state-shape rule:

- A step can intentionally or accidentally change the shape of `$current` for all following steps.
- In `@loop(each: ...)` scopes, use `$state` for parent/root state, `$item` for the stable loop input item, and `$loop` for loop metadata.
- For example, `core.echo(message: "$current.url")` yields `{ message: ... }`, so a following step like `http.get(url: "$current.url")` will no longer see `url` unless you preserved it earlier.
- Practical guidance: place transformation/logging steps that narrow shape (like `echo`) after steps that still need the original fields, or explicitly pass required fields forward.
- Prefer `core.tap` for diagnostics when downstream steps still need unchanged `$current`.
- `$current` remains supported as a compatibility alias, but `$state/$item/$loop` is the canonical namespace model.

Calls are represented by module/op + capability:

- module-qualified call: `Module.op(...)`
- bare call: `op(...)`

At runtime, module resolution uses:

1. explicit module from MIR step, if present
2. otherwise capability prefix before the first dot

## Query, Mutation, Subscription (Current Behavior)

At compile and runtime layers today, query/mutation/subscription are structurally equivalent.

- They are all lowered to MIR functions with call sequences.
- Runtime does not currently implement transactional mutation semantics.
- Runtime does not currently implement streaming/event loop subscription semantics.

They differ today mainly by function kind metadata and source intent.

## Executable Parameters and Tagged Variables (0.7.0)

Shipped under RFC-0004 Phases 1–2a. Normative author extract:
`docs/internal/language/params-and-tags-v1.md`.

### Executable parameters

`query` / `mutation` / `iterator` / `subscription` may declare named parameter lists with optional defaults. Parameters lower to MIR frame locals and bind at call sites or at the entrypoint:

```gr
query Hello($label: String = "world") {
  call Greet(label: $label)
}
```

Entrypoint binding:

```bash
grapheme run examples/params-call-bind.gr --args-json '{"label":"grapheme"}'
```

SDK: `with_entrypoint_args(json!({ "label": "grapheme" }))`.

### Tags and scoped `using`

`tag` schemas declare ambient bindings. Block `using` activates them for nested code only:

```gr
tag auth { $token: String }

query Demo {
  using auth(token: "secret") {
    core.echo(message: "token={$token}")
  }
}
```

Out of 0.7.0: tag-typed parameters as the fundamental call-edge model, and `uses` sugar (RFC-0004 Phase 3+).

Canonical examples: `examples/params-call-bind.gr`, `examples/tag-using-scope.gr`.

## Value Expressions (RFC-0007 Part B)

Any value position (step args, `set`/`apply` fields, list items, defaults) accepts a pure expression:

```gr
set {
  count: $state.count + 1,
  density: if $state.density == compact then comfortable else compact,
  big: $state.count >= 10 && !$state.archived
}
```

Precedence, loosest first: `if … then … else`, `||`, `&&`, comparison (`==` `!=` `<` `<=` `>` `>=`, non-associative), `+` `-`, `*` `/` `%`, unary `!` `-`. Parentheses group.

- `Int op Int` stays `Int`; `/` truncates toward zero. Mixing `Int` and `Float` gives `Float`. `+` also joins two strings.
- `==`/`!=` compare numbers by value (`1 == 1.0`) and other values structurally. Bare enum members compare as their string value.
- `&&`, `||` and `if` short-circuit. `false` and `null` are falsy.
- Integer overflow, division by zero and operand type mismatches fail the step with `EXPRESSION_ERROR`.
- Expressions cannot call executables or module ops; calls stay pipeline steps.
- Expressions lower to `{"$expr": {"op": "+", "args": [..]}}` in HIR/MIR and are evaluated by both Stage A and Stage B runtimes.

`.count` on a list or string reads its length (`$state.items.count`). A struct field named `count` still wins.

Operand types come from the input type, typed params and literals. Under `--type-policy strict` a definite mismatch is a type error; under `warn` it is an `expression-type` lint. Checked:

- arithmetic and ordering operands (`"a" - 1`, `$name > 3`);
- `&&`, `||`, `!` and `if` conditions take `Bool`;
- both `if` branches have the same type;
- a `set`/`core.merge` field's expression matches the field type (`Int` widens to `Float`);
- bare symbols compared with or assigned to an enum are members of it.

Operands with no declared type (locals, loop items, `Any`) are not checked.

A query body may be a single expression instead of a pipeline; its value is the query's result (RFC-0007 B.5):

```gr
query Doubled on Counter -> Int { $state.count * 2 }
query Label on Counter -> String { if $state.on then "on" else "off" }
```

- It lowers to one `core.value(value: <expr>)` step, so the run's final `current` is the value itself (a scalar, list or object), not the input object.
- Under `strict` the expression's type must fit the declared `-> T` (`Int` widens to `Float`; enum symbols must be members); under `warn` a mismatch is an `expression-type` lint.
- Only queries take expression bodies. Mutation, iterator and fragment bodies stay pipelines.
- A body that parses as a pipeline (for example a bare op name like `{ ping }`) stays a pipeline.

## State Stores (RFC-0007 Part A, step 1)

A `state` declaration is a store: a runtime object separate from the pipeline state, with typed fields and per-store revisions.

```gr
enum Mode { idle, busy }

state_machine Work on Mode {
  transition idle -> busy
  transition busy -> idle
}

state Counter {
  count: Int = 0
  const step: Int = 2
  transient pulse: Int = 0
  mode: Mode = idle
  derive double: Int = $count * 2
}

mutation Increment on Counter {
  apply { count: $count + $step, pulse: $pulse + 1 }
}

mutation Start on Counter {
  transition $mode -> busy
}

query Total on Counter {
  set { total: $count + $double }
}
```

- **Fields.** A plain field is written by `apply`. `const` is never written after creation. `transient` is written by `apply` and appears in runtime diffs but not history diffs. `derive` is computed from fields and other derives and is read-only. An enum field whose enum has exactly one `state_machine` is a lifecycle field, written only by `transition`.
- **Names.** Inside an executable `on Counter`, fields are bare `$count`; `$state.count` still works. Derives are inlined where they are read. A parameter named like a field is a compile error.
- **Writes.** `apply { … }` (no lane) writes the executable's own store. `apply state { … }` and `apply data { … }` keep their lane meaning. `apply` naming a const, derive, lifecycle or unknown field is a compile error, as is lane-less `apply` outside a store executable.
- **Mutations** on a store may only `apply`, `transition`, branch, and call executables on the same store; module/capability ops are not allowed (use a task). **Queries** on a store are read-only: no `apply` or `transition`, and they may only call queries.
- **Tasks** (`task Name on Store { … }`) are store-bound effect workflows. They may call capabilities. Each of the task's own `apply`/`transition` steps commits its own revision; earlier commits are kept if a later step fails. A mutation the task calls is one transaction: every `apply` in that call commits together, or none do if the call fails. `$state` is preserved across capability calls (a separate lane from `$current`), so `apply { posts: $current }` after `core.value` / `http.get` works, and bare `$field` in `if` / `match` reads that lane. Capability calls (not pure `core.value` / `core.echo`) produce effect receipts on `StoreCommit.effects`, with a `cause` of `{action, task, generation}`. A refused or failed call is a `failed` receipt, and a failed task returns that list on the error. A store field named `error` is data. `__input.error` still fails a step when the pipeline input is a previous op's result, not the store.
- **Defaults** may not read fields or variables; an enum default must be a member. Derive cycles are compile errors.
- **Transactions.** A mutation is one transaction: it runs against the latest committed fields and commits one revision `{store, revision, cause, diff, history_diff}` with `diff: {field: [old, new]}`. A run that changes nothing commits no revision. A failed step commits nothing. Each `transition` is checked against the state machine at commit time (illegal edge or terminal member → `invalid_transition`); a declared self-edge counts as a change.
- **Running.** Store executables are not file entrypoints. They run through a store session: `GraphemeEngine::store_session(source)` in the SDK, `StoreSession` in `grapheme-runtime`, or `grapheme session file.gr --invoke Increment --invoke 'Add:{"by":3}' --query Total`, which prints revisions, query results and each store's snapshot (fields plus derives) as JSON.
- **Not yet:** `singleton`/`transient state` lifetimes, `state … on View`, registries, `history`/`undo`, hot reload, `by Machine` for enums with several machines, store subscriptions, task `@concurrency` / `@if_unchanged` / `time.sleep`. Lifetimes and `on View` are compile errors for now.

## Views (RFC-0007 Parts C, D, H, I)

A `view` declares an element tree over a store. It is data: the compiler lowers it to View IR and the runtime evaluates it against the store's latest revision.

```gr
state Counter {
  count: Int = 0
  derive double: Int = $count * 2
}

mutation Add($by: Int) on Counter {
  apply { count: $count + $by }
}

view CounterView on Counter {
  Column(gap: 8) {
    value: Text("Count: {$count} (doubled: {$double})")
    add_five: Button(label: "Add 5") { on click: Add(by: 5) }
    footer: Footer
  }
}

view Footer {
  Text("Built with Grapheme views")
}
```

- **Shape.** `view Name [on Store] { Element }` has exactly one root element. An element is `[name:] Kind [(props) | (content)] [{ body }]`; the body holds child elements, `if <expr> { … }` and `for $item in <expr> key <expr> { … }`, and `on event: Mutation[(args)]` bindings, separated by newlines or commas. A `|>` step or a module call in a view is a compile error.
- **Catalog.** `Column` and `Row` (props `gap: Int`, `label: String`; children allowed), `Text` (positional content of any type; `{$field}` interpolation in strings) and `Button` (props `label: String`, `enabled: Bool`; event `click`; children allowed). Unknown elements, props, events, duplicate props, positional content on a primitive without it, and children of `Text` are compile errors. Any other element name must be a declared view, used without props or a body; a view `on` a store can only be used from a view on the same store.
- **Scope.** Inside `on Counter`, `$count` and derives are readable; reading an unknown field, or any field in a view without `on`, is a compile error.
- **Bindings.** The target must be a mutation or task on the view's store. Its arguments must match the target's parameters exactly (no missing, unknown or repeated arguments; literal and field types must fit). An event can be bound once per element.
- **Keys.** The root's key is the view name; a named child appends `/name`, an unnamed child appends its ordinal among unnamed siblings (`CounterView/0`). An unnamed `if` / `for` appends `/$if` or `/$for` (then `/$if1`, `/$for1`, …), which cannot match a child name. A referenced view's nodes are keyed under the reference's key. A `for` row key is a non-empty string or number with `/`, `#` and `%` escaped; a duplicate key fails the render.
- **Accessibility.** An element with an event binding and neither a `label` prop nor a `Text` descendant produces a `view-accessibility` lint.
- **Output.** `grapheme compile file.gr --emit view-ir` prints `{"$ir": "grapheme-ui/2", "views": [...]}`. A node is `$prim`, `$view`, `$if` (optional `$else`) or `$for`. `grapheme-ui/1` consumers drop the new fields and would render those children unconditionally, so compiler documents are v2. Programs without views compile to the same MIR as before.
- **Rendering.** `StoreSession::render(view)` (SDK: `GraphemeStoreSession::render`; CLI: `grapheme session file.gr --render CounterView`) evaluates props and binding arguments against the store snapshot, inlines referenced views, and returns `{view, store, revision, ir, actions}`. Each action is `{id, key, event, target, bound_args, payload_params, role, label}`; its `id` is the node key, the event and a **binding epoch** (`CounterView/add_five#click@1`), also written into the node's `events.<event>.id`. The epoch advances when the binding's target or evaluated arguments change. `invoke_action(id)` (CLI: `--action <id>`) re-evaluates the rendered views, rejects an id that is no longer current with `stale_action`, and otherwise runs the target exactly like `invoke`: one transaction, one revision.
- **UI glyphs.** `glyph CounterApp on Counter { expose Increment; CounterView }` mounts a store-backed app (RFC-0007 Part H). The body is one root element (a view reference or an inline tree); `expose Name, …` lists mutations, tasks, queries and views reachable outside the tree. `StoreSession::mount` / `GraphemeStoreSession::mount` (CLI: `grapheme session file.gr --mount`) returns `{glyph, store, revision, ir, actions}` where `actions` is the tree's bindings plus `expose:<Name>` entries for exposed mutations and tasks. `invoke_action("expose:Increment")` runs an exposed no-arg mutation or task; exposed queries stay on `query()`. Workflow glyphs (`glyph Name { pipeline }`) are unchanged and remain the only artifact entrypoints.
- **Conditionals and lists.** `if <expr> { elements } else { elements }` and `for $item, $index in <expr> key <expr> { elements }` are children of element bodies. The `if` condition must be `Bool`. Rendered trees (`"$ir": "grapheme-ui/1"`) omit false `if` branches (and use `else` when present) and expand `for` rows under `…/$for/<key>/…`. Nested `for` bodies see outer items. `$index` is the row's position. Inside a `for`, `$item` and `$item.field` read the current row.
- **Not yet:** styles, view params and event declarations (so tree-bound `payload_params` is always empty), slots, `VirtualList`, `transient state` on views, lazy/memoised `$view` evaluation, `expect_revision`, `origin: "agent"`, and any renderer.

## State Contract

Runtime state tracks:

- current
- diff
- errors
- pipeline
- proposed

Selection blocks can request these selectors via `state { ... }`.

## Module Dispatch Contract

Each module has a manifest with:

- module id
- exported ops
- ABI (`mir_v1` or `wasix_v1`)

Dispatch rule:

- if a wasm path is bound, runtime dispatches through Wasix
- otherwise runtime dispatches through declared manifest ABI

Current default deployment in this repo:

- core/docs/io/secrets: primarily wasm plugin path when bound
- http/tcp/smtp: host-backed by default in CLI runtime
- websearch: host-backed by default in CLI runtime (DuckDuckGo provider via `websearch` crate)

## Policy and Failure Contract

Execution gates:

1. capability policy allow-list
2. policy guard argument checks

Failure behavior:

- denied capability/policy -> fatal execution outcome
- host fatal/retryable errors map to runtime failure outcomes
- if `__input.error` exists on a previous op's result, runtime short-circuits with fatal failure. A store snapshot passed as the pipeline input is not that signal, so a field named `error` does not abort the step.

## Type Verification Contract (Current)

Verifier enforces selected arg requirements/types for known module ops.

- missing required args: compile-time error
- mismatched JSON type: compile-time error
- unknown op on known module: compile-time error

Unknown modules are currently tolerated by verifier type tables.

## Non-Goals (Current)

Not implemented as fully finalized language/runtime guarantees yet:

- unbounded recursion without runtime policy bounds
- transactional mutations
- native streaming subscriptions
- RFC-0004 Phase 3+ (tag-typed parameters as fundamental call-edge; `uses` sugar)

Current control-flow capabilities now include iterator/node loops, iterator/node invocation, and branch dispatch (`flow.branch`) lowered through compiler-to-MIR.

Core std helper expansion (current behavior):

- Core transform ops now include list/object/string helpers such as:
	- list/object: `map`, `filter`, `find`, `reduce`, `group_by`, `merge`, `pick`, `validate_schema`, `get_path`, `set_path`, `has_path`
	- flow/data helpers: `tap`, `value`, `pack_state_data`, `get_state`, `get_data`
	- string/text: `split`, `join`, `replace`, `trim`, `lower`, `upper`, `contains`
- These ops are exposed via module manifest discovery (`grapheme modules info/types core`) and enforced by verifier arg checks.

HTML conversion options passthrough (current behavior):

- `html.to_md` accepts an optional `options` object that is parsed as `html-to-markdown-rs` conversion options.
- This enables richer conversion and metadata/document extraction control from Grapheme programs.
- Backward compatibility is preserved: `html.to_md()` still works with defaults when `options` is omitted.
- `websearch.research_report` accepts optional `md_options` and forwards it to each internal `html.to_md` conversion.

Resilience composition sugar (current behavior):

- `@resilient` is supported on executable definitions as compile-time sugar.
- It expands into any of the nested directive objects it provides:
	- `loop: { ... }` -> `@loop(...)`
	- `retry: { ... }` -> `@retry(...)`
	- `timeout: { ... }` -> `@timeout(...)`
- `@resilient` cannot be combined with explicit `@loop`, `@retry`, or `@timeout` on the same definition.

Directive object shorthand:

- Directives support both argument-list and object forms.
- Equivalent forms:
	- `@retry(max: 2, on_fail: Fallback)`
	- `@retry { max: 2, on_fail: Fallback }`

Intent annotation and trace surfacing (current behavior):

- Executable definitions support an attribute-style intent annotation:
	- `#[intent(goal = "validate canary before 50% rollout", risk = high)]`
- Current compiler lowering maps it to runtime metadata fields:
	- `goal` (string)
	- `risk` (string or symbol)
- Runtime trace step entries carry this metadata as:
	- `intent_goal`
	- `intent_risk`

Branch target normalization contract (current behavior):

- Applies to `if ... then ... else ...` and `match case/default => ...` targets.
- Plain target forms stay plain symbols (for example: `return`, `Step`, `call Step`) and are emitted as direct branch/match targets.
- Inline pipeline target forms (for example: `transition ...`, `set {...}`, or chained `... |> ...`) are lowered into synthetic helper iterators named `__inline_target_N` and branch/match targets point to those helpers.
- Single-step target spellings that are semantically plain symbols are preserved as symbols to avoid changing verifier/runtime behavior.

## Iterator and Fragment Contract (Proposed vNext)

Current implementation behavior:

- `iterator` is the only reusable executable unit.
- Reuse is modeled as call targets (`call Step` or bare iterator invocation).

Proposed split (non-breaking, additive):

- `iterator`: executable runtime unit.
- `fragment`: compile-time composition unit that expands inline into its caller.

Proposed `iterator` contract:

- Can be a direct call target.
- Can declare runtime directives (`@loop`, `@recursive`, `@retry`, `@timeout`).
- Produces visible runtime call graph and trace boundaries.

Proposed `fragment` contract:

- Cannot be a runtime call target.
- Cannot declare runtime directives.
- Expands inline before MIR lowering.
- Exists to reduce authoring verbosity and improve local readability.

Proposed verifier rules for `fragment`:

- Reject runtime directives on `fragment`.
- Reject recursive `fragment` expansion cycles.
- Preserve existing typed field checks and state-machine transition checks after expansion.

Migration and compatibility guidance:

- Keep existing `iterator` behavior unchanged.
- Add `fragment` as purely additive syntax and lowering.
- Optional codemod path: convert "pure helper" iterators with no directives into fragments.
- Preserve error wording and line mapping where possible by tracking expansion source spans.

Loop policy shift (current draft behavior):

- `@loop max` is optional at compile time.
- When `@loop` is present without max, loop semantics are unbounded at language level.
- Runtime policy (step budget) is expected to bound execution operationally.

Recursive policy shift (current draft behavior):

- `@recursive max_depth` is optional at compile time.
- Runtime call-depth policy is expected to bound recursive execution operationally.

Control-flow design draft for loops and recursion:

- `docs/language/control-flow-v1.md`

## Compatibility Guidance

When extending the language, keep these invariants stable unless versioned:

- left-to-right pipeline execution
- `__input` threading contract
- single-entrypoint artifact execution
- capability + policy checks before dispatch
- module ABI resolution order