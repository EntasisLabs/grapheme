<script lang="ts">
	import { onMount } from 'svelte';

	let reveal = $state(false);
	onMount(() => {
		requestAnimationFrame(() => {
			reveal = true;
		});
	});
</script>

<svelte:head>
	<title>Grapheme — governed automation workflows</title>
	<meta
		name="description"
		content="Write workflows in .gr, compile into verified MIR, and run with policy controls. Now with an in-browser WASM playground."
	/>
</svelte:head>

<section class="hero" class:reveal aria-label="Grapheme">
	<div class="hero-copy">
		<p class="brand-lockup">grapheme</p>
		<h1>Workflows you can read. Runtimes you can trust.</h1>
		<p class="lede">
			A language and runtime for governed automation — explicit control flow, typed transitions, and
			capability-aware execution. Compile. Verify. Run — including in Wasm.
		</p>
		<div class="cta">
			<a class="primary" href="/playground">Open playground</a>
			<a class="ghost" href="/docs/quickstart">Read the docs</a>
		</div>
	</div>

	<div class="hero-visual" aria-hidden="true">
		<div class="glyph-field">
			{#each Array.from({ length: 24 }, (_, i) => i) as i}
				<span class="glyph" style={`--i:${i}`}>{['⟨', '⟩', '·', '→', '⊢', '⚙'][i % 6]}</span>
			{/each}
		</div>
		<pre class="snippet"><code>{`query Rollout on FlagState {
  FlagState { status: "planned", rollout: 0.0 }
  |> Run
}`}</code></pre>
	</div>
</section>

<section class="band">
	<h2>Built for production automation</h2>
	<p class="support">
		Intent stays visible in source. Side effects stay policy-scoped. Humans and agents share one
		truth.
	</p>
	<ul class="pillars">
		<li>
			<strong>Language</strong>
			<span>Queries, iterators, params, tags, and explicit transitions — not glue scripts.</span>
		</li>
		<li>
			<strong>Runtime</strong>
			<span>Verified MIR artifacts with allow-listed HTTP, SQL, SMTP, secrets, and more.</span>
		</li>
		<li>
			<strong>Wasm</strong>
			<span>Runtime-in-Wasm (RFC-0006) for edge hosts and the in-browser playground.</span>
		</li>
	</ul>
</section>

<section class="band split">
	<div>
		<h2>Try it without a server</h2>
		<p class="support">
			The playground ships the Grapheme compiler + runtime as a WASI module and executes
			<code>.gr</code> source entirely in your browser.
		</p>
		<a class="primary" href="/playground">Launch playground</a>
	</div>
	<div class="panel">
		<p class="mono label">wasm-safe stdlib</p>
		<ul>
			<li>core · json · csv · yaml · html</li>
			<li>host ops fail closed by design</li>
			<li>~3.6MB release artifact</li>
		</ul>
	</div>
</section>

<style>
	.hero {
		display: grid;
		grid-template-columns: 1.1fr 0.9fr;
		gap: clamp(1.5rem, 4vw, 3.5rem);
		align-items: end;
		min-height: calc(100vh - var(--nav-h));
		padding: clamp(2rem, 6vh, 4rem) clamp(1rem, 3vw, 2.5rem) clamp(2.5rem, 6vh, 4rem);
		opacity: 0;
		transform: translateY(12px);
		transition:
			opacity 700ms ease,
			transform 700ms ease;
	}

	.hero.reveal {
		opacity: 1;
		transform: none;
	}

	.brand-lockup {
		margin: 0 0 0.85rem;
		font-family: var(--font-display);
		font-weight: 800;
		font-size: clamp(3.2rem, 9vw, 6.5rem);
		line-height: 0.9;
		letter-spacing: -0.06em;
		color: var(--ink);
	}

	h1 {
		margin: 0;
		max-width: 14ch;
		font-family: var(--font-display);
		font-weight: 700;
		font-size: clamp(1.55rem, 3.2vw, 2.35rem);
		letter-spacing: -0.03em;
		line-height: 1.15;
	}

	.lede {
		margin: 1.1rem 0 1.6rem;
		max-width: 38rem;
		font-size: 1.12rem;
		color: var(--ink-soft);
	}

	.cta {
		display: flex;
		flex-wrap: wrap;
		gap: 0.75rem;
	}

	.primary,
	.ghost {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		padding: 0.8rem 1.2rem;
		border-radius: var(--radius);
		font-family: var(--font-display);
		font-weight: 700;
		text-decoration: none;
		transition:
			transform 180ms ease,
			background 180ms ease,
			color 180ms ease;
	}

	.primary {
		background: var(--sage-deep);
		color: var(--mist);
	}

	.primary:hover {
		background: var(--sage);
		color: var(--mist);
		transform: translateY(-1px);
	}

	.ghost {
		border: 1px solid var(--line);
		background: color-mix(in srgb, var(--mist) 70%, transparent);
	}

	.ghost:hover {
		border-color: var(--ink);
		color: var(--ink);
	}

	.hero-visual {
		position: relative;
		min-height: 22rem;
		border: 1px solid var(--line);
		background:
			linear-gradient(145deg, var(--sage), var(--sage-deep)),
			var(--sage-deep);
		color: var(--mist);
		overflow: hidden;
		box-shadow: 0 24px 60px var(--shadow);
	}

	.glyph-field {
		position: absolute;
		inset: 0;
		display: grid;
		grid-template-columns: repeat(6, 1fr);
		opacity: 0.22;
		pointer-events: none;
	}

	.glyph {
		display: grid;
		place-items: center;
		font-family: var(--font-mono);
		font-size: 1.1rem;
		animation: drift 8s ease-in-out infinite;
		animation-delay: calc(var(--i) * 120ms);
	}

	@keyframes drift {
		0%,
		100% {
			transform: translateY(0);
			opacity: 0.35;
		}
		50% {
			transform: translateY(-8px);
			opacity: 0.9;
		}
	}

	.snippet {
		position: relative;
		z-index: 1;
		margin: 0;
		padding: 1.6rem 1.4rem;
		height: 100%;
		display: flex;
		align-items: flex-end;
		font-size: 0.86rem;
		line-height: 1.5;
		background: linear-gradient(180deg, transparent 10%, color-mix(in srgb, var(--sage-deep) 78%, transparent));
	}

	.snippet code {
		white-space: pre-wrap;
	}

	.band {
		padding: clamp(3rem, 8vh, 5rem) clamp(1rem, 3vw, 2.5rem);
		border-top: 1px solid var(--line);
	}

	.band h2 {
		margin: 0;
		font-family: var(--font-display);
		font-size: clamp(1.6rem, 3vw, 2.2rem);
		letter-spacing: -0.03em;
	}

	.support {
		margin: 0.75rem 0 1.75rem;
		max-width: 40rem;
		color: var(--ink-soft);
	}

	.pillars {
		list-style: none;
		margin: 0;
		padding: 0;
		display: grid;
		grid-template-columns: repeat(3, 1fr);
		gap: 1.5rem;
	}

	.pillars li {
		display: grid;
		gap: 0.45rem;
		padding-top: 1rem;
		border-top: 2px solid var(--sage);
	}

	.pillars strong {
		font-family: var(--font-display);
		font-size: 1.15rem;
	}

	.pillars span {
		color: var(--ink-soft);
		font-size: 0.98rem;
	}

	.split {
		display: grid;
		grid-template-columns: 1.2fr 0.8fr;
		gap: 2rem;
		align-items: center;
	}

	.panel {
		padding: 1.4rem 1.3rem;
		border: 1px solid var(--line);
		background: color-mix(in srgb, var(--mist) 80%, white);
	}

	.panel .label {
		margin: 0 0 0.75rem;
		font-size: 0.78rem;
		letter-spacing: 0.08em;
		text-transform: uppercase;
		color: var(--signal);
	}

	.panel ul {
		margin: 0;
		padding-left: 1.1rem;
		color: var(--ink-soft);
	}

	@media (max-width: 900px) {
		.hero,
		.split,
		.pillars {
			grid-template-columns: 1fr;
		}

		.hero {
			min-height: auto;
			align-items: start;
		}

		.hero-visual {
			min-height: 16rem;
		}
	}
</style>
