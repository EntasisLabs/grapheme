# 02: Language Core

Goal: understand the primitives that make workflows readable and composable.

## Concepts to Learn

1. Executables (`query`, `mutation`, `iterator`, `glyph`)
2. State shaping with `set`, `pick`, `map`, `get_path`
3. Control-flow with `if`, `match`, `return`
4. Lifecycle transitions with `transition`

## Recommended examples

```bash
grapheme run examples/core-merge.gr --json
grapheme run examples/core-filter.gr --json
grapheme run examples/mutation-state-machine-apply.gr --json
```

## Exercise

Take one example and:

1. add one state field,
2. route one new branch,
3. keep output contract readable.

If teammates can review your flow quickly, you are using Grapheme correctly.
