<script lang="ts">
	import CodeBlock from './CodeBlock.svelte';
	import { highlightJson } from '$lib/highlight';
	import { SNIPPETS } from '$lib/snippets';

	let active = $state(SNIPPETS[1]!.id);
	const current = $derived(SNIPPETS.find((s) => s.id === active) ?? SNIPPETS[0]!);
</script>

<div class="tour">
	<div class="tabs" role="tablist" aria-label="More programs">
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
			<a class="open" href={`/playground?example=${current.id}`}>Open in playground →</a>
		</div>
		<div class="code">
			<CodeBlock code={current.source} title={`${current.id}.gr`} compact />
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
	.tabs {
		display: flex;
		flex-wrap: wrap;
		gap: 0 1.4rem;
		border-bottom: 1px solid var(--line);
	}

	.tabs button {
		padding: 0.55rem 0 0.6rem;
		margin-bottom: -1px;
		border: 0;
		border-bottom: 2px solid transparent;
		background: transparent;
		font-family: var(--font-mono);
		font-size: 0.82rem;
		color: var(--ink-soft);
		cursor: pointer;
		white-space: nowrap;
	}

	.tabs button.active {
		color: var(--sage-deep);
		border-bottom-color: var(--sage-deep);
	}

	.tabs button:hover:not(.active) {
		color: var(--sage-deep);
	}

	.panel {
		display: grid;
		grid-template-columns: minmax(0, 0.7fr) minmax(0, 1.3fr);
		gap: 2.5rem;
		padding-top: 1.5rem;
	}

	.copy h3 {
		margin: 0 0 0.5rem;
		font-size: 1.25rem;
		font-weight: 600;
		letter-spacing: -0.01em;
		color: var(--sage-deep);
	}

	.copy p {
		margin: 0 0 1rem;
		color: var(--ink-soft);
	}

	.open {
		font-family: var(--font-mono);
		font-size: 0.85rem;
		text-decoration: none;
		color: var(--sage);
	}

	.out {
		border-top: 1px solid var(--line);
		padding: 0.7rem 0 0;
		font-family: var(--font-mono);
		font-size: 0.8rem;
	}

	.out-label {
		font-size: 0.7rem;
		letter-spacing: 0.06em;
		text-transform: uppercase;
		color: var(--ink-soft);
		margin-bottom: 0.4rem;
	}

	.out-label span {
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
			grid-template-columns: minmax(0, 1fr);
			gap: 1.2rem;
		}
	}
</style>
