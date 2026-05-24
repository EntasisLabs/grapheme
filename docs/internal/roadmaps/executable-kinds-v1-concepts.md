# Executable Kinds v1 Concept Syntax Playground

Status: concept
Purpose: evaluate readability and ergonomics before implementation.

This file is intentionally syntax-heavy and policy-light.

## What We Are Testing

1. Kind-level intent: `query`, `mutation`, `node`.
2. Explicit write boundary: no hidden inline mutation in read flow.
3. State/data lane shape: `$current.state` and `$current.data`.
4. Pipeline feel: can we read the script quickly and predict shape changes?

## Direction Lock (current)

1. Mutation keyword preference: `apply`.
2. Write intent should read like first-class control syntax (same family as `transition`).
3. Root `$current` mutation should be discouraged; lane writes should be explicit (`state` vs `data`).
4. Primitive operations should move toward language-native or typed namespaces rather than a flat `core.*` surface.
5. `apply` is restricted to `mutation` declarations only.
6. This design pass prioritizes structural clarity over prototype-era compatibility.

## Baseline (Today)

```grapheme
enum ResearchStatus { collecting, synthesizing, done, failed }

state_machine ResearchLifecycle on ResearchStatus {
  transition collecting -> synthesizing
  transition synthesizing -> done
  transition synthesizing -> failed
  terminal done
  terminal failed
}

query Q on Any {
  websearch.research_materials(query: "Rust async runtime patterns")
  |> core.get_path(path: "sources")
  |> core.map(field: "citation")
  |> core.join(sep: "\n")
  |> core.set_path(path: "status", value: "collecting")
  |> ControlLoop
  |> core.get_path(path: "args.__input.text")
}
```

Pain:

1. Write (`set_path`) is mixed into read transform chain.
2. Root shape changes are inferred, not explicit.
3. Control state and payload data are not lane-separated.

## Concept A: `update` Keyword + `mutation` Kind

```grapheme
enum ResearchStatus { collecting, synthesizing, done, failed }

state_machine ResearchLifecycle on ResearchStatus {
  transition collecting -> synthesizing
  transition synthesizing -> done
  transition synthesizing -> failed
  terminal done
  terminal failed
}

query BuildCitationPack on Any -> Any {
  websearch.research_materials(query: "Rust async runtime patterns")
  |> core.get_path(path: "sources")
  |> core.map(field: "citation")
  |> core.join(sep: "\n")
  |> core.pack_state_data(state: { status: collecting })
}

mutation AdvanceLifecycle on Any -> Any {
  match $current.state.status {
    case collecting => update state { status: synthesizing }
    case synthesizing => if $current.data.text == "" then update state { status: failed } else update state { status: done }
    default => return
  }
}

node ControlLoop on Any @loop(max: 8, merge: "replace") {
  match $current.state.status {
    case done, failed => return
    default => AdvanceLifecycle
  }
}

query Q on Any {
  BuildCitationPack
  |> ControlLoop
  |> core.get_data
  |> core.get_path(path: "text")
}
```

Why this feels good:

1. `mutation` tells us exactly where writes occur.
2. `update state { ... }` is explicit and short.
3. read steps and write steps are visibly separate.

## Concept B: `patch` Keyword + `mutation` Kind

```grapheme
mutation AdvanceLifecycle on Any -> Any {
  match $current.state.status {
    case collecting => patch state { status: synthesizing }
    case synthesizing => if $current.data.text == "" then patch state { status: failed } else patch state { status: done }
    default => return
  }
}
```

Tradeoff:

1. Strongly conveys partial-write semantics.
2. Slightly less intuitive than `update` for non-technical readers.

## Concept C: `apply state { ... }` Form (preferred)

```grapheme
mutation AdvanceLifecycle on Any -> Any {
  match $current.state.status {
    case collecting => apply state { status: synthesizing }
    case synthesizing => if $current.data.text == "" then apply state { status: failed } else apply state { status: done }
    default => return
  }
}
```

Tradeoff:

1. Reads natural-language-like.
2. Slightly more verbose and less grep-friendly than `update`/`patch`.

## Explicit Lane Semantics (concept)

Goal: make shape changes obvious between calls.

Rules under strict profile:

1. `query` reads and derives by default.
2. `mutation` owns writes by default.
3. Writes target lanes explicitly:
   1. `apply state { ... }`
   2. `apply data { ... }`
4. Direct root writes are compatibility-only and linted.
5. `apply` in `query`/`node` is a verifier error, not a warning.

Example:

```grapheme
mutation AdvanceLifecycle on Any -> Any {
  match $current.state.status {
    case collecting => apply state { status: synthesizing }
    case synthesizing => if $current.data.text == "" then apply state { status: failed } else apply state { status: done }
    default => return
  }
}
```

## Operation Surface Concept (native + namespaced)

Pain today:

1. Long `core.*` chains hide value-domain intent.
2. Primitive operations feel like host capability calls, not language operations.

Proposed surface split:

1. Language-native primitives (compiler/runtime intrinsics):
   1. control + writes: `transition`, `apply`
   2. core selectors: `get`, `has`
2. Type/namespace methods for transforms:
   1. `string.join(...)`, `string.split(...)`
   2. `array.map(...)`, `array.filter(...)`
   3. `object.get_path(...)`, `object.set_path(...)`

Concept rewrite of baseline chain:

```grapheme
query BuildCitationPack on Any -> Any {
  websearch.research_materials(query: "Rust async runtime patterns")
  |> get("sources")
  |> array.map(field: "citation")
  |> string.join(sep: "\n")
  |> apply state { status: collecting }
}
```

Why this helps:

1. Operation intent is encoded by value domain.
2. Signature registry can be emitted per namespace/type.
3. Typed inference becomes more local and predictable.

## Syntax Size Check (quick feel)

Target: reduce cognitive load, not only line count.

Observations:

1. Baseline has one hidden write in a read chain.
2. Concept A separates write concerns by declaration type.
3. Concept A is easiest to scan for mutation boundaries (`mutation`, `update`).
4. Concept B is close second if we want explicit patch semantics.

## Proposed v1 Decision (for implementation)

1. Add `mutation` declaration kind.
2. Add `apply state { ... }` and `apply data { ... }` syntax as primary write constructs.
3. Keep `update` and `patch` as optional aliases only if ergonomics testing shows clear value.
4. Start de-emphasizing flat `core.*` transforms in favor of native + namespaced operation surface.

## Strict Mode Preview

In strict mode:

1. `query` cannot use write constructs.
2. `node` can orchestrate but cannot write directly.
3. `node` writes must occur through calls to `mutation` declarations.
4. `apply` is invalid outside `mutation`.
5. `mutation` is the only location for direct writes.

## Ergonomic Heuristics

This concept is successful if:

1. Humans can identify writes in under 3 seconds by scanning keywords.
2. LLM plans can choose call targets by kind without inferring hidden behavior.
3. The compiler can statically flag misplaced writes with low false positives.
