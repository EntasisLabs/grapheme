import { env } from '$env/dynamic/public';

/**
 * Set PUBLIC_SITE_URL at build time (e.g. in site/.env) so social cards get
 * absolute URLs. Falls back to the request origin during prerender/dev.
 */
export const SITE_URL = (env.PUBLIC_SITE_URL ?? '').replace(/\/$/, '');

export const SITE_NAME = 'Grapheme';
export const DEFAULT_TITLE = 'Grapheme — a typed workflow language with capability-gated side effects';
export const DEFAULT_DESCRIPTION =
	'Grapheme compiles .gr programs to a verified artifact and executes them step by step. Side effects are capabilities the host grants; every step is recorded in a trace. Runs native, in WASI, or in your browser.';
export const GITHUB_URL = 'https://github.com/EntasisLabs/grapheme';
