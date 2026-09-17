# Grapheme site

Product site for Grapheme — landing, docs, and an in-browser WASM playground (SvelteKit), in the spirit of svelte.dev.

## Develop

From the repo root:

```bash
bash scripts/build-runtime-wasm.sh   # once (or after runtime changes)
cd site
npm install
npm run sync-wasm
npm run sync-docs
npm run dev
```

Open http://localhost:5173

## Surfaces

| Route | Purpose |
| --- | --- |
| `/` | Brand landing |
| `/docs/*` | Product docs mirrored from `docs/` (excludes `docs/internal/`) |
| `/playground` | Client-side compile + execute via `grapheme-wasm.wasm` |

## Playground notes

- Loads the RFC-0006 WASI engine in the browser through `@bjorn3/browser_wasi_shim`.
- Wasm-safe stdlib only: `core`, `json`, `csv`, `yaml`, `html`.
- Host-only ops (`http`, `sql`, …) fail closed by design.

## Build / deploy

```bash
npm run build   # writes static site to site/build
npm run preview
```

The WASM binary is copied into `static/` at build time and is gitignored (~3.6MB). CI or host deploy should run `scripts/build-runtime-wasm.sh` before `npm run build`.
