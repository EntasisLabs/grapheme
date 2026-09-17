import adapter from '@sveltejs/adapter-static';
import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';

const siteUrl = process.env.PUBLIC_SITE_URL?.replace(/\/$/, '');
if (!siteUrl) {
	console.warn(
		'[site] PUBLIC_SITE_URL is not set: Open Graph / Twitter URLs will be relative. Set it (see .env.example) before publishing.'
	);
}

// URL prefix the site is served under. Empty for a root deploy (custom domain,
// `vite dev`); "/grapheme" for the GitHub Pages project site
// (https://entasislabs.github.io/grapheme/). CI derives it from
// actions/configure-pages, so it flips to "" automatically once a custom
// domain is attached. Must start with "/" and must not end with one.
const basePath = (process.env.BASE_PATH ?? '').replace(/\/$/, '');
if (basePath && !basePath.startsWith('/')) {
	throw new Error(`[site] BASE_PATH must start with "/" (got "${basePath}")`);
}

/** @type {import('@sveltejs/kit').Config} */
const config = {
	preprocess: vitePreprocess(),
	kit: {
		adapter: adapter({
			fallback: '404.html',
			precompress: false,
			strict: true
		}),
		paths: {
			base: basePath,
			// Emit absolute `${base}/...` URLs rather than `../` relative ones so
			// values that get concatenated onto an origin (og:image, canonical) stay valid.
			relative: false
		},
		prerender: {
			handleHttpError: 'warn',
			entries: ['*'],
			...(siteUrl ? { origin: siteUrl } : {})
		}
	}
};

export default config;
