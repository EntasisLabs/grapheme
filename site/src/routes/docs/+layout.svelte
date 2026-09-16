<script lang="ts">
	import { DOC_NAV } from '$lib/docs';

	let { children } = $props();

	const sections = [...new Set(DOC_NAV.map((d) => d.section).filter(Boolean))] as string[];
</script>

<div class="docs-shell">
	<aside>
		<p class="aside-title">Docs</p>
		{#each sections as section}
			<p class="section">{section}</p>
			<ul>
				{#each DOC_NAV.filter((d) => d.section === section) as item}
					<li>
						<a href={`/docs/${item.slug}`}>{item.title}</a>
					</li>
				{/each}
			</ul>
		{/each}
	</aside>
	<article class="doc">
		{@render children()}
	</article>
</div>

<style>
	.docs-shell {
		display: grid;
		grid-template-columns: minmax(12rem, 16rem) minmax(0, 1fr);
		gap: 2rem;
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
		margin: 0 0 1rem;
		font-family: var(--font-display);
		font-weight: 800;
		font-size: 1.1rem;
		letter-spacing: -0.03em;
	}

	.section {
		margin: 1rem 0 0.35rem;
		font-family: var(--font-mono);
		font-size: 0.72rem;
		letter-spacing: 0.08em;
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
		padding: 0.28rem 0;
		text-decoration: none;
		color: var(--ink-soft);
	}

	li a:hover {
		color: var(--ink);
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
