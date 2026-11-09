# RFC-0007: GraphemeUI — Views, Delegates and UI Applications v1

Status: draft
Authors: language + runtime
Created: 2026-11-07
Target release window: after RFC-0004 Part C (`uses`) and fragment Phase C

## Summary

Add the language pieces Grapheme needs to describe user interfaces, so that a separate GraphemeUI layer can render them natively (first target: [rndr](https://github.com/EntasisLabs/rndr)). Grapheme stays the workflow language; GraphemeUI is to Grapheme what SwiftUI is to Swift.

The design goal is to add as little new syntax as possible. Most UI behaviour maps onto constructs that already exist: struct literals, `mutation`, `query`, `iterator`, `subscription`, tags with `using`, directives and `@loop`. This RFC adds:

1. A pure **`view`** executable kind with parameters and a `View` result.
2. **Struct literals, view references and action references in value position**, so a view can return a tree.
3. **Defunctionalized delegates**: `Action` and `Template` values (a named executable plus bound arguments, never closures). These cover MudBlazor's `EventCallback<T>` and `RenderFragment<T>`.
4. **`@derive`**: pure, memoised queries whose results feed views through tags.
5. **`@task`**: iterators that run as cancellable UI tasks with a concurrency policy; mutation calls inside a task commit against live application state.
6. **UI glyphs**: directives on `glyph` (`@hot_reload`), `@expose` on definitions, and a defined lifecycle (initialise once, re-render per state revision).
7. Two verifier/runtime fixes found while writing the worked example: partial `set` patches under `-> T`, and `Int` preservation through `inc_field` / `dec_field`.
8. A serializable **View IR** that the compiler emits and renderers consume. Grapheme never depends on a renderer.

## Motivation

Grapheme already has typed state, explicit writes (`mutation`), governed effects (capabilities, policy, `@resilient`), event entrypoints (`subscription`) and scoped context (tags). Those are the hard parts of a UI architecture (Elm/SwiftUI/Blazor). What is missing is a way to *return a UI tree* from pure code and to *refer to behaviour* from that tree.

Without this, an app embedding Grapheme must describe UI in a second language and wire events by hand. With it:

- UI, state transitions and workflows are one verified program.
- Plugins can ship UI that is type-checked against the host's component catalog.
- A design canvas can export itself as Grapheme views.

## Layering

```text
GraphemeUI renderer (e.g. grapheme-ui + rndr)   ← consumes View IR, emits events
                    ▲ View IR (JSON, versioned)   │ Action invocations
Grapheme compiler/runtime                        ▼
  view (pure) · query @derive (pure) · mutation (writes) · iterator @task (effects)
```

Dependency rule: GraphemeUI depends on Grapheme and on its renderer. Grapheme does not depend on GraphemeUI or any renderer. The primitive catalog (`Column`, `Text`, `Button`, `VirtualList`, …) is published to the compiler as signatures, like a module.

## Design principles

1. **Views are pure.** A view may read `$state`, its parameters and tags, call other views and `@derive` queries, and nothing else. It can be re-run any time.
2. **Writes stay in `mutation`.** Matches the executable-kinds matrix (`docs/internal/roadmaps/executable-kinds-v1-draft.md`).
3. **Effects stay in `iterator`.** Capability calls, waits and host emits are workflows, under normal policy.
4. **Behaviour is referenced, not captured.** UI values hold `{ executable, args }`, never closures. The tree stays serializable, diffable and verifiable (RFC-0004 lists closures as a non-goal; this keeps it that way).
5. **No per-frame interpretation.** Views re-run per state revision; animation and painting are renderer data.
6. **The renderer owns input, focus, IME, text editing, layout, shaping and accessibility.** Text inputs are uncontrolled and report commits/changes as actions.

## What already works (verified against 0.7.2)

Checked with `grapheme compile --type-policy strict` and `grapheme run`:

| Need | Existing construct | Status |
| --- | --- | --- |
| App state type | `struct PanelState { … }` | works |
| Initial state | `query InitPanel on Any -> PanelState { PanelState { … } }` | works |
| Partial update | `mutation M($index: Int) on S { set { selected: $index } \|> inc_field(field: "presses") }` | works at runtime; rejected by verifier when `-> S` is declared (see Part G) |
| Counters | `inc_field` / `dec_field` | works; `Int` becomes `Float` (see Part G) |
| Conditional value | `if $state.selected > 0 then Label { … } else Label { … }` | works; the literal is merged into `$state` (see Part A.6) |
| Cascading context | `tag panel for [Header] { $selected_label: String }` + `using panel(…) { … }` | works |
| Inbound events | `subscription Pinged($count: Int) on S { call SetPings(count: $count) }` | works |
| Repeated steps | `iterator PulseStep on S @loop(max: 10, merge: "replace") { call DecPulse }` | works |
| String interpolation | `"{$state.presses} presses"` | works |
| Lane write | `apply state { … }` | writes the nested `state` lane, not root fields |

What does **not** fit today:

- `fragment` cannot take arguments, cannot be `call`ed, rejects directives and rejects expansion cycles. Components need all four (recursive trees such as layer panels need cycles). So components get their own kind rather than reusing `fragment`.
- `value` admits only literals, variables, symbols, lists and objects. A tree needs typed literals and references inside values.
- Objects become `serde_json` maps without `preserve_order`, so key order is not preserved. Child order must be a list.

## Worked example

The motivating panel, written against this RFC. Lines marked `NEW` depend on it.

```grapheme
import core from "grapheme/core"
import rows from "./rows"
import time from "grapheme/time"                                   // NEW module: time.sleep

struct PanelState {
  selected: Int
  presses: Int
  pulse: Int
  pings: Int
  shuffles: Int
}

struct Label {
  value: String
}

// Cascading values (MudBlazor CascadingValue): an RFC-0004 tag.
// With RFC-0004 Part C this becomes `view Panel uses panel …`.
tag panel for [Panel, Header] {
  $items: [rows.Item]
  $selected_label: Label
}

// ── Application root
glyph MyPanel @hot_reload(preserve: "state", tasks: "cancel") {     // NEW: directives on glyph
  InitPanel
  |> using panel(items: Items(), selected_label: SelectedLabel()) {  // NEW: derive results as tag values
       Panel
     }
}

query InitPanel on Any -> PanelState {
  PanelState { selected: 0, presses: 0, pulse: 0, pings: 0, shuffles: 0 }
}

// ── Derived values: pure, memoised on what they read ($state.shuffles)
query Items on PanelState -> [rows.Item] @derive {                 // NEW: @derive
  rows.items(count: 1000)
  |> rows.shuffle(seed: $state.shuffles)
}

query SelectedLabel on PanelState -> Label @derive {
  if $state.selected > 0 then Label { value: "#{$state.selected}" } else Label { value: "none" }
}

// ── Views
view Panel on PanelState -> View {                                 // NEW: view kind
  Column {                                                          // NEW: struct literal as a step value tree
    style: { width: full, height: full, fill: bg },
    children: [
      Header(),                                                     // NEW: view reference in value position
      Divider {},
      VirtualList {
        source: $items,
        key: "id",
        estimated_row_height: 28,
        style: { width: full, grow: 1 },
        row: rows.Row(selected: $state.selected, click: Select)     // NEW: Template (RenderFragment<T>)
      }
    ]
  }
}

view Header on PanelState -> View {
  Column {
    style: { width: full, padding: { x: 16, top: 14, bottom: 10 }, gap: 8 },
    children: [
      Row {
        style: { width: full, gap: 8, align: center },
        children: [
          Text { text: "Grapheme panel", style: { size: 15, weight: semibold } },
          Spacer {},
          Text {
            text: "{$items.count} rows · selected {$selected_label.value} · {$state.presses} presses",
            style: { size: 12, color: text_secondary }
          }
        ]
      },
      Row {
        style: { width: full, gap: 6, align: center },
        children: [
          PanelButton(label: "Shuffle", click: Shuffle),            // NEW: Action reference (EventCallback)
          PanelButton(label: "Pulse", click: Pulse),
          PanelButton(label: "Ping", click: Ping),
          Spacer {},
          Progress { value: $state.pulse, max: 10, style: { width: 120, height: 6 } },
          Text { text: "{$state.pings} pings", style: { width: 60, size: 12, color: text_secondary } }
        ]
      }
    ]
  }
}

view PanelButton($label: String, $click: Action) on Any -> View {   // NEW: view params, Action type
  Button {
    label: $label,
    click: $click,
    style: {
      padding: { x: 10, y: 4 }, radius: 6, fill: control_off, size: 12, weight: medium,
      hover: { fill: hover },
      pressed: { fill: press }
    }
  }
}

// ── State transitions (existing constructs; `-> PanelState` needs Part G)
mutation Select($index: Int) on PanelState -> PanelState @expose { // NEW: @expose
  set { selected: $index }
  |> inc_field(field: "presses")
}

mutation Shuffle on PanelState -> PanelState { inc_field(field: "shuffles") }
mutation SetPulse($step: Int) on PanelState -> PanelState { set { pulse: $step } }
mutation DecPulse on PanelState -> PanelState { dec_field(field: "pulse") }
mutation SetPings($count: Int) on PanelState -> PanelState { set { pings: $count } }
mutation IncPings on PanelState -> PanelState { inc_field(field: "pings") }

// ── Inbound events (existing)
subscription Pinged($count: Int) on PanelState {
  call SetPings(count: $count)
}

// ── Effects: iterators delegate writes to mutations
iterator Ping on PanelState @task(concurrency: "drop") {           // NEW: @task
  call IncPings
  |> host.emit(channel: "pinged", value: $state.pings)              // app-registered host module, policy-gated
}

iterator Pulse on PanelState @task(concurrency: "restart") {
  call SetPulse(step: 10)
  |> PulseStep
}

iterator PulseStep on PanelState @loop(max: 10, merge: "replace") {
  time.sleep(ms: 80)
  |> call DecPulse                                                  // commits live; the view re-renders
}

// ── Host-facing interface
query ReadSelected on PanelState @expose {
  core.pick(fields: ["selected"])
}

view BenchmarkPanel on PanelState -> View @expose {
  rows.Panel20(presses: $state.presses)
}
```

The `rows` module would declare, for example (all of it depends on this RFC):

```grapheme
view Row($item: Item, $index: Int, $selected: Int, $click: Action($index: Int)) on Any -> View {
  if $index == $selected
    then RowFrame(item: $item, highlighted: true, click: $click(index: $index))
    else RowFrame(item: $item, highlighted: false, click: $click(index: $index))
}
```

`VirtualList` supplies `$item` and `$index` for each visible row; `Row` binds the remaining `$index` argument of `Select` by partial application.

## Part A — `view` kind

```text
view Name ( $param: Type (= value)? , ... )? (uses tag, ...)? on In -> View directive* { pipeline+ }
```

Semantics:

1. Parameters follow RFC-0004 Part A (named, defaulted, bound at call sites).
2. `on In` is the state the view reads as `$state`. `on Any` marks a view that only uses parameters (reusable component).
3. The final step value must be a `View`: a primitive literal, a view reference, or a list of those.
4. Views may be recursive. Recursion is bounded by data (each level must be reached through a view reference, which the renderer evaluates lazily) and by the existing call-depth limit.
5. Views are callable (`call Header`, `Header()`), may carry directives (`@expose`), and create a runtime frame. This is the deliberate difference from `fragment`, which stays a directive-free inline macro.
6. **Result, not merge.** Today a struct-literal step merges into `$state` (`Label { value: "#2" }` on a `PanelState` yields `{ selected, presses, value }`). In a `view` and a `@derive` query, the value of the final step is the result and is not merged into the caller's state. Inside those kinds, `$state` is read-only.

Verifier rule matrix, extending executable-kinds v1:

| Construct | view | query `@derive` | query | mutation | iterator |
| --- | --- | --- | --- | --- | --- |
| Read `$state`, params, tags | allow | allow | allow | allow | allow |
| Call `view` | allow | deny | deny | deny | deny |
| Call `query @derive` | allow | allow | allow | allow | allow |
| Call other `query` / capability ops | deny | deny | allow | allow | allow |
| `set` / `apply` / write-like ops | deny | deny | per profile | allow | deny (delegate) |
| `transition` | deny | deny | allow | allow | allow |
| Produce `Action` / `Template` values | allow | deny | allow | allow | allow |

Pure stdlib transforms (`core.pick`, `core.merge`, string/array ops) are allowed in views and derives. The stdlib registry gains a `pure: bool` flag per op so the verifier can check this.

## Part B — Values: literals and references

Extend `value`:

```text
value = ... | typed_literal | reference
typed_literal = qualified_ident object_value            // Text { text: "hi" }
reference     = qualified_ident ("." ident)? arg_list   // Header(), rows.Row(selected: 1)
              | symbol_lit                               // Select, bg, full   (type-directed)
```

Bare symbols are resolved by the expected type: in an `Action` slot to an executable, in a style colour slot to a theme role, in an enum slot to a variant. An unresolved symbol is a compile error under `--type-policy strict`. Branch targets (`then Tick`) and `transition … -> approved` already resolve bare names this way.

Style values keep plain identifiers. Identifiers cannot contain `-`, so theme roles use `_` (`text_secondary`).

## Part C — Delegates (`Action`, `Template`)

Both are **defunctionalized**: a name plus a partial argument map, serialized as

```json
{ "$action": "Select", "args": { "index": 3 } }
{ "$view": "rows.Row", "args": { "selected": 0 } }
```

Types:

| Type | Meaning | MudBlazor analogue |
| --- | --- | --- |
| `Action` | Invocable with no remaining required args | `EventCallback` |
| `Action($value: String)` | Renderer/caller supplies `$value` | `EventCallback<string>` |
| `View` | A primitive or a fully applied view reference | `RenderFragment` |
| `Template($item: T, $index: Int)` | View reference the container completes per item | `RenderFragment<T>` |

Rules:

1. A reference is valid in an `Action` slot if it names a `mutation` or `@task iterator` whose `on` type matches the glyph state, and the bound plus slot-supplied args satisfy its parameters exactly. Checked at compile time.
2. Partial application: `$click(index: $index)` adds args to an `Action` value. Binding an unknown or already-bound name is an error.
3. Primitives declare which args they supply: `Button.click: Action`, `TextField.change: Action($value: String)`, `VirtualList.row: Template($item: T, $index: Int)`, where `T` is the element type of `source`. Until RFC-0004 Part F generics land, this element-type link is a built-in rule for catalog primitives only.
4. Binding (`@bind-Value`) is not hidden two-way state. It is written as `TextField { value: $state.query, change: SetQuery }`. A `bind:` sugar that generates the setter mutation is deferred.
5. Child content (`ChildContent`) is a `children: [View]` parameter; named slots are `View`-typed parameters (`header: View`).

## Part D — Derived values (`@derive`)

`@derive` marks a query as pure and memoised.

1. The verifier rejects capability calls and writes (matrix above).
2. The compiler records its read set (`$state.shuffles`, params, tags). The runtime recomputes it only when one of those changes. Reading `$state` as a whole falls back to every revision.
3. Derived values reach views through tags: `using panel(items: Items())`. With Part C of RFC-0004, views declare `uses panel`.
4. Module ops called from a derive (`rows.items`) must be declared pure in their signature.

## Part E — Tasks and signals

`@task(concurrency: "restart" | "drop" | "queue" | "parallel")` on an iterator makes it usable as an `Action` and defines what happens when it is triggered while already running. Default is `drop`.

1. A task runs against a snapshot of the app state. Each `call` to a mutation inside it is committed against the **live** state, and the task's `$state` is refreshed to the result. This replaces a `dispatch` keyword.
2. Every committed mutation produces a state revision; the renderer re-renders once per frame, coalescing revisions.
3. `restart` cancels at the next suspension point (`time.sleep`, capability call). Committed mutations are not rolled back.
4. `@retry`, `@timeout` and `@resilient` work unchanged on tasks.
5. Inbound signals are `subscription` entrypoints; the host delivers events with parameters. Outbound signals are module calls such as `host.emit(…)`, so they are capability-gated.
6. `time.sleep(ms:)` is a new op in a `time` module, policy-gated like any capability.

## Part F — UI glyphs

A glyph whose pipeline ends in a view is a **UI glyph**:

1. Steps before the first `using`/view step run once to produce initial state.
2. The view stage (including `using` with derives) is re-evaluated per state revision and emitted as View IR.
3. Events from the renderer arrive as `Action` invocations; the runtime runs the mutation or starts the task, then re-renders.

Directives:

- `glyph_def` gains `directive*`.
- `@hot_reload(preserve: "state" | "none", tasks: "cancel" | "keep")`. Preserved state is migrated by field: new fields take their declared default (required for new non-optional fields), removed fields are dropped, fields whose type changed are reset to their default with a diagnostic.
- `@expose` (optionally `@expose(name: "panel20")`) on `query`, `mutation`, `view` makes it part of the glyph's host API: queries are readable, mutations invocable, views mountable in a host slot.

## Part G — Verifier and runtime fixes

1. **Partial `set` under `-> T`.** `set { selected: $index }` in a mutation `on S -> S` is rejected today ("missing required fields in literal state patches") although the runtime carries the other fields over. When the input type is `S`, unpatched fields of `S` should count as present. This is the Grapheme form of `...$state` and avoids adding spread to struct literals.
2. **`Int` preservation.** `inc_field` on an `Int` field returns a `Float` (`5` → `6.0`). It should preserve the field's declared numeric type.

## Part H — View IR

The compiler emits view results as versioned JSON (`"$ir": "grapheme-ui/1"`):

```json
{
  "$prim": "Column",
  "key": "Panel",
  "props": { "style": { "width": "full", "fill": { "$role": "bg" } } },
  "children": [
    { "$view": "Header", "args": {} },
    { "$prim": "VirtualList", "key": "Panel/2",
      "props": { "source": { "$derive": "Items" }, "key": "id", "estimated_row_height": 28 },
      "slots": { "row": { "$view": "rows.Row", "args": { "selected": 0, "click": { "$action": "Select", "args": {} } } } } }
  ]
}
```

1. **Identity.** Each node's key is its view path plus list position, or an explicit `key:`. Renderers map keys to stable widget ids, preserving focus, scroll and local state.
2. **Laziness.** `$view` references are evaluated by the renderer on demand and memoised by `(view, args, read-set revision)`. A `VirtualList` evaluates `row` only for visible items, so 1,000 rows cost the visible rows.
3. **Large data by reference.** `$derive` values can be passed by handle rather than inlined, so the IR stays small.
4. **Primitives** are a catalog the renderer publishes (names, props, slots, supplied action args). The compiler type-checks literals against it, like module signatures today.

## Non-goals

1. Arithmetic and general expressions in values (`$state.pulse * 12`). Use semantic primitives (`Progress`) or derives; general expressions belong to the control-flow roadmap.
2. Closures or first-class functions.
3. Per-frame scripting, custom painting or animation code in Grapheme.
4. CSS-style selectors or cascade. Styles are values; shared styling uses tags or theme roles.
5. Any renderer dependency in Grapheme crates.

## Open questions

1. **State lane.** Should UI state be the root `$state` (as here) or the `data` lane per the executable-kinds convention (`apply data { … }`)? Root reads are shorter; lanes match strict profiles.
2. **Commas.** Object fields require `,` separators. Optional separators on newlines would make view trees much quieter, but is a grammar-wide decision.
3. **`view` vs extended `fragment`.** This RFC adds a kind. The alternative is giving fragments params, call-ability and recursion, which changes their inline-macro contract.
4. **Where the catalog lives.** Built into GraphemeUI, or published by each renderer as a module manifest.

## Rollout

1. **Phase 0:** Part G fixes (independently useful).
2. **Phase 1:** `view` kind, typed literals and references in values, `Action`/`Template` checking against a stub catalog; `grapheme compile --emit view-ir` for golden tests. No renderer.
3. **Phase 2:** `@derive` with read sets; UI glyph lifecycle in the runtime with a headless event-replay harness.
4. **Phase 3:** `@task`, `time.sleep`, `@hot_reload`, `@expose`.
5. **Phase 4:** GraphemeUI renderer on rndr consuming View IR.
6. **Phase 5:** LSP: completion and hover for primitives, props and action signatures.

## Acceptance

1. The worked example compiles under `--type-policy strict` and its View IR matches a golden file.
2. A headless replay of `click Select(index: 3)`, `click Pulse` and `subscription Pinged(count: 2)` produces the expected state revisions and IR diffs.
3. Verifier errors for: capability call in a view; write in a derive; `Action` bound to a query; missing or unknown action arg; unresolved symbol under strict policy.
4. Existing examples compile and run unchanged.
