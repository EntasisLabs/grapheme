# RFC-0007: GraphemeUI — State Stores, Views and Tasks v1

Status: draft (revision 3)
Authors: language + runtime
Created: 2026-11-07
Revised: 2026-11-09
Target release window: after the RFC-0004 0.7.0 parameter and tag work

## Summary

Add the language pieces Grapheme needs to describe interactive applications, so that a separate GraphemeUI layer can render them natively (first target: [rndr](https://github.com/EntasisLabs/rndr)) and agents can operate them headlessly. Grapheme stays the workflow language; GraphemeUI is to Grapheme what SwiftUI is to Swift.

A program has three surfaces, each with its own construct and its own rules:

| Surface | Construct | Contract |
| --- | --- | --- |
| **State** | `state Name { … }`, `register Name on Store { … }` | Typed fields with lifetimes and kinds, lifecycle fields, registries, atomic transactions, revisions and field diffs |
| **Views** | `view Name { … }` | Pure declarative projections of state, written as element blocks |
| **Behaviour** | `mutation`, `task`, `subscription` | Pure transactions, governed effects, inbound signals |

This RFC adds:

1. **State stores** with keyword-level semantics:
   - store lifetimes (`singleton state`, `state`, `transient state … on Component`);
   - field kinds (`const`, `transient`, `derive`) and lifecycle fields governed by the existing `state_machine`;
   - registries (`register … on Store`) that give a store its dependencies and capability grants;
   - transactions (`apply { … }`), revisions, a revision log (`history N`) and opt-in undo (`undo by action`).
2. A small **pure expression grammar** usable in any value position.
3. A **`view` kind** whose body is an element tree (`Name(props) { style, children, on event }`), never a pipeline.
4. **Typed events and delegates**: event declarations and event bindings are distinct and checked; behaviour is referenced, never captured.
5. A **`task` kind** for effects, with concurrency policies, generation fencing and optimistic guards.
6. **UI glyphs** with an `expose` clause; hot-reload behaviour follows from field and store kinds.
7. A versioned **View IR** and a **headless app session** API, so the same source drives a native renderer and an agent.
8. Two independent verifier/runtime fixes.

## Revision history

- **r1 (2026-11-07).** UI expressed through existing constructs: `struct` + init query for state, `fragment`-like `view` returning struct literals with `children: [ … ]`, `@derive` queries passed through tags, `@task` iterators delegating writes to helper mutations.
- **r2.** After review, a middle ground between r1 and the original friendly draft:
  - State becomes a declared store instead of a bare `$state` blob. Init queries, derive tags and helper mutations disappear.
  - Views get their own element grammar, so UI is visibly different from executable code, and named children keep their order.
  - Expressions (`+`, `==`, `if … then … else`) are added instead of avoided.
  - `task` becomes a kind; `apply` inside a task commits directly.
  - Event declarations vs bindings, `apply` across async boundaries, and what the revision log promises are specified (Parts D, F, A.8).
  - Headless agent interface added (Part I).
- **r3 (this revision).** Keywords replace `@` directives for what a store or property *is*:
  - Field kinds are leading keywords: `const`, `transient`, `derive`. `@transient` is gone.
  - Store lifetimes are declared on the store: `singleton state`, `state`, `transient state … on Component`. `transient` means "wipeable" at both levels.
  - Lifecycle fields reuse `state_machine` unchanged: a field whose enum has a machine is written only by `transition`.
  - `register Name on Store { … }` declares a store's dependencies (other stores) and capability grants; everything `on` the store inherits them.
  - `history N` and `undo by action` are store clauses; `expose` is a glyph clause; `@hot_reload` is removed.
  - `$state.` is dropped where the store is known from scope: `$selected`, and `$Prefs.density` for inherited stores.
  - Catalog ops are marked `pure query …`. Tasks are not undoable (no `@undoable` yet).

Syntax lineage for one fragment of the panel:

```grapheme
// original draft
shuffle: PanelButton { label: "Shuffle"  click: Shuffle }
// r1
PanelButton(label: "Shuffle", click: Shuffle),
// r2, r3
shuffle: PanelButton(label: "Shuffle") { on click: Shuffle }
```

And for state:

```grapheme
// r2
state Panel @history(limit: 100) @undo(group: action) {
  pulse: Int = 0 @transient
}
mutation Select($index: Int) on Panel { apply { selected: $index, presses: $state.presses + 1 } }
// r3
state Panel {
  history 100
  undo by action
  transient pulse: Int = 0
}
mutation Select($index: Int) on Panel { apply { selected: $index, presses: $presses + 1 } }
```

## Motivation

Grapheme already has typed data, explicit writes (`mutation`), governed effects (capabilities, policy, `@retry`, `@timeout`, `@resilient`), inbound events (`subscription`) and scoped context (tags). Those are the hard parts of a UI architecture. What is missing is a first-class place for application state, a way to describe a UI tree that reads differently from a workflow, and a way for that tree to *refer to* behaviour.

With them:

- UI, state transitions and workflows are one verified program.
- Plugins ship UI that is type-checked against the host's component catalog.
- A design canvas can export itself as Grapheme views.
- Agents operate the same application through its state, its legal actions and its diffs, rather than through pixels.

## Layering

```text
 native renderer (grapheme-ui + rndr)       agent / test harness
              ▲ View IR      │ action            ▲ snapshot   │ invoke
              └──────────────┴──── app session API ┴────────────┘
 Grapheme compiler/runtime
   state (store) · view (pure) · mutation (transaction) · task (effects) · subscription (inbound)
```

Dependency rule: GraphemeUI depends on Grapheme and on its renderer. Grapheme does not depend on GraphemeUI or any renderer. The primitive catalog (`Column`, `Text`, `Button`, `VirtualList`, …) is published to the compiler as signatures, like a module.

## Design principles

1. **Two shapes, never mixed.** UI is `Name(props) { … }` and only appears in `view`. Code is `op(args) |> op(args)` and never appears in `view`.
2. **State lives in a store.** Only `apply` writes it; everything else reads a revisioned snapshot.
3. **Views are pure.** They read state, parameters and tags, and can be re-run at any time.
4. **Mutations are pure transactions; tasks own effects.** Capability calls, waits and host emits happen in tasks, under normal policy.
5. **Behaviour is referenced, not captured.** UI values hold `{ executable, args }`, never closures, so the tree is serializable, diffable and verifiable.
6. **No per-frame interpretation.** Views re-run per state revision, only for the fields they read. Animation and painting are renderer data.
7. **The renderer owns input, focus, IME, text editing, layout, shaping and accessibility.** Text inputs are uncontrolled and report changes as actions.
8. **Humans and agents use the same surface.** Anything a user can click, an agent can invoke, through the same checks and policy.
9. **Keywords say what a thing is; directives say how it runs.** Lifetimes, field kinds, lifecycles, registration and exposure are keywords. Execution policy (`@concurrency`, `@retry`, `@timeout`, `@loop`) stays as directives.

## What already works (verified against 0.7.2)

Checked with `grapheme compile --emit hir --type-policy strict` and `grapheme run`:

| Need | Existing construct | Status |
| --- | --- | --- |
| Partial root patch | `set { selected: $index }` | works |
| Counter update | `inc_field(field: "presses")` | works; `Int` becomes `Float` (Part J) |
| Inbound signal with payload | `subscription Pinged($count: Int) on Panel { set { pings: $count } }` | works |
| Repetition | `iterator … @loop(max: 3, merge: "replace")` | works |
| String interpolation | `"{$state.presses} presses"` | works |
| Scoped context | `tag` + `using` (RFC-0004) | works |
| Lane write | `apply state { … }` | writes the nested `state` lane, not root fields |
| Enum lifecycle | `state_machine L on Status { transition a -> b; terminal b }` | works; bound by enum type, and consecutive literal writes in a pipeline are checked against its edges |

What does **not** fit today, and why r2 adds syntax instead:

- `value` admits only literals, variables, symbols, lists and objects. `1 + 2` and `$a == $b` do not parse in values; comparisons exist only in `if`/`match` steps.
- A bare expression (`$state.selected`) is not a valid query body.
- Objects become `serde_json` maps without `preserve_order`, so named children cannot be carried by object literals. Views need their own grammar.
- `fragment` cannot take arguments, cannot be called, rejects directives and rejects cycles. Views need all four.
- Iterators cannot write state under the strict profile, which forced r1's helper mutations.
- `apply { a: 1 }` already parses, as a struct-init step of a type named `apply`, and merges into the root. This RFC reserves `apply` with no lane as the store transaction (Part E). No existing example uses it.
- `state { current }` is a selection-set selector today. The top-level `state Name { … }` definition is a different context and does not conflict.
- `transition $state.status -> approved` writes through the `state` lane today (`transition $state.mode` against a plain struct is rejected with "field 'state.mode' is not declared"). Lifecycle fields (A.4) need it to target a store field, with its edge check enforced at commit time as well as at compile time.
- Program-scoped `using mutable $h: Tag(…)` parses, but handles are not resolved at runtime yet, so they are not a basis for stores.

## Worked example

```grapheme
import rows from "./rows"
import time from "grapheme/time"

// ── Lifecycles (existing syntax, unchanged) ─────────────────────
enum PanelMode { idle, pulsing }
enum Density { comfortable, compact }

state_machine PulseLifecycle on PanelMode {
  transition idle -> pulsing
  transition pulsing -> idle
}

// ── State ────────────────────────────────────────────────────────
// Keywords say what a store or field is (Part A):
//   singleton state / state / transient state   store lifetimes
//   const / transient / derive                  field kinds
// `mode` is a lifecycle field: PulseLifecycle governs PanelMode.
state Panel {
  history 100
  undo by action

  const page_size: Int = 1000
  selected: Int = 0
  presses: Int = 0
  transient pulse: Int = 0
  mode: PanelMode = idle
  pings: Int = 0
  shuffles: Int = 0

  derive items: [rows.Item] =
    rows.items(count: $page_size) |> rows.shuffle(seed: $shuffles)

  derive selected_label: String =
    if $selected > 0 then "#{$selected}" else "none"
}

singleton state Prefs {
  density: Density = comfortable
}

// Attached to a component: one per PanelButton instance, wiped when it unmounts.
transient state ButtonHint on PanelButton {
  shown: Bool = false
}

// ── Registration ─────────────────────────────────────────────────
// Everything `on Panel` inherits these stores and capability grants.
register PanelRegistry on Panel {
  Prefs
  time
  host
}

// ── Application ──────────────────────────────────────────────────
glyph MyPanel on Panel {
  expose Select, ReadSelected, BenchmarkPanel

  PanelView
}

// ── Views ────────────────────────────────────────────────────────
// Inside `on Panel`, Panel's fields are `$field`; inherited and attached
// stores are qualified: `$Prefs.density`, `$ButtonHint.shown`.
view PanelView on Panel {
  Column {
    style { width: full, height: full, fill: bg }

    header: Header
    divider: Box { style { width: full, height: 1, fill: separator } }

    items: VirtualList(source: $items, key: "id", estimated_row_height: 28) {
      style { width: full, grow: 1 }

      row($item, $index): rows.Row(item: $item, selected: $index == $selected) {
        on click: Select(index: $index)
      }
    }
  }
}

view Header on Panel {
  Column {
    style { width: full, padding: { x: 16, top: 14, bottom: 10 }, gap: if $Prefs.density == compact then 4 else 8 }

    heading: Row {
      style { width: full, gap: 8, align: center }

      title: Text("Grapheme panel") { style { size: 15, weight: semibold } }
      Spacer
      summary: Text("{$items.count} rows · selected {$selected_label} · {$presses} presses") {
        style { size: 12, color: text_secondary }
      }
    }

    controls: Row {
      style { width: full, gap: 6, align: center }

      shuffle: PanelButton(label: "Shuffle") { on click: Shuffle }
      pulse:   PanelButton(label: "Pulse")   { on click: Pulse }
      ping:    PanelButton(label: "Ping")    { on click: Ping }
      Spacer

      progress: Box {
        style { width: 120, height: 6, radius: 3, fill: separator }
        bar: Box { style { width: $pulse * 12, height: 6, radius: 3, fill: "#009fff" } }
      }

      pings: Text("{$pings} pings") { style { width: 60, size: 12, color: text_secondary } }
    }
  }
}

view PanelButton($label: String, $hint: String = "", on click: Action!) {
  Button(label: $label) {
    on click: $click
    on long_press: ShowHint
    style {
      padding: { x: 10, y: 4 }, radius: 6, fill: control_off
      hover { fill: hover }
      pressed { fill: press }
    }
    Text($label) { style { size: 12, weight: medium } }
    if $ButtonHint.shown { Tooltip($hint) }
  }
}

// ── State transitions ────────────────────────────────────────────
mutation Select($index: Int) on Panel {
  apply { selected: $index, presses: $presses + 1 }
}

mutation Shuffle on Panel {
  apply { shuffles: $shuffles + 1 }
}

// Runs against the ButtonHint of the PanelButton that raised the event.
mutation ShowHint on ButtonHint {
  apply { shown: true }
}

mutation ToggleDensity on Panel {
  apply Prefs { density: if $Prefs.density == compact then comfortable else compact }
}

// ── Inbound signal ───────────────────────────────────────────────
subscription Pinged($count: Int) on Panel {
  apply { pings: $count }
}

// ── Effects ──────────────────────────────────────────────────────
task Ping on Panel {
  apply { pings: $pings + 1 }
  |> host.emit(channel: "pinged", value: $pings)
}

task Pulse on Panel @concurrency(restart) {
  transition $mode -> pulsing
  |> apply { pulse: 10 }
  |> loop(max: 10) {
       time.sleep(ms: 80)
       |> apply { pulse: $pulse - 1 }
     }
  |> transition $mode -> idle
}

// ── Host-facing interface ────────────────────────────────────────
query ReadSelected on Panel -> Int {
  $selected
}

view BenchmarkPanel on Panel {
  rows.Panel20(presses: $presses)
}
```

The `rows` module would declare, for example (illustrative; bodies elided):

```grapheme
struct Item { id: String, label: String }

pure query items($count: Int) on Any -> [Item] { … }
pure query shuffle($seed: Int) on [Item] -> [Item] { … }

view Row($item: Item, $selected: Bool, on click: Action) {
  Box(label: $item.label) {
    on click: $click
    style { width: full, height: 28, padding: { x: 16 }, fill: if $selected then selection else transparent }
    Text($item.label) { style { size: 13 } }
  }
}
```

`VirtualList` declares a `row` template slot that supplies `$item` and `$index` for each visible row. The panel fills it with `rows.Row` and binds `Row`'s `click` event to `Select(index: $index)`.

## Part A — State stores

```text
store      = lifetime? "state" Name ("on" ViewName)? "{" store_item+ "}"
lifetime   = "singleton" | "transient"
store_item = clause | field | derive
clause     = "history" int_lit
           | "undo" "by" ("action" | "transaction")
field      = field_kind? ident ":" type_ref "=" expr
field_kind = "const" | "transient"
derive     = "derive" ident ":" type_ref "=" (expr | pure_pipeline)

registry   = "register" Name "on" Store "{" (Store | module | module "." op)+ "}"
```

Separators between store items and registry entries are newlines or commas. No store or field semantics use `@` directives. `singleton`, `transient`, `const`, `derive`, `history`, `undo`, `register` and `expose` are contextual keywords: reserved only at the start of a top-level definition, store item or glyph clause, so existing identifiers and op names keep working. Only `const` is already a keyword (`using const`).

### A.1 Fields

| Keyword | Written by | Revision log and undo | Hot reload |
| --- | --- | --- | --- |
| *(none)* | `apply` | logged; undoable | kept |
| `const` | nothing after creation; `apply` is a compile error | — | kept, unless its default changed |
| `transient` | `apply` | not logged; not undone | reset to default |
| `derive` | nothing; computed (A.2) | not logged (follows from fields) | recomputed |
| lifecycle (A.4) | `transition` only | logged; undoable along legal edges | kept |

1. Every field has a type and a default expression. Defaults may not read other fields. The store's first revision (revision 0) is built from the defaults, plus host-supplied initial values for `const` fields. No init query is needed.
2. Field writes go through `apply` (Part E) or `transition` (A.4). Everywhere else, fields are read-only values from one revision.
3. A store is a type: `on Panel` means "runs against the `Panel` store". Mutations and subscriptions on a store have an implied output type (the store); `-> T` is not written.
4. A store is a runtime object, separate from the pipeline `AgentState`. Stores are available to any glyph, UI or not.

### A.2 Derives

1. A derive is a read-only field computed from fields, other derives and pure module ops. It is read like a field: `$items`.
2. The verifier rejects capability calls and writes in a derive. Module ops it calls must be declared `pure query …` in their signature.
3. The compiler records each derive's read set (`items` reads `page_size` and `shuffles`). The runtime recomputes a derive only when a field in its read set changes, and lazily, on first read after that change.
4. Derive cycles are a compile error.

### A.3 Transactions and revisions

1. Each committed transaction produces exactly one revision: a monotonically increasing integer plus a field diff `{ field: [old, new] }`. Derives are not part of diffs.
2. A mutation is one transaction, however many `apply` and `transition` steps it contains. A task commits one transaction per step (Part F).
3. The store serializes commits; there is one writer at a time. Revisions are applied in commit order.
4. Views and derives are invalidated per field: a revision re-runs only the views whose compile-time read set intersects its diff. The renderer coalesces revisions per frame.

### A.4 Lifecycle fields

A field whose enum type has a `state_machine` is a **lifecycle field**. The machine is the existing construct, unchanged:

```grapheme
enum PanelMode { idle, pulsing }

state_machine PulseLifecycle on PanelMode {
  transition idle -> pulsing
  transition pulsing -> idle
}

state Panel {
  mode: PanelMode = idle
}
```

1. Only `transition $mode -> pulsing` writes a lifecycle field. Naming it in `apply` is a compile error.
2. Edges are checked statically where the current value is known (as today) and at commit time always. An illegal edge fails with `invalid_transition` and commits nothing.
3. Once the field reaches a `terminal` member, no further transition is accepted.
4. The default must be a member of the enum. If more than one machine is declared for the same enum, the field names its machine: `mode: PanelMode = idle by PulseLifecycle`.
5. `transition` keeps its optional payload object, whose fields are applied in the same transaction: `transition $mode -> idle { pulse: 0 }`.

### A.5 Store lifetimes

A store's lifetime is declared on the store, with the same words a field uses:

| Declaration | Instances | Shared? | On reload or unmount |
| --- | --- | --- | --- |
| `singleton state Prefs { … }` | one per registration, created at start-up | every registry that lists it gets the same instance | kept on hot reload |
| `state Panel { … }` | one per owner: the glyph that is `on` it, or the one registry that lists it | not shareable | kept on hot reload |
| `transient state Hint on PanelButton { … }` | one per instance of the owning view | each owner instance has its own | wiped when its owner unmounts, and on hot reload |

1. `transient` means **wipeable** at both levels. A transient field is a wipeable value inside a durable store; a transient store is wipeable as a whole.
2. A transient store's `on` names a view, not a store. Inside that view it is read qualified (`$ButtonHint.shown`). A mutation `on ButtonHint` bound to an event inside that view runs against the instance owned by the node that raised the event (identified by its View IR key).
3. Transient stores are never logged or undone.

### A.6 Registries

A registry declares what a store brings with it:

```grapheme
register PanelRegistry on Panel {
  Prefs      // a store dependency
  time       // a capability grant: every op in the module
  host.emit  // a capability grant: one op
}
```

1. **Inheritance.** Everything `on Panel` (its glyph, views, mutations, tasks and subscriptions) inherits the registry. Inherited stores are read as `$Prefs.density` and written with `apply Prefs { … }`. There is no per-definition `uses`.
2. **Capabilities.** Grants are a declarative layer over the host's capability policy. They can only narrow it: a call must be allowed by both. A task on a store with a registry may only call ops its registry grants; an ungranted call is a compile error. A store with no registry falls back to the host policy alone.
3. **Same store in several registries.** The outcome follows the store's lifetime:
   - `singleton` — one shared instance;
   - `transient` — one instance per registry;
   - plain `state` — a compile error (`ambiguous_store_owner`). The compiler cannot decide which owner the single instance belongs to.
4. A registry may not list its own store, and registry dependency cycles are a compile error.

### A.7 Names in scope

1. Inside a definition `on Panel`, Panel's fields and derives are in scope as `$field`. There is no `$state.` prefix.
2. Inherited (A.6) and attached (A.5) stores are read qualified by store name: `$Prefs.density`.
3. A parameter, payload variable or slot variable with the same name as a field in scope is a compile error, never silent shadowing.
4. Bare identifiers stay symbols (`fill: bg`), so `$` still marks every value read.

### A.8 `history` and `undo`

`history N` promises a **revision log**, nothing more. Each entry records:

```json
{ "revision": 42, "cause": { "kind": "mutation", "name": "Select", "args": { "index": 3 },
  "origin": "user", "action_group": 17, "task": null },
  "diff": { "selected": [0, 3], "presses": [4, 5] },
  "effects": [] }
```

- `cause.kind` is `mutation`, `task`, `subscription`, `undo`, `redo` or `reload`. `origin` is `user`, `agent`, `host` or `system`.
- A task's capability calls appear as `effects` entries (`{ "op": "host.emit", "args": { … } }`) on the revision they follow, or as entries with no diff. They are marked irreversible.
- Transient fields never appear in diffs.
- The log is bounded to `N` entries. Without `history`, the runtime still keeps the current revision number and the last diff, which invalidation needs.

`undo by action | transaction` is a separate opt-in layered on the log:

1. An **undo group** is either every revision caused by one action invocation (`by action`) or each revision (`by transaction`).
2. Only groups caused by mutations are undoable. Tasks and subscriptions are not undoable in this RFC.
3. Undo restores the group's non-transient fields to their values before the group, as a **new** revision with `cause.kind: "undo"`. It never rewinds the log. A lifecycle field is restored only if the reverse edge is legal; otherwise undo is rejected with `undo_conflict`.
4. Undo never reverts effects.
5. If a field the group wrote has since been written by a non-undoable cause, undo is rejected with `undo_conflict` and the store is unchanged.
6. Redo re-applies the group's diff as a new revision. Any new undoable group clears the redo stack.

### A.9 Hot reload

Reload behaviour follows from the keywords; there is no `@hot_reload` directive.

- Plain and lifecycle fields are kept. Added fields take their default, removed fields are dropped, and fields whose type changed are reset with a diagnostic.
- `transient` fields and `transient` stores are reset.
- `const` fields are kept unless their default changed.
- Derives are recomputed.
- Running tasks are cancelled through the generation fence (F.2).
- The migration is one revision with `cause.kind: "reload"`. The revision counter continues.

## Part B — Expressions

A small pure expression grammar, allowed in every value position (arguments, `apply`, `set`, props, style values, defaults, derives), in workflows as well as UI:

```text
expr    = or
or      = and ("||" and)*
and     = cmp ("&&" cmp)*
cmp     = add (("==" | "!=" | "<=" | ">=" | "<" | ">") add)?
add     = mul (("+" | "-") mul)*
mul     = unary (("*" | "/" | "%") unary)*
unary   = ("!" | "-")? primary
primary = value | variable | "(" expr ")" | "if" expr "then" expr "else" expr
```

1. Types are checked statically under `--type-policy strict`. `Int op Int` is `Int` (division truncates toward zero; overflow is a runtime error); mixing `Int` and `Float` promotes to `Float`; `+` on two `String`s concatenates.
2. `if … then … else` as an expression requires both branches to have the same type. The existing `if` **step**, whose branches are step targets, is unchanged; the parser distinguishes them by position (value vs pipeline step) and by branch shape.
3. Lists and strings have one built-in read-only property, `.count`.
4. Expressions cannot call executables or module ops. Calls stay pipeline steps.
5. A query, derive or view body may be a single expression, whose value is the result (`query ReadSelected on Panel -> Int { $selected }`).

This was already on the language-ergonomics roadmap; it ships first because everything else uses it.

## Part C — Views

```text
view Name ( view_param ("," view_param)* )? ("on" Store)? directive* { element+ }

view_param = "$" ident ":" type_ref ("=" expr)?        // prop
           | "on" ident ":" action_type "!"?            // event declaration (Part D)

element    = (ident ":")? Name content? props? body?
content    = "(" expr ")"                                // primitives with a `content` param only
props      = "(" ident ":" expr ("," ident ":" expr)* ")"
body       = "{" (style | event_binding | slot | element | view_if | view_for)* "}"

style      = "style" "{" (ident ":" expr | state_name style_block)* "}"
slot       = ident "(" variable ("," variable)* ")" ":" element
view_if    = "if" expr "{" element* "}" ("else" "{" element* "}")?
view_for   = "for" variable ("," variable)? "in" expr "key" expr "{" element* "}"
```

1. **Elements only.** A view body contains elements, style blocks, event bindings, slots, `if` and `for`. A `|>` step or a bare module call is a compile error in a view; a view element is a compile error outside one.
2. **Props versus children.** Props are named arguments in `()` and follow RFC-0004 Part A. Children are elements in the body. A primitive with a declared `content` param accepts it positionally: `Text("…")`.
3. **Named children** (`header: Header`) give the child a stable key. If the parent declares a slot of that name, the child fills the slot; otherwise it is an ordinary child with that key. Body order is preserved, because the view grammar does not go through object literals.
4. **Separators** are newlines or commas inside view bodies and style blocks.
5. **Style** values are expressions. Bare identifiers (`bg`, `full`, `center`) are resolved by the expected type: a theme role in a colour slot, an enum variant in an enum slot. Identifiers cannot contain `-`, so roles use `_` (`text_secondary`). Interaction states (`hover`, `pressed`, `focused`, `disabled`) are nested blocks.
6. **`on Store`** puts the store's fields in scope read-only (`$selected`), plus its registry's stores (`$Prefs.density`), per A.7. A view without `on` sees its params, tags, and any `transient state` attached to it; it is a reusable component.
7. **Conditional and repeated children.** `if` swaps subtrees. `for` requires an explicit `key`, for identity; large lists should use `VirtualList`, which is lazy.
8. **Recursion** is allowed (layers panels, trees). Each level is a view reference that the renderer evaluates lazily, bounded by data and by the call-depth limit.
9. `fragment` keeps its inline-macro contract unchanged.

## Part D — Events and delegates

Behaviour is **defunctionalized**: a reference to a named executable plus a partial argument map. It is serialized as data and never captures a closure:

```json
{ "$action": "Select", "args": { "index": 3 } }
```

### D.1 Declaration versus binding

Both use `on`, but they are different constructs:

| Form | Where | Meaning |
| --- | --- | --- |
| `on click: Action` | view parameter list | **declares** an optional event the caller may bind |
| `on click: Action!` | view parameter list | declares a **required** event (existing `!` marker) |
| `on change: Action($value: String)` | view parameter list | declares an event whose payload supplies `$value` |
| `on click: Shuffle` / `on click: Select(index: $index)` | element body | **binds** an event to an executable |
| `on click: $click` | element body | **forwards** a declared event to a child |

Inside the view, a declared event is the variable `$click`. An unbound optional event does nothing. An unbound required event is a compile error at the call site.

### D.2 Binding checks

At compile time, for each binding:

1. The target is a `mutation` or `task` on the glyph's store, on a store its registry lists, or on a `transient state` attached to the enclosing view; or it is a forwarded event parameter. Queries, views and subscriptions are rejected.
2. Arguments bound at the binding site, plus the arguments the event's payload supplies, must match the target's parameters **exactly**: no missing required arguments, no unknown ones, no argument supplied twice, and compatible types.
3. A forwarded `$click` must have a type compatible with the child's event declaration.

### D.3 Delegate types and MudBlazor

| Grapheme | Meaning | MudBlazor analogue |
| --- | --- | --- |
| `on click: Action` | event with no payload | `EventCallback` |
| `on change: Action($value: String)` | event with a payload | `EventCallback<string>` |
| named child / named slot | a subtree in a named position | `RenderFragment` |
| `row($item, $index): rows.Row(…)` | slot the container fills per item | `RenderFragment<T>` |
| `register … on Store`, tags (RFC-0004) | inherited stores, explicit scoped context | `CascadingValue` |
| `value: $query` + `on change: SetQuery` | explicit controlled value | `@bind-Value` |

Primitives declare their slots and the payloads they supply in the catalog. Illustrative manifest form:

```grapheme
primitive VirtualList($source: [T], $key: String, $estimated_row_height: Int = 24) {
  slot row($item: T, $index: Int)
  on scroll: Action($offset: Float)
}
```

Until RFC-0004 Part F generics land, the element type `T` linking `source` to `row` is a built-in rule for catalog primitives only. A `bind:` sugar that generates the setter mutation is deferred.

## Part E — Mutations and `apply`

```text
apply_store = "apply" object_value              // new: the executable's own store
apply_named = "apply" ident object_value        // new when ident is a registry store; else the existing lane write
transition  = "transition" variable "->" value object_value?   // existing; targets lifecycle fields (A.4)
```

1. `apply { … }` names fields of the executable's own store. `apply Prefs { … }` names fields of a store its registry lists. Writing a derive, a `const` field, a lifecycle field, an unknown field, or a value of the wrong type is a compile error.
2. `apply name { … }` resolves `name` as a store if the registry lists one by that name, and otherwise as a lane, as today. Store names are `PascalCase` and lanes are `snake_case` by convention; a store and lane with the same name in one scope is a compile error.
3. Unnamed fields keep their values; there is no spread.
4. All right-hand sides in one `apply` read the same revision, so `apply { a: $b, b: $a }` swaps.
5. A mutation on a store is a pure, synchronous transaction. Its `apply` and `transition` steps are combined into one revision, across every store it writes. It may call other mutations on the same store, which join the same transaction. It may not call capability ops; effects belong in tasks.
6. Mutations on plain structs keep their current semantics.

## Part F — Tasks

```text
task Name ( variable_defs )? on Store directive* { pipeline+ }
```

A task is a workflow that runs against a store and may call capabilities under normal policy.

### F.1 `apply` across async boundaries

1. **Latest committed revision.** Each `apply` or `transition` in a task is its own transaction. When it commits, its field reads see the latest committed revision, not a snapshot from when the task started. Reading and committing are one atomic step in the store.
2. **Field-level writes.** An `apply` writes only the fields it names. Two tasks writing different fields never overwrite each other.
3. **Optimistic guard.** `apply @if_unchanged(pulse) { … }` fails with `stale_state` if any listed field was written by someone else since this task last read or wrote it. The failure goes through the normal error path, so `@retry` can retry it (and the retry re-reads the latest revision).
4. Outside `apply`, a task's field reads refresh after each of its own commits and after each suspension point.

### F.2 Concurrency and fencing

`@concurrency(drop | restart | queue | parallel)`. The default is `drop`.

| Policy | Triggered while running |
| --- | --- |
| `drop` | the new trigger is ignored |
| `restart` | the running instance is cancelled and a new one starts |
| `queue` | the new trigger runs after the current one finishes |
| `parallel` | both run |

1. Each task instance has a **generation**. `restart` increments it before starting the replacement.
2. The store **rejects** any `apply` from an instance whose generation is no longer current. The instance ends with `cancelled`, not an error. An older instance can never commit after its replacement has started, even if its `time.sleep` completed concurrently with the restart.
3. Cancellation is delivered at suspension points (`time.sleep`, capability calls). Committed transactions are not rolled back.
4. `@retry`, `@timeout` and `@resilient` work unchanged on tasks.
5. Capability calls are checked against the store's registry grants (A.6) and then the host policy.

### F.3 Steps

- `loop(max: N) { pipeline }` is a step form of the existing `@loop` semantics: the body runs up to `N` times, sequentially.
- `time.sleep(ms:)` is a new op in a `time` module, policy-gated like any capability.
- A task may call mutations on its store; each call is one transaction.

## Part G — Signals

1. **Inbound:** a `subscription` on a store is an entrypoint the host (or another glyph) delivers events to. It behaves like a mutation: pure, one transaction, no capability calls. It may start a task with `call`.
2. **Outbound:** module calls such as `host.emit(channel:, value:)`, in tasks only, capability-gated and recorded as effects in the revision log.

## Part H — UI glyphs

`glyph Name on Store { glyph_clause* element }` declares a UI application:

1. The store is created from its defaults, and its registry's singleton stores are created at start-up if they do not exist yet.
2. The body's final element is the root. It is re-evaluated per revision, only where read sets intersect the diff, and emitted as View IR.
3. Events from the renderer or the app session (Part I) arrive as action invocations. The runtime commits the mutation or starts the task.

Clauses:

- `glyph_def` gains `on Store`.
- `expose Name, …` makes queries readable, mutations and tasks invocable, and views mountable in a host slot, outside the rendered tree.
- Hot reload has no clause; it follows A.9.

## Part I — View IR and the headless app session

### I.1 View IR

The compiler and runtime emit view results as versioned JSON (`"$ir": "grapheme-ui/1"`):

```json
{
  "$prim": "Column", "key": "PanelView",
  "props": { "style": { "width": "full", "fill": { "$role": "bg" } } },
  "children": [
    { "$view": "Header", "key": "PanelView/header", "args": {} },
    { "$prim": "VirtualList", "key": "PanelView/items",
      "props": { "source": { "$derive": "items" }, "key": "id", "estimated_row_height": 28 },
      "slots": { "row": { "$view": "rows.Row", "params": ["item", "index"],
        "args": { "item": { "$param": "item" }, "selected": { "$expr": "…" } },
        "events": { "click": { "$action": "Select", "args": { "index": { "$param": "index" } } } } } } }
  ]
}
```

1. **Identity.** A node's key is its parent's key plus its child name, its `for` key, or its ordinal among unnamed siblings. Renderers map keys to stable widget ids, preserving focus, scroll and local state.
2. **Laziness.** `$view` references are evaluated on demand and memoised by `(view, args, read-set revision)`. A `VirtualList` evaluates `row` only for visible items.
3. **Large data by reference.** `$derive` values are passed by handle, not inlined.
4. **Semantics.** Every interactive node carries a role and an accessible name (`label` or text content). The verifier warns when an interactive element has neither. rndr maps these onto its accessibility tree.

### I.2 App session

The SDK exposes a renderer-free session over a running UI glyph. The native renderer and an agent use the same API:

| Call | Returns |
| --- | --- |
| `snapshot()` | revision, View IR, and the **action list**: every event bound in the current tree (`{ id, key, event, target, bound_args, payload_params, role, label }`) plus every `expose`d executable |
| `invoke(action_id, payload, expect_revision?)` | the new revision and diff, or an error |
| `query(name, args)` | an exposed query's value |
| `revisions(since)` | revision log entries (with `history N`) |
| `undo()` / `redo()` | the new revision, or `undo_conflict` |

Rules:

1. An agent may invoke only actions in the **current** action list or exposed executables. It cannot click a button that is not rendered.
2. Invocations go through the same binding checks (D.2) and the same capability policy as a user's click, and are recorded with `origin: "agent"`.
3. `expect_revision` rejects the call with `stale_snapshot` if the tree has changed since the snapshot in a way that removed or rebound that action.
4. The headless replay harness for tests is this API. An MCP adapter that turns the action list into tools is a natural follow-up, outside this RFC.

## Part J — Independent fixes

1. **Partial `set` under `-> T`.** `set { selected: $index }` in a mutation `on S -> S` is rejected today ("missing required fields in literal state patches") although the runtime carries the other fields over. Unpatched fields of the input type should count as present. Store mutations do not need this, but existing struct mutations do.
2. **`Int` preservation.** `inc_field` on an `Int` field returns a `Float` (`5` → `6.0`). It should preserve the field's declared numeric type.

## Verifier matrix

| Construct | `view` | `derive` | `query` | `mutation` on store | `task` | `subscription` on store |
| --- | --- | --- | --- | --- | --- | --- |
| Read store fields, params, tags | allow | allow | allow | allow | allow | allow |
| Element syntax | **only here** | deny | deny | deny | deny | deny |
| Pipelines / `\|>` | deny | pure ops | allow | allow | allow | allow |
| Capability ops | deny | deny | per profile | deny | registry grants ∩ host policy | deny |
| `apply { … }`, `apply Store { … }` | deny | deny | deny | allow (one transaction) | allow (one per `apply`) | allow (one transaction) |
| `transition` on a lifecycle field | deny | deny | deny | allow | allow | allow |
| Write `const`, derive or lifecycle field with `apply` | deny | deny | deny | deny | deny | deny |
| Call mutation on store | deny | deny | deny | allow (same transaction) | allow | allow |
| Start a task | deny | deny | deny | deny | allow | allow |
| Event bindings | allow | — | — | — | — | — |

## Non-goals

1. Closures or first-class functions.
2. Per-frame scripting, custom painting or animation code in Grapheme.
3. CSS-style selectors or cascade. Styles are values; shared styling uses tags or theme roles.
4. Hidden two-way binding.
5. Reverting external effects on undo.
6. Any renderer dependency in Grapheme crates.

## Open questions

1. **Registries without a store.** Should a glyph-level `register Name { … }` exist for programs with no store, or is the host policy enough there?
2. **Singleton initialisation.** A `singleton state` is created at start-up from its defaults. Should the host be able to seed it (for example from saved preferences) before the first glyph runs?
3. **Transient wipes beyond unmount.** Should a `VirtualList` row scrolling out of view count as an unmount for stores attached to its row view? This RFC says yes.
4. **Trailing `@pure`.** Keep it as an accepted alias for `pure query`, or reject it once this lands?

Resolved since r2:

- Store writes use `apply` (E).
- `$state.` is dropped where the store is known from scope (A.7).
- The component catalog is new, owned by GraphemeUI, not an existing Grapheme catalog.
- Stores are available to non-UI glyphs from the start (A.1.4).
- Tasks are not undoable in this RFC (A.8).
- Store and field semantics are keywords, not directives (principle 9).

Resolved since r1: state lane (a store, A.1.4); commas (optional inside view, style and state blocks only); `view` versus `fragment` (a new kind).

## Rollout

1. **Phase 0:** Part J fixes (independently useful).
2. **Phase 1:** expressions (Part B), including expression bodies. Useful to workflows on their own.
3. **Phase 2:** state stores with field kinds, lifecycle fields, `apply`, store mutations, revisions and diffs (Parts A.1–A.4, A.7, E); a headless session with `invoke`, `query` and diffs. Usable by non-UI glyphs.
4. **Phase 3:** view grammar, events and delegates, View IR, `grapheme compile --emit view-ir` for golden tests (Parts C, D, I.1); `snapshot()` with the action list.
5. **Phase 4:** store lifetimes, registries and capability grants, tasks, concurrency, fencing, `@if_unchanged`, `time.sleep`, subscriptions on stores (Parts A.5, A.6, F, G).
6. **Phase 5:** UI glyph lifecycle, `expose`, `history`, `undo`, hot reload (Parts A.8, A.9, H).
7. **Phase 6:** GraphemeUI renderer on rndr consuming View IR.
8. **Phase 7:** LSP completion and hover for primitives, props and event signatures; MCP adapter.

## Acceptance

1. The worked example compiles under `--type-policy strict`, and its View IR matches a golden file.
2. A headless replay produces the expected revisions and diffs:
   - `click Select(index: 3)` → one revision, `selected: [0, 3]`, `presses: [0, 1]`.
   - `click Pulse` twice 200 ms apart → no revision from the first generation after the second starts, and `pulse` reaches 0 ten steps after the second click.
   - `subscription Pinged(count: 2)` → `pings: [·, 2]`.
   - `click Pulse` → `mode: [idle, pulsing]`, then `mode: [pulsing, idle]` after the loop; a stale `Pulse` generation never moves `mode`.
   - `undo()` after `Select` restores `selected` and `presses`; `pulse` changes are absent from the log; a `Ping`'s `host.emit` stays in the log after undo; a task's revisions are not undoable.
   - `ToggleDensity` from two glyphs whose registries both list `Prefs` changes one shared instance.
   - Unmounting a `PanelButton` drops its `ButtonHint`; remounting starts from `shown: false`.
3. `snapshot()` lists exactly the actions bound in the rendered tree plus the exposed ones; invoking an unrendered, unexposed action is rejected.
4. Verifier errors for:
   - element syntax outside a view, or a pipeline inside one;
   - a capability call in a view, derive or store mutation;
   - an `apply` to a derive, `const`, lifecycle or unknown field;
   - a `transition` along an undeclared edge or out of a `terminal` member;
   - a plain `state` listed in two registries (`ambiguous_store_owner`);
   - a task calling an op its registry does not grant;
   - a parameter that shadows a field in scope;
   - an event bound to a query;
   - missing, unknown or duplicate action arguments;
   - an unbound required event;
   - an unresolved symbol under strict policy;
   - a derive cycle.
5. Existing examples compile and run unchanged.
