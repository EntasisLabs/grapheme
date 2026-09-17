<script lang="ts">
	import { onMount } from 'svelte';
	import { replaceState } from '$app/navigation';
	import { resolve } from '$app/paths';
	import Seo from '$lib/components/Seo.svelte';
	import { highlightGr, highlightJson } from '$lib/highlight';
	import { SNIPPETS, snippetById } from '$lib/snippets';
	import { runGrapheme, WASM_URL, type ExecuteResponse } from '$lib/wasm/runtime';

	type PipelineEntry = {
		index: number;
		function_name: string;
		op: string;
		ok: boolean;
		error?: unknown;
		output?: unknown;
		iteration_index?: number | null;
		call_depth?: number;
	};

	let source = $state(SNIPPETS[0]!.source);
	let argsJson = $state('');
	let argsOpen = $state(false);
	let activeId = $state<string | null>(SNIPPETS[0]!.id);
	let wasmReady = $state(false);
	let wasmError = $state('');
	let running = $state(false);
	let result = $state<ExecuteResponse | null>(null);
	let elapsed = $state<number | null>(null);
	let tab = $state<'state' | 'trace' | 'json'>('state');
	let copied = $state(false);
	let editorEl = $state<HTMLTextAreaElement | null>(null);
	let highlightEl = $state<HTMLElement | null>(null);

	const highlighted = $derived(highlightGr(source) + (source.endsWith('\n') ? ' ' : ''));
	const lineCount = $derived(source.split('\n').length);

	const pipeline = $derived(
		((result?.final_state as { pipeline?: PipelineEntry[] } | undefined)?.pipeline ?? []) as PipelineEntry[]
	);
	const current = $derived((result?.final_state as { current?: unknown } | undefined)?.current);
	const outcome = $derived(result?.execution?.outcome ?? (result && !result.ok ? 'failed' : null));

	function load(id: string, updateUrl = true) {
		const s = snippetById(id);
		if (!s) return;
		activeId = s.id;
		source = s.source;
		argsJson = s.args ? JSON.stringify(s.args, null, 2) : '';
		argsOpen = !!s.args;
		result = null;
		elapsed = null;
		if (updateUrl) syncUrl();
		requestAnimationFrame(() => {
			document
				.querySelector('.examples button.active')
				?.scrollIntoView({ block: 'nearest', inline: 'center' });
		});
	}

	function encodeShare(): string {
		const payload = JSON.stringify({ s: source, a: argsJson || undefined });
		return btoa(unescape(encodeURIComponent(payload)));
	}

	function decodeShare(hash: string): { s: string; a?: string } | null {
		try {
			return JSON.parse(decodeURIComponent(escape(atob(hash))));
		} catch {
			return null;
		}
	}

	function safeReplace(url: URL) {
		try {
			replaceState(url, {});
		} catch {
			history.replaceState(history.state, '', url);
		}
	}

	function syncUrl() {
		if (typeof window === 'undefined') return;
		const url = new URL(window.location.href);
		url.hash = '';
		if (activeId) url.searchParams.set('example', activeId);
		else url.searchParams.delete('example');
		safeReplace(url);
	}

	async function share() {
		const url = new URL(window.location.href);
		url.searchParams.delete('example');
		url.hash = `code=${encodeShare()}`;
		try {
			await navigator.clipboard.writeText(url.toString());
		} catch {
			// clipboard may be unavailable; URL still updates below
		}
		safeReplace(url);
		copied = true;
		setTimeout(() => (copied = false), 1600);
	}

	async function checkWasm() {
		try {
			const res = await fetch(WASM_URL, { method: 'HEAD' });
			if (!res.ok) throw new Error(`grapheme-wasm.wasm missing (${res.status})`);
			wasmReady = true;
		} catch (e) {
			wasmError = e instanceof Error ? e.message : String(e);
		}
	}

	async function run() {
		if (running || !wasmReady) return;
		running = true;
		result = null;
		const t0 = performance.now();
		try {
			let args: unknown = null;
			if (argsJson.trim()) args = JSON.parse(argsJson);
			result = await runGrapheme({ source, initial_current: {}, args });
		} catch (e) {
			result = {
				ok: false,
				error: { code: 'CLIENT_ERROR', message: e instanceof Error ? e.message : String(e) }
			};
		} finally {
			elapsed = Math.round(performance.now() - t0);
			running = false;
			if (result && !result.ok && result.error?.code?.startsWith('COMPILE')) tab = 'state';
		}
	}

	function onInput() {
		activeId = null;
	}

	function onScroll() {
		if (editorEl && highlightEl) {
			highlightEl.scrollTop = editorEl.scrollTop;
			highlightEl.scrollLeft = editorEl.scrollLeft;
		}
	}

	function onKey(e: KeyboardEvent) {
		if ((e.metaKey || e.ctrlKey) && e.key === 'Enter') {
			e.preventDefault();
			run();
		}
		if (e.key === 'Tab' && editorEl) {
			e.preventDefault();
			const start = editorEl.selectionStart;
			const end = editorEl.selectionEnd;
			source = source.slice(0, start) + '  ' + source.slice(end);
			requestAnimationFrame(() => {
				if (editorEl) editorEl.selectionStart = editorEl.selectionEnd = start + 2;
			});
		}
	}

	onMount(() => {
		void checkWasm();

		const hash = window.location.hash;
		if (hash.startsWith('#code=')) {
			const decoded = decodeShare(hash.slice('#code='.length));
			if (decoded) {
				source = decoded.s;
				argsJson = decoded.a ?? '';
				argsOpen = !!decoded.a;
				activeId = null;
			}
			return;
		}

		const ex = new URLSearchParams(window.location.search).get('example');
		if (ex && snippetById(ex)) load(ex, false);
	});

	function shapeOf(v: unknown): string {
		if (v === null || v === undefined) return 'null';
		if (Array.isArray(v)) return `list[${v.length}]`;
		if (typeof v === 'object') {
			const o = v as Record<string, unknown>;
			if (o._kind === 'object' && typeof o._keys === 'number') return `object{${o._keys}}`;
			return `object{${Object.keys(o).length}}`;
		}
		return typeof v;
	}
</script>

<Seo
	title="Playground · Grapheme"
	description="Write and run Grapheme in your browser. The compiler and runtime execute as a WASI module — no server."
/>

<div class="pg">
	<aside class="rail">
		<div class="rail-head">
			<h1>Playground</h1>
			<p class="sub">Compiler and runtime as Wasm, in this tab.</p>
		</div>
		<p class="rail-label">Examples</p>
		<ul class="examples">
			{#each SNIPPETS as s}
				<li>
					<button type="button" class:active={s.id === activeId} onclick={() => load(s.id)}>
						<span>{s.label}</span>
						<small>{s.tags.slice(0, 3).join(' · ')}</small>
					</button>
				</li>
			{/each}
		</ul>
		<div class="rail-foot">
			<p><strong>Granted here:</strong> core · json · csv · yaml · html. Host ops such as <code>http</code> fail closed.</p>
			<a href={resolve('/docs/language-tour')}>Language tour →</a>
		</div>
	</aside>

	<section class="work">
		<div class="toolbar">
			<div class="left">
				<span class="pill" class:ok={wasmReady} class:bad={!!wasmError}>
					{#if wasmError}wasm unavailable{:else if wasmReady}wasm ready{:else}loading wasm…{/if}
				</span>
				{#if elapsed != null}<span class="pill muted">{elapsed} ms</span>{/if}
				{#if result?.artifact_id}<span class="pill muted mono">{result.artifact_id}</span>{/if}
			</div>
			<div class="right">
				<button type="button" class="ghost" onclick={share}>{copied ? 'Copied link' : 'Share'}</button>
				<button type="button" class="run" disabled={running || !wasmReady} onclick={run}>
					{running ? 'Running…' : 'Run'}
					<kbd>⌘↵</kbd>
				</button>
			</div>
		</div>

		<div class="split">
			<div class="editor-wrap">
				<div class="pane-label">
					<span>source</span>
					<span class="pane-meta">
						<button
							type="button"
							class="args-toggle"
							class:on={argsOpen}
							aria-expanded={argsOpen}
							aria-controls="args-panel"
							onclick={() => (argsOpen = !argsOpen)}
						>
							args{argsJson.trim() ? ' · set' : ''}
						</button>
						<span class="mono">{lineCount} lines</span>
					</span>
				</div>
				{#if argsOpen}
					<div class="args" id="args-panel">
						<div class="pane-label"><span>entrypoint args · json</span></div>
						<textarea
							class="args-editor"
							bind:value={argsJson}
							spellcheck="false"
							autocomplete="off"
							placeholder={'{ "label": "grapheme" }'}
							aria-label="Entrypoint args JSON"
						></textarea>
					</div>
				{/if}
				<div class="editor">
					<pre class="hl" bind:this={highlightEl} aria-hidden="true"><code>{@html highlighted}</code></pre>
					<textarea
						bind:this={editorEl}
						bind:value={source}
						spellcheck="false"
						autocomplete="off"
						autocapitalize="off"
						oninput={onInput}
						onscroll={onScroll}
						onkeydown={onKey}
						aria-label="Grapheme source"
					></textarea>
				</div>
			</div>

			<div class="result">
				<div class="result-head">
					<div class="tabs" role="tablist">
						<button role="tab" type="button" class:active={tab === 'state'} onclick={() => (tab = 'state')}>State</button>
						<button role="tab" type="button" class:active={tab === 'trace'} onclick={() => (tab = 'trace')}>
							Trace{#if pipeline.length}<span class="count">{pipeline.length}</span>{/if}
						</button>
						<button role="tab" type="button" class:active={tab === 'json'} onclick={() => (tab = 'json')}>Raw</button>
					</div>
					{#if outcome}
						<span class="outcome" class:good={outcome === 'succeeded'} class:bad={outcome !== 'succeeded'}>
							{outcome}
						</span>
					{/if}
				</div>

				<div class="result-body">
					{#if !result}
						<div class="empty">
							<p>Press <strong>Run</strong> (or ⌘/Ctrl + Enter).</p>
							<p class="dim">Compiled to a verified artifact, then executed, all inside Wasm.</p>
						</div>
					{:else if !result.ok && !result.execution}
						<div class="errbox">
							<div class="errcode mono">{result.error?.code}</div>
							<pre>{result.error?.message}</pre>
						</div>
					{:else if tab === 'state'}
						{#if !result.ok && result.error}
							<div class="errbox inline">
								<div class="errcode mono">{result.error.code}</div>
								<pre>{result.error.message}</pre>
							</div>
						{/if}
						<pre class="json">{@html highlightJson(JSON.stringify(current ?? null, null, 2))}</pre>
						{#if result.lint_warnings?.length}
							<div class="lints">
								<div class="pane-label"><span>lint</span></div>
								{#each result.lint_warnings as w}
									<pre class="lint">{JSON.stringify(w)}</pre>
								{/each}
							</div>
						{/if}
					{:else if tab === 'trace'}
						<ol class="trace">
							{#each pipeline as p (p.index)}
								<li class:fail={!p.ok} style={`--depth:${p.call_depth ?? 0}`}>
									<span class="idx mono">{String(p.index + 1).padStart(2, '0')}</span>
									<span class="fn" class:internal={p.function_name.startsWith('__inline')}>
										{p.function_name.startsWith('__inline') ? 'inline' : p.function_name}
									</span>
									<span class="op mono">{p.op}</span>
									<span class="shape mono">{shapeOf(p.output)}</span>
									{#if p.iteration_index != null}<span class="iter mono">#{p.iteration_index}</span>{/if}
								</li>
							{/each}
						</ol>
					{:else}
						<pre class="json">{@html highlightJson(JSON.stringify(result, null, 2))}</pre>
					{/if}
				</div>
			</div>
		</div>
	</section>
</div>

<style>
	.pg {
		display: grid;
		grid-template-columns: 17rem minmax(0, 1fr);
		min-height: calc(100vh - var(--nav-h));
	}

	/* rail */
	.rail {
		display: flex;
		flex-direction: column;
		min-width: 0;
		border-right: 1px solid var(--line);
		background: color-mix(in srgb, var(--mist) 60%, transparent);
	}

	.rail-head {
		padding: 1.2rem 1.2rem 1rem;
		border-bottom: 1px solid var(--line);
	}

	.rail h1 {
		margin: 0 0 0.2rem;
		font-weight: 600;
		font-size: 1.05rem;
		letter-spacing: -0.01em;
		line-height: 1.2;
		color: var(--ink);
	}

	.sub {
		margin: 0;
		font-size: 0.86rem;
		color: var(--ink-soft);
	}

	.rail-label {
		margin: 1rem 1.2rem 0.4rem;
		font-family: var(--font-mono);
		font-size: 0.68rem;
		letter-spacing: 0.1em;
		text-transform: uppercase;
		color: var(--ink-soft);
	}

	.examples {
		list-style: none;
		margin: 0;
		padding: 0;
	}

	.examples button {
		width: 100%;
		display: grid;
		gap: 0.15rem;
		text-align: left;
		padding: 0.6rem 1.2rem;
		border: 0;
		border-left: 3px solid transparent;
		background: transparent;
		cursor: pointer;
		color: var(--ink);
		font-weight: 600;
		font-size: 0.9rem;
	}

	.examples small {
		font-family: var(--font-mono);
		font-weight: 400;
		font-size: 0.68rem;
		color: var(--ink-soft);
	}

	.examples button:hover {
		background: color-mix(in srgb, var(--sage) 8%, transparent);
	}

	.examples button.active {
		border-left-color: var(--sage);
		background: color-mix(in srgb, var(--sage) 12%, transparent);
		color: var(--sage-deep);
	}

	.rail-foot {
		margin-top: auto;
		padding: 1rem 1.2rem 1.4rem;
		border-top: 1px solid var(--line);
		font-size: 0.82rem;
		color: var(--ink-soft);
	}

	.rail-foot p {
		margin: 0 0 0.6rem;
	}

	.rail-foot strong {
		color: var(--sage-deep);
	}

	.rail-foot a {
		text-decoration: none;
		color: var(--sage);
		font-weight: 600;
	}

	/* work */
	.work {
		display: flex;
		flex-direction: column;
		min-width: 0;
	}

	.toolbar,
	.split,
	.editor-wrap,
	.result {
		min-width: 0;
	}

	.toolbar {
		display: flex;
		justify-content: space-between;
		align-items: center;
		gap: 1rem;
		padding: 0.7rem 1.2rem;
		border-bottom: 1px solid var(--line);
	}

	.left,
	.right {
		display: flex;
		gap: 0.5rem;
		align-items: center;
		flex-wrap: wrap;
	}

	.pill {
		display: inline-flex;
		align-items: center;
		padding: 0.3rem 0.6rem;
		border: 1px solid var(--line);
		background: var(--mist);
		font-family: var(--font-mono);
		font-size: 0.72rem;
		color: var(--ink-soft);
	}

	.pill.ok {
		border-color: color-mix(in srgb, var(--sage) 55%, transparent);
		color: var(--sage);
	}

	.pill.bad {
		border-color: color-mix(in srgb, var(--ember) 55%, transparent);
		color: var(--ember);
	}

	.run {
		display: inline-flex;
		align-items: center;
		gap: 0.6rem;
		padding: 0.55rem 1.1rem;
		border: 0;
		border-radius: var(--radius);
		background: var(--sage);
		color: var(--mist);
		font-weight: 600;
		cursor: pointer;
	}

	.run:hover:not(:disabled) {
		background: var(--sage-deep);
	}

	.run:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	.run kbd {
		font-family: var(--font-mono);
		font-size: 0.68rem;
		opacity: 0.75;
	}

	.ghost {
		padding: 0.55rem 0.9rem;
		border: 1px solid var(--line);
		background: transparent;
		border-radius: var(--radius);
		font-weight: 600;
		color: var(--ink);
		cursor: pointer;
	}

	.ghost:hover {
		border-color: var(--sage);
		color: var(--sage);
	}

	.split {
		display: grid;
		grid-template-columns: 1.1fr 0.9fr;
		flex: 1;
		min-height: 0;
	}

	.pane-label {
		display: flex;
		justify-content: space-between;
		padding: 0.5rem 1rem;
		border-bottom: 1px solid var(--line);
		font-family: var(--font-mono);
		font-size: 0.68rem;
		letter-spacing: 0.08em;
		text-transform: uppercase;
		color: var(--ink-soft);
	}

	.editor-wrap {
		display: flex;
		flex-direction: column;
		border-right: 1px solid var(--line);
		min-height: 0;
	}

	.editor {
		position: relative;
		flex: 1;
		min-height: 22rem;
		background: color-mix(in srgb, var(--mist) 92%, white);
	}

	.editor .hl,
	.editor textarea {
		position: absolute;
		inset: 0;
		margin: 0;
		padding: 1rem 1.1rem;
		font-family: var(--font-mono);
		font-size: 0.86rem;
		line-height: 1.55;
		tab-size: 2;
		white-space: pre-wrap;
		overflow-wrap: break-word;
		overflow: auto;
		border: 0;
	}

	.editor .hl {
		pointer-events: none;
		color: var(--ink);
	}

	.editor .hl code {
		white-space: inherit;
		overflow-wrap: inherit;
	}

	.editor textarea {
		background: transparent;
		color: transparent;
		caret-color: var(--sage-deep);
		resize: none;
		outline: none;
	}

	.editor textarea::selection {
		background: color-mix(in srgb, var(--sage) 25%, transparent);
	}

	.hl :global(.t-kw) { color: #2f6f4a; font-weight: 500; }
	.hl :global(.t-def) { color: #1f3528; font-weight: 600; }
	.hl :global(.t-type) { color: #6a4f2b; }
	.hl :global(.t-str) { color: #8a4b2a; }
	.hl :global(.t-interp) { color: #b85c38; font-weight: 500; }
	.hl :global(.t-var) { color: #3b5f8a; }
	.hl :global(.t-dir) { color: #7a5a1e; }
	.hl :global(.t-num),
	.hl :global(.t-lit) { color: #8a4b2a; }
	.hl :global(.t-pipe) { color: #2f6f4a; font-weight: 700; }
	.hl :global(.t-arrow) { color: #2f6f4a; font-weight: 600; }
	.hl :global(.t-mod) { color: #4a5c4e; }
	.hl :global(.t-fn) { color: #1f3528; }
	.hl :global(.t-key) { color: #4a5c4e; }
	.hl :global(.t-cm) { color: #7a877c; font-style: italic; }
	.hl :global(.t-p) { opacity: 0.7; }

	.pane-meta {
		display: inline-flex;
		align-items: center;
		gap: 0.9rem;
	}

	.args-toggle {
		border: 0;
		padding: 0;
		background: none;
		cursor: pointer;
		font: inherit;
		letter-spacing: inherit;
		text-transform: inherit;
		color: var(--sage);
	}

	.args-toggle:hover,
	.args-toggle.on {
		color: var(--ink);
	}

	.args-toggle.on {
		text-decoration: underline;
		text-underline-offset: 0.25em;
	}

	.args {
		border-bottom: 1px solid var(--line);
		background: color-mix(in srgb, var(--mist) 60%, transparent);
	}

	.args-editor {
		display: block;
		width: 100%;
		min-height: 4.5rem;
		padding: 0.7rem 1.1rem;
		border: 0;
		background: transparent;
		font-family: var(--font-mono);
		font-size: 0.8rem;
		color: var(--ink);
		resize: vertical;
		outline: none;
	}

	/* result */
	.result {
		display: flex;
		flex-direction: column;
		min-height: 0;
		background: color-mix(in srgb, var(--mist) 80%, transparent);
	}

	.result-head {
		display: flex;
		justify-content: space-between;
		align-items: center;
		border-bottom: 1px solid var(--line);
		padding-right: 1rem;
	}

	.tabs {
		display: flex;
	}

	.tabs button {
		padding: 0.6rem 1rem;
		border: 0;
		border-bottom: 2px solid transparent;
		background: transparent;
		font-weight: 600;
		font-size: 0.85rem;
		color: var(--ink-soft);
		cursor: pointer;
		display: inline-flex;
		align-items: center;
		gap: 0.4rem;
	}

	.tabs button.active {
		color: var(--sage-deep);
		border-bottom-color: var(--sage);
	}

	.count {
		font-family: var(--font-mono);
		font-size: 0.68rem;
		padding: 0 0.35rem;
		border: 1px solid var(--line);
	}

	.outcome {
		font-family: var(--font-mono);
		font-size: 0.72rem;
		padding: 0.25rem 0.55rem;
		border: 1px solid var(--line);
	}

	.outcome.good {
		color: #2f7a4f;
		border-color: color-mix(in srgb, #2f7a4f 45%, transparent);
		background: color-mix(in srgb, #2f7a4f 8%, transparent);
	}

	.outcome.bad {
		color: var(--ember);
		border-color: color-mix(in srgb, var(--ember) 45%, transparent);
		background: color-mix(in srgb, var(--ember) 8%, transparent);
	}

	.result-body {
		flex: 1;
		overflow: auto;
		min-height: 22rem;
	}

	.empty {
		padding: 1.5rem 1.2rem;
		color: var(--ink-soft);
	}

	.empty p {
		margin: 0 0 0.5rem;
	}

	.empty strong {
		color: var(--sage-deep);
	}

	.dim {
		font-size: 0.9rem;
		opacity: 0.8;
	}

	.json {
		margin: 0;
		padding: 1rem 1.1rem;
		font-family: var(--font-mono);
		font-size: 0.82rem;
		line-height: 1.55;
		white-space: pre-wrap;
		word-break: break-word;
	}

	.json :global(.t-key) { color: var(--sage-deep); }
	.json :global(.t-str) { color: #8a4b2a; }
	.json :global(.t-num) { color: #3b5f8a; }
	.json :global(.t-lit) { color: #6a4f2b; }

	.errbox {
		margin: 1rem 1.1rem;
		border: 1px solid color-mix(in srgb, var(--ember) 45%, transparent);
		border-left: 3px solid var(--ember);
		background: color-mix(in srgb, var(--ember) 6%, var(--mist));
	}

	.errbox.inline {
		margin-bottom: 0;
	}

	.errcode {
		padding: 0.4rem 0.8rem;
		font-size: 0.7rem;
		letter-spacing: 0.08em;
		color: var(--ember);
		border-bottom: 1px dashed color-mix(in srgb, var(--ember) 35%, transparent);
	}

	.errbox pre {
		margin: 0;
		padding: 0.75rem 0.8rem;
		font-family: var(--font-mono);
		font-size: 0.82rem;
		white-space: pre-wrap;
		color: var(--ink);
	}

	.lints {
		border-top: 1px solid var(--line);
	}

	.lint {
		margin: 0;
		padding: 0.5rem 1.1rem;
		font-family: var(--font-mono);
		font-size: 0.74rem;
		color: #7a5a1e;
		white-space: pre-wrap;
	}

	.trace {
		list-style: none;
		margin: 0;
		padding: 0.4rem 0;
	}

	.trace li {
		display: grid;
		grid-template-columns: 2.2rem 1fr auto auto auto;
		gap: 0.7rem;
		align-items: baseline;
		padding: 0.3rem 1.1rem 0.3rem calc(1.1rem + var(--depth) * 0.9rem);
		font-size: 0.82rem;
		border-bottom: 1px dashed var(--line);
	}

	.trace li.fail {
		background: color-mix(in srgb, var(--ember) 8%, transparent);
	}

	.trace .idx {
		color: var(--ink-soft);
		font-size: 0.72rem;
	}

	.trace .fn {
		font-weight: 600;
		color: var(--sage-deep);
	}

	.trace .fn.internal {
		color: var(--ink-soft);
		font-weight: 400;
		font-style: italic;
	}

	.trace .op {
		color: var(--ink-soft);
		font-size: 0.76rem;
		overflow-wrap: anywhere;
	}

	.trace .shape,
	.trace .iter {
		font-size: 0.7rem;
		color: var(--signal);
	}

	.mono {
		font-family: var(--font-mono);
	}

	@media (max-width: 1000px) {
		.pg {
			grid-template-columns: 1fr;
			min-height: 0;
		}

		.rail {
			border-right: 0;
			border-bottom: 1px solid var(--line);
		}

		.rail-head {
			padding: 0.9rem 1rem 0.5rem;
			border-bottom: 0;
		}

		.rail-foot,
		.rail-label {
			display: none;
		}

		.examples {
			display: flex;
			gap: 0.35rem;
			overflow-x: auto;
			scrollbar-width: none;
			padding: 0.25rem 1rem 0.8rem;
		}

		.examples::-webkit-scrollbar {
			display: none;
		}

		.examples button {
			width: auto;
			flex: 0 0 auto;
			display: block;
			padding: 0.5rem 0.8rem;
			border: 1px solid var(--line);
			border-radius: var(--radius);
			font-weight: 500;
			font-size: 0.85rem;
			white-space: nowrap;
		}

		.examples small {
			display: none;
		}

		.examples button.active {
			border-color: var(--ink);
			background: var(--ink);
			color: var(--paper);
		}

		.split {
			grid-template-columns: 1fr;
		}

		.editor-wrap {
			border-right: 0;
			border-bottom: 1px solid var(--line);
		}

		/* Let the editor grow with its content: highlight layer in flow, textarea painted over it. */
		.editor {
			min-height: 10rem;
		}

		.editor .hl {
			position: relative;
			overflow: hidden;
		}

		.editor textarea {
			overflow: hidden;
		}

		.run kbd {
			display: none;
		}

		.result-body {
			min-height: 12rem;
		}

		/* Run bar moves to the thumb: sticky at the bottom of the viewport. */
		.toolbar {
			order: 3;
			position: sticky;
			bottom: 0;
			z-index: 5;
			padding: 0.6rem 1rem;
			background: var(--paper);
			border-bottom: 0;
			border-top: 1px solid var(--line);
		}

		.right {
			flex: 1;
			justify-content: flex-end;
		}

		.run {
			flex: 1;
			justify-content: center;
			min-height: 2.75rem;
		}

		.ghost {
			min-height: 2.75rem;
		}

		.tabs button {
			padding: 0.8rem 1rem;
		}
	}

	@media (max-width: 640px) {
		.pill.mono {
			display: none;
		}

		.trace li {
			grid-template-columns: 2rem minmax(0, 1fr) auto;
			gap: 0.5rem;
			padding-left: calc(1rem + var(--depth) * 0.6rem);
			padding-right: 1rem;
		}

		.trace .shape,
		.trace .iter {
			display: none;
		}

		.editor .hl,
		.editor textarea,
		.args-editor,
		.json {
			padding-left: 1rem;
			padding-right: 1rem;
		}
	}
</style>
