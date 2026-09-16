<script lang="ts">
	import { onMount } from 'svelte';
	import { DEFAULT_EXAMPLE, PLAYGROUND_EXAMPLES } from '$lib/examples';
	import { runGrapheme, type ExecuteResponse } from '$lib/wasm/runtime';

	let exampleId = $state(DEFAULT_EXAMPLE.id);
	let source = $state(DEFAULT_EXAMPLE.source);
	let argsJson = $state(
		DEFAULT_EXAMPLE.args ? JSON.stringify(DEFAULT_EXAMPLE.args, null, 2) : ''
	);
	let running = $state(false);
	let status = $state<'idle' | 'loading-wasm' | 'running' | 'done' | 'error'>('idle');
	let result = $state<ExecuteResponse | null>(null);
	let errorMessage = $state('');
	let elapsedMs = $state<number | null>(null);
	let wasmReady = $state(false);

	function selectExample(id: string) {
		const ex = PLAYGROUND_EXAMPLES.find((e) => e.id === id);
		if (!ex) return;
		exampleId = ex.id;
		source = ex.source;
		argsJson = ex.args ? JSON.stringify(ex.args, null, 2) : '';
		result = null;
		errorMessage = '';
		status = wasmReady ? 'idle' : 'loading-wasm';
	}

	async function warmWasm() {
		status = 'loading-wasm';
		try {
			const res = await fetch('/grapheme-wasm.wasm', { method: 'HEAD' });
			if (!res.ok) throw new Error(`WASM missing (${res.status})`);
			wasmReady = true;
			status = 'idle';
		} catch (e) {
			wasmReady = false;
			status = 'error';
			errorMessage =
				e instanceof Error
					? e.message
					: 'Could not load grapheme-wasm.wasm. Build with scripts/build-runtime-wasm.sh';
		}
	}

	async function run() {
		running = true;
		status = 'running';
		errorMessage = '';
		result = null;
		elapsedMs = null;
		const started = performance.now();
		try {
			let args: unknown = null;
			if (argsJson.trim()) {
				args = JSON.parse(argsJson);
			}
			const response = await runGrapheme({
				source,
				initial_current: {},
				args
			});
			result = response;
			elapsedMs = Math.round(performance.now() - started);
			status = response.ok ? 'done' : 'error';
			if (!response.ok) {
				errorMessage = response.error?.message ?? 'Execution failed';
			}
		} catch (e) {
			status = 'error';
			errorMessage = e instanceof Error ? e.message : String(e);
			elapsedMs = Math.round(performance.now() - started);
		} finally {
			running = false;
		}
	}

	onMount(() => {
		warmWasm();
	});
</script>

<svelte:head>
	<title>Playground · Grapheme</title>
	<meta
		name="description"
		content="Compile and run Grapheme workflows in your browser with the RFC-0006 WASM runtime."
	/>
</svelte:head>

<section class="play">
	<header class="play-head">
		<div>
			<p class="eyebrow">Playground</p>
			<h1>grapheme</h1>
			<p class="lede">Client-side compile + execute via <code>grapheme-wasm</code> (WASI).</p>
		</div>
		<div class="meta">
			<span class="pill" class:ok={wasmReady} class:bad={status === 'error' && !wasmReady}>
				{#if status === 'loading-wasm'}Loading Wasm…
				{:else if wasmReady}Wasm ready
				{:else}Wasm unavailable{/if}
			</span>
			{#if elapsedMs != null}
				<span class="pill muted">{elapsedMs} ms</span>
			{/if}
		</div>
	</header>

	<div class="toolbar">
		<label>
			Example
			<select bind:value={exampleId} onchange={() => selectExample(exampleId)}>
				{#each PLAYGROUND_EXAMPLES as ex}
					<option value={ex.id}>{ex.label}</option>
				{/each}
			</select>
		</label>
		<button class="run" type="button" disabled={running || !wasmReady} onclick={run}>
			{running ? 'Running…' : 'Run'}
		</button>
	</div>

	<div class="grid">
		<div class="pane">
			<div class="pane-label">Source · .gr</div>
			<textarea class="editor" bind:value={source} spellcheck="false"></textarea>
			<div class="args">
				<div class="pane-label">Args JSON (optional)</div>
				<textarea
					class="args-editor"
					bind:value={argsJson}
					spellcheck="false"
					placeholder={'{}'}
				></textarea>
			</div>
		</div>
		<div class="pane">
			<div class="pane-label">Result</div>
			{#if errorMessage && status === 'error'}
				<pre class="out error">{errorMessage}</pre>
			{/if}
			{#if result}
				<pre class="out">{JSON.stringify(result, null, 2)}</pre>
			{:else if status === 'running'}
				<pre class="out muted">Compiling and executing in Wasm…</pre>
			{:else}
				<pre class="out muted">Run a workflow to see JSON output here.</pre>
			{/if}
		</div>
	</div>

	<p class="note">
		Wasm-safe stdlib only: <code>core</code>, <code>json</code>, <code>csv</code>, <code>yaml</code>,
		<code>html</code>. Host capabilities like <code>http</code> / <code>sql</code> fail closed — same
		as the WASI engine outside the browser.
	</p>
</section>

<style>
	.play {
		padding: 1.5rem clamp(1rem, 3vw, 2.5rem) 3rem;
	}

	.play-head {
		display: flex;
		flex-wrap: wrap;
		justify-content: space-between;
		gap: 1rem;
		margin-bottom: 1.25rem;
	}

	.eyebrow {
		margin: 0;
		font-family: var(--font-mono);
		font-size: 0.75rem;
		letter-spacing: 0.08em;
		text-transform: uppercase;
		color: var(--signal);
	}

	h1 {
		margin: 0.2rem 0;
		font-family: var(--font-display);
		font-weight: 800;
		font-size: clamp(2.4rem, 6vw, 3.6rem);
		letter-spacing: -0.05em;
		line-height: 0.95;
	}

	.lede {
		margin: 0;
		color: var(--ink-soft);
	}

	.meta {
		display: flex;
		gap: 0.5rem;
		align-items: flex-start;
	}

	.pill {
		display: inline-flex;
		align-items: center;
		padding: 0.35rem 0.65rem;
		border: 1px solid var(--line);
		background: var(--mist);
		font-family: var(--font-mono);
		font-size: 0.75rem;
	}

	.pill.ok {
		border-color: color-mix(in srgb, var(--signal) 50%, var(--line));
		color: var(--signal);
	}

	.pill.bad {
		border-color: color-mix(in srgb, var(--ember) 50%, var(--line));
		color: var(--ember);
	}

	.pill.muted {
		color: var(--ink-soft);
	}

	.toolbar {
		display: flex;
		flex-wrap: wrap;
		gap: 0.75rem;
		align-items: end;
		margin-bottom: 1rem;
	}

	label {
		display: grid;
		gap: 0.3rem;
		font-family: var(--font-display);
		font-weight: 600;
		font-size: 0.85rem;
	}

	select {
		min-width: 12rem;
		padding: 0.55rem 0.7rem;
		border: 1px solid var(--line);
		background: var(--mist);
		border-radius: var(--radius);
	}

	.run {
		padding: 0.65rem 1.3rem;
		border: none;
		border-radius: var(--radius);
		background: var(--ink);
		color: var(--mist);
		font-family: var(--font-display);
		font-weight: 700;
		cursor: pointer;
	}

	.run:hover:not(:disabled) {
		background: var(--signal);
	}

	.run:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	.grid {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 1rem;
		min-height: 28rem;
	}

	.pane {
		display: flex;
		flex-direction: column;
		min-height: 0;
		border: 1px solid var(--line);
		background: color-mix(in srgb, var(--mist) 85%, white);
	}

	.pane-label {
		padding: 0.55rem 0.85rem;
		border-bottom: 1px solid var(--line);
		font-family: var(--font-mono);
		font-size: 0.72rem;
		letter-spacing: 0.06em;
		text-transform: uppercase;
		color: var(--ink-soft);
	}

	.editor,
	.args-editor,
	.out {
		flex: 1;
		margin: 0;
		padding: 0.9rem 1rem;
		border: none;
		resize: vertical;
		background: transparent;
		font-family: var(--font-mono);
		font-size: 0.84rem;
		line-height: 1.45;
		color: var(--ink);
		min-height: 16rem;
	}

	.args {
		border-top: 1px solid var(--line);
	}

	.args-editor {
		min-height: 5rem;
		width: 100%;
	}

	.out {
		overflow: auto;
		white-space: pre-wrap;
		word-break: break-word;
	}

	.out.muted {
		color: var(--ink-soft);
	}

	.out.error {
		color: var(--ember);
		min-height: auto;
		flex: 0;
		border-bottom: 1px solid var(--line);
	}

	.note {
		margin: 1rem 0 0;
		font-size: 0.9rem;
		color: var(--ink-soft);
	}

	@media (max-width: 900px) {
		.grid {
			grid-template-columns: 1fr;
		}
	}
</style>
