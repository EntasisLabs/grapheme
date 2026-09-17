<script lang="ts">
	import { resolve } from '$app/paths';
	import Seo from '$lib/components/Seo.svelte';
	import LiveRun from '$lib/components/LiveRun.svelte';
	import { HERO_SNIPPET, POLICY_SNIPPET } from '$lib/snippets';

	const install =
		'cargo install --git https://github.com/entasislabs/grapheme.git grapheme-cli --bin grapheme';
</script>

<Seo />

<section class="hero wrap">
	<h1>A typed workflow language with capability-gated side effects.</h1>
	<p class="lede">
		Compiles to a verified artifact. Runs one recorded step at a time. Calls only what the host
		has granted. Below, the real compiler and runtime in Wasm.
	</p>
	<div class="cta">
		<a class="btn" href={resolve('/playground')}>Open the playground</a>
		<a class="quiet" href={resolve('/docs/quickstart')}>Install the CLI →</a>
	</div>
</section>

<section class="demo wrap" aria-label="Live run">
	<LiveRun snippet={HERO_SNIPPET} file="release.gr" />
</section>

<section class="refusal wrap">
	<div class="refusal-copy">
		<p class="eyebrow">Fails closed</p>
		<h2>Call a module the host didn't grant, and the run stops there.</h2>
		<p>
			This host grants only the Wasm stdlib. <code>http.get</code> compiles, then is refused at
			step 01.
		</p>
	</div>
	<LiveRun snippet={POLICY_SNIPPET} file="needs-http.gr" autorun="visible" stack />
</section>

<section class="next wrap">
	<h2>Next: change <code>release.gr</code> and run it.</h2>
	<a class="btn" href={`${resolve('/playground')}?example=${HERO_SNIPPET.id}`}>Open the playground</a>
	<p class="install">
		CLI: <code>{install}</code>
		<a href={resolve('/docs/quickstart')}>Quickstart →</a>
	</p>
</section>

<style>
	.wrap {
		max-width: 76rem;
		margin: 0 auto;
		padding-left: clamp(1rem, 4vw, 3rem);
		padding-right: clamp(1rem, 4vw, 3rem);
	}

	code {
		font-size: 0.9em;
	}

	.eyebrow {
		margin: 0 0 0.8rem;
		font-size: 0.74rem;
		font-weight: 500;
		letter-spacing: 0.06em;
		text-transform: uppercase;
		color: var(--ink-soft);
	}

	h1,
	h2 {
		margin: 0;
		font-weight: 600;
		letter-spacing: -0.02em;
		line-height: 1.15;
		color: var(--ink);
	}

	h1 {
		font-size: clamp(1.75rem, 3vw, 2.5rem);
		max-width: 24ch;
	}

	h2 {
		font-size: clamp(1.25rem, 2vw, 1.6rem);
		max-width: 30ch;
	}

	.hero {
		padding-top: clamp(2.5rem, 7vh, 4.5rem);
		padding-bottom: clamp(1.5rem, 3.5vh, 2.25rem);
	}

	.lede {
		margin: 1.1rem 0 1.6rem;
		max-width: 58ch;
		font-size: 1.05rem;
		line-height: 1.55;
		color: var(--ink-soft);
	}

	.cta {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 0.75rem 1.4rem;
	}

	.btn {
		display: inline-flex;
		align-items: center;
		padding: 0.7rem 1.1rem;
		border-radius: var(--radius);
		background: var(--ink);
		color: var(--paper);
		font-weight: 500;
		font-size: 0.92rem;
		text-decoration: none;
	}

	.btn:hover {
		background: var(--sage-deep);
		color: var(--paper);
	}

	.quiet {
		font-weight: 500;
		font-size: 0.92rem;
		color: var(--ink-soft);
		text-decoration: none;
	}

	.quiet:hover {
		color: var(--ink);
	}

	.demo {
		padding-bottom: clamp(3rem, 8vh, 5.5rem);
	}

	.refusal {
		display: grid;
		grid-template-columns: minmax(0, 1fr) minmax(0, 1.3fr);
		gap: clamp(1.5rem, 4vw, 3.5rem);
		align-items: start;
		padding-top: clamp(2.5rem, 6vh, 4rem);
		padding-bottom: clamp(3rem, 8vh, 5.5rem);
		border-top: 1px solid var(--line);
	}

	.refusal-copy p:last-child {
		margin: 1rem 0 0;
		color: var(--ink-soft);
		max-width: 46ch;
	}

	.next {
		display: grid;
		gap: 1.3rem;
		justify-items: start;
		padding-top: clamp(2.5rem, 6vh, 4rem);
		padding-bottom: clamp(3rem, 8vh, 5.5rem);
		border-top: 1px solid var(--line);
	}

	.install {
		margin: 0;
		max-width: 100%;
		font-size: 0.85rem;
		color: var(--ink-soft);
		overflow-wrap: anywhere;
	}

	.install code {
		font-size: 0.86em;
		color: var(--ink);
	}

	.install a {
		margin-left: 0.4rem;
		color: var(--ink);
		text-decoration: none;
	}

	@media (max-width: 1000px) {
		.refusal {
			grid-template-columns: 1fr;
		}
	}

	@media (max-width: 600px) {
		.hero {
			padding-top: 1.75rem;
		}

		.lede {
			font-size: 1rem;
		}
	}
</style>
