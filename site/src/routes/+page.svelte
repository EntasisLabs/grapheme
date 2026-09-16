<script lang="ts">
	import { onMount } from 'svelte';
	import Seo from '$lib/components/Seo.svelte';
	import LiveHero from '$lib/components/LiveHero.svelte';
	import LanguageTour from '$lib/components/LanguageTour.svelte';
	import CodeBlock from '$lib/components/CodeBlock.svelte';
	import { GITHUB_URL } from '$lib/site';

	let reveal = $state(false);
	onMount(() => requestAnimationFrame(() => (reveal = true)));

	const wasmSafe = ['core', 'json', 'csv', 'yaml', 'html'];
	const hostOnly = ['http', 'sql', 'smtp', 'secrets', 'tcp', 'memory', 'data', 'pdf', 'image', 'plot', 'media'];

	const install = `cargo install --git https://github.com/entasislabs/grapheme.git grapheme-cli --bin grapheme

grapheme examples init --out .
grapheme run examples/hello-world.gr --json`;

	const policy = `GRAPHEME_ALLOWED_HTTP_DOMAINS=api.internal \\
GRAPHEME_ALLOWED_SECRETS=deploy-token \\
  grapheme run release.gr --native-modules --json`;
</script>

<Seo />

<!-- ───────────────────────── HERO ───────────────────────── -->
<section class="hero" class:reveal>
	<div class="hero-copy">
		<p class="kicker mono">v0.7 · Apache-2.0 · Rust · Wasm</p>
		<h1>
			<span class="word">grapheme</span>
			<span class="line">A small language for<br />workflows that have to be right.</span>
		</h1>
		<p class="lede">
			Write the steps. Grapheme checks the types, keeps every side effect behind a permission you
			grant, and records what happened. It compiles once and runs on a server, at the edge, or in
			your browser.
		</p>
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

<!-- ───────────────────────── USE CASE ───────────────────────── -->
<section class="usecase">
	<div class="uc-head">
		<p class="eyebrow">What that program above is doing</p>
		<h2>A deployment rollout, written the way you'd explain it.</h2>
	</div>
	<ol class="uc-steps">
		<li>
			<span class="n">1</span>
			<div>
				<h3>Say what the state looks like</h3>
				<p>
					<code>struct Release</code> declares a service, a step counter, and a status. Misspell a
					field anywhere and the compiler stops you before anything runs.
				</p>
			</div>
		</li>
		<li>
			<span class="n">2</span>
			<div>
				<h3>Loop with a budget, branch on status</h3>
				<p>
					<code>Ramp</code> runs at most ten times (<code>@loop(max: 10)</code>) and reads like a
					checklist: if we're <code>complete</code>, stop; otherwise advance. No runaway loops, no
					hidden goto.
				</p>
			</div>
		</li>
		<li>
			<span class="n">3</span>
			<div>
				<h3>Get a receipt</h3>
				<p>
					Every step lands in a trace — who ran, which operation, what came out. The panel on the
					right of the hero is that trace. Swap the counter for a canary health check and an
					<code>http</code> call and you have a real release gate.
				</p>
			</div>
		</li>
	</ol>
	<p class="uc-foot">
		The same shape fits a data clean-up job, an approval flow, or an agent that must not call the
		outside world without permission.
	</p>
</section>

<!-- ───────────────────────── TOUR ───────────────────────── -->
<section class="band">
	<div class="band-head">
		<p class="eyebrow">Language at a glance</p>
		<h2>Eight small programs. All of them actually ran.</h2>
		<p class="support">
			Outputs below are the real results from <code>grapheme-wasm</code>. Open any of them in the
			playground and break it.
		</p>
	</div>
	<LanguageTour />
</section>

<!-- ───────────────────────── FIT ───────────────────────── -->
<section class="band fit">
	<div class="band-head">
		<p class="eyebrow">Where it fits</p>
		<h2>Grapheme is for some jobs. Not all of them.</h2>
		<p class="support">
			Not <a href="https://temporal.io">Temporal</a> — there is no durable execution. Not
			<a href="https://aws.amazon.com/step-functions/">Step Functions</a> — not a cloud state machine.
			Not <a href="https://dagster.io">Dagster</a> — not a data platform. And not a replacement for
			ordinary Python or TypeScript when a script is actually enough.
		</p>
	</div>
	<div class="fit-grid">
		<div class="fit-col is">
			<h3>Reach for it when</h3>
			<ul>
				<li>The workflow is short enough to read in one sitting but important enough to audit.</li>
				<li>You need to say which hosts, secrets, and databases a run may touch — and have that enforced.</li>
				<li>Humans write it, agents run it, or the other way round. Same source either way.</li>
				<li>You want it to run without a control plane: a CLI, a container, an edge function, a browser tab.</li>
			</ul>
		</div>
		<div class="fit-col isnt">
			<h3>Don't, yet, when</h3>
			<ul>
				<li>You need durable execution across process crashes over days. There is no persistence layer or scheduler.</li>
				<li>You're building a general application. It's a workflow language, not a replacement for your stack.</li>
				<li>Your problem is a data platform with lineage, backfills, and hundreds of assets.</li>
				<li>You need a hosted UI, alerting, or a marketplace of integrations today.</li>
			</ul>
		</div>
	</div>
</section>

<!-- ───────────────────────── UNDER THE HOOD ───────────────────────── -->
<section class="band hood">
	<div class="band-head">
		<p class="eyebrow">Under the hood</p>
		<h2>Modules are the standard library. Policy is the perimeter.</h2>
		<p class="support">
			Source is parsed, type-checked, and lowered to a verified artifact; one runtime executes it
			natively, inside WASI, or in the browser. Pure transforms run anywhere. Anything that touches
			the outside world is a host capability behind an explicit allow-list.
		</p>
	</div>
	<div class="caps-strip">
		<div class="caps-row">
			<span class="badge ok">wasm-safe</span>
			<span class="mods">{#each wasmSafe as m, i}<code>{m}</code>{#if i < wasmSafe.length - 1}<i>·</i>{/if}{/each}</span>
			<span class="caps-note">pure transforms — run in this browser tab</span>
		</div>
		<div class="caps-row">
			<span class="badge host">host</span>
			<span class="mods">{#each hostOnly as m, i}<code>{m}</code>{#if i < hostOnly.length - 1}<i>·</i>{/if}{/each}</span>
			<span class="caps-note">side effects — refused unless <code>GRAPHEME_ALLOWED_*</code> says otherwise</span>
		</div>
	</div>
	<div class="policy">
		<div>
			<h3>Grant, don't hope.</h3>
			<p>
				Policy lives outside the source. The same <code>release.gr</code> runs in CI with one
				perimeter and in production with another — no code change.
			</p>
		</div>
		<CodeBlock code={policy} title="shell" compact />
	</div>
</section>

<!-- ───────────────────────── NUMBERS + INSTALL ───────────────────────── -->
<section class="band numbers">
	<div class="band-head">
		<p class="eyebrow">By the numbers</p>
		<h2>Early, and measurable.</h2>
		<p class="support">
			No adopter logos yet. These are the signals we can stand behind today, taken from the
			repository and from this page.
		</p>
	</div>
	<ul class="stats">
		<li>
			<strong>~16 ms</strong>
			<span>to compile, verify, and run a 5,000-step loop inside Wasm (Node 22, warm). The hero above reports its own time on every load.</span>
		</li>
		<li>
			<strong>47</strong>
			<span>example programs in <code>examples/</code>, from hello-world to Stage B containers.</span>
		</li>
		<li>
			<strong>10 crates</strong>
			<span>at 0.7.1 plus a VS Code extension, gated by a conformance workflow on every push.</span>
		</li>
		<li>
			<strong>3 targets</strong>
			<span>one runtime: native CLI, WASI module, and <code>wasm32-unknown-unknown</code> for embedding.</span>
		</li>
	</ul>
	<div class="install">
		<div class="install-copy">
			<h3>Install</h3>
			<p>
				CLI with <code>parse</code>, <code>compile</code>, <code>run</code>, and JSON output on
				everything. LSP and VS Code extension ship with each release. Rust SDK for embedding,
				including a slim profile for iOS and Wasm.
			</p>
		</div>
		<CodeBlock code={install} title="terminal" compact />
	</div>
</section>

<!-- ───────────────────────── CLOSE ───────────────────────── -->
<section class="close">
	<h2>Write the workflow you'd want to read at 3 a.m.</h2>
	<div class="cta">
		<a class="btn primary" href="/playground">Try it in the browser</a>
		<a class="btn ghost" href="/docs/why-grapheme">Why Grapheme</a>
		<a class="btn ghost" href={GITHUB_URL}>Source on GitHub ↗</a>
	</div>
</section>

<style>
	/* shared */
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

	/* hero */
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
		/* Syne 800 'grapheme' is ~6.4em wide; keep it inside the column. */
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

	/* use case */
	.usecase {
		padding: clamp(2.5rem, 6vh, 4.25rem) clamp(1rem, 4vw, 3rem);
		background: var(--sage-deep);
		color: var(--mist);
	}

	.usecase .eyebrow {
		color: var(--signal-bright);
	}

	.usecase h2 {
		color: var(--mist);
		max-width: 24ch;
	}

	.uc-head {
		margin-bottom: 2rem;
	}

	.uc-steps {
		list-style: none;
		margin: 0;
		padding: 0;
		display: grid;
		grid-template-columns: repeat(3, 1fr);
		gap: 2rem;
	}

	.uc-steps li {
		display: grid;
		grid-template-columns: 2.2rem 1fr;
		gap: 0.9rem;
		align-items: start;
	}

	.uc-steps .n {
		display: grid;
		place-items: center;
		width: 2.2rem;
		height: 2.2rem;
		border: 1px solid var(--signal-bright);
		color: var(--signal-bright);
		font-family: var(--font-mono);
		font-size: 0.85rem;
	}

	.uc-steps h3 {
		margin: 0.2rem 0 0.45rem;
		font-family: var(--font-display);
		font-size: 1.1rem;
		letter-spacing: -0.01em;
	}

	.uc-steps p {
		margin: 0;
		color: color-mix(in srgb, var(--mist) 78%, transparent);
		font-size: 0.98rem;
	}

	.usecase code {
		font-family: var(--font-mono);
		font-size: 0.86em;
		color: var(--signal-bright);
	}

	.uc-foot {
		margin: 1.8rem 0 0;
		max-width: 46rem;
		padding-left: 1rem;
		border-left: 2px solid var(--signal-bright);
		color: color-mix(in srgb, var(--mist) 85%, transparent);
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

	/* hood */
	.caps-strip {
		display: grid;
		gap: 1px;
		background: var(--line);
		border: 1px solid var(--line);
		margin-bottom: 2rem;
	}

	.caps-row {
		display: grid;
		grid-template-columns: 6.5rem 1fr auto;
		gap: 1rem;
		align-items: center;
		padding: 0.75rem 1rem;
		background: color-mix(in srgb, var(--mist) 85%, white);
	}

	.mods {
		display: flex;
		flex-wrap: wrap;
		gap: 0.25rem 0.4rem;
		font-family: var(--font-mono);
		font-size: 0.86rem;
		color: var(--sage-deep);
	}

	.mods i {
		color: var(--ink-soft);
		opacity: 0.6;
		font-style: normal;
	}

	.caps-note {
		font-family: var(--font-mono);
		font-size: 0.72rem;
		color: var(--ink-soft);
		text-align: right;
	}

	.caps-note code {
		color: var(--ember);
	}

	.badge {
		justify-self: start;
		padding: 0.15rem 0.5rem;
		font-size: 0.68rem;
		letter-spacing: 0.06em;
		text-transform: uppercase;
	}

	.badge.ok {
		background: var(--sage);
		color: var(--mist);
	}

	.badge.host {
		background: color-mix(in srgb, var(--ember) 15%, var(--mist));
		color: var(--ember);
		border: 1px solid color-mix(in srgb, var(--ember) 40%, transparent);
	}

	.policy {
		display: grid;
		grid-template-columns: 0.9fr 1.1fr;
		gap: 2rem;
		align-items: center;
	}

	.policy h3,
	.install-copy h3 {
		margin: 0 0 0.5rem;
		font-family: var(--font-display);
		font-size: 1.4rem;
		color: var(--sage-deep);
	}

	.policy p,
	.install-copy p {
		margin: 0;
		color: var(--ink-soft);
	}

	/* numbers */
	.stats {
		list-style: none;
		margin: 0 0 2rem;
		padding: 0;
		display: grid;
		grid-template-columns: repeat(4, 1fr);
		gap: 1.5rem;
	}

	.stats li {
		display: grid;
		gap: 0.4rem;
		padding-top: 0.9rem;
		border-top: 2px solid var(--sage);
	}

	.stats strong {
		font-family: var(--font-display);
		font-weight: 800;
		font-size: 2rem;
		letter-spacing: -0.04em;
		line-height: 1;
		color: var(--sage-deep);
	}

	.stats span {
		font-size: 0.92rem;
		color: var(--ink-soft);
	}

	.install {
		display: grid;
		grid-template-columns: 0.9fr 1.1fr;
		gap: 2rem;
		align-items: center;
	}

	/* close */
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
		.policy,
		.install {
			grid-template-columns: 1fr;
		}

		.caps-row {
			grid-template-columns: 6.5rem 1fr;
		}

		.caps-note {
			grid-column: 2;
			text-align: left;
		}

		.uc-steps,
		.stats {
			grid-template-columns: repeat(2, 1fr);
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

		.uc-steps,
		.stats {
			grid-template-columns: 1fr;
		}

		.caps-row {
			grid-template-columns: 1fr;
		}

		.caps-note {
			grid-column: 1;
		}
	}
</style>
