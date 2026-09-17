<script lang="ts">
	import { asset } from '$app/paths';
	import { page } from '$app/state';
	import { SITE_URL, SITE_NAME, DEFAULT_TITLE, DEFAULT_DESCRIPTION } from '$lib/site';

	let {
		title = DEFAULT_TITLE,
		description = DEFAULT_DESCRIPTION,
		image = asset('/og.png'),
		type = 'website'
	}: { title?: string; description?: string; image?: string; type?: string } = $props();

	// During prerender without PUBLIC_SITE_URL the origin is SvelteKit's placeholder host;
	// emit relative URLs then rather than a fake absolute one.
	const origin = $derived(
		SITE_URL || (page.url.origin.includes('sveltekit-prerender') ? '' : page.url.origin)
	);
	const url = $derived(`${origin}${page.url.pathname}`);
	const imageUrl = $derived(image.startsWith('http') ? image : `${origin}${image}`);
</script>

<svelte:head>
	<title>{title}</title>
	<meta name="description" content={description} />
	<link rel="canonical" href={url} />

	<meta property="og:type" content={type} />
	<meta property="og:site_name" content={SITE_NAME} />
	<meta property="og:title" content={title} />
	<meta property="og:description" content={description} />
	<meta property="og:url" content={url} />
	<meta property="og:image" content={imageUrl} />
	<meta property="og:image:width" content="1200" />
	<meta property="og:image:height" content="630" />
	<meta property="og:image:alt" content="Grapheme — a typed workflow language with capability-gated side effects" />

	<meta name="twitter:card" content="summary_large_image" />
	<meta name="twitter:title" content={title} />
	<meta name="twitter:description" content={description} />
	<meta name="twitter:image" content={imageUrl} />
	<meta name="theme-color" content="#1f3528" />
</svelte:head>
