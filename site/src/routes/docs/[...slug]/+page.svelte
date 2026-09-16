<script lang="ts">
	import Seo from '$lib/components/Seo.svelte';

	let { data } = $props();
</script>

<Seo title={`${data.title} · Grapheme Docs`} description={data.description} type="article" />

<header class="doc-head">
	<p class="eyebrow">{data.section}</p>
	<h1>{data.title}</h1>
</header>

<div class="prose">
	{@html data.html}
</div>

<footer class="doc-foot">
	<div class="pager">
		{#if data.prev}
			<a class="prev" href={`/docs/${data.prev.slug}`}>
				<small>Previous</small>
				<span>{data.prev.title}</span>
			</a>
		{:else}<span></span>{/if}
		{#if data.next}
			<a class="next" href={`/docs/${data.next.slug}`}>
				<small>Next</small>
				<span>{data.next.title}</span>
			</a>
		{/if}
	</div>
	<a class="edit" href={data.editUrl} rel="noreferrer" target="_blank">Edit this page on GitHub ↗</a>
</footer>

<style>
	.doc-head {
		margin-bottom: 1.75rem;
		padding-bottom: 1.1rem;
		border-bottom: 1px solid var(--line);
	}

	.eyebrow {
		margin: 0 0 0.35rem;
		font-size: 0.74rem;
		font-weight: 500;
		letter-spacing: 0.06em;
		text-transform: uppercase;
		color: var(--ink-soft);
	}

	h1 {
		margin: 0;
		font-size: clamp(1.9rem, 3.2vw, 2.6rem);
		letter-spacing: -0.035em;
		color: var(--sage-deep);
	}

	.prose :global(h1) {
		display: none;
	}

	.prose :global(h2),
	.prose :global(h3) {
		letter-spacing: -0.02em;
		margin-top: 2.2rem;
		color: var(--sage-deep);
	}

	.prose :global(h2) {
		font-size: 1.5rem;
		padding-top: 1.2rem;
		border-top: 1px solid var(--line);
	}

	.prose :global(p),
	.prose :global(li) {
		color: var(--ink-soft);
	}

	.prose :global(strong) {
		color: var(--ink);
	}

	.prose :global(a) {
		color: var(--sage);
	}

	.prose :global(pre) {
		overflow-x: auto;
		padding: 1rem 1.1rem;
		border: 1px solid var(--line);
		background: color-mix(in srgb, var(--mist) 88%, var(--paper-deep)) !important;
		font-size: 0.85rem;
		line-height: 1.55;
	}

	.prose :global(pre.gr) {
		background: var(--panel) !important;
		color: #ece8df;
		border-color: #2c2a27;
	}

	.prose :global(pre.gr .t-kw) { color: #a8d5b3; }
	.prose :global(pre.gr .t-def) { color: #f6efe0; font-weight: 600; }
	.prose :global(pre.gr .t-type) { color: #e8c38f; }
	.prose :global(pre.gr .t-str) { color: #f0b48a; }
	.prose :global(pre.gr .t-interp) { color: #ffd2a8; }
	.prose :global(pre.gr .t-var) { color: #9ec9f5; }
	.prose :global(pre.gr .t-dir) { color: #e9d18a; }
	.prose :global(pre.gr .t-num),
	.prose :global(pre.gr .t-lit) { color: #f0b48a; }
	.prose :global(pre.gr .t-pipe) { color: #8ff0b8; font-weight: 700; }
	.prose :global(pre.gr .t-arrow) { color: #8ff0b8; }
	.prose :global(pre.gr .t-mod) { color: #bfd3c4; }
	.prose :global(pre.gr .t-fn) { color: #f6efe0; }
	.prose :global(pre.gr .t-key) { color: #cfe0d3; }
	.prose :global(pre.gr .t-cm) { color: #8faa96; font-style: italic; }
	.prose :global(pre.gr .t-p) { color: #bfd3c4; }

	.prose :global(code) {
		font-family: var(--font-mono);
		font-size: 0.9em;
	}

	.prose :global(:not(pre) > code) {
		padding: 0.1em 0.35em;
		background: color-mix(in srgb, var(--sage) 10%, transparent);
		color: var(--sage-deep);
		border-radius: 2px;
	}

	.prose :global(ul),
	.prose :global(ol) {
		padding-left: 1.2rem;
	}

	.prose :global(li) {
		margin: 0.25rem 0;
	}

	.prose :global(blockquote) {
		margin: 1.2rem 0;
		padding: 0.2rem 0 0.2rem 1rem;
		border-left: 3px solid var(--sage);
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

	.prose :global(th) {
		color: var(--sage-deep);
		background: color-mix(in srgb, var(--sage) 8%, transparent);
	}

	.doc-foot {
		margin-top: 3rem;
		padding-top: 1.5rem;
		border-top: 1px solid var(--line);
		display: grid;
		gap: 1rem;
	}

	.pager {
		display: flex;
		justify-content: space-between;
		gap: 1rem;
	}

	.pager a {
		display: grid;
		gap: 0.15rem;
		padding: 0.8rem 1rem;
		border: 1px solid var(--line);
		text-decoration: none;
		min-width: 12rem;
	}

	.pager a:hover {
		border-color: var(--sage);
	}

	.pager .next {
		text-align: right;
		margin-left: auto;
	}

	.pager small {
		font-size: 0.7rem;
		font-weight: 500;
		letter-spacing: 0.06em;
		text-transform: uppercase;
		color: var(--ink-soft);
	}

	.pager span {
		font-weight: 600;
		color: var(--sage-deep);
	}

	.edit {
		font-size: 0.85rem;
		color: var(--ink-soft);
		text-decoration: none;
	}

	.edit:hover {
		color: var(--sage);
	}
</style>
