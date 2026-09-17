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
| GitHub Pages project site (current) | `/grapheme` | `https://entasislabs.github.io` |
| Custom domain at the root (later) | *(empty)* | `https://your-domain.example` |
| Local `vite dev` / `preview` | *(empty)* | *(optional)* |

```bash
BASE_PATH=/grapheme PUBLIC_SITE_URL=https://entasislabs.github.io npm run build
npm run preview   # serves at http://localhost:4173/grapheme/
```

`PUBLIC_SITE_URL` is scheme + host only (no path); it is combined with the base path for canonical and Open Graph URLs.

## Deploy (GitHub Pages)

**Live site:** https://entasislabs.github.io/grapheme/

Deployment is automated by [`.github/workflows/pages-site.yml`](../.github/workflows/pages-site.yml). On every push to `main` (and on manual `workflow_dispatch`) it:

1. Installs the stable Rust toolchain with the `wasm32-wasip1` target and builds the WASI engine with `scripts/build-runtime-wasm.sh` (`cargo build -p grapheme-wasm --release --target wasm32-wasip1`), failing if the `.wasm` is missing.
2. Runs `actions/configure-pages`, which reports the Pages `base_path` (`/grapheme`) and `origin` (`https://entasislabs.github.io`); these are passed to the build as `BASE_PATH` and `PUBLIC_SITE_URL`.
3. Runs `npm ci && npm run build` in `site/` on Node 22 (`prebuild` mirrors `docs/` and copies the `.wasm` into `static/`), then verifies `build/index.html`, `build/404.html`, and `build/grapheme-wasm.wasm` exist.
4. Uploads `site/build` with `actions/upload-pages-artifact` and publishes it with `actions/deploy-pages` to the `github-pages` environment.

Cargo and npm caches are kept between runs (`Swatinem/rust-cache`, `actions/setup-node` cache). The job uses only `contents: read`, `pages: write`, `id-token: write`.

**One-time setup:** in the repository go to *Settings → Pages → Build and deployment* and set *Source* to **GitHub Actions**. Until that is done, the `Configure Pages` step fails with `Get Pages site failed`. Nothing else needs configuring; the `github-pages` environment is created automatically on the first deploy.

Notes:

- `static/.nojekyll` is shipped so the `_app/` directory is never subject to Jekyll processing.
- `404.html` is the `adapter-static` fallback. All routes are prerendered, so it only serves genuinely unknown URLs; GitHub Pages serves it for those automatically.
- The root `/` of the deployed site is `index.html`; `/docs` is a prerendered redirect to `/docs/why-grapheme`.

### Adding a custom domain later

No workflow change is required. `actions/configure-pages` reads the domain from the Pages settings, so once a custom domain is attached it reports an empty `base_path` and the site is rebuilt for `/`.

1. **DNS** (at your DNS provider):
   - Apex domain (`example.com`): `A` records to GitHub Pages' IPs `185.199.108.153`, `185.199.109.153`, `185.199.110.153`, `185.199.111.153` (and optionally `AAAA` to `2606:50c0:8000::153`, `2606:50c0:8001::153`, `2606:50c0:8002::153`, `2606:50c0:8003::153`).
   - Subdomain (`www.example.com` or `docs.example.com`): `CNAME` to `entasislabs.github.io` (no repository name in the target).
   - Current values: https://docs.github.com/pages/configuring-a-custom-domain-for-your-github-pages-site/managing-a-custom-domain-for-your-github-pages-site
2. **GitHub:** *Settings → Pages → Custom domain*, enter the domain, save, wait for the DNS check, then tick *Enforce HTTPS* once the certificate is issued.
3. **`CNAME` file:** because the site is deployed from an Actions artifact (not a branch), add a `site/static/CNAME` file containing just the domain (e.g. `docs.example.com`) so it is included in every deploy and the custom-domain setting is not lost when the artifact is replaced. Commit it together with step 2.
4. Optionally add the domain under *Settings → Pages → Verified domains* (org level) to prevent takeover.
5. Push to `main` (or run the workflow manually). The next build picks up `BASE_PATH=""` and `PUBLIC_SITE_URL=https://<your-domain>` automatically; the old `entasislabs.github.io/grapheme/` URL redirects to the custom domain.
