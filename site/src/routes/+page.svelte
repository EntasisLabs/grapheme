<script lang="ts">
	import { onMount } from 'svelte';
	import Seo from '$lib/components/Seo.svelte';
	import LiveHero from '$lib/components/LiveHero.svelte';
	import LanguageTour from '$lib/components/LanguageTour.svelte';
	import CodeBlock from '$lib/components/CodeBlock.svelte';
	import { GITHUB_URL } from '$lib/site';

	let reveal = $state(false);
	onMount(() => requestAnimationFrame(() => (reveal = true)));

	const install = `cargo install --git https://github.com/entasislabs/grapheme.git grapheme-cli --bin grapheme

grapheme examples init --out .
grapheme run examples/hello-world.gr --json`;
</script>

<Seo />

<section class="hero" class:reveal>
	<div class="hero-copy">
		<p class="kicker mono">v0.7 · Apache-2.0 · Rust · Wasm</p>
		<h1>
			<span class="word">grapheme</span>
			<span class="line">A small language for<br />workflows that have to be right.</span>
		</h1>
		<p class="lede">Every side effect is a permission. Every step leaves a receipt.</p>
		<div class="cta">
			<a class="btn primary" href="/playground">Open playground</a>
			<a class="btn ghost" href="/docs/quickstart">Install in 2 minutes</a>
		</div>
		<p class="proof mono">↓ compiled &amp; executed by the real runtime, in Wasm, on this page</p>
	</div>

	<div class="hero-live">
		<LiveHero />
	</div>
</section>

<section class="band">
	<div class="band-head">
		<p class="eyebrow">Same runtime</p>
		<h2>Eight programs. All of them actually ran.</h2>
	</div>
	<LanguageTour />
</section>

<section class="band fit">
	<div class="band-head">
		<p class="eyebrow">Where it fits</p>
		<h2>For some jobs. Not all of them.</h2>
		<p class="support">
			Not <a href="https://temporal.io">Temporal</a>, not
			<a href="https://aws.amazon.com/step-functions/">Step Functions</a>, not
			<a href="https://dagster.io">Dagster</a>, and not a replacement for a script when a script is
			enough.
		</p>
	</div>
	<div class="fit-grid">
		<div class="fit-col is">
			<h3>Reach for it when</h3>
			<ul>
				<li>The workflow fits on one screen but would hurt if it went wrong.</li>
				<li>You need hosts, secrets, and databases on an allow-list — enforced, not hoped.</li>
			</ul>
		</div>
		<div class="fit-col isnt">
			<h3>Don't, yet, when</h3>
			<ul>
				<li>A job has to survive a crash over a weekend. There is no scheduler.</li>
				<li>You're building an application, a data platform, or a hosted control plane.</li>
			</ul>
		</div>
	</div>
</section>

<section class="band install-band">
	<div class="install">
		<div class="install-copy">
			<p class="eyebrow">Install</p>
			<h2>Two minutes on your machine.</h2>
		</div>
		<CodeBlock code={install} title="terminal" compact />
	</div>
</section>

<section class="close">
	<h2>Write the workflow you'd want to read at 3 a.m.</h2>
	<div class="cta">
		<a class="btn primary" href="/playground">Try it in the browser</a>
		<a class="btn ghost" href={GITHUB_URL}>Source on GitHub ↗</a>
	</div>
</section>

<style>
	.eyebrow {
		margin: 0 0 0.5rem;
		font-family: var(--font-mono);
		font-size: 0.74rem;
		letter-spacing: 0.1em;
		text-transform: uppercase;
		color: var(--signal);
	}

	h2 {
		margin: 0;
		font-family: var(--font-display);
		font-weight: 700;
		font-size: clamp(1.6rem, 3vw, 2.3rem);
		letter-spacing: -0.03em;
		line-height: 1.15;
		color: var(--sage-deep);
		max-width: 26ch;
	}

	.support {
		margin: 0.8rem 0 0;
		max-width: 44rem;
		color: var(--ink-soft);
	}

	.band {
		padding: clamp(2.5rem, 6vh, 4.25rem) clamp(1rem, 4vw, 3rem);
		border-top: 1px solid var(--line);
	}

	.band-head {
		margin-bottom: 1.6rem;
	}

	.btn {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		padding: 0.85rem 1.3rem;
		border-radius: var(--radius);
		font-family: var(--font-display);
		font-weight: 700;
		text-decoration: none;
		transition:
			transform 160ms ease,
			background 160ms ease,
			border-color 160ms ease;
	}

	.btn.primary {
		background: var(--sage);
		color: var(--mist);
	}

	.btn.primary:hover {
		background: var(--sage-deep);
		transform: translateY(-1px);
	}

	.btn.ghost {
		border: 1px solid color-mix(in srgb, var(--sage) 45%, transparent);
		color: var(--sage-deep);
		background: color-mix(in srgb, var(--mist) 60%, transparent);
	}

	.btn.ghost:hover {
		border-color: var(--sage);
	}

	.cta {
		display: flex;
		flex-wrap: wrap;
		gap: 0.75rem;
	}

	.hero {
		display: grid;
		grid-template-columns: minmax(0, 0.8fr) minmax(0, 1.2fr);
		gap: clamp(1.5rem, 4vw, 3.5rem);
		align-items: center;
		padding: clamp(2rem, 6vh, 4.5rem) clamp(1rem, 4vw, 3rem) clamp(2.5rem, 7vh, 4.5rem);
		overflow: hidden;
		opacity: 0;
		transform: translateY(10px);
		transition:
			opacity 700ms ease,
			transform 700ms ease;
	}

	.hero.reveal {
		opacity: 1;
		transform: none;
	}

	.kicker {
		margin: 0 0 1rem;
		font-size: 0.74rem;
		letter-spacing: 0.1em;
		text-transform: uppercase;
		color: var(--signal);
	}

	h1 {
		margin: 0;
		font-family: var(--font-display);
	}

	.hero-copy {
		min-width: 0;
		container-type: inline-size;
	}

	.word {
		display: block;
		font-weight: 800;
		font-size: clamp(2.6rem, 14.5cqw, 5.4rem);
		line-height: 0.9;
		letter-spacing: -0.06em;
		color: var(--sage-deep);
		margin-bottom: 1rem;
	}

	.line {
		display: block;
		font-weight: 700;
		font-size: clamp(1.3rem, 2.4vw, 1.9rem);
		line-height: 1.2;
		letter-spacing: -0.025em;
		color: var(--ink);
	}

	.lede {
		margin: 1.2rem 0 1.6rem;
		max-width: 34rem;
		font-size: 1.08rem;
		color: var(--ink-soft);
	}

	.proof {
		margin: 1.4rem 0 0;
		font-size: 0.74rem;
		color: var(--signal);
	}

	.hero-live {
		min-width: 0;
	}

	.fit .support a {
		color: var(--sage-deep);
	}

	.fit-grid {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 1.5rem;
	}

	.fit-col {
		padding: 1.4rem 1.5rem 1.5rem;
		border: 1px solid var(--line);
		background: color-mix(in srgb, var(--mist) 85%, white);
	}

	.fit-col.is {
		border-top: 3px solid var(--sage);
	}

	.fit-col.isnt {
		border-top: 3px solid var(--ember);
	}

	.fit-col h3 {
		margin: 0 0 0.9rem;
		font-family: var(--font-display);
		font-size: 1.15rem;
		color: var(--sage-deep);
	}

	.fit-col ul {
		margin: 0;
		padding: 0;
		list-style: none;
		display: grid;
		gap: 0.6rem;
	}

	.fit-col li {
		padding-left: 1.1rem;
		position: relative;
		color: var(--ink-soft);
		font-size: 0.98rem;
	}

	.fit-col li::before {
		content: '';
		position: absolute;
		left: 0;
		top: 0.6em;
		width: 0.45rem;
		height: 0.45rem;
		background: var(--sage);
	}

	.fit-col.isnt li::before {
		background: var(--ember);
	}

	.install {
		display: grid;
		grid-template-columns: 0.9fr 1.1fr;
		gap: 2rem;
		align-items: center;
	}

	.close {
		padding: clamp(2.5rem, 7vh, 4.5rem) clamp(1rem, 4vw, 3rem);
		border-top: 1px solid var(--line);
		display: grid;
		gap: 1.5rem;
		justify-items: start;
	}

	.close h2 {
		max-width: 22ch;
		font-size: clamp(1.8rem, 3.6vw, 2.8rem);
	}

	@media (max-width: 1280px) {
		.hero {
			grid-template-columns: 1fr;
			align-items: start;
		}

		.word {
			font-size: clamp(2.6rem, 12cqw, 5.4rem);
		}
	}

	@media (max-width: 1000px) {
		.fit-grid,
		.install {
			grid-template-columns: 1fr;
		}
	}

	@media (max-width: 600px) {
		.hero {
			padding-top: 1.6rem;
		}

		.word {
			font-size: clamp(1.9rem, 9cqw, 2.5rem);
			margin-bottom: 0.7rem;
		}

		.line {
			font-size: 1.2rem;
		}

		.lede {
			font-size: 1rem;
		}
	}
</style>
