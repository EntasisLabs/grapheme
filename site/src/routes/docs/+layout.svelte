<script lang="ts">
	import { page } from '$app/state';
	import { DOC_NAV } from '$lib/docs';

	let { children } = $props();

	const sections = [...new Set(DOC_NAV.map((d) => d.section).filter(Boolean))] as string[];
	const current = $derived(page.url.pathname.replace(/^\/docs\//, '').replace(/\/$/, ''));
	const here = $derived(DOC_NAV.find((d) => d.slug === current));

	let open = $state(false);

	// Close the drawer on navigation and lock body scroll while it is open.
	$effect(() => {
		current;
		open = false;
	});
	$effect(() => {
		if (typeof document === 'undefined') return;
		document.body.style.overflow = open ? 'hidden' : '';
		return () => {
			document.body.style.overflow = '';
		};
	});
</script>

<div class="docs-shell" class:open>
	<button
		class="docs-bar"
		type="button"
		aria-expanded={open}
		aria-controls="docs-nav"
		onclick={() => (open = !open)}
	>
		<span class="crumb">
			<span class="crumb-section">{here?.section ?? 'Docs'}</span>
			<span class="crumb-title">{here?.title ?? 'Contents'}</span>
		</span>
		<span class="toggle">{open ? 'Close' : 'Contents'}</span>
	</button>

	<aside id="docs-nav">
		<a class="aside-title" href="/docs/why-grapheme">Docs</a>
		{#each sections as section}
			<p class="section">{section}</p>
			<ul>
				{#each DOC_NAV.filter((d) => d.section === section) as item}
					<li>
						<a
							href={`/docs/${item.slug}`}
							class:active={current === item.slug}
							aria-current={current === item.slug ? 'page' : undefined}>{item.title}</a
						>
					</li>
				{/each}
			</ul>
		{/each}
		<p class="section">Reference</p>
		<ul>
			<li><a href="/playground">Playground</a></li>
			<li>
				<a class="ext" href="https://github.com/EntasisLabs/grapheme/tree/main/docs/internal" rel="noreferrer"
					>Internals</a
				>
			</li>
			<li>
				<a class="ext" href="https://github.com/EntasisLabs/grapheme/blob/main/CHANGELOG.md" rel="noreferrer"
					>Changelog</a
				>
			</li>
		</ul>
	</aside>

	<article class="doc">
		{@render children()}
	</article>
</div>

<style>
	.docs-shell {
		display: grid;
		grid-template-columns: minmax(12rem, 15rem) minmax(0, 1fr);
		gap: 3rem;
		max-width: 76rem;
		margin: 0 auto;
		padding: 1.5rem clamp(1rem, 4vw, 3rem) 3rem;
		align-items: start;
	}

	.docs-bar {
		display: none;
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
		margin: 0 0 0.75rem;
		font-weight: 600;
		font-size: 1rem;
		text-decoration: none;
		color: var(--ink);
	}

	.section {
		margin: 1.1rem 0 0.35rem;
		font-size: 0.72rem;
		font-weight: 500;
		letter-spacing: 0.06em;
		text-transform: uppercase;
		color: var(--ink-soft);
	}

	ul {
		list-style: none;
		margin: 0;
		padding: 0;
	}

	li a {
		display: block;
		padding: 0.3rem 0 0.3rem 0.75rem;
		border-left: 2px solid transparent;
		text-decoration: none;
		color: var(--ink-soft);
	}

	li a:hover {
		color: var(--ink);
	}

	li a.active {
		color: var(--ink);
		border-left-color: var(--sage);
		font-weight: 600;
	}

	.doc {
		max-width: 46rem;
		min-width: 0;
		padding: 0.25rem 0 1rem;
	}

	@media (max-width: 860px) {
		.docs-shell {
			display: block;
			padding: 0 0 2rem;
		}

		/* Sticky contents bar under the site nav. */
		.docs-bar {
			position: sticky;
			top: var(--nav-h);
			z-index: 30;
			display: flex;
			align-items: center;
			justify-content: space-between;
			gap: 1rem;
			width: 100%;
			min-height: 3rem;
			padding: 0.5rem 1.25rem;
			border: 0;
			border-bottom: 1px solid var(--line);
			background: var(--paper);
			color: var(--ink);
			text-align: left;
			cursor: pointer;
		}

		.crumb {
			display: flex;
			align-items: baseline;
			gap: 0.6rem;
			min-width: 0;
		}

		.crumb-section {
			flex: none;
			font-size: 0.72rem;
			font-weight: 500;
			letter-spacing: 0.06em;
			text-transform: uppercase;
			color: var(--ink-soft);
		}

		.crumb-title {
			font-weight: 600;
			font-size: 0.92rem;
			white-space: nowrap;
			overflow: hidden;
			text-overflow: ellipsis;
		}

		.toggle {
			flex: none;
			font-size: 0.85rem;
			font-weight: 500;
			color: var(--sage);
		}

		/* The sidebar becomes a drawer below the bar. */
		aside {
			display: none;
			position: fixed;
			top: calc(var(--nav-h) + 3rem);
			left: 0;
			right: 0;
			bottom: 0;
			z-index: 29;
			max-height: none;
			overflow: auto;
			padding: 0.25rem 1.25rem 3rem;
			background: var(--paper);
			font-size: 0.95rem;
			-webkit-overflow-scrolling: touch;
		}

		.open aside {
			display: block;
		}

		.aside-title {
			display: none;
		}

		.section {
			margin: 1.25rem 0 0.4rem;
		}

		li a {
			padding: 0.55rem 0 0.55rem 0.85rem;
		}

		.doc {
			max-width: none;
			padding: 1.25rem 1.25rem 0;
		}
	}
</style>
