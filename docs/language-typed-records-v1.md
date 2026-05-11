# Language Typed Records v1 Proposal

Status: proposed

## Goal

Introduce first-class typed records ("structs") that improve correctness and editor intelligence without sacrificing pipeline ergonomics.

## Why Now

Recent control-flow/runtime upgrades made stateful programs practical. The next bottleneck is shape ambiguity:

1. Function inputs/outputs are implicit.
2. Field accesses are mostly unchecked at compile time.
3. LSP cannot reliably infer flow-sensitive state shape.

Typed records solve these together when paired with typed function signatures and module contracts.

## Design Principles

1. Keep records data-first (no methods, no inheritance).
2. Prefer structural compatibility first; defer nominal strictness.
3. Make type metadata available compiler -> MIR -> runtime -> LSP.
4. Preserve existing untyped programs (gradual typing).

## MVP Surface

### 1) Record Declarations

```aql
struct FibState {
  a: number
  b: number
  i: number
  threshold: number
  message?: string
}
```

Rules:

1. Required field: `name: type`
2. Optional field: `name?: type`
3. Allowed scalar types: `string | number | bool | null`
4. Composite types (MVP):
   - `array<T>`
   - `record<string, T>`

### 2) Typed Executable Signatures

```aql
iterator FibUntilThreshold on FibState -> FibState @loop(max: 64, merge: "replace") {
  ...
}
```

MVP support:

1. `query Name on InputType -> OutputType`
2. `mutation Name on InputType -> OutputType`
3. `iterator Name on InputType -> OutputType`
4. `subscription Name on InputType -> OutputType`

Backward compatible forms without types remain valid.

### 3) Typed Module Contracts (Read-Only in v1)

Add optional arg/return types to module-op metadata so compiler/LSP can validate and hint:

1. `core.add(a: number, b: number) -> { value: number }`
2. `core.set_fields(fields: record<string, any>) -> same_as_input`
3. `html.to_md(html: string) -> { text: string, markdown: string }`

## Type System Behavior (MVP)

### Structural Assignability

Type `A` is assignable to `B` if:

1. `A` has at least all required fields of `B`.
2. Shared fields are compatible by type.
3. Optional fields in `B` may be absent in `A`.

### Field Access Rules

1. Access to unknown field is compile-time error in typed scope.
2. Access to optional field without guard emits warning (not error in v1).
3. Untyped scopes keep current permissive behavior.

### Branch Narrowing (v1.1 target)

After branch predicates, record shape can narrow:

1. `when: { field: "status", eq: "ready" }` narrows `status` in then branch.
2. Numeric predicates (`gt/gte/lt/lte`) can narrow numeric ranges for diagnostics.

## Compiler and Runtime Plan

### Phase A: Parse + HIR Types

1. Parse `struct` declarations.
2. Parse typed executable signatures.
3. Attach declared input/output type refs in HIR.

### Phase B: Verifier Type Checks

1. Validate referenced types exist.
2. Validate call arg types for known module contracts.
3. Validate pipeline field access against current inferred shape.
4. Validate return shape compatibility with executable output type.

### Phase C: MIR Type Metadata

1. Add optional type table in artifact metadata.
2. Preserve hashes and backward compatibility when metadata absent.

### Phase D: Runtime Contract Guards (opt-in)

1. Optional runtime validation for boundary calls (entrypoint/module).
2. Controlled by policy flag; off by default in v1.

## LSP Plan

1. Hover shows inferred record at cursor.
2. Completion proposes known fields from inferred record.
3. Signature help uses typed executable and module contracts.
4. Diagnostics include expected vs actual type details with one fix hint.

## Migration Strategy

1. v1 ships as gradual typing.
2. Existing programs compile unchanged.
3. Typed mode can be enabled file-by-file.

## Non-Goals for v1

1. Generics on user-defined structs.
2. Nominal typing and interfaces/traits.
3. Full expression type inference.
4. Exhaustive pattern matching.

## Acceptance Criteria

1. Typed records parse and verify in compiler.
2. At least 3 showcase programs run with typed signatures.
3. LSP hover/completion reflect record fields in typed files.
4. Untyped programs remain source and artifact compatible.

## Suggested First Implementation Slice

1. Add parser/HIR support for struct declarations and typed signatures.
2. Add verifier checks for unknown fields and missing required fields.
3. Add LSP field completion from declared input/output record types.
4. Add one typed showcase: fibonacci threshold with `FibState`.
