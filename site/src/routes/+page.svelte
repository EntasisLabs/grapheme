<script lang="ts">
	import { onMount } from 'svelte';
	import LiveHero from '$lib/components/LiveHero.svelte';
	import LanguageTour from '$lib/components/LanguageTour.svelte';
	import CodeBlock from '$lib/components/CodeBlock.svelte';

	let reveal = $state(false);
	onMount(() => requestAnimationFrame(() => (reveal = true)));

	const wasmSafe = [
		['core', 'set · pick · merge · filter · map · reduce · group_by · validate_schema · strings · paths'],
		['json', 'parse'],
		['csv', 'to_list'],
		['yaml', 'to_json'],
		['html', 'to_md · clean_text']
	];

	const hostOnly = [
		['http', 'get · post', 'GRAPHEME_ALLOWED_HTTP_DOMAINS'],
		['sql', 'query · execute · transaction · health', ''],
		['smtp', 'send_mail', 'GRAPHEME_ALLOWED_SMTP_DOMAINS'],
		['secrets', 'handle · sign', 'GRAPHEME_ALLOWED_SECRETS'],
		['tcp', 'connect · send · receive', 'GRAPHEME_ALLOWED_TCP_TARGETS'],
		['memory', 'roundtrip state across runs', ''],
		['data', 'read_csv · filter · group_by · aggregate (Polars)', ''],
		['pdf · image · plot', 'Wasm capability plugins', ''],
		['media', 'probe · transcode (ffmpeg)', '']
	];

	const install = `# install the CLI
cargo install --git https://github.com/entasislabs/grapheme.git grapheme-cli --bin grapheme

# scaffold examples and run one
grapheme examples init --out .
grapheme run examples/hello-world.gr --json

# pass parameters
grapheme run examples/params-call-bind.gr --args-json '{"label":"grapheme"}'`;

	const policy = `GRAPHEME_ALLOWED_HTTP_DOMAINS=api.internal \\
GRAPHEME_ALLOWED_SECRETS=deploy-token \\
  grapheme run release.gr --native-modules --json`;
</script>

<svelte:head>
	<title>Grapheme — a language for governed automation</title>
	<meta
		name="description"
		content="Grapheme is a small, explicit language for workflows that matter: typed state, visible control flow, capability-scoped side effects. Compiles to verified MIR. Runs native, in WASI, or in your browser."
	/>
</svelte:head>

<!-- ───────────────────────── HERO ───────────────────────── -->
<section class="hero" class:reveal>
	<div class="hero-copy">
		<p class="kicker mono">v0.7 · Apache-2.0 · Rust · Wasm</p>
		<h1>
			<span class="word">grapheme</span>
			<span class="line">A small language for<br />workflows that have to be right.</span>
		</h1>
		<p class="lede">
			Typed state. Visible control flow. Side effects as capabilities you grant, not defaults you
			forget. Source compiles to a verified artifact and runs natively, in WASI, or right here in
			your browser.
		</p>
		<div class="cta">
			<a class="btn primary" href="/playground">Open playground</a>
			<a class="btn ghost" href="/docs/quickstart">Install in 2 minutes</a>
		</div>
		<p class="proof mono">
			↓ this program is compiled &amp; executed by the real runtime, in Wasm, on this page
		</p>
	</div>

	<div class="hero-live">
		<LiveHero />
	</div>
</section>

<!-- ───────────────────────── THESIS ───────────────────────── -->
<section class="thesis">
	<div class="thesis-grid">
		<div>
			<p class="eyebrow">The bet</p>
			<h2>Automation should not become unreadable the moment it becomes important.</h2>
		</div>
		<div class="thesis-copy">
			<p>
				Most teams pick a failure mode: fast scripts with no control plane, or rigid platforms that
				punish iteration. Grapheme rejects both. Workflow logic stays compact and expressive
				<em>and</em> governed in production.
			</p>
			<ul class="principles">
				<li><strong>Intent is visible in source.</strong> If you can't read it, you can't trust it.</li>
				<li><strong>Side effects are governable.</strong> Network, data, secrets — all policy-scoped.</li>
				<li><strong>Humans and agents share one truth.</strong> The same source works for both.</li>
				<li><strong>Composition beats glue.</strong> Capability modules with contracts, not script sprawl.</li>
			</ul>
		</div>
	</div>
</section>

<!-- ───────────────────────── TOUR ───────────────────────── -->
<section class="band">
	<div class="band-head">
		<p class="eyebrow">Language at a glance</p>
		<h2>Read it like a runbook. Run it like a program.</h2>
		<p class="support">
			Every example below was executed by <code>grapheme-wasm</code>; the output shown is the real
			final state. Click through, then open one in the playground and break it.
		</p>
	</div>
	<LanguageTour />
</section>

<!-- ───────────────────────── PIPELINE ───────────────────────── -->
<section class="band pipeline">
	<div class="band-head">
		<p class="eyebrow">How it runs</p>
		<h2>Compile once. Verify. Run anywhere the runtime fits.</h2>
	</div>
	<ol class="stages">
		<li>
			<span class="n">01</span>
			<h3>Parse</h3>
			<p>A PEG grammar (<code>grapheme.pest</code>) turns <code>.gr</code> into an AST. Comments, params, tags, directives.</p>
			<code class="crate">grapheme-compiler</code>
		</li>
		<li>
			<span class="n">02</span>
			<h3>Lower &amp; verify</h3>
			<p>AST → HIR → MIR. Type checks on struct fields, entrypoint resolution, capability lints, loop budgets.</p>
			<code class="crate">verifier · mir_lower</code>
		</li>
		<li>
			<span class="n">03</span>
			<h3>Artifact</h3>
			<p>A content-addressed envelope (<code>gph-…</code>) with MIR and metadata. Ship it, sign it, diff it.</p>
			<code class="crate">grapheme-artifact</code>
		</li>
		<li>
			<span class="n">04</span>
			<h3>Execute</h3>
			<p>The <code>RuntimeEngine</code> walks MIR with policy, step budgets, and a full trace of every op.</p>
			<code class="crate">grapheme-runtime</code>
		</li>
	</ol>
	<div class="targets">
		<div>
			<h4>Native host</h4>
			<p>Full stdlib, Wasix plugins, hotload, LSP. The <code>grapheme</code> CLI.</p>
		</div>
		<div>
			<h4>Runtime-in-Wasm</h4>
			<p>Compiler + runtime as one WASI module. Browser, edge, embedded. <em>This page.</em></p>
		</div>
		<div>
			<h4>Stage B container</h4>
			<p>AOT workflow container; host fulfils capabilities across rounds. Stage A parity.</p>
		</div>
	</div>
</section>

<!-- ───────────────────────── CAPABILITIES ───────────────────────── -->
<section class="band caps">
	<div class="band-head">
		<p class="eyebrow">Capabilities</p>
		<h2>Modules are the standard library. Policy is the perimeter.</h2>
		<p class="support">
			Pure transforms run anywhere — including inside Wasm. Anything that touches the outside world
			is a host capability behind an explicit allow-list.
		</p>
	</div>
	<div class="caps-grid">
		<div class="caps-col">
			<div class="caps-title">
				<span class="badge ok">wasm-safe</span>
				runs in the browser playground
			</div>
			<ul>
				{#each wasmSafe as [m, ops]}
					<li><code>{m}</code><span>{ops}</span></li>
				{/each}
			</ul>
		</div>
		<div class="caps-col">
			<div class="caps-title">
				<span class="badge host">host</span>
				fails closed without policy
			</div>
			<ul>
				{#each hostOnly as [m, ops, env]}
					<li>
						<code>{m}</code>
						<span>{ops}</span>
						{#if env}<em class="env">{env}</em>{/if}
					</li>
				{/each}
			</ul>
		</div>
	</div>
	<div class="policy">
		<div>
			<h3>Grant, don't hope.</h3>
			<p>
				Runtime policy lives outside the source. The same <code>release.gr</code> runs in CI with
				one perimeter and in production with another — without a code change.
			</p>
		</div>
		<CodeBlock code={policy} title="shell" compact />
	</div>
</section>

<!-- ───────────────────────── TOOLING ───────────────────────── -->
<section class="band tooling">
	<div class="band-head">
		<p class="eyebrow">Tooling</p>
		<h2>A real toolchain, not a DSL in a YAML file.</h2>
	</div>
	<div class="tool-grid">
		<div class="tool">
			<h3>CLI</h3>
			<p><code>parse</code> · <code>compile</code> · <code>build</code> · <code>run</code> · <code>modules</code> · <code>examples</code>. JSON output on everything.</p>
		</div>
		<div class="tool">
			<h3>LSP + VS Code</h3>
			<p>Diagnostics, hover, and completion for <code>.gr</code>. Ships as a VSIX alongside every release.</p>
		</div>
		<div class="tool">
			<h3>Rust SDK</h3>
			<p>Embed the engine. A <code>slim</code> profile builds for iOS and <code>wasm32-unknown-unknown</code>.</p>
		</div>
		<div class="tool">
			<h3>Traces</h3>
			<p>Every run yields a step-by-step pipeline: function, op, output shape, errors. Debug from the artifact, not from logs.</p>
		</div>
	</div>
	<div class="install">
		<CodeBlock code={install} title="terminal" compact />
	</div>
</section>

<!-- ───────────────────────── TIMELINE ───────────────────────── -->
<section class="band timeline">
	<div class="band-head">
		<p class="eyebrow">Momentum</p>
		<h2>Shipping, on the record.</h2>
	</div>
	<ol class="releases">
		<li>
			<span class="v">0.6.0</span>
			<div>
				<h3>Extensible platform</h3>
				<p>Opt-in capability modules (<code>data</code>, <code>pdf</code>, <code>image</code>, <code>plot</code>, <code>media</code>), dynamic Wasm discovery with hotload, typed result envelopes.</p>
			</div>
		</li>
		<li>
			<span class="v">0.7.0</span>
			<div>
				<h3>Language + Stage B</h3>
				<p>Executable parameters and tagged variables (RFC-0004). Wasm-compilable Stage B AOT container with host fulfilment (RFC-0005).</p>
			</div>
		</li>
		<li>
			<span class="v">0.7.1</span>
			<div>
				<h3>Slim SDK</h3>
				<p>Dependency-light core profile for iOS and Wasm. Host module registration for embedders.</p>
			</div>
		</li>
		<li class="now">
			<span class="v">next</span>
			<div>
				<h3>Runtime-in-Wasm</h3>
				<p>The full engine as a WASI module (RFC-0006). You are looking at it.</p>
			</div>
		</li>
	</ol>
</section>

<!-- ───────────────────────── CLOSE ───────────────────────── -->
<section class="close">
	<h2>Write the workflow you'd want to read at 3 a.m.</h2>
	<div class="cta">
		<a class="btn primary" href="/playground">Try it in the browser</a>
		<a class="btn ghost" href="/docs/why-grapheme">Why Grapheme</a>
		<a class="btn ghost" href="https://github.com/EntasisLabs/grapheme">Source on GitHub ↗</a>
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
		padding: clamp(3.5rem, 9vh, 6rem) clamp(1rem, 4vw, 3rem);
		border-top: 1px solid var(--line);
	}

	.band-head {
		margin-bottom: 2rem;
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
		padding: clamp(2.5rem, 7vh, 5rem) clamp(1rem, 4vw, 3rem) clamp(3rem, 8vh, 5rem);
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

	.word {
		display: block;
		font-weight: 800;
		font-size: clamp(3.4rem, 8vw, 6rem);
		line-height: 0.9;
		letter-spacing: -0.06em;
		color: var(--sage-deep);
		margin-bottom: 1rem;
	}

	.line {
		display: block;
		font-weight: 700;
		font-size: clamp(1.35rem, 2.4vw, 1.9rem);
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

	/* thesis */
	.thesis {
		padding: clamp(3rem, 8vh, 5rem) clamp(1rem, 4vw, 3rem);
		background: var(--sage-deep);
		color: var(--mist);
	}

	.thesis .eyebrow {
		color: var(--signal-bright);
	}

	.thesis h2 {
		color: var(--mist);
		max-width: 20ch;
	}

	.thesis-grid {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 3rem;
		align-items: start;
	}

	.thesis-copy p {
		margin: 0 0 1.4rem;
		font-size: 1.08rem;
		color: color-mix(in srgb, var(--mist) 85%, transparent);
	}

	.thesis-copy em {
		color: var(--signal-bright);
		font-style: normal;
	}

	.principles {
		list-style: none;
		margin: 0;
		padding: 0;
		display: grid;
		gap: 0.75rem;
	}

	.principles li {
		padding-left: 1.1rem;
		border-left: 2px solid var(--signal-bright);
		color: color-mix(in srgb, var(--mist) 80%, transparent);
	}

	.principles strong {
		color: var(--mist);
		font-family: var(--font-display);
	}

	/* pipeline */
	.stages {
		list-style: none;
		margin: 0 0 2rem;
		padding: 0;
		display: grid;
		grid-template-columns: repeat(4, 1fr);
		gap: 1px;
		background: var(--line);
		border: 1px solid var(--line);
	}

	.stages li {
		position: relative;
		padding: 1.4rem 1.2rem 1.6rem;
		background: color-mix(in srgb, var(--mist) 85%, white);
	}

	.stages .n {
		font-family: var(--font-mono);
		font-size: 0.72rem;
		color: var(--signal);
	}

	.stages h3 {
		margin: 0.4rem 0 0.5rem;
		font-family: var(--font-display);
		font-size: 1.15rem;
		color: var(--sage-deep);
	}

	.stages p {
		margin: 0 0 0.9rem;
		font-size: 0.95rem;
		color: var(--ink-soft);
	}

	.crate {
		font-size: 0.72rem;
		color: var(--sage);
	}

	.targets {
		display: grid;
		grid-template-columns: repeat(3, 1fr);
		gap: 1.5rem;
	}

	.targets h4 {
		margin: 0 0 0.35rem;
		font-family: var(--font-display);
		font-size: 1rem;
		color: var(--sage-deep);
	}

	.targets p {
		margin: 0;
		font-size: 0.95rem;
		color: var(--ink-soft);
	}

	.targets em {
		color: var(--sage);
		font-style: normal;
		font-weight: 600;
	}

	/* caps */
	.caps-grid {
		display: grid;
		grid-template-columns: 1fr 1.3fr;
		gap: 1.5rem;
		margin-bottom: 2.5rem;
	}

	.caps-col {
		border: 1px solid var(--line);
		background: color-mix(in srgb, var(--mist) 85%, white);
	}

	.caps-title {
		display: flex;
		align-items: center;
		gap: 0.6rem;
		padding: 0.75rem 1rem;
		border-bottom: 1px solid var(--line);
		font-family: var(--font-mono);
		font-size: 0.78rem;
		color: var(--ink-soft);
	}

	.badge {
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

	.caps-col ul {
		list-style: none;
		margin: 0;
		padding: 0.4rem 0;
	}

	.caps-col li {
		display: grid;
		grid-template-columns: 7rem 1fr;
		gap: 0.6rem;
		padding: 0.45rem 1rem;
		font-size: 0.92rem;
		border-bottom: 1px dashed var(--line);
	}

	.caps-col li:last-child {
		border-bottom: 0;
	}

	.caps-col li code {
		color: var(--sage-deep);
		font-weight: 500;
	}

	.caps-col li span {
		color: var(--ink-soft);
	}

	.env {
		grid-column: 2;
		font-family: var(--font-mono);
		font-size: 0.7rem;
		font-style: normal;
		color: var(--ember);
	}

	.policy {
		display: grid;
		grid-template-columns: 0.9fr 1.1fr;
		gap: 2rem;
		align-items: center;
	}

	.policy h3 {
		margin: 0 0 0.5rem;
		font-family: var(--font-display);
		font-size: 1.4rem;
		color: var(--sage-deep);
	}

	.policy p {
		margin: 0;
		color: var(--ink-soft);
	}

	/* tooling */
	.tool-grid {
		display: grid;
		grid-template-columns: repeat(4, 1fr);
		gap: 1.5rem;
		margin-bottom: 2rem;
	}

	.tool h3 {
		margin: 0 0 0.4rem;
		padding-top: 0.75rem;
		border-top: 2px solid var(--sage);
		font-family: var(--font-display);
		font-size: 1.05rem;
		color: var(--sage-deep);
	}

	.tool p {
		margin: 0;
		font-size: 0.95rem;
		color: var(--ink-soft);
	}

	.install {
		max-width: 60rem;
	}

	/* timeline */
	.releases {
		list-style: none;
		margin: 0;
		padding: 0;
		display: grid;
		grid-template-columns: repeat(4, 1fr);
		gap: 1.5rem;
	}

	.releases li {
		display: grid;
		gap: 0.6rem;
		padding-top: 0.9rem;
		border-top: 1px solid var(--line);
	}

	.releases .v {
		font-family: var(--font-mono);
		font-size: 0.78rem;
		color: var(--signal);
	}

	.releases h3 {
		margin: 0 0 0.3rem;
		font-family: var(--font-display);
		font-size: 1.05rem;
		color: var(--sage-deep);
	}

	.releases p {
		margin: 0;
		font-size: 0.92rem;
		color: var(--ink-soft);
	}

	.releases .now {
		border-top-color: var(--sage);
	}

	.releases .now .v {
		color: var(--sage-deep);
		font-weight: 600;
	}

	/* close */
	.close {
		padding: clamp(3.5rem, 10vh, 6rem) clamp(1rem, 4vw, 3rem);
		border-top: 1px solid var(--line);
		display: grid;
		gap: 1.5rem;
		justify-items: start;
	}

	.close h2 {
		max-width: 22ch;
		font-size: clamp(1.8rem, 3.6vw, 2.8rem);
	}

	@media (max-width: 1000px) {
		.hero,
		.thesis-grid,
		.policy,
		.caps-grid {
			grid-template-columns: 1fr;
		}

		.stages,
		.tool-grid,
		.releases {
			grid-template-columns: repeat(2, 1fr);
		}

		.targets {
			grid-template-columns: 1fr;
		}
	}

	@media (max-width: 600px) {
		.stages,
		.tool-grid,
		.releases {
			grid-template-columns: 1fr;
		}

		.caps-col li {
			grid-template-columns: 1fr;
		}

		.env {
			grid-column: 1;
		}
	}
</style>
