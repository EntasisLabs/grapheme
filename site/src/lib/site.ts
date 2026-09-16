import { env } from '$env/dynamic/public';

/**
 * Set PUBLIC_SITE_URL at build time (e.g. in site/.env) so social cards get
 * absolute URLs. Falls back to the request origin during prerender/dev.
 */
export const SITE_URL = (env.PUBLIC_SITE_URL ?? '').replace(/\/$/, '');

export const SITE_NAME = 'Grapheme';
export const DEFAULT_TITLE = 'Grapheme — a small language for workflows that must not go wrong quietly';
export const DEFAULT_DESCRIPTION =
	'Typed state, visible control flow, side effects as capabilities you grant. Compiles to a verified artifact; runs native, in WASI, or in your browser.';
export const GITHUB_URL = 'https://github.com/EntasisLabs/grapheme';
