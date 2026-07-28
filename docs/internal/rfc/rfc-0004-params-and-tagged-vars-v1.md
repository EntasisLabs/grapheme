# RFC-0004: Executable Parameters and Tagged Variables v1

Status: draft
Authors: language + runtime + lsp
Created: 2026-07-28
Target release window: after typed-records verifier baseline

## Summary

Introduce two complementary binding mechanisms that move Grapheme closer to Lua-class composability without embedding a general-purpose scripting VM:

1. **Executable parameters v1** — finish the existing GraphQL-style `variable_defs` surface so `query` / `mutation` / `iterator` / `subscription` act as typed callables with named arguments.
2. **Tagged variables v1** — explicit ambient bindings declared at a high scope, visible only to an allow-listed set of executables, activated through **C#-style `using` scopes** that support multiple bindings, nesting, and deterministic drop-on-exit.

These are intentionally separate:

| Mechanism | Job |
|---|---|
| Parameters (`$ticket_id`) | Explicit inputs at the call edge — function API |
| `$state` / `$current` | Pipeline value flowing step → step |
| Tagged vars + `using` | Cross-cutting context (auth, trace, budget) with scoped lifetime, without stuffing `$state` |

## Motivation

### Current strengths

1. Typed executable signatures (`on InputType -> OutputType`) already parse, lower to HIR, and verify.
2. Grammar already accepts GraphQL-style params: `query Foo($x: String = "a")`.
3. `call Target(args…)` and bare `Target(args…)` already lower call args into `HirStep.args` / MIR `Call.args`.
4. Runtime already tracks call depth and has a `TemplateScope` for `$state` / `$item` / `$loop`.
5. Struct-shaped `$state` is a workable “arg bag” for many workflows today.

### Current gaps

1. `QueryDef.variables` / `MutationDef.variables` / `SubscriptionDef.variables` are parsed into AST, then **dropped at HIR** — they do not become runtime locals.
2. `IteratorDef` / `FragmentDef` / `NodeDef` have no `variable_defs` in grammar (only `on In -> Out`).
3. Call-site args on `call` are mostly ignored except special keys such as `max_depth`; they are **not** bound into the callee frame.
4. There is a single mutable `AgentState.current` for all scopes — nested calls and loops share one blob (`docs/internal/runtime/runtime-state-flow.md`).
5. Cross-cutting values (tokens, request ids, budgets) must either pollute `$state` or rely on host/env side channels.

### Desired outcome

- Authors can write reusable callables with named parameters.
- Authors can declare ambient context with explicit visibility and **scoped** lifetime.
- Authors can activate **multiple tags together** and nest scopes (C# `using` analogue).
- LLM authoring stays explicit (allow-lists, named bindings, visible scope braces) rather than inferred borrow checking.
- Governance model stays intact: compile-time ACL + runtime scope stack, no soft `eval`.

## Goals

1. Lower executable parameters from AST → HIR → MIR → runtime frame locals.
2. Bind call-site named args into callee locals with deterministic missing/default/type rules.
3. Expose params in template resolution as `$name` / `$args.name` without breaking `$state`.
4. Add tagged variable declarations with executable allow-lists.
5. Activate tags via `using` scopes that support **one or many bindings**, nesting, and drop-on-exit.
6. Enforce tag visibility by **current frame executable membership** (not ancestor leakage).
7. Keep tag values off the `$state` data plane by default (no silent escape).
8. Preserve untyped / signature-only programs (gradual adoption).

## Non-Goals

1. General lexical `let` for arbitrary pipeline locals (deferred; `using` is only for declared tags).
2. Rust-style lifetime inference, borrows, or exclusive mutable alias analysis.
3. First-class function values / closures / higher-order `map` callables (follow-on).
4. General expression AST for `until` / arithmetic (orthogonal; see control-flow deferred items).
5. Embedding Lua / Rhai / JS or any guest scripting VM.
6. Mutable tagged slots / mid-scope rebind in v1 (read-only after `using` enter).
7. Cross-execution persistence of tags (tags die with scope exit / run end).
8. IDisposable-style host resource finalizers in v1 (`using` scopes values; optional `@dispose` hook is follow-on).

## Relationship to Existing Surfaces

### Keep and document

```grapheme
iterator Step on SupportTicketState -> SupportTicketState { ... }
query Run on Any -> SupportTicketState { call Step }
```

`on In -> Out` remains the typed **pipeline value** contract for `$state`.

### Finish (params)

```grapheme
iterator Step($priority: String) on SupportTicketState -> SupportTicketState {
  echo(message: "priority={$priority}")
}

query Run {
  call Step(priority: "high")
}
```

### Add (tags + scoped `using`)

```grapheme
tag auth for [Entrypoint, FetchUser, Authorize, Audit] {
  $token: String
  $request_id: String
}

tag trace for [Entrypoint, FetchUser, Authorize, Audit, FormatHtml] {
  $correlation_id: String
}

tag budget for [Authorize, ExpensiveModel] {
  $max_usd: Float
}

query Entrypoint {
  using auth { token: $env.token, request_id: "r-1" },
        trace { correlation_id: "c-9" } {
    call FetchUser
    |> using budget { max_usd: 0.25 } {
         call Authorize
       }
    |> call Audit
  }
  |> call FormatHtml   // auth/trace scopes ended; $token not visible here
}
```

## Part A — Executable Parameters v1

### A.1 Surface syntax

Extend parameter lists consistently:

```text
query Name ( $param: Type (= value)? , ... )? (on In (-> Out)?)? { ... }
mutation Name ( $param: Type (= value)? , ... )? (on In (-> Out)?)? { ... }
subscription Name ( $param: Type (= value)? , ... )? (on In (-> Out)?)? { ... }
iterator Name ( $param: Type (= value)? , ... )? on In (-> Out)? { ... }
node Name ( $param: Type (= value)? , ... )? on In (-> Out)? { ... }
```

Notes:

1. `query` / `mutation` / `subscription` already have `variable_defs` in `grapheme.pest`.
2. Add the same optional `variable_defs` to `iterator_def` / `node_def` (and optionally `fragment_def` once fragment call semantics stabilize).
3. Param names are `$`-prefixed at declaration (existing `VariableDef`) and referenced as `$priority` in bodies.
4. Call sites use bare keys: `call Step(priority: "high")` — matching existing `named_arg` grammar.

Compatibility:

- Programs with only `on In -> Out` remain valid.
- Programs with only params and no signature remain valid.
- Entrypoint params may be supplied by CLI/SDK initial bindings (see A.5).

### A.2 Semantics

For each invocation of executable `E`:

1. Create a **call frame** with:
   - `locals: Map<String, JsonValue>` (params)
   - optional link to active tag env (Part B)
   - existing call-depth bookkeeping
2. Resolve each declared param:
   - If call-site arg present → use it (after template resolution in caller scope).
   - Else if default present → use default (resolved in caller scope).
   - Else → compile error for required params at verified call sites; runtime structured error for entrypoints missing SDK/CLI bindings.
3. `$state` / `$current` continue to carry the pipeline value.
4. Params do **not** automatically merge into `$state`.
5. On return, frame locals are dropped. Callee mutations to `$state` remain the return value (current passthrough model).

Template resolution order (v1):

1. Exact `$param` / `$param.field…` from frame locals
2. Existing `$state` / `$current` / `$item` / `$loop`
3. Active tagged bindings if allowed (Part B)
4. Unresolved → null / existing template behavior (choose one in open questions; prefer hard error in typed mode)

Alias (optional sugar, same object):

- `$args.priority` ≡ `$priority`

### A.3 Compiler / IR changes

#### AST (mostly done)

- `VariableDef { name, type_ref, default }` already exists.
- `CallStep.args` / `FieldCall.args` already exist.

#### HIR

Extend `HirExecutable`:

```rust
pub struct HirParam {
    pub name: String,              // without leading '$'
    pub type_ref: TypeRef,
    pub default: Option<JsonValue>,
    pub required: bool,
}

pub struct HirExecutable {
    // existing fields…
    pub params: Vec<HirParam>,
}
```

Lowering rules:

1. Preserve `Query/Mutation/Subscription.variables` into `params` (today dropped).
2. Parse new iterator/node variable lists into the same field.
3. Keep call args on `HirStep.args` as today.

Verifier additions:

1. Param names unique per executable.
2. No collision with reserved roots: `state`, `current`, `item`, `loop`, `args`, `env` (policy TBD for `$env`).
3. Call-site unknown arg names → error.
4. Missing required args → error when target params are known.
5. Type checks reuse typed-records assignability when types are named/scalars.
6. `on InputType` field checks remain about `$state`, not params.

#### MIR / artifact

Extend `MirFunction`:

```rust
pub struct MirParam {
    pub name: String,
    pub type_name: Option<String>,
    pub default: Option<JsonValue>,
    pub required: bool,
}

pub struct MirFunction {
    // existing fields…
    pub params: Vec<MirParam>,
}
```

`MirInst::Call.args` already carries call-site args JSON object — retain that. Runtime binds using callee `params` + instruction `args`.

Artifact format bump: include `params` with default `[]` for backward compatibility.

### A.4 Runtime changes

1. Introduce `CallFrame` (or extend the call path around `execute_function`) with `locals`.
2. When entering a function from `module == "call"`:
   - build locals from callee `params` + instruction args
   - do **not** require args to be stuffed into `state.current`
3. Extend `TemplateScope` with `locals: &'a Map<String, JsonValue>` (and later `tags`).
4. Resolve `$priority` from locals before `$state` path handling.
5. Trace: record bound param **names** always; values subject to existing redaction / `TracePolicy` (same posture as DB params in RFC-0003).

Entrypoint binding (CLI/SDK):

```rust
// SDK sketch
engine.execute(ExecuteRequest {
    source: Some(src),
    entrypoint_args: json!({ "priority": "high" }),
    ..
})?;
```

CLI sketch: `--arg priority=high` (repeatable) or `--args-json '{}'`.

### A.5 Acceptance for Part A

1. `iterator`/`query` with params round-trip AST→HIR→MIR.
2. `call Step(priority: "high")` binds `$priority` inside `Step`.
3. Missing required param fails at compile time for static call sites.
4. Defaulted params work when omitted.
5. Params do not appear in `$state` unless author explicitly `set`/`merge` them.
6. Existing no-param programs keep identical behavior.

## Part B — Tagged Variables v1 (scoped `using`)

### B.1 Concept

A **tag** is a named ambient environment schema:

1. Declares one or more typed bindings.
2. Declares an allow-list of executable names that may observe those bindings.
3. Is activated only inside an explicit **`using` scope** (C# `using` analogue).
4. Supports **multiple tag activations in one `using`**.
5. Supports **nested `using`** scopes.
6. Drops deterministically on scope exit (not merely on function return).

This is **not** Rust lifetime inference. It is an explicit ACL + scope stack.

`using` scopes **tag environments only**. `$state` still threads through the scope body and out to the continuation unchanged in shape (unless body steps mutate it).

### B.2 Why `using` instead of bare `bind`

A frame-lifetime `bind` is too coarse for real workflows:

1. Authors often need auth+trace together for a region, then neither afterward.
2. Nested regions need tighter budgets/secrets without ending the outer scope.
3. Multiple ambient contexts should activate/deactivate as one unit.
4. Visible braces are LLM- and audit-friendly (“what is live here?”).

v1 activation surface is therefore **`using`**, not free-floating `bind`.

### B.3 Surface syntax (proposed)

Top-level declaration (schema only):

```grapheme
tag auth for [Entrypoint, FetchUser, Authorize, Audit] {
  $token: String
  $request_id: String
}
```

Scoped activation (single or multiple bindings):

```grapheme
using auth { token: $env.token, request_id: "r-1" } {
  call FetchUser
  |> call Authorize
}

using auth { token: $env.token, request_id: "r-1" },
      trace { correlation_id: "c-9" },
      budget { max_usd: 1.5 } {
  call FetchUser
  |> call Authorize
}
```

Nested scopes:

```grapheme
using auth { token: $env.token, request_id: "r-1" } {
  call FetchUser
  |> using budget { max_usd: 0.25 } {
       call ExpensiveModel
     }
  |> call Authorize   // budget ended; auth still active
}
```

As a pipeline step (scope is still a block, not an open-ended suffix):

```grapheme
call Prepare
|> using auth { token: $env.token, request_id: "r-1" },
         trace { correlation_id: "c-9" } {
     call FetchUser
     |> call Authorize
   }
|> call FormatHtml
```

References inside allowed executables:

```grapheme
iterator Authorize on Request -> Decision {
  http.fetch(
    url: "https://example.test/authz",
    headers: { authorization: "Bearer {$token}" }
  )
}
```

Grammar sketch:

```pest
tag_def = {
    "tag" ~ ident ~ "for" ~ "[" ~ ident ~ ("," ~ ident)* ~ "]" ~ "{"
    ~ variable_def+
    ~ "}"
}

using_binding = {
    ident ~ object_value
}

using_step = {
    "using"
    ~ using_binding
    ~ ("," ~ using_binding)*
    ~ "{"
    ~ pipeline+
    ~ "}"
}
```

`tag_def` becomes a `Definition`. `using_step` becomes a `PipelineStep` (peer of `set` / `call` / `match`).

Semantic sugar (optional, same IR):

```grapheme
using (auth = { token: $env.token }, trace = { correlation_id: "c-9" }) { ... }
```

v1 normative form is the keyword+object form above.

### B.4 Visibility and lifetime rules

**Visibility (v1, strict):**

A tagged binding `$token` from tag `auth` is readable in the current frame **iff**:

1. An activation of `auth` is on the **scope stack**, and
2. Current executable name ∈ `auth.allow_list`.

Being under an allowed ancestor is **not** enough for a disallowed callee:

```text
Entrypoint
  using auth, trace {
    FetchUser        // listed: sees $token / $correlation_id
    FormatHtml       // if not listed for auth: cannot see $token
    Authorize        // listed: sees them again
  }
  FormatHtml         // auth/trace scopes popped
```

**Lifetime (v1):**

1. Entering `using` pushes one **scope frame** containing N tag activations (atomic with respect to body entry).
2. Body steps and nested calls observe those activations subject to allow-lists.
3. Leaving the `using` block pops that scope frame and drops all of its activations together.
4. Nested `using` pushes another scope frame; inner exit pops only the inner frame.
5. Function return pops any scopes still open in that call frame (safety net for early `$return` / failure paths).
6. Same tag activated again in an inner `using`: **forbid in v1** (no shadow/rebind). Revisit after fixtures exist.

**Failure / control-flow:**

1. If a body step fails fatally, runtime still pops the scope (drop is best-effort/deterministic before propagating failure).
2. `match` / `if` / `call` targets that `$return` out of the owning executable pop open scopes for that frame.
3. Jumping into a `using` body without entering it is impossible — scopes are structured.

**Mutability (v1):**

- Bindings are read-only after scope enter.
- No mid-scope mutation API in v1.

**Multi-bind atomicity:**

- Either all bindings in a `using` header validate and activate, or none do (fail before body).
- Duplicate tag names in one header → compile error.
- Unknown field for a tag → compile error.

### B.5 Escape / governance rules

1. Verifier rejects reading tagged names outside allow-listed executables.
2. Verifier rejects reading tagged names in code regions that are not statically nested in a `using` that activates that tag (best-effort static); runtime still enforces presence.
3. Verifier rejects copying tagged bindings into `set` / `apply` / `merge` / struct init **by default**.
4. Escape hatch (optional, explicit), only inside an active scope:

   ```grapheme
   set { request_id: $request_id } @promote(tag: auth, fields: [request_id])
   ```

   Without `@promote`, promotion is a compile error. This preserves “tags are not `$state`.”

5. Capability policy remains orthogonal: tags do not grant host capabilities; they only transport values.
6. Host dispose hooks (true C# `IDisposable`) are **out of v1**; a future `@dispose(op: secrets.forget)` on a tag binding may appear later.

### B.6 Compiler / IR changes

#### AST

```rust
pub struct TagDef {
    pub name: String,
    pub allow_list: Vec<String>,
    pub variables: Vec<VariableDef>,
}

pub struct UsingBinding {
    pub tag: String,
    pub fields: Vec<(String, Value)>,
}

pub struct UsingStep {
    pub bindings: Vec<UsingBinding>, // 1..N, atomic
    pub pipelines: Vec<Pipeline>,    // scoped body
}
```

`PipelineStep` gains `Using(UsingStep)`.

#### HIR

```rust
pub struct HirTagDef {
    pub name: String,
    pub allow_list: Vec<String>,
    pub bindings: Vec<HirParam>, // reuse param shape
}

pub struct HirUsingStep {
    pub scope_id: u32,
    pub bindings: Vec<(String /* tag */, JsonValue /* fields */)>,
    pub body: HirPipeline,
}

pub struct HirProgram {
    // existing…
    pub tag_defs: Vec<HirTagDef>,
}
```

Prefer dedicated MIR ops (not host capability calls):

```rust
pub enum MirInst {
    // existing Call / BranchCall / MatchCall…
    UsingEnter {
        scope_id: u32,
        activations: Vec<MirTagActivation>,
    },
    UsingExit {
        scope_id: u32,
    },
}

pub struct MirTagActivation {
    pub tag: String,
    pub fields: JsonValue,
}
```

Lowering:

1. `using a {…}, b {…} { body }` → `UsingEnter` + lowered body instructions + `UsingExit`.
2. Nested `using` → nested enter/exit pairs with distinct `scope_id`s.
3. Verifier/MIR emitter guarantee matching enter/exit on all exits from the body (including early return lowering).

Artifact carries `tag_defs` beside functions.

Verifier:

1. Tag names unique.
2. Allow-list executables must exist.
3. Binding names unique within a tag; recommend globally unique tagged names in v1 for bare `$token` (or require `$auth.token` — open question).
4. Every `$token`-style read resolves to param, reserved root, or exactly one tag binding declaration.
5. Static region check: reads of tag bindings must occur in executables that can be invoked from within an activating `using` body **and** are allow-listed (conservative; runtime remains source of truth for dynamic call graphs).
6. Owner executable may read tag values inside the `using` body only if listed in `for […]` (proposed default: list itself explicitly).

### B.7 Runtime changes

Extend engine state with a **scope stack** (orthogonal to call stack, but always cleared on call-frame return):

```rust
struct TagActivation {
    tag: String,
    values: Map<String, JsonValue>,
}

struct UsingScope {
    scope_id: u32,
    activations: Vec<TagActivation>,
}

struct CallFrame {
    function_name: String,
    locals: Map<String, JsonValue>,
    using_stack: Vec<UsingScope>,
}
```

Ops:

1. `UsingEnter` — validate fields, push `UsingScope`.
2. Execute body with `$state` threading as today.
3. `UsingExit` — pop matching `scope_id` (assert top), emit drop trace.
4. On call-frame return / fatal failure — pop all remaining `using_stack` entries outermost-last or innermost-first (pick innermost-first to mirror C# dispose order).

Lookup for `$token`:

1. Frame locals (params)
2. Reserved templates (`$state` / `$item` / `$loop`)
3. From **top of `using_stack` downward**, find first activation containing `token` whose tag allow-list includes **current** function name
4. Else unresolved

Trace:

- `using_enter` with `scope_id`, tag names, field names (values redacted)
- `using_exit` / `using_drop` with `scope_id`
- Nested scopes appear as nested enter/exit pairs

### B.8 Acceptance for Part B

1. `tag` + `using` parse and appear in artifact metadata.
2. Multi-bind `using a {…}, b {…}` activates both atomically.
3. Nested `using` pops only the inner scope at inner exit.
4. Allowed executable inside scope can read `$token`; disallowed cannot.
5. Steps after scope exit cannot read those bindings.
6. Early failure / `$return` still drops open scopes for the frame.
7. Promotion into `$state` without `@promote` fails verification.
8. Trace shows enter/exit boundaries for each scope.
9. No-tag programs unchanged.

## Combined Mental Model

```mermaid
flowchart TD
  A[Caller frame] -->|call Step priority: high| B[Callee frame locals]
  A -->|using auth, trace| C[Push using scope]
  C --> D[Body pipeline]
  D -->|nested using budget| E[Push inner scope]
  E --> F[Inner body]
  F --> G[Pop inner scope]
  G --> H[Outer body continues]
  H --> I[Pop outer scope]
  B --> J["$priority from locals"]
  D --> K["$state threads through"]
  D --> L[Authorize allow-listed reads $token]
  I --> M[After exit: tagged names gone]
```

## Sequencing

### Phase 0 — Spec lock

1. Land this RFC.
2. Add language-contract subsections + examples fixtures skeletons (compile-fail + run-ok).

### Phase 1 — Params v1

1. HIR/MIR `params` plumbing (stop dropping AST variables).
2. Grammar: iterator/node `variable_defs`.
3. Runtime frame locals + template resolution.
4. Verifier call-arity/name checks.
5. CLI/SDK entrypoint arg binding.
6. Docs + LSP hover for params.

### Phase 2 — Tagged vars + `using` v1

1. Grammar `tag` / `using` (multi-bind + nested blocks).
2. HIR tag defs + MIR `UsingEnter` / `UsingExit`.
3. Runtime scope stack + allow-list lookup.
4. Verifier allow-list, region, multi-bind, anti-escape rules.
5. Trace enter/exit + redaction; drop on failure/`$return`.
6. Docs + LSP: “who can see this tag?” / scope highlighting.

### Phase 3 — Hardening

1. Conformance fixtures for multi-bind, nest/pop order, defaults, missing args, deny lists.
2. Policy-profile checks for redacted param/tag values in traces.
3. Decide follow-ons: shadow/rebind, `$auth.token` qualification, `@dispose`, HOFs, lexical `let`.

## Testing Strategy

1. **Parser/AST**: param lists on iterators; `tag` / multi-bind `using` / nested `using`.
2. **HIR/MIR golden**: params preserved; matching `UsingEnter`/`UsingExit` pairs; artifact backward compatible defaults.
3. **Verifier**:
   - unknown call arg
   - missing required param
   - tag allow-list unknown executable
   - duplicate tag in one `using` header
   - tagged read outside allow-list / outside activating region
   - illegal promotion into `set`
4. **Runtime**:
   - multi-bind activate both
   - nested pop order
   - allowed call reads value; disallowed does not
   - after scope exit, bindings gone
   - failure / `$return` still drops scopes
   - params do not leak into `$state`
5. **SDK/CLI**: entrypoint `--arg` / `entrypoint_args` wiring.
6. **LSP** (soft gate): signature help shows params; scope/tag visibility hints later.

## Observability

1. Pipeline / step context gains optional `params_bound: [names…]`, `tags_active: [tag…]`, `using_scope_id`.
2. Values governed by existing `TracePolicy` redaction.
3. Deterministic event order: `using_enter` → body steps → `using_exit` (nested pairs nest in the trace).

## Security Considerations

1. Tags are not a capability bypass — host ops still pass `PolicyGuard`.
2. Secrets in tags must default to redacted in traces (treat like secrets-module handles where possible).
3. Forbid serializing entire tag env into Wasm plugin args implicitly; only explicit arg references pass values.
4. Allow-list is deny-by-default for ambient reads.

## Documentation Plan

1. This RFC under `docs/internal/rfc/`.
2. Add language notes:
   - `docs/internal/language/params-v1.md` (normative contract extract)
   - `docs/internal/language/tagged-vars-v1.md` (normative contract extract)
3. Update `docs/internal/language-contract.md` with binding precedence.
4. Update `docs/internal/runtime/runtime-state-flow.md` with frame locals + tag activations.
5. Tutorial touch: one realworld example using params; one using multi-bind `using auth, trace`.

## Open Questions

1. **Param reference style:** is `$priority` enough, or require `$args.priority` for disambiguation?
2. **Tagged reference style:** bare `$token` (v1, globally unique names) vs qualified `$auth.token`?
3. **May the owning executable read tags inside its own `using` body without being in `for […]`?** Default proposed: no (must list itself).
4. **Unresolved template policy:** null (legacy) vs hard error in typed programs?
5. **Should `fragment` support params in v1** or wait until fragment invocation rules settle?
6. **CLI UX:** repeatable `--arg k=v` vs single `--args-json`?
7. **Inner re-activation of same tag:** forbid in v1 (proposed) or allow nested shadow?
8. **Promotion:** is `@promote` required, or is a dedicated `promote auth.request_id -> state.request_id` step clearer?
9. **Interaction with `@loop each`:** does `$item` shadow params of the same name? Proposed: yes, `$item` wins inside each-body templates; params remain addressable via `$args.name`.
10. **`using` as expression-step only vs also statement-prefix?** Proposed: both, as long as body braces are required (no open-ended `using` that leaks to function end).
11. **Dispose order on multi-bind exit:** reverse header order (C# style) vs declaration order? Proposed: reverse header order.
12. **Should v1 include a non-block `bind` sugar** that means “using for remainder of current pipeline”? Proposed: **no** — braces required.

## Acceptance Criteria (RFC-level)

1. Spec distinguishes params, `$state`, and tagged `using` scopes with non-overlapping jobs.
2. Part A and Part B each have phased implementation checklists grounded in current AST/HIR/MIR/runtime types.
3. Strict allow-list visibility + **scoped** lifetime (`UsingEnter`/`UsingExit`) for tags is normative.
4. Multi-bind and nested `using` behavior is specified, including drop-on-failure.
5. Backward compatibility for programs that use neither feature is explicit.
6. Open questions are listed with proposed defaults so implementation can start on Phase 1 without blocking on Part B syntax bikesheds.

## Appendix — Current Code Anchors

| Area | Path | Notes |
|---|---|---|
| Grammar `variable_defs` | `crates/grapheme-compiler/src/grapheme.pest` | Present on query/mutation/subscription; missing on iterator/node |
| AST `VariableDef` | `crates/grapheme-compiler/src/ast.rs` | Parsed today |
| HIR drop of variables | `crates/grapheme-compiler/src/hir.rs` | `HirExecutable` has no `params` |
| Call arg lowering | `crates/grapheme-compiler/src/hir.rs` `lower_step` | Args kept on step; not callee schema |
| MIR ISA | `crates/grapheme-artifact/src/mir.rs` | `Call` / `BranchCall` / `MatchCall` only |
| Template scope | `crates/grapheme-runtime/src/runtime.rs` | `$state` / `$item` / `$loop` |
| State model pressure | `docs/internal/runtime/runtime-state-flow.md` | Single global `current` |
| Typed signatures | `docs/internal/language/typed-records-v1.md` | `on In -> Out` baseline |
