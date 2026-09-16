<script lang="ts">
	import Seo from '$lib/components/Seo.svelte';
	import LiveHero from '$lib/components/LiveHero.svelte';
	import LanguageTour from '$lib/components/LanguageTour.svelte';
	import { GITHUB_URL } from '$lib/site';
	import rawCli from '$lib/raw-cli.txt?raw';
</script>

<Seo />

<section class="hero">
	<div class="copy">
		<h1>
			<span class="word">grapheme</span>
			<span class="line">A small language for workflows that must not go wrong quietly.</span>
		</h1>
		<p class="lede">
			You declare the state, chain steps with <code>|&gt;</code>, branch with <code>match</code>,
			loop with a budget. The compiler checks the shape of your data. The runtime refuses any host,
			database, or secret you didn't allow-list, and hands back a trace of every step. The program next
			to this text is being compiled and run by that runtime, in Wasm, in this tab.
		</p>
		<div class="cta">
			<a class="btn" href="/playground">Open the playground</a>
			<a class="quiet" href="/docs/quickstart">or install the CLI</a>
		</div>
	</div>
	<div class="live">
		<LiveHero />
	</div>
</section>

<section class="who">
	<h2>Who this is for</h2>
	<div class="prose">
		<p>
			Most automation starts as a script and ends as a liability. Somebody wrote a Python file that
			deploys the service, calls three APIs, and reads a token from an environment variable, and now
			nobody wants to touch it. The usual fix is a workflow platform, and the platform trades the
			liability for a cluster, a vendor, and a lot of JSON.
		</p>
		<p>
			Grapheme is the other fix. It is deliberately small. There is no scheduler, no UI, no
			general-purpose standard library. What you get is a language whose control flow you can read
			in one sitting, a type check on your state, a capability perimeter that fails closed, and an
			execution trace you can diff. It compiles to one artifact that runs from a CLI, inside a
			container, or in a browser.
		</p>
		<p>
			Use it for the release gate, the nightly data clean-up, the approval flow, the agent that
			shouldn't be allowed to call the internet without asking. The workflow fits on a screen but
			would hurt if it went wrong.
		</p>
		<p>
			Don't use it when a job has to survive a crash over a weekend. That is <a
				href="https://temporal.io">Temporal</a
			>'s problem and we don't solve it. Don't use it for a data platform with lineage and backfills;
			that's <a href="https://dagster.io">Dagster</a>. If you live entirely in AWS and like <a
				href="https://aws.amazon.com/step-functions/">Step Functions</a
			>, keep them. And if you're really building an application, write the application.
		</p>
		<p class="status">
			Where it stands: version 0.7.1, ten crates, 47 example programs, a VS Code extension, a
			conformance suite in CI, one GitHub star, and no production adopters that we know of. It is
			early. The runtime is real, which is why it is on this page as a program instead of a diagram.
		</p>
	</div>
</section>

<section class="more">
	<h2>More programs <span>every one of these was run through the same Wasm binary; outputs are unedited</span></h2>
	<LanguageTour />
</section>

<section class="raw">
	<h2>What the CLI prints <span>one run of <code>examples/hello-world.gr</code>, pasted as-is</span></h2>
	<pre class="dump">{rawCli}</pre>
</section>

<section class="install">
	<h2>Install</h2>
	<pre class="cmd">cargo install --git https://github.com/entasislabs/grapheme.git grapheme-cli --bin grapheme</pre>
	<p>
		Then <a href="/docs/quickstart">the quickstart</a>, <a href="/docs/language-tour">the language tour</a>,
		or <a href={GITHUB_URL}>the source</a>. Apache-2.0.
	</p>
</section>

<style>
	section {
		padding: 0 clamp(1rem, 4vw, 3rem);
	}

	h2 {
		display: flex;
		flex-wrap: wrap;
		align-items: baseline;
		gap: 0.4rem 1rem;
		margin: 0 0 1.4rem;
		padding-top: 1rem;
		border-top: 1px solid var(--sage-deep);
		font-size: 1.05rem;
		font-weight: 600;
		letter-spacing: 0;
		color: var(--sage-deep);
	}

	h2 span {
		font-family: var(--font-mono);
		font-size: 0.74rem;
		font-weight: 400;
		color: var(--ink-soft);
	}

	h2 span code {
		font-size: inherit;
	}

	/* hero */
	.hero {
		display: grid;
		grid-template-columns: minmax(0, 0.8fr) minmax(0, 1.2fr);
		gap: clamp(1.5rem, 4vw, 3.5rem);
		align-items: start;
		padding-top: clamp(2rem, 6vh, 4rem);
		padding-bottom: clamp(2.5rem, 6vh, 4rem);
	}

	.copy {
		min-width: 0;
		container-type: inline-size;
	}

	h1 {
		margin: 0;
	}

	.word {
		display: block;
		font-family: var(--font-mono);
		font-weight: 700;
		font-size: clamp(2rem, 9cqw, 3.4rem);
		letter-spacing: -0.04em;
		line-height: 1;
		color: var(--sage-deep);
		margin-bottom: 1.1rem;
	}

	.word::before {
		content: '|> ';
		color: var(--signal);
		font-weight: 500;
	}

	.line {
		display: block;
		font-weight: 600;
		font-size: clamp(1.35rem, 2.2vw, 1.75rem);
		line-height: 1.25;
		letter-spacing: -0.015em;
		color: var(--ink);
		max-width: 22ch;
	}

	.lede {
		margin: 1.2rem 0 1.6rem;
		max-width: 36rem;
		color: var(--ink-soft);
	}

	.lede code {
		font-size: 0.88em;
		color: var(--sage-deep);
	}

	.cta {
		display: flex;
		align-items: center;
		gap: 1.2rem;
		flex-wrap: wrap;
	}

	.btn {
		display: inline-block;
		padding: 0.8rem 1.25rem;
		background: var(--sage-deep);
		color: var(--mist);
		font-weight: 600;
		text-decoration: none;
	}

	.btn:hover {
		background: var(--sage);
		color: var(--mist);
	}

	.quiet {
		color: var(--ink-soft);
	}

	.live {
		min-width: 0;
	}

	/* who */
	.who {
		padding-bottom: clamp(2.5rem, 6vh, 4rem);
	}

	.prose {
		max-width: 44rem;
		font-size: 1.1rem;
		line-height: 1.6;
	}

	.prose p {
		margin: 0 0 1.1rem;
	}

	.prose a {
		color: var(--sage-deep);
	}

	.status {
		padding-top: 1rem;
		border-top: 1px solid var(--line);
		color: var(--ink-soft);
	}

	/* more */
	.more {
		padding-bottom: clamp(2.5rem, 6vh, 4rem);
	}

	/* raw */
	.raw {
		padding-bottom: clamp(2.5rem, 6vh, 4rem);
	}

	.dump {
		margin: 0;
		padding: 0;
		max-height: 34rem;
		overflow: auto;
		font-size: 0.78rem;
		line-height: 1.45;
		color: var(--ink);
		white-space: pre;
		tab-size: 2;
	}

	/* install */
	.install {
		padding-bottom: clamp(3rem, 8vh, 5rem);
	}

	.cmd {
		margin: 0 0 0.9rem;
		font-size: 0.86rem;
		white-space: pre-wrap;
		word-break: break-all;
		color: var(--sage-deep);
	}

	.install p {
		margin: 0;
		color: var(--ink-soft);
	}

	.install a {
		color: var(--sage-deep);
	}

	@media (max-width: 1280px) {
		.hero {
			grid-template-columns: minmax(0, 1fr);
		}

		.word {
			font-size: clamp(2rem, 7cqw, 3.2rem);
		}

		.line {
			max-width: 30ch;
		}
	}

	@media (max-width: 600px) {
		.word {
			font-size: clamp(1.7rem, 8cqw, 2.2rem);
		}

		.line {
			font-size: 1.25rem;
		}

		.prose {
			font-size: 1.02rem;
		}
	}
</style>
