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

	const visibleSteps = $derived(steps.slice(Math.max(0, shown - 7), shown));
	const settled = $derived((phase === 'ok' || phase === 'refused') && shown >= steps.length);
	const failedAt = $derived(steps.find((s) => !s.ok));
</script>

<div class="live" class:stack bind:this={root}>
	<div class="src">
		<CodeBlock code={snippet.source} title={file} />
	</div>
	<div class="run">
		<div class="run-head">
			<span class="status">
				{#if phase === 'idle'}
					<i class="dot"></i> ready
				{:else if phase === 'loading'}
					<i class="dot pulse"></i> loading runtime
				{:else if phase === 'running'}
					<i class="dot pulse"></i> compiling · verifying · executing
				{:else if phase === 'ok'}
					<i class="dot ok"></i> executed in your browser
				{:else if phase === 'refused'}
					<i class="dot bad"></i> runtime refused at step
					{String((failedAt?.index ?? 0) + 1).padStart(2, '0')}
				{:else}
					<i class="dot bad"></i> failed
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
						<span class="op">{s.ok ? s.op : `${s.op} ✕`}</span>
					</li>
				{/each}
				{#if phase === 'loading' || phase === 'running'}
					<li class="ghost"><span class="idx">··</span><span class="fn">waiting</span></li>
				{/if}
			</ol>

			<div class="state" class:bad={phase === 'refused'}>
				<div class="label">{phase === 'refused' ? 'runtime message' : 'final state'}</div>
				{#if settled && phase === 'ok'}
					<pre>{@html highlightJson(finalState)}</pre>
				{:else if settled && phase === 'refused'}
					<pre>{message}</pre>
				{:else}
					<pre class="dim">{snippet.output}</pre>
				{/if}
			</div>

			{#if artifactId}
				<div class="receipt">
					<span class="label">artifact</span>
					<span class="id">{artifactId}</span>
				</div>
			{/if}
		{/if}

		<div class="run-foot">
			<button type="button" onclick={go} disabled={phase === 'running' || phase === 'loading'}>
				Run again
			</button>
			<a href={`/playground?example=${snippet.id}`}>Edit in playground →</a>
		</div>
	</div>
</div>

<style>
	.live {
		display: grid;
		grid-template-columns: minmax(0, 1.45fr) minmax(17rem, 0.8fr);
		gap: 0;
		min-width: 0;
		border: 1px solid var(--sage-deep);
	}

	.live.stack {
		grid-template-columns: minmax(0, 1fr);
	}

	.live.stack .run {
		border-left: 0;
		border-top: 1px solid var(--line);
	}

	.src {
		min-width: 0;
	}

	.src :global(.code) {
		border: 0;
		height: 100%;
	}

	.run {
		display: flex;
		flex-direction: column;
		min-width: 0;
		background: var(--mist);
		border-left: 1px solid var(--line);
		font-family: var(--font-mono);
		font-size: 0.78rem;
	}

	.run-head {
		display: flex;
		flex-wrap: wrap;
		justify-content: space-between;
		align-items: center;
		gap: 0.3rem 0.75rem;
		padding: 0.6rem 0.9rem;
		border-bottom: 1px solid var(--line);
	}

	.status {
		display: inline-flex;
		align-items: center;
		gap: 0.5rem;
		color: var(--ink);
	}

	.meta {
		color: var(--ink-soft);
	}

	.dot {
		width: 0.55rem;
		height: 0.55rem;
		border-radius: 50%;
		background: var(--signal);
		display: inline-block;
	}

	.dot.pulse {
		animation: pulse 1s ease-in-out infinite;
	}

	.dot.ok {
		background: #3f8f5f;
	}

	.dot.bad {
		background: var(--ember);
	}

	@keyframes pulse {
		0%,
		100% {
			opacity: 0.35;
		}
		50% {
			opacity: 1;
		}
	}

	.trace {
		list-style: none;
		margin: 0;
		padding: 0.5rem 0;
		min-height: 3.2rem;
		border-bottom: 1px solid var(--line);
	}

	.trace li {
		display: grid;
		grid-template-columns: 2rem 1fr auto;
		gap: 0.6rem;
		padding: 0.18rem 0.9rem;
		animation: slide 220ms ease;
	}

	.trace .ghost {
		opacity: 0.4;
	}

	.trace .failed .op {
		color: var(--ember);
	}

	@keyframes slide {
		from {
			transform: translateY(4px);
			opacity: 0;
		}
	}

	.idx {
		color: var(--ink-soft);
		opacity: 0.7;
	}

	.fn {
		color: var(--sage-deep);
		font-weight: 500;
	}

	.op {
		color: var(--ink-soft);
	}

	.state {
		flex: 1;
		padding: 0.6rem 0.9rem 0.8rem;
	}

	.state.bad {
		border-left: 3px solid var(--ember);
	}

	.label {
		font-size: 0.68rem;
		letter-spacing: 0.08em;
		text-transform: uppercase;
		color: var(--signal);
		margin-bottom: 0.35rem;
	}

	.state pre {
		margin: 0;
		font-size: 0.8rem;
		line-height: 1.5;
		white-space: pre-wrap;
		overflow-wrap: anywhere;
	}

	.state.bad pre {
		color: var(--ember);
	}

	.state pre.dim {
		opacity: 0.35;
	}

	.state :global(.t-key) { color: var(--sage-deep); }
	.state :global(.t-str) { color: #8a4b2a; }
	.state :global(.t-num) { color: #3b5f8a; }
	.state :global(.t-lit) { color: #6a4f2b; }

	.receipt {
		display: flex;
		align-items: baseline;
		gap: 0.6rem;
		padding: 0.5rem 0.9rem;
		border-top: 1px solid var(--line);
	}

	.receipt .label {
		margin: 0;
	}

	.receipt .id {
		color: var(--ink-soft);
		overflow-wrap: anywhere;
	}

	.err {
		margin: 0;
		padding: 0.9rem;
		color: var(--ember);
		white-space: pre-wrap;
	}

	.run-foot {
		display: flex;
		justify-content: space-between;
		align-items: center;
		gap: 0.5rem;
		padding: 0.6rem 0.9rem;
		border-top: 1px solid var(--line);
	}

	.run-foot button,
	.run-foot a {
		white-space: nowrap;
	}

	.run-foot button {
		border: 1px solid var(--line);
		background: transparent;
		padding: 0.3rem 0.65rem;
		cursor: pointer;
		font-family: var(--font-mono);
		font-size: 0.75rem;
		color: var(--ink);
	}

	.run-foot button:hover:not(:disabled) {
		border-color: var(--sage);
		color: var(--sage);
	}

	.run-foot a {
		text-decoration: none;
		color: var(--sage);
		font-weight: 500;
	}

	@media (max-width: 760px) {
		.live {
			grid-template-columns: minmax(0, 1fr);
		}

		.run {
			border-left: 0;
			border-top: 1px solid var(--line);
		}
	}
</style>
