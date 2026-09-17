<script lang="ts">
	import { highlightGrLines } from '$lib/highlight';

	let {
		code,
		title = '',
		compact = false
	}: {
		code: string;
		title?: string;
		compact?: boolean;
	} = $props();

	const html = $derived(highlightGrLines(code.replace(/\n$/, '')));
</script>

<div class="code" class:compact>
	{#if title}
		<div class="bar">{title}</div>
	{/if}
	<pre><code>{@html html}</code></pre>
</div>

<style>
	.code {
		background: var(--panel);
		color: var(--panel-fg);
		overflow: hidden;
	}

	.bar {
		padding: 0.55rem 1.1rem;
		border-bottom: 1px solid var(--panel-line);
		font-family: var(--font-mono);
		font-size: 0.7rem;
		letter-spacing: 0.06em;
		color: var(--panel-muted);
	}

	pre {
		margin: 0;
		padding: 1rem 1.1rem 1.1rem;
		font-family: var(--font-mono);
		font-size: 0.84rem;
		line-height: 1.6;
		tab-size: 2;
		white-space: pre-wrap;
		overflow-wrap: break-word;
	}

	/* One block per source line; wrapped continuations hang 2ch past the line's own indent. */
	pre :global(.ln) {
		display: block;
		padding-left: calc((var(--in, 0) + 2) * 1ch);
		text-indent: calc((var(--in, 0) + 2) * -1ch);
	}

	.compact pre {
		padding: 0.8rem 1.1rem 0.9rem;
		font-size: 0.8rem;
	}

	.code :global(.t-kw) { color: #a8d5b3; }
	.code :global(.t-def) { color: #f6efe0; font-weight: 600; }
	.code :global(.t-type) { color: #e8c38f; }
	.code :global(.t-str) { color: #f0b48a; }
	.code :global(.t-interp) { color: #ffd2a8; }
	.code :global(.t-var) { color: #9ec9f5; }
	.code :global(.t-dir) { color: #e9d18a; }
	.code :global(.t-num) { color: #f0b48a; }
	.code :global(.t-lit) { color: #f0b48a; }
	.code :global(.t-pipe) { color: #8ff0b8; font-weight: 700; }
	.code :global(.t-arrow) { color: #8ff0b8; }
	.code :global(.t-mod) { color: #bfd3c4; }
	.code :global(.t-fn) { color: #f6efe0; }
	.code :global(.t-key) { color: #cfe0d3; }
	.code :global(.t-cm) { color: #8faa96; font-style: italic; }
	.code :global(.t-p) { color: #8e9a91; }
</style>
