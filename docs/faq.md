# FAQ

## Is Grapheme only for this repository?

No. Grapheme is a language and CLI/runtime model. This repository is one implementation and source distribution.

Start with `hero-workflow.md` if you want the shortest path to understanding Grapheme as a product.

## Why do many examples show `cargo run`?

Because much of the existing documentation was written from a contributor/workspace perspective. Product usage should prefer installed `grapheme` CLI commands.

## Do I need to compile the full codebase to use Grapheme?

Not always. If you install the CLI binary, you can run workflows without day-to-day repository development tasks.

## Is Grapheme a general-purpose language?

Grapheme is specialized for governed automation workflows and capability-driven orchestration.

## Where should I go for internals and contributor setup?

Use docs under `docs/internal/` for compiler/runtime internals, release workflows, and deep implementation detail.

## How do I pass entrypoint parameters?

Declare `$param` lists on `query` / `mutation` / `iterator` / `subscription`, then bind at the CLI:

```bash
grapheme run examples/params-call-bind.gr --args-json '{"label":"grapheme"}'
```

In the SDK, use `with_entrypoint_args`. See `docs/internal/language/params-and-tags-v1.md`.

## What are tags and `using`?

A `tag` declares ambient bindings; a block `using` activates them for nested code only. Example: `examples/tag-using-scope.gr`.

## What is Stage B?

Stage B is the AOT workflow-container path: MIR walks inside a Wasm-safe container, and host-only capabilities are fulfilled across rounds. `grapheme build` defaults to `stage_b`. Build the container asset with `./scripts/build-aot-container.sh`. Details: `docs/internal/cli.md` and `docs/internal/release/release-0.7.0.md`.

## What if policy blocks my workflow?

That is expected behavior for side-effecting capabilities. Configure allow-list environment variables for the capability you intend to use.

## How do I share runtime feedback safely?

Use telemetry export from the CLI to generate redacted report files.
