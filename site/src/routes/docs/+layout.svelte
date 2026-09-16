<script lang="ts">
	import { page } from '$app/state';
	import { DOC_NAV } from '$lib/docs';

	let { children } = $props();

	const sections = [...new Set(DOC_NAV.map((d) => d.section).filter(Boolean))] as string[];
	const current = $derived(page.url.pathname.replace(/^\/docs\//, '').replace(/\/$/, ''));
</script>

<div class="docs-shell">
	<aside>
		<a class="aside-title" href="/docs/why-grapheme">Docs</a>
		{#each sections as section}
			<p class="section">{section}</p>
			<ul>
				{#each DOC_NAV.filter((d) => d.section === section) as item}
					<li>
						<a href={`/docs/${item.slug}`} class:active={current === item.slug} aria-current={current === item.slug ? 'page' : undefined}>{item.title}</a>
					</li>
				{/each}
			</ul>
		{/each}
		<p class="section">Reference</p>
		<ul>
			<li><a href="/playground">Playground</a></li>
			<li><a href="https://github.com/EntasisLabs/grapheme/tree/main/docs/internal" rel="noreferrer">Internals ↗</a></li>
			<li><a href="https://github.com/EntasisLabs/grapheme/blob/main/CHANGELOG.md" rel="noreferrer">Changelog ↗</a></li>
		</ul>
	</aside>
	<article class="doc">
		{@render children()}
	</article>
</div>

<style>
	.docs-shell {
		display: grid;
		grid-template-columns: minmax(12rem, 16rem) minmax(0, 1fr);
		gap: 2.5rem;
		padding: 1.5rem clamp(1rem, 3vw, 2.5rem) 3rem;
		align-items: start;
	}

	aside {
		position: sticky;
		top: calc(var(--nav-h) + 1rem);
		max-height: calc(100vh - var(--nav-h) - 2rem);
		overflow: auto;
		padding-right: 0.5rem;
		font-size: 0.92rem;
	}

	.aside-title {
		display: block;
		margin: 0 0 1rem;
		font-family: var(--font-display);
		font-weight: 800;
		font-size: 1.1rem;
		letter-spacing: -0.03em;
		text-decoration: none;
		color: var(--sage-deep);
	}

	.section {
		margin: 1rem 0 0.35rem;
		font-family: var(--font-mono);
		font-size: 0.7rem;
		letter-spacing: 0.1em;
		text-transform: uppercase;
		color: var(--signal);
	}

	ul {
		list-style: none;
		margin: 0;
		padding: 0;
	}

	li a {
		display: block;
		padding: 0.28rem 0 0.28rem 0.75rem;
		border-left: 2px solid transparent;
		text-decoration: none;
		color: var(--ink-soft);
	}

	li a:hover {
		color: var(--sage-deep);
	}

	li a.active {
		color: var(--sage-deep);
		border-left-color: var(--sage);
		font-weight: 600;
	}

	.doc {
		max-width: 48rem;
		padding: 0.25rem 0 2rem;
	}

	@media (max-width: 860px) {
		.docs-shell {
			grid-template-columns: 1fr;
		}

		aside {
			position: static;
			max-height: none;
			border-bottom: 1px solid var(--line);
			padding-bottom: 1rem;
		}
	}
</style>
