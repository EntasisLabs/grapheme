# RFC-0007: GraphemeUI — State Stores, Views and Tasks v1

Status: Accepted — Implementation in progress (revision 4; rollout steps 0–2, Part H headless mount, Part F tasks headless slice, Part C.7 view `if` / `for`)
Authors: language + runtime
Created: 2026-11-07
Revised: 2026-11-09
Target release window: after the RFC-0004 0.7.0 parameter and tag work

## Summary

Add the language pieces Grapheme needs to describe interactive applications, so that a separate GraphemeUI layer can render them natively (first target: [rndr](https://github.com/EntasisLabs/rndr)) and agents can operate them headlessly. Grapheme stays the workflow language; GraphemeUI is to Grapheme what SwiftUI is to Swift.

A program has three surfaces, each with its own construct and its own rules:

| Surface | Construct | Contract |
| --- | --- | --- |
| **State** | `state Name { … }`, `register Name on Store { … }` | Typed fields with lifetimes and kinds, lifecycle fields, registries with read/write/use permissions, single-store transactions, revisions and diffs |
| **Views** | `view Name { … }` | Pure declarative projections of state, written as element blocks |
| **Behaviour** | `mutation`, `task`, `subscription` | Pure transactions, governed effects, inbound signals |

This RFC adds:

1. **State stores** with keyword-level semantics:
   - store lifetimes (`singleton state`, `state`, `transient state … on Component`);
   - field kinds (`const`, `transient`, `derive`) and lifecycle fields governed by the existing `state_machine`;
   - registries (`register … on Store`) that grant `read` and `write` access to other stores and `use` of capabilities;
   - single-store transactions (`apply { … }`), revisions, runtime and history diffs, a revision log (`history N`) and opt-in undo (`undo by action`).
2. A small **pure expression grammar** usable in any value position.
3. A **`view` kind** whose body is an element tree (`Name(props) { style, children, on event }`), never a pipeline.
4. **Typed events and delegates**: event declarations and event bindings are distinct and checked; behaviour is referenced, never captured.
5. A **`task` kind** for effects, with concurrency policies, generation fencing, optimistic guards and effect receipts.
6. **UI glyphs** with an `expose` clause; hot-reload behaviour follows from field and store kinds.
7. A versioned **View IR** with distinct store, node and action identities, and a **headless app session** API, so the same source drives a native renderer and an agent.
8. Three independent verifier/runtime fixes.

## Revision history

- **r1 (2026-11-07).** UI expressed through existing constructs: `struct` + init query for state, `fragment`-like `view` returning struct literals with `children: [ … ]`, `@derive` queries passed through tags, `@task` iterators delegating writes to helper mutations.
- **r2.** After review, a middle ground between r1 and the original friendly draft:
  - State becomes a declared store instead of a bare `$state` blob. Init queries, derive tags and helper mutations disappear.
  - Views get their own element grammar, so UI is visibly different from executable code, and named children keep their order.
  - Expressions (`+`, `==`, `if … then … else`) are added instead of avoided.
  - `task` becomes a kind; `apply` inside a task commits directly.
  - Event declarations vs bindings, `apply` across async boundaries, and what the revision log promises are specified (Parts D, F, A.8).
  - Headless agent interface added (Part I).
- **r3.** Keywords replace `@` directives for what a store or property *is*:
  - Field kinds are leading keywords: `const`, `transient`, `derive`. `@transient` is gone.
  - Store lifetimes are declared on the store: `singleton state`, `state`, `transient state … on Component`. `transient` means "wipeable" at both levels.
  - Lifecycle fields reuse `state_machine` unchanged: a field whose enum has a machine is written only by `transition`.
  - `register Name on Store { … }` declares a store's dependencies (other stores) and capability grants; everything `on` the store inherits them.
  - `history N` and `undo by action` are store clauses; `expose` is a glyph clause; `@hot_reload` is removed.
  - `$state.` is dropped where the store is known from scope: `$selected`, and `$Prefs.density` for inherited stores.
  - Catalog ops are marked `pure query …`. Tasks are not undoable (no `@undoable` yet).
- **r4 (this revision, accepted).** No new syntax beyond registry permissions; the remaining semantic contracts are pinned down so compiler and runtime cannot diverge:
  - Registry entries are `read Store`, `write Store` and `use module[.op]`. One transaction writes one store (A.6, E).
  - Cancelling a task ends that run, not the field's lifecycle. Self-transitions are legal only as declared edges; a lifecycle field may be `transient` (A.4, F.2).
  - `pure` means effect-free **and** deterministic in its declared inputs; derives may only use such ops (A.2).
  - Store id, node key and action id are separate identities. Action ids carry a binding epoch, so a stale action from a recycled row is rejected (I.3).
  - Effect receipts with an `indeterminate` outcome; no exactly-once promise (F.4).
  - Runtime diffs (complete) are separated from history diffs (filtered); undo is bounded by retained history and validated like any transition (A.3, A.8).
  - The rollout is re-ordered around a vertical slice: a counter rendered by rndr and driven headlessly, before concurrency and ownership work. An adversarial conformance suite is part of acceptance.

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
// r3, r4
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

**Integration model A (current):** [`grapheme-ui`](https://github.com/EntasisLabs/grapheme-ui) is a separate repository and the only bridge. Host applications that want Grapheme UI depend on `grapheme-ui` directly (full-app runner or an rndr component that mounts a `GraphemeSurface`). rndr does not depend on Grapheme. A later optional rndr facade re-export is model B and is deferred.

## Design principles

1. **Two shapes, never mixed.** UI is `Name(props) { … }` and only appears in `view`. Code is `op(args) |> op(args)` and never appears in `view`.
2. **State lives in a store.** Only `apply` and `transition` write it, one store per transaction; everything else reads a revisioned snapshot.
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
| Counter update | `inc_field(field: "presses")` | works; `Int` stays `Int` (fixed in Part J.2) |
| Inbound signal with payload | `subscription Pinged($count: Int) on Panel { set { pings: $count } }` | works |
| Repetition | `iterator … @loop(max: 3, merge: "replace")` | works |
| String interpolation | `"{$state.presses} presses"` | works |
| Scoped context | `tag` + `using` (RFC-0004) | works |
| Lane write | `apply state { … }` | writes the nested `state` lane, not root fields |
| Enum lifecycle | `state_machine L on Status { transition a -> b; terminal b }` | works; bound by enum type, and consecutive literal writes in a pipeline are checked against its edges |
| Self-edge | `transition pulsing -> pulsing` in a `state_machine` | parses and is accepted; today a literal write of the current value is skipped as a no-op before any edge check |

What does **not** fit today, and why r2 adds syntax instead:

- Before Part B, `value` admitted only literals, variables, symbols, lists and objects; `1 + 2` and `$a == $b` did not parse in values. Part B's grammar and runtime now ship (see Part B status).
- A bare expression (`$state.selected`) is not a valid query body.
- Objects become `serde_json` maps without `preserve_order`, so named children cannot be carried by object literals. Views need their own grammar.
- `fragment` cannot take arguments, cannot be called, rejects directives and rejects cycles. Views need all four.
- Iterators cannot write state under the strict profile, which forced r1's helper mutations.
- `apply { a: 1 }` already parses, as a struct-init step of a type named `apply`, and merges into the root. This RFC reserves `apply` with no lane as the store transaction (Part E). No existing example uses it.
- `state { current }` is a selection-set selector today. The top-level `state Name { … }` definition is a different context and does not conflict.
- `transition $state.status -> approved` writes the root field `status` of the value being transformed (fixed in Part J.3; it previously wrote a literal `state.status` key). Lifecycle fields (A.4) need it to target a store field, with its edge check enforced at commit time as well as at compile time.
- Program-scoped `using mutable $h: Tag(…)` parses, but handles are not resolved at runtime yet, so they are not a basis for stores.

## Worked example

```grapheme
import rows from "./rows"
import time from "grapheme/time"

// ── Lifecycles (existing syntax, unchanged) ─────────────────────
enum PanelMode { idle, pulsing }
enum Density { comfortable, compact }

// `pulsing -> pulsing` is declared so a restarted Pulse can re-enter (A.4).
state_machine PulseLifecycle on PanelMode {
  transition idle -> pulsing
  transition pulsing -> pulsing
  transition pulsing -> idle
}

// ── State ────────────────────────────────────────────────────────
// Keywords say what a store or field is (Part A):
//   singleton state / state / transient state   store lifetimes
//   const / transient / derive                  field kinds
// `mode` is a lifecycle field: PulseLifecycle governs PanelMode. It is also
// transient, so a reload resets it to idle instead of leaving it mid-pulse.
state Panel {
  history 100
  undo by action

  const page_size: Int = 1000
  selected: Int = 0
  presses: Int = 0
  transient pulse: Int = 0
  transient mode: PanelMode = idle
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
// Everything `on Panel` inherits these permissions (A.6).
register PanelRegistry on Panel {
  write Prefs
  use time
  use host.emit
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

      density: PanelButton(label: "Density") { on click: ToggleDensity }
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

// One transaction writes one store, so this runs on Prefs itself. Panel's views
// may bind it because PanelRegistry grants `write Prefs`.
mutation ToggleDensity on Prefs {
  apply { density: if $density == compact then comfortable else compact }
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

// On restart the old run is fenced off while `mode` is still pulsing; the new
// run re-enters through the declared `pulsing -> pulsing` edge.
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

registry   = "register" Name "on" Store "{" grant+ "}"
grant      = "read" Store | "write" Store | "use" module ("." op)?
```

Separators between store items and registry entries are newlines or commas. No store or field semantics use `@` directives. `singleton`, `transient`, `const`, `derive`, `history`, `undo`, `register`, `read`, `write`, `use` and `expose` are contextual keywords: reserved only at the start of a top-level definition, store item, registry entry or glyph clause, so existing identifiers and op names keep working. Only `const` is already a keyword (`using const`).

**Implementation status:** A.1–A.4, A.7 and the store half of Part E are implemented for plain `state` stores, with a headless store session (`invoke`, `query`, `snapshot`, `revisions`) in the runtime, SDK and `grapheme session`. Not yet: lifetimes and `on View` (A.5), registries (A.6), `history`/`undo` (A.8), hot reload (A.9), `by Machine` (A.4.4), host-seeded `const` fields outside the runtime API, and store subscriptions. Store mutations cannot call module ops yet.

### A.1 Fields

| Keyword | Written by | Runtime diff | History and undo | Hot reload |
| --- | --- | --- | --- | --- |
| *(none)* | `apply` | yes | logged; undoable | kept |
| `const` | nothing after creation; `apply` is a compile error | — | — | kept, unless its default changed |
| `transient` | `apply` | yes | not logged; not undone | reset to default |
| `derive` | nothing; computed (A.2) | no (follows from fields) | not logged | recomputed |
| lifecycle (A.4) | `transition` only | yes | logged; undoable along declared edges | kept |
| `transient` lifecycle | `transition` only | yes | not logged; not undone | reset to default |

1. Every field has a type and a default expression. Defaults may not read other fields. The store's first revision (revision 0) is built from the defaults, plus host-supplied initial values for `const` fields. No init query is needed.
2. Field writes go through `apply` (Part E) or `transition` (A.4). Everywhere else, fields are read-only values from one revision.
3. A store is a type: `on Panel` means "runs against the `Panel` store". Mutations and subscriptions on a store have an implied output type (the store); `-> T` is not written.
4. A store is a runtime object, separate from the pipeline `AgentState`. Stores are available to any glyph, UI or not.
5. Field kinds combine only as listed: `transient` may precede a plain or lifecycle field; `const` and `derive` stand alone.

### A.2 Derives

1. A derive is a read-only field computed from fields, other derives and pure module ops. It is read like a field: `$items`.
2. **`pure` is one contract with two parts:** no observable effects, **and** deterministic: the same declared inputs always give the same result. An op that reads a clock, a random source, the environment, the network or any state outside its arguments is not `pure`, even if it writes nothing.
3. The verifier rejects capability calls and writes in a derive. Module ops it calls must be declared `pure query …` in their signature. A trailing `@pure` directive is accepted as an alias. A module declaring an op `pure` is asserting both parts; the SDK's conformance harness checks determinism by repeated evaluation.
4. The compiler records each derive's read set (`items` reads `page_size` and `shuffles`). The read set is the derive's complete input: pure ops cannot read stores, so module calls add no hidden dependencies. Because of 2, the runtime may cache a derive while its read set is unchanged, recompute it lazily on first read after a change, and rebuild it from fields alone; derives are never logged or replayed.
5. Derive cycles are a compile error, including cycles that pass through derives declared in imported modules.
6. Data from outside the program reaches state only through a task, a subscription or a host-supplied `const` field, never through a derive.

### A.3 Transactions and revisions

1. **One store per transaction.** A transaction writes exactly one store instance. Atomic commits across stores are not part of v1; a task that writes two stores commits two transactions.
2. Each committed transaction produces exactly one revision of its store: the store id, a monotonically increasing integer per store instance, and a **runtime diff** `{ field: [old, new] }` covering every field it changed, `transient` fields included. Derives are not part of diffs. Runtime diffs drive invalidation, subscriptions and the result of `invoke` (I.2), so a revision that changed the UI never reports an empty diff.
3. The **history diff** of a revision is its runtime diff filtered by history policy (A.8): `transient` fields are removed. Only history diffs are retained in the log and used by undo.
4. A mutation is one transaction, however many `apply` and `transition` steps it contains. A task commits one transaction per step (Part F).
5. The store serializes commits; there is one writer at a time. Revisions are applied in commit order.
6. Views and derives are invalidated per field: a revision re-runs only the views whose compile-time read set intersects its diff. The renderer coalesces revisions per frame.

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
6. **No implicit self-transitions.** `transition $mode -> pulsing` while `mode` is `pulsing` is legal only if the machine declares `transition pulsing -> pulsing`; otherwise it fails with `invalid_transition`. The existing no-op skip for plain literal writes to struct fields is unchanged.
7. **Cancellation ends a run, not a lifecycle.** A cancelled or failed task leaves its lifecycle fields at their last committed value. Recovery is explicit: a declared re-entry edge, a declared edge back to a resting member, or a `transient` lifecycle field, which hot reload resets to its default.
8. A lifecycle write by undo (A.8) is checked exactly like a `transition`.

### A.5 Store lifetimes

A store's lifetime is declared on the store, with the same words a field uses:

| Declaration | Instances | Shared? | On reload or unmount |
| --- | --- | --- | --- |
| `singleton state Prefs { … }` | one per registration, created at start-up | every registry that lists it gets the same instance | kept on hot reload |
| `state Panel { … }` | one per owner: the glyph that is `on` it, or the one registry that lists it | not shareable | kept on hot reload |
| `transient state Hint on PanelButton { … }` | one per instance of the owning view | each owner instance has its own | wiped when its owner unmounts, and on hot reload |

1. `transient` means **wipeable** at both levels. A transient field is a wipeable value inside a durable store; a transient store is wipeable as a whole.
2. A transient store's `on` names a view, not a store. Inside that view it is read qualified (`$ButtonHint.shown`). A mutation `on ButtonHint` bound to an event inside that view runs against the instance owned by the node that raised the event (identified by its View IR key).
3. Transient stores produce runtime diffs but are never logged or undone.
4. A row realized by a virtualized container (`VirtualList`) unmounts when it leaves the visible window, so stores attached to its row view are wiped on scroll-out. This matches rndr, whose virtual-list row subtrees are destroyed before a recycled row is rebound, with only the logical focus key kept by item id. Per-item state that must survive scrolling belongs in a durable store keyed by item id, not in a transient store.

### A.6 Registries

A registry declares what a store brings with it. Access to a store and authority to change it are separate grants:

```grapheme
register PanelRegistry on Panel {
  read Prefs      // observe another store
  write Session   // transact against it; implies read
  use time        // invoke every op in a module
  use host.emit   // invoke one op
}
```

1. **Inheritance.** Everything `on Panel` (its glyph, views, mutations, tasks and subscriptions) inherits the registry. There is no per-definition `uses`.
2. **`read`** puts the store in scope qualified: `$Prefs.density`. Views, derives, queries, mutations, tasks and subscriptions may read it.
3. **`write`** implies `read`, and is required for every write to that store from `on Panel` code:
   - a task on Panel may `apply Session { … }` or `transition $Session.x -> …`, each its own transaction (A.3.1);
   - a view on Panel may bind an event to a mutation or task `on Session`.

   A mutation or subscription writes only its own store, so `apply Session { … }` inside a mutation `on Panel` is a compile error (`cross_store_write`). Write the mutation `on Session` and bind it.
4. **`use`** grants capabilities as a declarative layer over the host's capability policy. Grants can only narrow it: a call must be allowed by both. A task on a store with a registry may only call ops its registry grants; an ungranted call is a compile error. A store with no registry falls back to the host policy alone.
5. **Same store in several registries** (by `read` or `write`). The outcome follows the store's lifetime:
   - `singleton` — one shared instance;
   - `transient` — one instance per registry;
   - plain `state` — a compile error (`ambiguous_store_owner`). The compiler cannot decide which owner the single instance belongs to.
6. A registry may not list its own store, a store may appear once per registry, and registry dependency cycles are a compile error.
7. Any read or write of a store not granted is a compile error (`ungranted_store`).

### A.7 Names in scope

1. Inside a definition `on Panel`, Panel's fields and derives are in scope as `$field`. There is no `$state.` prefix.
2. Stores granted by the registry (A.6) and attached transient stores (A.5) are read qualified by store name: `$Prefs.density`. Inside a definition `on Prefs`, Prefs' own fields are bare again (`$density`).
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
- `diff` is the **history diff** (A.3.3): `transient` fields never appear in it, although they do appear in the runtime diff that `invoke` returns.
- A task's capability calls appear as `effects` entries holding effect receipts (F.4) on the revision they follow, or as entries with no diff. They are marked irreversible.
- The log is bounded to `N` entries. Without `history`, the runtime still keeps the current revision number and the last runtime diff, which invalidation needs.

`undo by action | transaction` is a separate opt-in layered on the log:

1. An **undo group** is either every revision caused by one action invocation (`by action`) or each revision (`by transaction`).
2. Only groups caused by mutations are undoable. Tasks and subscriptions are not undoable in this RFC.
3. Undo restores the group's non-transient fields to their values before the group, as a **new** revision with `cause.kind: "undo"`. It never rewinds the log.
4. **Undo is bounded by retained history.** A group whose entries have been evicted from the `N`-entry log, in whole or in part, is no longer undoable; `undo()` reports `undo_unavailable` and the store is unchanged.
5. **Undo is validated like any write.** A lifecycle field is restored by a transition from its current value to its earlier value, checked against the machine's declared edges and terminals exactly as A.4 checks `transition`. If it is not a declared edge, undo is rejected with `undo_conflict` and nothing is committed.
6. Undo never reverts effects.
7. If a field the group wrote has since been written by a non-undoable cause, undo is rejected with `undo_conflict` and the store is unchanged.
8. Redo is validated the same way, and re-applies the group's diff as a new revision. Any new undoable group clears the redo stack.

### A.9 Hot reload

Reload behaviour follows from the keywords; there is no `@hot_reload` directive.

- Plain and lifecycle fields are kept; `transient` lifecycle fields are reset. Added fields take their default, removed fields are dropped, and fields whose type changed are reset with a diagnostic.
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

**Implementation status:** the grammar, precedence and runtime semantics in this part (including `Int` truncating division and overflow as a runtime error) are implemented in Stage A and Stage B, as are static typing (1–2; a lint under `warn`) and `.count` (3). Expression bodies (5) are implemented for queries, including store queries, and are checked against the declared `-> T`; derive and view expression bodies arrive with their declarations (derive fields already take a single expression).

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

**Implementation status (rollout step 2 + C.7 + TextField):** the element grammar (named children, positional content, props, view references, rule 1's rejection of steps and module calls), rules 2–4, 6 and 7 (`if` / `for`) for `on Store` views, with a built-in catalog of `Column`, `Row`, `Text`, `Button` and `TextField`. Not yet: view params, `style`, slots, `VirtualList`, `transient state` on views, registry stores in scope, and recursion beyond eager inlining.

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

1. The target is a `mutation` or `task` on the view's store, on a store its registry grants `write` to, or on a `transient state` attached to the enclosing view; or it is a forwarded event parameter. Queries, views and subscriptions are rejected, and so are targets on a store granted only `read`.
2. Arguments bound at the binding site, plus the arguments the event's payload supplies, must match the target's parameters **exactly**: no missing required arguments, no unknown ones, no argument supplied twice, and compatible types.
3. A forwarded `$click` must have a type compatible with the child's event declaration.

**Implementation status (rollout step 2 + Part F + TextField slice):** element-body bindings `on click: Target(args)` / `on change: Target` with check 1 for mutations or tasks on the view's own store and check 2 for bound arguments plus catalog event payloads (`TextField` `change` → `value` → `payload_params`). `invoke_action(id, payload)` merges the payload over those params. Not yet: event declarations and forwarding (D.1 rows 1–3 and 5), registry `write` stores.

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
apply_named = "apply" ident object_value        // new when ident is a store granted `write`; else the existing lane write
transition  = "transition" variable "->" value object_value?   // existing; targets lifecycle fields (A.4)
```

1. `apply { … }` names fields of the executable's own store. `apply Prefs { … }`, allowed only in tasks, names fields of a store the registry grants `write` to (A.6.3). Writing a derive, a `const` field, a lifecycle field, an unknown field, or a value of the wrong type is a compile error.
2. `apply name { … }` resolves `name` as a store if the registry grants one by that name, and otherwise as a lane, as today. Store names are `PascalCase` and lanes are `snake_case` by convention; a store and lane with the same name in one scope is a compile error.
3. Unnamed fields keep their values; there is no spread.
4. All right-hand sides in one `apply` read the same revision, so `apply { a: $b, b: $a }` swaps.
5. A mutation on a store is a pure, synchronous transaction against **that store only**. Its `apply` and `transition` steps are combined into one revision. It may call other mutations on the same store, which join the same transaction; calling a mutation on another store is a compile error (`cross_store_write`). It may not call capability ops; effects belong in tasks.
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
4. **Cancellation ends a run, not the store's lifecycle.** Lifecycle fields keep their last committed value (A.4.7). The replacement run starts from the latest committed revision, so a restartable task whose first step is a `transition` must have a declared edge from every member a cancelled run can leave behind, including a self-edge where needed.
5. A capability call in flight when its run is cancelled is not retried; its receipt ends `indeterminate` if it was already dispatched (F.4).
6. `@retry`, `@timeout` and `@resilient` work unchanged on tasks.
7. Capability calls are checked against the store's registry grants (A.6) and then the host policy.

### F.3 Steps

- `loop(max: N) { pipeline }` is a step form of the existing `@loop` semantics: the body runs up to `N` times, sequentially.
- `time.sleep(ms:)` is a new op in a `time` module, policy-gated like any capability.
- A task may call mutations on its store, or on a store its registry grants `write` to; each call is one transaction.

### F.4 Effect receipts

Every capability call a task makes produces an **effect receipt**:

```json
{ "effect": 7, "op": "host.emit", "args": { "channel": "pinged", "value": 3 },
  "cause": { "action": "a:PanelView/header/controls/ping:click#2", "task": "Ping", "generation": 4 },
  "store": "Panel", "revision": 42,
  "status": "acknowledged", "error": null }
```

| Status | Meaning |
| --- | --- |
| `requested` | the task reached the call; policy and grants passed |
| `dispatched` | handed to the capability provider |
| `acknowledged` | the provider reported success |
| `failed` | the provider reported failure, or the call was refused before dispatch |
| `indeterminate` | dispatched, but the outcome is unknown (cancellation, timeout, provider loss) |

1. Status moves forward only: `requested → dispatched → acknowledged | failed | indeterminate`, or `requested → failed`.
2. `revision` is the store revision the task had last committed or read when it made the call.
3. Receipts are the effects entries of the revision log (A.8), and appear in the task's result in the app session.
4. A call that ends `indeterminate` is surfaced as an error to the task, which `@retry` may handle. Retrying is the program's choice.
5. **No exactly-once promise.** Delivery is at most once per attempt; whether a retry duplicates an external effect depends on the provider. Idempotency keys, durable dispatch and replay of effects are deferred to a later RFC.

**Implementation status (headless slice):** `task Name on Store { … }` parses and lowers as `MirFunctionKind::Task`. Tasks may call capabilities; each `apply`/`transition` commits immediately (earlier commits kept on later failure). Store-bound runs keep `$state` on a separate lane from `$current`. `StoreCommit.effects` carries acknowledged/failed receipts for non-apply calls. View bindings and `expose` accept tasks. Not yet: `@concurrency`, generation fencing, `@if_unchanged`, `time.sleep`, registries/`use` grants, or subscription-started tasks. Example: `examples/feed-headless.gr`.

## Part G — Signals

1. **Inbound:** a `subscription` on a store is an entrypoint the host (or another glyph) delivers events to. It behaves like a mutation: pure, one transaction, no capability calls. It may start a task with `call`.
2. **Outbound:** module calls such as `host.emit(channel:, value:)`, in tasks only, capability-gated and recorded as effect receipts (F.4).

## Part H — UI glyphs

`glyph Name on Store { glyph_clause* element }` declares a UI application:

1. The store is created from its defaults, and its registry's singleton stores are created at start-up if they do not exist yet.
2. The body's final element is the root. It is re-evaluated per revision, only where read sets intersect the diff, and emitted as View IR.
3. Events from the renderer or the app session (Part I) arrive as action invocations. The runtime commits the mutation or starts the task.

Clauses:

- `glyph_def` gains `on Store`.
- `expose Name, …` makes queries readable, mutations and tasks invocable, and views mountable in a host slot, outside the rendered tree.
- Hot reload has no clause; it follows A.9.

**Implementation status:** `glyph Name on Store { expose …; Element }` parses and lowers to `MirUiGlyph` (a bare view reference reuses that view; any other element becomes a synthetic view named after the glyph). `expose` accepts mutations, tasks and queries on the glyph's store, and views on that store. `StoreSession::mount` / `app_snapshot` and `grapheme session --mount` return the app snapshot; exposed mutations and tasks appear as `expose:<Name>` actions. Workflow glyphs (`glyph Name { pipeline }`) are unchanged. Not yet: registry singletons at start-up, host-slot mounting of exposed views, and hot reload.

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

1. **Identity.** A node's key is its parent's key plus its child name, its `for` key, its virtualized item key, or its ordinal among unnamed siblings. Renderers map keys to stable widget ids, preserving focus, scroll and local state. See I.3.
2. **Laziness.** `$view` references are evaluated on demand and memoised by `(view, args, read-set revision)`. A `VirtualList` evaluates `row` only for visible items.
3. **Large data by reference.** `$derive` values are passed by handle, not inlined.
4. **Semantics.** Every interactive node carries a role and an accessible name (`label` or text content). The verifier warns when an interactive element has neither. rndr maps these onto its accessibility tree.

### I.2 App session

The SDK exposes a renderer-free session over a running UI glyph. The native renderer and an agent use the same API:

| Call | Returns |
| --- | --- |
| `snapshot()` | revision, View IR, and the **action list**: every event bound in the current tree (`{ id, key, event, target, bound_args, payload_params, role, label }`) plus every `expose`d executable |
| `invoke(action_id, payload, expect_revision?)` | the new revision and its runtime diff, plus effect receipts for a task, or an error |
| `query(name, args)` | an exposed query's value |
| `revisions(since)` | revision log entries (with `history N`) |
| `undo()` / `redo()` | the new revision, or `undo_conflict` |

Rules:

1. An agent may invoke only actions in the **current** action list or exposed executables. It cannot click a button that is not rendered.
2. Invocations go through the same binding checks (D.2) and the same capability policy as a user's click, and are recorded with `origin: "agent"`.
3. **Staleness is checked per action.** `invoke` rejects an action id whose binding epoch (I.3) is no longer current with `stale_action`, even if the global revision moved for unrelated reasons. `expect_revision` is an optional, stricter check: the call is rejected with `stale_snapshot` unless the target store is still at exactly that revision.
4. The headless replay harness for tests is this API. An MCP adapter that turns the action list into tools is a natural follow-up, outside this RFC.

### I.3 Identities

Three identities exist and are never interchangeable:

| Identity | Names | Assigned by | Stable across |
| --- | --- | --- | --- |
| **Store id** | which store instance owns the data: `Panel`, `Prefs`, or `ButtonHint@<node key>` for an attached transient store | runtime, at creation | revisions; a transient store id dies with its owner node |
| **Node key** | which UI instance exists (I.1) | compiler + runtime, from the tree | revisions in which the node stays mounted |
| **Action id** | which legal operation is currently bound: node key, event name and **binding epoch** | runtime, per snapshot | revisions that leave that node's binding unchanged |

1. **Virtualized rows are keyed by item, not slot.** A `VirtualList` row's node key is the list's key plus the row's item key (`key: "id"`). The renderer's recycled row slot is never part of any identity.
2. **Binding epoch.** Each node event binding has an epoch, incremented whenever the node is remounted or its target or bound arguments change. An action id is valid only while its epoch is current. A recycled rndr row that is rebound to item B produces a new node key; the old action id for item A is rejected with `stale_action` and can never execute against B.
3. A store id never appears in an action id. The store a binding targets is resolved from the binding's target, at invocation, through the registry (A.6).
4. Action ids are opaque to clients; they may only be obtained from `snapshot()` or an `expose` list.

**Implementation status:** I.1 node keys, roles and the `view-accessibility` lint (rule 4), `grapheme compile --emit view-ir`; I.2 as a store session (`render(view)` with the action list, `invoke_action(id, payload)` with `stale_action` and payload merge for `payload_params`) and as an app session over a mounted UI glyph (`mount` / `app_snapshot`, with `expose:<Name>` actions for exposed mutations/tasks); I.3 binding epochs. Not yet: lazy memoised `$view` evaluation, `$derive` handles, `expect_revision`, `origin: "agent"`, `revisions(since)`, `undo`/`redo`, and virtualized-row keys.

## Part J — Independent fixes

All three are implemented (rollout step 0).

1. **Partial `set` under `-> T`.** Literal patches (`set`, `core.set_fields`, `core.merge`) in a query or mutation `on S -> T` may omit required fields of `T` that the input type `S` declares; the runtime carries them over. Required fields the input does not declare are still reported as missing.
2. **`Int` preservation.** `inc_field` and `dec_field` keep an integer value an integer (`5` → `6`). Float values stay floats, and a missing field still starts from `0.0`.
3. **`transition $state.<field>`.** The step writes the root field (`status`), not a literal `state.status` key, so it type-checks against the declared output type. `examples/mutation-state-machine-apply.gr` (tutorial 02) compiles under both policies and ends with `status: approved`.

## Verifier matrix

| Construct | `view` | `derive` | `query` | `mutation` on store | `task` | `subscription` on store |
| --- | --- | --- | --- | --- | --- | --- |
| Read store fields, params, tags | allow | allow | allow | allow | allow | allow |
| Element syntax | **only here** | deny | deny | deny | deny | deny |
| Pipelines / `\|>` | deny | pure ops | allow | allow | allow | allow |
| Capability ops | deny | deny | per profile | deny | registry grants ∩ host policy | deny |
| `apply { … }` (own store) | deny | deny | deny | allow (one transaction) | allow (one per `apply`) | allow (one transaction) |
| `apply Store { … }` (granted `write`) | deny | deny | deny | deny (`cross_store_write`) | allow (one per `apply`) | deny |
| `transition` on a lifecycle field | deny | deny | deny | allow (own store) | allow (own or granted `write`) | allow (own store) |
| Read a store granted `read` or `write` | allow | allow | allow | allow | allow | allow |
| Call a non-deterministic op | deny | deny | per profile | deny | allow (capability) | deny |
| Write `const`, derive or lifecycle field with `apply` | deny | deny | deny | deny | deny | deny |
| Call mutation on store | deny | deny | deny | allow (same store, same transaction) | allow (own or granted `write`) | allow (same store) |
| Start a task | deny | deny | deny | deny | allow | allow |
| Event bindings | allow | — | — | — | — | — |

## Non-goals

1. Closures or first-class functions.
2. Per-frame scripting, custom painting or animation code in Grapheme.
3. CSS-style selectors or cascade. Styles are values; shared styling uses tags or theme roles.
4. Hidden two-way binding.
5. Reverting external effects on undo.
6. Any renderer dependency in Grapheme crates.

Deferred to later RFCs:

1. Atomic transactions across stores.
2. Durable execution: exactly-once effects, idempotency keys, durable dispatch and effect replay.
3. Automatic data binding (`bind:` sugar).
4. Custom drawing and animation scripting.
5. Component-local state that survives a virtualized unmount.
6. Renderers other than rndr.
7. Registries without a store, and host seeding of singletons.

## Open questions

None. The language design is frozen at r4; changes from here go through implementation feedback or a follow-up RFC.

Resolved in r4:

- Registry grants are `read`, `write` and `use`; one transaction writes one store (A.6, A.3.1, E).
- Restart and lifecycle: cancellation ends a run, not a lifecycle; self-transitions only as declared edges; lifecycle fields may be `transient` (A.4, F.2).
- `pure` means effect-free and deterministic (A.2).
- Store, node and action identities, with per-action binding epochs (I.3).
- Effect receipts, including `indeterminate`, with no exactly-once promise (F.4).
- Runtime diffs versus history diffs; undo bounded by history and validated like a transition (A.3, A.8).
- Registries without a store, and host seeding of singletons, are deferred (Non-goals).

Resolved since r2:

- Store writes use `apply` (E).
- `$state.` is dropped where the store is known from scope (A.7).
- The component catalog is new, owned by GraphemeUI, not an existing Grapheme catalog.
- Stores are available to non-UI glyphs from the start (A.1.4).
- Tasks are not undoable in this RFC (A.8).
- Store and field semantics are keywords, not directives (principle 9).
- Trailing `@pure` stays accepted as an alias for `pure query` (A.2).
- A virtualized row scrolling out of view is an unmount (A.5), matching rndr's row recycling.

Resolved since r1: state lane (a store, A.1.4); commas (optional inside view, style and state blocks only); `view` versus `fragment` (a new kind).

## Rollout

The biggest integration risk is not the parser but whether the compiler, runtime, View IR and rndr agree on identity, update propagation and action dispatch. The rollout reaches a rendered, headlessly driven program before concurrency and ownership work.

0. **Fixes:** Part J (independently useful).
1. **Language core:** expressions (Part B), including expression bodies; stores with field kinds and lifecycle fields; single-store mutations; revisions and runtime diffs; scoped names (A.1–A.4, A.7, E). A headless session with `invoke` and `query`. Usable by non-UI glyphs.
2. **View IR:** the view grammar, enough for `Column`, `Text` and `Button` with one event binding; binding checks; node keys, action ids and binding epochs; `grapheme compile --emit view-ir` golden tests; `snapshot()` with the action list (C, D, I.1, I.3).
3. **Vertical slice:** the counter below, rendered by GraphemeUI on rndr. `Increment` is invoked once from a native click and once through the headless session, and both produce the same revision and diff. GraphemeUI lives outside the Grapheme crates (Non-goal 6).

   ```grapheme
   state Counter { count: Int = 0 }

   mutation Increment on Counter {
     apply { count: $count + 1 }
   }

   view CounterView on Counter {
     Column {
       value: Text("{$count}")
       increment: Button(label: "Increment") { on click: Increment }
     }
   }

   glyph CounterApp on Counter {
     CounterView
   }
   ```

4. **Concurrency and ownership:** store lifetimes, registries with `read`/`write`/`use`, transient stores, `VirtualList`, tasks, concurrency, fencing, cancellation, `@if_unchanged`, `time.sleep`, effect receipts, subscriptions on stores, commit-time lifecycle validation (A.5, A.6, F, G). The full worked panel.
5. **Operational polish:** UI glyph lifecycle, `expose`, `history`, `undo`, hot reload (A.8, A.9, H); accessibility verification against rndr's tree; LSP completion and hover for primitives, props and event signatures; an MCP adapter; expanded benchmarks.

## Acceptance

1. The worked example compiles under `--type-policy strict`, and its View IR matches a golden file.
2. A headless replay produces the expected revisions and diffs:
   - `click Select(index: 3)` → one revision, `selected: [0, 3]`, `presses: [0, 1]`.
   - `click Pulse` twice 200 ms apart → no revision from the first generation after the second starts; the second generation commits `mode: [pulsing, pulsing]` through the declared self-edge, and `pulse` reaches 0 ten steps after the second click.
   - `subscription Pinged(count: 2)` → `pings: [·, 2]`.
   - `click Pulse` → `mode: [idle, pulsing]`, then `mode: [pulsing, idle]` after the loop; a stale `Pulse` generation never moves `mode`. Runtime diffs from `invoke` include `pulse` and `mode`; the history log has neither.
   - `undo()` after `Select` restores `selected` and `presses`; a `Ping`'s `host.emit` receipt stays in the log after undo; a task's revisions are not undoable.
   - `ToggleDensity` from two glyphs whose registries both grant `write Prefs` changes one shared instance.
   - Unmounting a `PanelButton` drops its `ButtonHint`; remounting starts from `shown: false`.
3. `snapshot()` lists exactly the actions bound in the rendered tree plus the exposed ones; invoking an unrendered, unexposed action is rejected.
4. **Vertical slice** (rollout step 3): the counter renders in rndr; a native click and a headless `invoke` of the same action produce identical revisions and diffs; the snapshot's node keys and roles match the nodes in rndr's accessibility tree.
5. Verifier errors for:
   - element syntax outside a view, or a pipeline inside one;
   - a capability call in a view, derive or store mutation;
   - an `apply` to a derive, `const`, lifecycle or unknown field;
   - a `transition` along an undeclared edge or out of a `terminal` member;
   - a plain `state` listed in two registries (`ambiguous_store_owner`);
   - a read or write of a store the registry does not grant (`ungranted_store`), or an event bound to a mutation on a store granted only `read`;
   - a mutation writing, or calling a mutation on, a store other than its own (`cross_store_write`);
   - a task calling an op its registry does not grant;
   - a derive calling an op not declared `pure`;
   - a statically known self-transition with no declared self-edge;
   - a parameter that shadows a field in scope;
   - an event bound to a query;
   - missing, unknown or duplicate action arguments;
   - an unbound required event;
   - an unresolved symbol under strict policy;
   - a derive cycle.
6. **Adversarial conformance cases**, run through the headless session:
   - **stale virtualized row action:** snapshot, scroll row A out and item B into the recycled slot, invoke A's old action id → `stale_action`, B unchanged;
   - **concurrent store writes:** two tasks `apply` different fields of one store → both kept; two `@if_unchanged` writers of the same field → exactly one `stale_state`;
   - **restart during a suspended capability call:** restart `Pulse` while the first run waits in `time.sleep` or a provider call → the old run commits nothing further, and a dispatched call's receipt ends `indeterminate`;
   - **cancel mid-lifecycle, then recover:** cancel `Pulse` while `mode` is `pulsing` → `mode` stays `pulsing`; the next `Pulse` re-enters and finishes at `idle`;
   - **derive cycles through imported modules** → compile error;
   - **undo across a lifecycle boundary:** undo a group whose lifecycle restore is not a declared edge → `undo_conflict`, no commit; undo of an evicted group → `undo_unavailable`;
   - **hot reload while a task is active** → the task is fenced and ends `cancelled`, `transient` fields and the `transient` lifecycle `mode` reset to their defaults, plain fields are kept, and the reload is one revision.
7. Existing examples compile and run unchanged (after J.3).
