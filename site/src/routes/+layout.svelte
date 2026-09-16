<script lang="ts">
	import '../app.css';
	import SiteNav from '$lib/components/SiteNav.svelte';

	let { children } = $props();
</script>

<svelte:head>
	<title>Grapheme</title>
</svelte:head>

<SiteNav />
<main>
	{@render children()}
</main>

<footer class="foot">
	<div>
		<span class="brand">grapheme</span>
		<span class="sep">·</span>
		<span>governed automation, explicit by design</span>
	</div>
	<div class="links">
		<a href="/docs/why-grapheme">Docs</a>
		<a href="/playground">Playground</a>
		<a href="https://github.com/EntasisLabs/grapheme">GitHub</a>
	</div>
</footer>

<style>
	main {
		min-height: calc(100vh - var(--nav-h) - 5rem);
	}

	.foot {
		display: flex;
		flex-wrap: wrap;
		justify-content: space-between;
		gap: 1rem;
		padding: 1.5rem clamp(1rem, 3vw, 2.5rem) 2rem;
		border-top: 1px solid var(--line);
		font-size: 0.92rem;
		color: var(--ink-soft);
	}

	.brand {
		font-family: var(--font-display);
		font-weight: 700;
		color: var(--ink);
	}

	.sep {
		margin: 0 0.4rem;
		opacity: 0.5;
	}

	.links {
		display: flex;
		gap: 1.2rem;
		font-family: var(--font-display);
		font-weight: 600;
	}

	.links a {
		text-decoration: none;
	}
</style>
