<script lang="ts">
	import CodeBlock from './CodeBlock.svelte';
	import { highlightJson } from '$lib/highlight';
	import { SNIPPETS } from '$lib/snippets';

	let active = $state(SNIPPETS[0]!.id);
	const current = $derived(SNIPPETS.find((s) => s.id === active) ?? SNIPPETS[0]!);
</script>

<div class="tour">
	<div class="tabs" role="tablist" aria-label="Language tour">
		{#each SNIPPETS as s}
			<button
				role="tab"
				type="button"
				class:active={s.id === active}
				aria-selected={s.id === active}
				onclick={() => (active = s.id)}
			>
				{s.label}
			</button>
		{/each}
	</div>

	<div class="panel">
		<div class="copy">
			<h3>{current.title}</h3>
			<p>{current.blurb}</p>
			<ul class="chips">
				{#each current.tags as t}
					<li>{t}</li>
				{/each}
			</ul>
			<a class="open" href={`/playground?example=${current.id}`}>Open in playground →</a>
		</div>
		<div class="code">
			<CodeBlock code={current.source} title={`${current.id}.gr`} dark={false} compact />
			<div class="out" class:bad={current.id === 'policy'}>
				<div class="out-label">
					{current.id === 'policy' ? 'runtime refused' : 'final state'}
					{#if current.steps}<span>· {current.steps} steps</span>{/if}
					{#if current.args}<span>· args {JSON.stringify(current.args)}</span>{/if}
				</div>
				<pre>{@html current.id === 'policy' ? current.output : highlightJson(current.output)}</pre>
			</div>
		</div>
	</div>
</div>

<style>
	.tour {
		border: 1px solid var(--line);
		background: color-mix(in srgb, var(--mist) 70%, transparent);
	}

	.tabs {
		display: flex;
		flex-wrap: wrap;
		border-bottom: 1px solid var(--line);
	}

	.tabs button {
		flex: 1 1 auto;
		padding: 0.75rem 0.9rem;
		border: 0;
		border-right: 1px solid var(--line);
		background: transparent;
		font-family: var(--font-display);
		font-weight: 600;
		font-size: 0.85rem;
		color: var(--ink-soft);
		cursor: pointer;
		white-space: nowrap;
	}

	.tabs button:last-child {
		border-right: 0;
	}

	.tabs button.active {
		background: var(--sage);
		color: var(--mist);
	}

	.tabs button:hover:not(.active) {
		color: var(--sage-deep);
		background: color-mix(in srgb, var(--sage) 8%, transparent);
	}

	.panel {
		display: grid;
		grid-template-columns: 0.8fr 1.2fr;
		gap: 1.5rem;
		padding: 1.5rem;
	}

	.copy h3 {
		margin: 0 0 0.6rem;
		font-family: var(--font-display);
		font-size: 1.35rem;
		letter-spacing: -0.02em;
		color: var(--sage-deep);
	}

	.copy p {
		margin: 0 0 1rem;
		color: var(--ink-soft);
	}

	.chips {
		list-style: none;
		margin: 0 0 1.2rem;
		padding: 0;
		display: flex;
		flex-wrap: wrap;
		gap: 0.4rem;
	}

	.chips li {
		font-family: var(--font-mono);
		font-size: 0.72rem;
		padding: 0.2rem 0.5rem;
		border: 1px solid var(--line);
		color: var(--sage-deep);
		background: color-mix(in srgb, var(--mist) 80%, white);
	}

	.open {
		font-family: var(--font-display);
		font-weight: 700;
		text-decoration: none;
		color: var(--sage);
	}

	.code {
		display: grid;
		gap: 0;
	}

	.out {
		border: 1px solid var(--line);
		border-top: 0;
		background: color-mix(in srgb, var(--mist) 95%, white);
		padding: 0.7rem 0.95rem 0.9rem;
		font-family: var(--font-mono);
		font-size: 0.8rem;
	}

	.out.bad {
		border-left: 3px solid var(--ember);
	}

	.out-label {
		font-size: 0.68rem;
		letter-spacing: 0.08em;
		text-transform: uppercase;
		color: var(--signal);
		margin-bottom: 0.4rem;
	}

	.out-label span {
		color: var(--ink-soft);
		text-transform: none;
		letter-spacing: 0;
		margin-left: 0.3rem;
	}

	.out pre {
		margin: 0;
		white-space: pre-wrap;
		line-height: 1.5;
	}

	.bad pre {
		color: var(--ember);
	}

	.out :global(.t-key) { color: var(--sage-deep); }
	.out :global(.t-str) { color: #8a4b2a; }
	.out :global(.t-num) { color: #3b5f8a; }
	.out :global(.t-lit) { color: #6a4f2b; }

	@media (max-width: 900px) {
		.panel {
			grid-template-columns: 1fr;
		}
	}
</style>
