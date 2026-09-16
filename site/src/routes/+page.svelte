<script lang="ts">
	import Seo from '$lib/components/Seo.svelte';
	import LiveRun from '$lib/components/LiveRun.svelte';
	import { HERO_SNIPPET, POLICY_SNIPPET } from '$lib/snippets';

	const install =
		'cargo install --git https://github.com/entasislabs/grapheme.git grapheme-cli --bin grapheme';
</script>

<Seo />

<section class="hero wrap">
	<p class="kicker mono">v0.7 · Apache-2.0 · Rust</p>
	<h1>
		A typed workflow language. Side effects are capabilities the host grants. Every step is
		recorded.
	</h1>
	<p class="lede">
		Grapheme compiles <code>.gr</code> programs to a verified artifact and executes them one step
		at a time. A step can only call a module the host has granted; the runtime refuses the rest.
		The compiler and runtime below are the real ones, built to Wasm and running in this tab.
	</p>
	<div class="cta">
		<a class="btn" href="/playground">Open the playground</a>
		<a class="quiet" href="/docs/quickstart">or install the CLI →</a>
	</div>
</section>

<section class="demo wrap" aria-label="Live demo">
	<LiveRun snippet={HERO_SNIPPET} file="release.gr" />
	<p class="caption mono">
		<code>struct Release</code> is the state. <code>@loop(max: 10)</code> is the budget. The right
		pane is the trace: one row per step, then the final state and the artifact id.
	</p>
</section>

<section class="refusal wrap">
	<div class="refusal-copy">
		<p class="eyebrow mono">Fails closed</p>
		<h2>Call a module the host did not grant, and the run stops at that step.</h2>
		<p>
			<code>http</code>, <code>sql</code>, <code>smtp</code>, secrets: each is a capability the
			host provides to a program, not something a program takes. In this tab the host grants only
			the Wasm stdlib (<code>core</code>, <code>json</code>, <code>csv</code>, <code>yaml</code>,
			<code>html</code>). This program compiles, gets an artifact id, and is refused at step 01
			with the runtime's own message.
		</p>
	</div>
	<LiveRun snippet={POLICY_SNIPPET} file="needs-http.gr" autorun="visible" stack />
</section>

<section class="next wrap">
	<h2>Next: change <code>release.gr</code> and run it.</h2>
	<a class="btn" href="/playground?example={HERO_SNIPPET.id}">Open the playground</a>
	<p class="install mono">
		Or on your machine: <code>{install}</code>
		<a href="/docs/quickstart">Quickstart →</a>
	</p>
</section>

<style>
	.wrap {
		max-width: 78rem;
		margin: 0 auto;
		padding-left: clamp(1rem, 4vw, 3rem);
		padding-right: clamp(1rem, 4vw, 3rem);
	}

	.mono {
		font-family: var(--font-mono);
	}

	code {
		font-size: 0.92em;
		color: var(--sage-deep);
	}

	.kicker,
	.eyebrow {
		margin: 0 0 1rem;
		font-size: 0.74rem;
		letter-spacing: 0.1em;
		text-transform: uppercase;
		color: var(--signal);
	}

	h1,
	h2 {
		margin: 0;
		font-family: var(--font-display);
		font-weight: 700;
		letter-spacing: -0.03em;
		line-height: 1.12;
		color: var(--sage-deep);
	}

	h1 {
		font-size: clamp(1.75rem, 3.3vw, 2.7rem);
		max-width: 30ch;
	}

	h2 {
		font-size: clamp(1.4rem, 2.4vw, 1.9rem);
		max-width: 30ch;
	}

	.hero {
		padding-top: clamp(2.5rem, 7vh, 5rem);
		padding-bottom: clamp(1.75rem, 4vh, 2.5rem);
	}

	.lede {
		margin: 1.2rem 0 1.6rem;
		max-width: 58ch;
		font-size: 1.08rem;
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
		padding: 0.85rem 1.3rem;
		border-radius: var(--radius);
		background: var(--sage);
		color: var(--mist);
		font-family: var(--font-display);
		font-weight: 700;
		text-decoration: none;
	}

	.btn:hover {
		background: var(--sage-deep);
		color: var(--mist);
	}

	.quiet {
		font-family: var(--font-display);
		font-weight: 600;
		font-size: 0.95rem;
		color: var(--sage-deep);
		text-decoration: none;
	}

	.quiet:hover {
		text-decoration: underline;
	}

	.demo {
		padding-bottom: clamp(3rem, 8vh, 5.5rem);
	}

	.caption {
		margin: 0.8rem 0 0;
		max-width: 70ch;
		font-size: 0.76rem;
		color: var(--ink-soft);
	}

	.caption code {
		font-size: 1em;
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
		max-width: 44ch;
	}

	.next {
		display: grid;
		gap: 1.4rem;
		justify-items: start;
		padding-top: clamp(2.5rem, 6vh, 4rem);
		padding-bottom: clamp(3rem, 8vh, 5.5rem);
		border-top: 1px solid var(--line);
	}

	.next h2 {
		max-width: 24ch;
	}

	.install {
		margin: 0;
		max-width: 100%;
		font-size: 0.76rem;
		color: var(--ink-soft);
		overflow-wrap: anywhere;
	}

	.install code {
		font-size: 1em;
	}

	.install a {
		margin-left: 0.4rem;
		color: var(--sage);
		text-decoration: none;
		font-weight: 500;
	}

	@media (max-width: 1000px) {
		.refusal {
			grid-template-columns: 1fr;
		}
	}

	@media (max-width: 600px) {
		.hero {
			padding-top: 1.8rem;
		}

		.lede {
			font-size: 1rem;
		}
	}
</style>
