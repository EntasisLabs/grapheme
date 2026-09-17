<script lang="ts">
	import { resolve } from '$app/paths';
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
			<a class="prev" href={resolve('/docs/[...slug]', { slug: data.prev.slug })}>
				<small>Previous</small>
				<span>{data.prev.title}</span>
			</a>
		{:else}<span></span>{/if}
		{#if data.next}
			<a class="next" href={resolve('/docs/[...slug]', { slug: data.next.slug })}>
				<small>Next</small>
				<span>{data.next.title}</span>
			</a>
		{/if}
	</div>
	<a class="edit ext" href={data.editUrl} rel="noreferrer" target="_blank">Edit this page on GitHub</a>
</footer>

<style>
	.doc-head {
		margin-bottom: 1.5rem;
		padding-bottom: 1rem;
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
		font-size: clamp(1.6rem, 2.6vw + 0.6rem, 2.4rem);
		letter-spacing: -0.02em;
		line-height: 1.15;
		color: var(--ink);
	}

	.prose :global(h1) {
		display: none;
	}

	.prose :global(h2),
	.prose :global(h3) {
		letter-spacing: -0.015em;
		line-height: 1.25;
		color: var(--ink);
	}

	.prose :global(h2) {
		font-size: clamp(1.25rem, 1.2vw + 0.8rem, 1.5rem);
		margin: 2.5rem 0 0.75rem;
	}

	.prose :global(h3) {
		font-size: 1.05rem;
		margin: 1.75rem 0 0.5rem;
	}

	.prose :global(p),
	.prose :global(li) {
		color: var(--ink-soft);
		line-height: 1.6;
	}

	.prose :global(p) {
		margin: 0.85rem 0;
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
		white-space: pre-wrap;
		overflow-wrap: break-word;
	}

	.prose :global(pre.gr .ln) {
		display: block;
		padding-left: calc((var(--in, 0) + 2) * 1ch);
		text-indent: calc((var(--in, 0) + 2) * -1ch);
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
		display: block;
		width: 100%;
		overflow-x: auto;
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
		margin-top: 2.5rem;
		padding-top: 1.25rem;
		border-top: 1px solid var(--line);
		display: grid;
		gap: 1rem;
	}

	.pager {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 0.75rem;
	}

	.pager a {
		display: grid;
		gap: 0.15rem;
		padding: 0.8rem 1rem;
		border: 1px solid var(--line);
		border-radius: var(--radius);
		text-decoration: none;
		min-height: 3.5rem;
	}

	.pager a:hover {
		border-color: var(--sage);
	}

	.pager .next {
		grid-column: 2;
		text-align: right;
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
		color: var(--ink);
	}

	.edit {
		font-size: 0.85rem;
		color: var(--ink-soft);
		text-decoration: none;
	}

	.edit:hover {
		color: var(--sage);
	}

	@media (max-width: 860px) {
		/* The sticky contents bar already names the section. */
		.eyebrow {
			display: none;
		}

		.doc-head {
			margin-bottom: 1.25rem;
		}

		.pager {
			grid-template-columns: 1fr;
		}

		.pager .next {
			grid-column: 1;
			text-align: left;
		}

		.prose :global(pre) {
			padding: 0.9rem 1rem;
		}

		.prose :global(ul),
		.prose :global(ol) {
			padding-left: 1.1rem;
		}
	}
</style>
