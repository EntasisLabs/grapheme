<script lang="ts">
	let { data } = $props();
</script>

<svelte:head>
	<title>{data.title} · Grapheme Docs</title>
</svelte:head>

<header class="doc-head">
	<p class="eyebrow">Documentation</p>
	<h1>{data.title}</h1>
</header>

<div class="prose">
	{@html data.html}
</div>

<style>
	.doc-head {
		margin-bottom: 1.5rem;
		padding-bottom: 1rem;
		border-bottom: 1px solid var(--line);
	}

	.eyebrow {
		margin: 0 0 0.35rem;
		font-family: var(--font-mono);
		font-size: 0.75rem;
		letter-spacing: 0.08em;
		text-transform: uppercase;
		color: var(--signal);
	}

	h1 {
		margin: 0;
		font-family: var(--font-display);
		font-size: clamp(1.8rem, 3vw, 2.4rem);
		letter-spacing: -0.03em;
	}

	.prose {
		color: var(--ink);
	}

	.prose :global(h1) {
		display: none; /* page title already shown */
	}

	.prose :global(h2),
	.prose :global(h3) {
		font-family: var(--font-display);
		letter-spacing: -0.02em;
		margin-top: 2rem;
	}

	.prose :global(p),
	.prose :global(li) {
		color: var(--ink-soft);
	}

	.prose :global(a) {
		color: var(--signal);
	}

	.prose :global(pre) {
		overflow-x: auto;
		padding: 1rem 1.1rem;
		border: 1px solid var(--line);
		background: color-mix(in srgb, var(--mist) 88%, var(--paper-deep));
		font-size: 0.86rem;
	}

	.prose :global(code) {
		font-family: var(--font-mono);
		font-size: 0.9em;
	}

	.prose :global(:not(pre) > code) {
		padding: 0.1em 0.35em;
		background: color-mix(in srgb, var(--paper-deep) 80%, white);
		border-radius: 2px;
	}

	.prose :global(ul),
	.prose :global(ol) {
		padding-left: 1.2rem;
	}

	.prose :global(blockquote) {
		margin: 1.2rem 0;
		padding: 0.2rem 0 0.2rem 1rem;
		border-left: 3px solid var(--signal);
		color: var(--ink-soft);
	}

	.prose :global(table) {
		width: 100%;
		border-collapse: collapse;
		font-size: 0.92rem;
	}

	.prose :global(th),
	.prose :global(td) {
		border: 1px solid var(--line);
		padding: 0.45rem 0.6rem;
		text-align: left;
	}
</style>
