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
			<p>Governed workflows, in Rust. Apache-2.0. Entasis Labs.</p>
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
			<a class="ext" href="https://github.com/EntasisLabs/grapheme/tree/main/examples" rel="noreferrer">Examples</a>
			<a class="ext" href="https://github.com/EntasisLabs/grapheme/tree/main/extensions/grapheme-vscode" rel="noreferrer">VS Code</a>
			<a class="ext" href="https://github.com/EntasisLabs/grapheme/blob/main/CHANGELOG.md" rel="noreferrer">Changelog</a>
		</div>
		<div>
			<p class="h">Design</p>
			<a class="ext" href="https://github.com/EntasisLabs/grapheme/tree/main/docs/internal/rfc" rel="noreferrer">RFCs</a>
			<a class="ext" href="https://github.com/EntasisLabs/grapheme/blob/main/docs/internal/architecture.md" rel="noreferrer">Architecture</a>
			<a class="ext" href="https://github.com/EntasisLabs/grapheme" rel="noreferrer">GitHub</a>
		</div>
	</div>
	<p class="fine">v0.7 · the playground runs grapheme-wasm in your browser; no server.</p>
</footer>

<style>
	main {
		min-height: calc(100vh - var(--nav-h) - 5rem);
	}

	.foot {
		max-width: 76rem;
		margin: 0 auto;
		padding: 2.5rem clamp(1rem, 4vw, 3rem) 2rem;
		border-top: 1px solid var(--line);
		color: var(--ink-soft);
		font-size: 0.9rem;
	}

	.cols {
		display: grid;
		grid-template-columns: 1.4fr 1fr 1fr 1fr;
		gap: 2rem;
		margin-bottom: 2rem;
	}

	.brand {
		font-weight: 600;
		font-size: 1rem;
		letter-spacing: -0.01em;
		color: var(--ink);
	}

	.brand-col p {
		margin: 0.5rem 0 0;
		max-width: 24rem;
	}

	.h {
		margin: 0 0 0.5rem;
		font-size: 0.72rem;
		font-weight: 500;
		letter-spacing: 0.06em;
		text-transform: uppercase;
		color: var(--ink-soft);
	}

	.cols a {
		display: block;
		padding: 0.15rem 0;
		text-decoration: none;
		color: var(--ink-soft);
	}

	.cols a:hover {
		color: var(--ink);
	}

	.fine {
		margin: 0;
		font-size: 0.78rem;
		opacity: 0.8;
	}

	@media (max-width: 800px) {
		.cols {
			grid-template-columns: 1fr 1fr;
		}
	}
</style>
