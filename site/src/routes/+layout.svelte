<script lang="ts">
	import '../app.css';
	import SiteNav from '$lib/components/SiteNav.svelte';

	let { children } = $props();
</script>

<SiteNav />
<main>
	{@render children()}
</main>

<footer class="foot">
	<div class="cols">
		<div class="brand-col">
			<span class="brand">grapheme</span>
			<p>A small language for governed automation. Apache-2.0. Built in Rust by Entasis Labs.</p>
		</div>
		<div>
			<p class="h">Learn</p>
			<a href="/docs/why-grapheme">Why Grapheme</a>
			<a href="/docs/quickstart">Quickstart</a>
			<a href="/docs/language-tour">Language tour</a>
			<a href="/docs/tutorials">Tutorials</a>
		</div>
		<div>
			<p class="h">Use</p>
			<a href="/playground">Playground</a>
			<a href="https://github.com/EntasisLabs/grapheme/tree/main/examples" rel="noreferrer">Examples ↗</a>
			<a href="https://github.com/EntasisLabs/grapheme/tree/main/extensions/grapheme-vscode" rel="noreferrer">VS Code ↗</a>
			<a href="https://github.com/EntasisLabs/grapheme/blob/main/CHANGELOG.md" rel="noreferrer">Changelog ↗</a>
		</div>
		<div>
			<p class="h">Design</p>
			<a href="https://github.com/EntasisLabs/grapheme/tree/main/docs/internal/rfc" rel="noreferrer">RFCs ↗</a>
			<a href="https://github.com/EntasisLabs/grapheme/blob/main/docs/internal/architecture.md" rel="noreferrer">Architecture ↗</a>
			<a href="https://github.com/EntasisLabs/grapheme" rel="noreferrer">GitHub ↗</a>
		</div>
	</div>
	<p class="fine mono">grapheme-wasm · runtime-in-wasm · RFC-0006 — the playground on this site executes without a server.</p>
</footer>

<style>
	main {
		min-height: calc(100vh - var(--nav-h) - 5rem);
	}

	.foot {
		padding: 2.5rem clamp(1rem, 4vw, 3rem) 2rem;
		border-top: 1px solid var(--line);
		background: color-mix(in srgb, var(--paper-deep) 55%, transparent);
		color: var(--ink-soft);
		font-size: 0.92rem;
	}

	.cols {
		display: grid;
		grid-template-columns: 1.4fr 1fr 1fr 1fr;
		gap: 2rem;
		margin-bottom: 2rem;
	}

	.brand {
		font-family: var(--font-display);
		font-weight: 800;
		font-size: 1.3rem;
		letter-spacing: -0.04em;
		color: var(--sage-deep);
	}

	.brand-col p {
		margin: 0.5rem 0 0;
		max-width: 24rem;
	}

	.h {
		margin: 0 0 0.5rem;
		font-family: var(--font-mono);
		font-size: 0.7rem;
		letter-spacing: 0.1em;
		text-transform: uppercase;
		color: var(--signal);
	}

	.cols a {
		display: block;
		padding: 0.15rem 0;
		text-decoration: none;
		color: var(--ink-soft);
	}

	.cols a:hover {
		color: var(--sage);
	}

	.fine {
		margin: 0;
		font-size: 0.72rem;
		opacity: 0.8;
	}

	@media (max-width: 800px) {
		.cols {
			grid-template-columns: 1fr 1fr;
		}
	}
</style>
