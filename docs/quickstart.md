# Quickstart

Goal: run your first Grapheme workflow and understand what success looks like.

Estimated time: 10 minutes.

Requires Rust **1.92+** / edition **2024** when building from source (`rust-toolchain.toml`).

## Install Paths

### Path A: Use the CLI directly (recommended)

If you only want to use Grapheme workflows, install the CLI:

```bash
cargo install --path crates/grapheme-cli --locked
```

If installing from git:

```bash
cargo install --git https://github.com/entasislabs/grapheme.git grapheme-cli --bin grapheme
```

### Path B: Run from source workspace

If you are exploring internals or contributing, run through Cargo:

```bash
cargo run -- run examples/hello-world.gr --json
```

## First Run (CLI)

Initialize bundled examples locally:

```bash
grapheme examples init --out .
```

Run hello world:

```bash
grapheme run examples/hello-world.gr --json
```

Expected outcome:

- Output includes `"outcome": "succeeded"`.
- Final state includes `"message": "LETS GO?!!!!!"`.

If you see those two signals, your Grapheme runtime path is healthy.

## 0.7.0 language path (params + tags)

Pass named entrypoint parameters:

```bash
grapheme run examples/params-call-bind.gr --args-json '{"label":"grapheme"}' --json
```

Activate a tagged ambient binding with scoped `using`:

```bash
grapheme run examples/tag-using-scope.gr --json
```

Author notes: `docs/internal/language/params-and-tags-v1.md`.

## Stage B AOT path (0.7.0)

`grapheme build` defaults to Stage B. For a quick smoke:

```bash
./scripts/build-aot-container.sh   # once per checkout (Wasix / emission asset)
grapheme build examples/hello-world.gr --aot-stage stage_b --json
grapheme run examples/hello-world.gr --aot-stage stage_b --json
```

Details: `docs/internal/cli.md`, `docs/internal/release/release-0.7.0.md`.

## Discover What You Can Do

List modules:

```bash
grapheme modules
```

Search capabilities:

```bash
grapheme modules search web --detail concise --yaml
```

Find runnable module examples:

```bash
grapheme modules examples core
grapheme modules examples data
```

## Capability modules (0.6.0+, still current)

Build Wasm plugins, scan, activate, and run the platform demo:

```bash
bash plugins/build-plugins.sh
grapheme modules scan
grapheme modules activate plot
grapheme modules activate pdf
grapheme run examples/platform-release-060.gr
```

Native examples (no Wasm activate):

```bash
grapheme run examples/data-read-csv.gr
grapheme run examples/media-probe.gr   # requires ffmpeg/ffprobe on PATH
```

See `docs/internal/sdk-feature-flags.md` and [CHANGELOG.md](../CHANGELOG.md#070---2026-08-21) for the **0.7.0** release notes (and the 0.6.0 capability train beneath it).

## Next Step

- Run the flagship end-to-end tutorial in `hero-workflow.md`.
- Follow the full curriculum in `tutorials/README.md`.
- Run production-style scenario labs in `../examples/realworld/README.md`.
- Learn language concepts in `language-tour.md`.
- Run practical scenarios in `playbooks.md`.
