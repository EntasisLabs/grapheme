# Quickstart

Goal: run your first Grapheme workflow and understand what success looks like.

Estimated time: 10 minutes.

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
```

## Next Step

- Run the flagship end-to-end tutorial in `hero-workflow.md`.
- Learn language concepts in `language-tour.md`.
- Run practical scenarios in `playbooks.md`.
