import adapter from '@sveltejs/adapter-static';
import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';

const siteUrl = process.env.PUBLIC_SITE_URL?.replace(/\/$/, '');
if (!siteUrl) {
	console.warn(
		'[site] PUBLIC_SITE_URL is not set: Open Graph / Twitter URLs will be relative. Set it (see .env.example) before publishing.'
	);
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
		prerender: {
			handleHttpError: 'warn',
			entries: ['*'],
			...(siteUrl ? { origin: siteUrl } : {})
		}
	}
};

export default config;
