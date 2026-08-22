# Parameters and Tagged Variables v1 (0.7.0)

Status: normative extract for authors
Source RFCs: `docs/internal/rfc/rfc-0004-params-and-tagged-vars-v1.md`
Release: **0.7.0** (Phases 1–2a shipped)

## Executable parameters

`query` / `mutation` / `iterator` / `subscription` may declare GraphQL-style parameter lists:

```gr
query Hello($label: String = "world") {
  call Greet(label: $label)
}

iterator Greet($label: String) on Any {
  core.echo(message: "hello {$label}")
}
```

Rules in 0.7.0:

1. Parameters lower to MIR/runtime frame locals.
2. Defaults apply when a call site / entrypoint omits the name.
3. Call sites bind with named args (`call Greet(label: $label)`).
4. Entrypoint binding:
   - CLI: `grapheme run <file.gr> --args-json '{"label":"grapheme"}'`
   - SDK: `GraphemeEngine::builder().with_entrypoint_args(json!({ "label": "grapheme" }))`

Canonical example: `examples/params-call-bind.gr`.

## Tags and scoped `using`

A `tag` declares named ambient bindings. A block `using` activates them for its body only:

```gr
tag auth {
  $token: String
}

query Demo {
  using auth(token: "secret") {
    core.echo(message: "token={$token}")
  }
  |> core.echo(message: "after")
}
```

Rules in 0.7.0:

1. Reading a tagged name outside an activating `using` is rejected (static best-effort + runtime).
2. Nested `using` scopes stack; inner bindings shadow outer ones for the same name.
3. Phase 3+ (tag-typed parameters as the fundamental call-edge model, `uses` sugar) is **not** in 0.7.0.

Canonical example: `examples/tag-using-scope.gr`.

## Not the same as SQL bind params

`examples/sql-query-params.gr` demonstrates SQL `?` placeholders for `sql.query` — that is host SQL binding, not RFC-0004 executable parameters.

## See also

- Language contract: `docs/internal/language-contract.md`
- CLI: `docs/internal/cli.md` (`--args-json`)
- SDK: `docs/internal/sdk.md` (`with_entrypoint_args`)
- Full design: RFC-0004
