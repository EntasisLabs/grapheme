<script lang="ts">
	import { onMount } from 'svelte';
	import CodeBlock from './CodeBlock.svelte';
	import { highlightJson } from '$lib/highlight';
	import type { Snippet } from '$lib/snippets';
	import { runGrapheme, type ExecuteResponse } from '$lib/wasm/runtime';

	type Step = { index: number; function_name: string; op: string; ok: boolean };
	type Phase = 'idle' | 'loading' | 'running' | 'ok' | 'refused' | 'error';

	let {
		snippet,
		file,
		autorun = 'immediate',
		stack = false
	}: {
		snippet: Snippet;
		file: string;
		/** `visible` defers the run until the panel scrolls into view. */
		autorun?: 'immediate' | 'visible';
		/** Source above the trace instead of beside it; for short programs. */
		stack?: boolean;
	} = $props();

	let root = $state<HTMLElement | null>(null);

	// Initial value only; the run itself is kicked off in onMount.
	// svelte-ignore state_referenced_locally
	let phase = $state<Phase>(autorun === 'visible' ? 'idle' : 'loading');
	let elapsed = $state<number | null>(null);
	let steps = $state<Step[]>([]);
	let shown = $state(0);
	let artifactId = $state('');
	let finalState = $state('');
	let message = $state('');
	let wasmBytes = $state<number | null>(null);

	async function go() {
		phase = 'loading';
		try {
			const head = await fetch('/grapheme-wasm.wasm', { method: 'HEAD' });
			const len = head.headers.get('content-length');
			if (len) wasmBytes = Number(len);
			phase = 'running';
			const t0 = performance.now();
			const res: ExecuteResponse = await runGrapheme({
				source: snippet.source,
				initial_current: {},
				args: snippet.args ?? null
			});
			elapsed = Math.round(performance.now() - t0);
			artifactId = res.artifact_id ?? '';

			const fs = res.final_state as { current?: unknown; pipeline?: Step[] } | undefined;
			steps = (fs?.pipeline ?? []).map((p) => ({
				index: p.index,
				function_name: p.function_name.startsWith('__inline') ? 'inline' : p.function_name,
				op: p.op,
				ok: p.ok
			}));

			if (res.ok) {
				finalState = JSON.stringify(fs?.current ?? null, null, 2);
				phase = 'ok';
			} else if (steps.length > 0) {
				// It compiled and started; the runtime stopped it mid-trace.
				message = res.execution?.message ?? res.error?.message ?? 'execution stopped';
				phase = 'refused';
			} else {
				message = res.error?.message ?? 'execution failed';
				phase = 'error';
				return;
			}

			shown = 0;
			const total = steps.length;
			const tick = () => {
				if (shown < total) {
					shown += 1;
					setTimeout(tick, 55);
				}
			};
			tick();
		} catch (e) {
			phase = 'error';
			message = e instanceof Error ? e.message : String(e);
		}
	}

	onMount(() => {
		if (autorun === 'immediate' || !root || !('IntersectionObserver' in window)) {
			void go();
			return;
		}
		const io = new IntersectionObserver(
			(entries) => {
				if (entries.some((e) => e.isIntersecting)) {
					io.disconnect();
					void go();
				}
			},
			{ rootMargin: '200px 0px' }
		);
		io.observe(root);
		return () => io.disconnect();
	});

	const visibleSteps = $derived(steps.slice(Math.max(0, shown - 10), shown));
	const settled = $derived((phase === 'ok' || phase === 'refused') && shown >= steps.length);
	const failedAt = $derived(steps.find((s) => !s.ok));
</script>

<div class="live" class:stack bind:this={root}>
	<div class="src">
		<CodeBlock code={snippet.source} title={file} />
	</div>
	<div class="run">
		<div class="head">
			<span class="status" class:bad={phase === 'refused' || phase === 'error'}>
				{#if phase === 'idle'}
					ready
				{:else if phase === 'loading'}
					loading runtime
				{:else if phase === 'running'}
					compiling · executing
				{:else if phase === 'ok'}
					ok
				{:else if phase === 'refused'}
					refused · step {String((failedAt?.index ?? 0) + 1).padStart(2, '0')}
				{:else}
					failed
				{/if}
			</span>
			{#if phase === 'ok' || phase === 'refused'}
				<span class="meta">
					{steps.length} {steps.length === 1 ? 'step' : 'steps'} · {elapsed} ms
					{#if wasmBytes}
						· {(wasmBytes / 1_048_576).toFixed(1)} MB wasm{/if}
				</span>
			{/if}
		</div>

		{#if phase === 'error'}
			<pre class="err">{message}</pre>
		{:else}
			<ol class="trace" aria-label="execution trace">
				{#each visibleSteps as s (s.index)}
					<li class:failed={!s.ok}>
						<span class="idx">{String(s.index + 1).padStart(2, '0')}</span>
						<span class="fn">{s.function_name}</span>
						<span class="op">{s.op}</span>
					</li>
				{/each}
			</ol>

			<div class="state">
				<div class="label">{phase === 'refused' ? 'runtime' : 'final state'}</div>
				{#if settled && phase === 'ok'}
					<pre>{@html highlightJson(finalState)}</pre>
				{:else if settled && phase === 'refused'}
					<pre class="bad">{message}</pre>
				{:else}
					<pre class="dim">{snippet.output}</pre>
				{/if}
			</div>
		{/if}

		<div class="foot">
			{#if artifactId}
				<span class="artifact"><span class="label">artifact</span> {artifactId}</span>
			{:else}
				<span></span>
			{/if}
			<span class="actions">
				<button type="button" onclick={go} disabled={phase === 'running' || phase === 'loading'}>
					run again
				</button>
				<a href={`/playground?example=${snippet.id}`}>edit in playground →</a>
			</span>
		</div>
	</div>
</div>

<style>
	.live {
		display: grid;
		grid-template-columns: minmax(0, 1.45fr) minmax(17rem, 0.8fr);
		min-width: 0;
		background: var(--panel);
		color: var(--panel-fg);
		border-radius: var(--radius);
		overflow: hidden;
	}

	.live.stack {
		grid-template-columns: minmax(0, 1fr);
	}

	.src {
		min-width: 0;
	}

	.src :global(.code) {
		height: 100%;
	}

	.run {
		display: flex;
		flex-direction: column;
		min-width: 0;
		background: var(--panel-2);
		border-left: 1px solid var(--panel-line);
		font-family: var(--font-mono);
		font-size: 0.76rem;
		line-height: 1.5;
	}

	.live.stack .run {
		border-left: 0;
		border-top: 1px solid var(--panel-line);
	}

	.head {
		display: flex;
		flex-wrap: wrap;
		justify-content: space-between;
		gap: 0.2rem 1rem;
		padding: 0.55rem 1.1rem;
		border-bottom: 1px solid var(--panel-line);
		font-size: 0.7rem;
		letter-spacing: 0.06em;
	}

	.status {
		color: #a8d5b3;
	}

	.status.bad {
		color: var(--ember-light);
	}

	.meta {
		color: var(--panel-muted);
	}

	.trace {
		list-style: none;
		margin: 0;
		padding: 0.6rem 0 0.5rem;
		min-height: 2.6rem;
	}

	.trace li {
		display: grid;
		grid-template-columns: 1.6rem 1fr auto;
		gap: 0.75rem;
		padding: 0.12rem 1.1rem;
		animation: slide 200ms ease;
	}

	@keyframes slide {
		from {
			transform: translateY(3px);
			opacity: 0;
		}
	}

	.idx {
		color: var(--panel-dim);
	}

	.fn {
		color: var(--panel-fg);
	}

	.op {
		color: var(--panel-muted);
	}

	.failed .op {
		color: var(--ember-light);
	}

	.state {
		flex: 1;
		padding: 0.8rem 1.1rem 1rem;
		border-top: 1px solid var(--panel-line);
	}

	.label {
		font-size: 0.66rem;
		letter-spacing: 0.08em;
		text-transform: uppercase;
		color: var(--panel-dim);
	}

	.state .label {
		margin-bottom: 0.4rem;
	}

	.state pre,
	.err {
		margin: 0;
		font-size: 0.78rem;
		line-height: 1.55;
		white-space: pre-wrap;
		overflow-wrap: anywhere;
	}

	.state pre.dim {
		opacity: 0.3;
	}

	.state pre.bad,
	.err {
		color: var(--ember-light);
	}

	.err {
		flex: 1;
		padding: 0.8rem 1.1rem;
	}

	.state :global(.t-key) { color: #cfe0d3; }
	.state :global(.t-str) { color: #f0b48a; }
	.state :global(.t-num) { color: #9ec9f5; }
	.state :global(.t-lit) { color: #e8c38f; }

	.foot {
		display: flex;
		flex-wrap: wrap;
		justify-content: space-between;
		align-items: baseline;
		gap: 0.3rem 1rem;
		padding: 0.55rem 1.1rem;
		border-top: 1px solid var(--panel-line);
		font-size: 0.7rem;
	}

	.artifact {
		color: var(--panel-muted);
		overflow-wrap: anywhere;
	}

	.artifact .label {
		margin-right: 0.35rem;
	}

	.actions {
		display: inline-flex;
		gap: 1.1rem;
		white-space: nowrap;
	}

	.actions button {
		border: 0;
		padding: 0;
		background: none;
		cursor: pointer;
		font: inherit;
		color: var(--panel-fg);
		text-decoration: underline;
		text-decoration-color: var(--panel-dim);
		text-underline-offset: 0.2em;
	}

	.actions button:disabled {
		color: var(--panel-dim);
		cursor: default;
		text-decoration: none;
	}

	.actions button:hover:not(:disabled),
	.actions a:hover {
		color: #a8d5b3;
	}

	.actions a {
		color: var(--panel-fg);
		text-decoration: none;
	}

	@media (max-width: 760px) {
		.live {
			grid-template-columns: minmax(0, 1fr);
		}

		.run {
			border-left: 0;
			border-top: 1px solid var(--panel-line);
		}
	}
</style>
