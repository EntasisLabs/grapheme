# Language Contract

This document defines the current behavior contract for Grapheme Grapheme Lang in this repository.

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
- schema
- module proposal

Only executable definitions are lowered to MIR functions:

- query
- mutation
- subscription
- iterator

## Entrypoint and Execution Scope

Artifact execution runs exactly one MIR function: the artifact entrypoint.

- If an explicit entrypoint is provided, that function is used.
- Otherwise, the first MIR function is used.

Implication: definitions after the selected entrypoint are not executed in that run.

## Pipeline Semantics

A pipeline is an ordered list of call steps.

- Steps execute left-to-right.
- Each step output becomes current AgentState.
- Runtime injects previous current state into step args as `__input`.

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

## Variables and Defaults

Variable definitions parse and are represented in AST.

Current runtime substitution behavior is limited:

- Variables used in values are lowered as string placeholders like `$name`.
- Automatic runtime binding/substitution of variable values is not fully implemented.

Treat variable interpolation as provisional until full binding semantics are introduced.

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

## Policy and Failure Contract

Execution gates:

1. capability policy allow-list
2. policy guard argument checks

Failure behavior:

- denied capability/policy -> fatal execution outcome
- host fatal/retryable errors map to runtime failure outcomes
- if `__input.error` exists, runtime currently short-circuits with fatal failure

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
- complete variable binding model

Current control-flow capabilities now include iterator loops, iterator invocation, and branch dispatch (`flow.branch`) lowered through compiler-to-MIR.

Loop policy shift (current draft behavior):

- `@loop max` is optional at compile time.
- When `@loop` is present without max, loop semantics are unbounded at language level.
- Runtime policy (step budget) is expected to bound execution operationally.

Recursive policy shift (current draft behavior):

- `@recursive max_depth` is optional at compile time.
- Runtime call-depth policy is expected to bound recursive execution operationally.

Control-flow design draft for loops and recursion:

- `docs/language-control-flow-v1.md`

## Compatibility Guidance

When extending the language, keep these invariants stable unless versioned:

- left-to-right pipeline execution
- `__input` threading contract
- single-entrypoint artifact execution
- capability + policy checks before dispatch
- module ABI resolution order