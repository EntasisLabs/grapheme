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

## Build

```bash
npm run build   # writes static site to site/build
npm run preview
```

The WASM binary is copied into `static/` at build time and is gitignored (~3.6MB). CI or host deploy should run `scripts/build-runtime-wasm.sh` before `npm run build`.

### Base path

The site can be served from the domain root or under a URL prefix. `BASE_PATH` (read in `svelte.config.js`) sets SvelteKit's `kit.paths.base`; every internal link and asset URL goes through `resolve()` / `asset()` from `$app/paths`, so the prefix is applied everywhere at build time.

| Deploy target | `BASE_PATH` | `PUBLIC_SITE_URL` |
| --- | --- | --- |
| GitHub Pages, custom domain (current) | *(empty)* | `https://grapheme-lang.org` |
| GitHub Pages project site (if the custom domain is removed) | `/grapheme` | `https://entasislabs.github.io` |
| Local `vite dev` / `preview` | *(empty)* | *(optional)* |

```bash
PUBLIC_SITE_URL=https://grapheme-lang.org npm run build
npm run preview   # serves at http://localhost:4173/

# Project-site layout, for checking that base-path handling still works:
BASE_PATH=/grapheme PUBLIC_SITE_URL=https://entasislabs.github.io npm run build
npm run preview   # serves at http://localhost:4173/grapheme/
```

`PUBLIC_SITE_URL` is scheme + host only (no path); it is combined with the base path for canonical and Open Graph URLs.

## Deploy (GitHub Pages)

**Live site:** https://grapheme-lang.org/ (custom domain; https://entasislabs.github.io/grapheme/ redirects there)

Deployment is automated by [`.github/workflows/pages-site.yml`](../.github/workflows/pages-site.yml). On every push to `main` (and on manual `workflow_dispatch`) it:

1. Installs the stable Rust toolchain with the `wasm32-wasip1` target and builds the WASI engine with `scripts/build-runtime-wasm.sh` (`cargo build -p grapheme-wasm --release --target wasm32-wasip1`), failing if the `.wasm` is missing.
2. Runs `actions/configure-pages`, which reports the Pages `base_path` — empty with the custom domain attached, `/grapheme` without it — and passes it to the build as `BASE_PATH`. `PUBLIC_SITE_URL` is pinned to `https://grapheme-lang.org` in the workflow rather than taken from configure-pages' `origin`, because that output mirrors the Pages `html_url` and stays `http://` until *Enforce HTTPS* is enabled.
3. Runs `npm ci && npm run build` in `site/` on Node 22 (`prebuild` mirrors `docs/` and copies the `.wasm` into `static/`), then verifies `build/index.html`, `build/404.html`, `build/grapheme-wasm.wasm`, and `build/CNAME` (containing `grapheme-lang.org`) exist.
4. Uploads `site/build` with `actions/upload-pages-artifact` and publishes it with `actions/deploy-pages` to the `github-pages` environment.

Cargo and npm caches are kept between runs (`Swatinem/rust-cache`, `actions/setup-node` cache). The job uses only `contents: read`, `pages: write`, `id-token: write`.

**One-time setup:** in the repository go to *Settings → Pages → Build and deployment* and set *Source* to **GitHub Actions**. Until that is done, the `Configure Pages` step fails with `Get Pages site failed`. Nothing else needs configuring; the `github-pages` environment is created automatically on the first deploy.

Notes:

- `static/.nojekyll` is shipped so the `_app/` directory is never subject to Jekyll processing.
- `static/CNAME` (`grapheme-lang.org`) is shipped in every artifact. Because the site is deployed from an Actions artifact rather than a branch, a deploy without it would detach the custom domain; the workflow fails the build if it is missing.
- `404.html` is the `adapter-static` fallback. All routes are prerendered, so it only serves genuinely unknown URLs; GitHub Pages serves it for those automatically.
- The root `/` of the deployed site is `index.html`; `/docs` is a prerendered redirect to `/docs/why-grapheme`.

### Custom domain: `grapheme-lang.org`

The site is served from the apex domain `grapheme-lang.org`. The pieces, and where each one lives:

| Piece | Where | Value |
| --- | --- | --- |
| Pages custom domain | Repo *Settings → Pages → Custom domain* (also settable via the Pages API `cname` field) | `grapheme-lang.org` |
| `CNAME` file in every deploy | [`static/CNAME`](static/CNAME) | `grapheme-lang.org` |
| Canonical / OG origin | `PUBLIC_SITE_URL` in [`pages-site.yml`](../.github/workflows/pages-site.yml) | `https://grapheme-lang.org` |
| Base path | `BASE_PATH` from `actions/configure-pages` | *(empty)* |

**DNS** (at the registrar / DNS provider for `grapheme-lang.org`):

| Name | Type | Value |
| --- | --- | --- |
| `@` (apex) | `A` | `185.199.108.153` |
| `@` (apex) | `A` | `185.199.109.153` |
| `@` (apex) | `A` | `185.199.110.153` |
| `@` (apex) | `A` | `185.199.111.153` |
| `@` (apex), optional | `AAAA` | `2606:50c0:8000::153`, `2606:50c0:8001::153`, `2606:50c0:8002::153`, `2606:50c0:8003::153` |
| `www`, optional | `CNAME` | `entasislabs.github.io` (no repository name in the target) |

All four `A` records are required; do not put a `CNAME` on the apex. If `www` is added, GitHub redirects `www.grapheme-lang.org` to the apex automatically once the apex is the configured custom domain. GitHub's current values: https://docs.github.com/pages/configuring-a-custom-domain-for-your-github-pages-site/managing-a-custom-domain-for-your-github-pages-site

Verify with `dig +short grapheme-lang.org A` (expect the four IPs) and `dig +short www.grapheme-lang.org CNAME` (expect `entasislabs.github.io.`).

**After DNS propagates:**

1. *Settings → Pages* shows "DNS check successful". GitHub then requests a Let's Encrypt certificate, which can take up to an hour after the check passes.
2. Once the certificate exists, tick *Enforce HTTPS*. Until then `http://grapheme-lang.org` works but `https://` does not, and the *Enforce HTTPS* checkbox is greyed out.
3. Optionally add `grapheme-lang.org` under the organisation's *Settings → Pages → Verified domains* to prevent takeover.

**Removing or changing the domain:** update `static/CNAME`, `PUBLIC_SITE_URL` in the workflow, and the Pages setting together. If the custom domain is removed, `actions/configure-pages` reports `BASE_PATH=/grapheme` again on the next build and the site is served from `https://entasislabs.github.io/grapheme/`; set `PUBLIC_SITE_URL` back to `https://entasislabs.github.io` in the workflow in that case.
