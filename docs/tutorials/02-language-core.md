# 02: Language Core

Goal: understand the primitives that make workflows readable and composable.

## Concepts to Learn

1. Executables (`query`, `mutation`, `iterator`, `glyph`)
2. **Parameters (0.7.0):** `$param` lists, defaults, `--args-json`
3. **Tags / `using` (0.7.0):** ambient bindings scoped to a block
4. State shaping with `set`, `pick`, `map`, `get_path`
5. Control-flow with `if`, `match`, `return`
6. Lifecycle transitions with `transition`

## Recommended examples

```bash
grapheme run examples/hello-world.gr --json
grapheme run examples/params-call-bind.gr --args-json '{"label":"grapheme"}' --json
grapheme run examples/tag-using-scope.gr --json
grapheme run examples/core-merge.gr --json
grapheme run examples/core-filter.gr --json
grapheme run examples/mutation-state-machine-apply.gr --json
```

Author extract: `docs/internal/language/params-and-tags-v1.md`.

## Exercise

Take one example and:

1. add one state field **or** one executable parameter,
2. route one new branch **or** one scoped `using` block,
3. keep output contract readable.

If teammates can review your flow quickly, you are using Grapheme correctly.
