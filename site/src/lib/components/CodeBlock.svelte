<script lang="ts">
	import { highlightGr } from '$lib/highlight';

	let {
		code,
		title = '',
		compact = false,
		dark = true
	}: {
		code: string;
		title?: string;
		compact?: boolean;
		dark?: boolean;
	} = $props();

	const html = $derived(highlightGr(code.replace(/\n$/, '')));
</script>

<div class="code" class:compact class:dark>
	{#if title}
		<div class="bar">
			<span class="dots" aria-hidden="true"><i></i><i></i><i></i></span>
			<span class="title">{title}</span>
		</div>
	{/if}
	<pre><code>{@html html}</code></pre>
</div>

<style>
	.code {
		border: 1px solid var(--line);
		background: color-mix(in srgb, var(--mist) 90%, var(--paper-deep));
		color: var(--ink);
		overflow: hidden;
	}

	.code.dark {
		background: var(--code-dark);
		color: #ece8df;
		border-color: #2c2a27;
	}

	.bar {
		display: flex;
		align-items: center;
		gap: 0.6rem;
		padding: 0.55rem 0.9rem;
		border-bottom: 1px solid color-mix(in srgb, currentColor 15%, transparent);
		font-family: var(--font-mono);
		font-size: 0.72rem;
		letter-spacing: 0.04em;
		opacity: 0.9;
	}

	.dots {
		display: inline-flex;
		gap: 0.3rem;
	}

	.dots i {
		width: 0.5rem;
		height: 0.5rem;
		border-radius: 50%;
		background: currentColor;
		opacity: 0.35;
	}

	pre {
		margin: 0;
		padding: 1rem 1.1rem;
		overflow-x: auto;
		font-family: var(--font-mono);
		font-size: 0.86rem;
		line-height: 1.55;
		tab-size: 2;
	}

	.compact pre {
		padding: 0.8rem 0.95rem;
		font-size: 0.8rem;
	}

	/* light palette */
	.code :global(.t-kw) { color: #2f6f4a; font-weight: 500; }
	.code :global(.t-def) { color: #1f3528; font-weight: 600; }
	.code :global(.t-type) { color: #6a4f2b; }
	.code :global(.t-str) { color: #8a4b2a; }
	.code :global(.t-interp) { color: #b85c38; font-weight: 500; }
	.code :global(.t-var) { color: #3b5f8a; }
	.code :global(.t-dir) { color: #7a5a1e; }
	.code :global(.t-num) { color: #8a4b2a; }
	.code :global(.t-lit) { color: #8a4b2a; }
	.code :global(.t-pipe) { color: #2f6f4a; font-weight: 700; }
	.code :global(.t-arrow) { color: #2f6f4a; font-weight: 600; }
	.code :global(.t-mod) { color: #4a5c4e; }
	.code :global(.t-fn) { color: #1f3528; }
	.code :global(.t-key) { color: #4a5c4e; }
	.code :global(.t-cm) { color: #7a877c; font-style: italic; }
	.code :global(.t-p) { opacity: 0.7; }

	/* dark palette */
	.dark :global(.t-kw) { color: #a8d5b3; }
	.dark :global(.t-def) { color: #f6efe0; font-weight: 600; }
	.dark :global(.t-type) { color: #e8c38f; }
	.dark :global(.t-str) { color: #f0b48a; }
	.dark :global(.t-interp) { color: #ffd2a8; }
	.dark :global(.t-var) { color: #9ec9f5; }
	.dark :global(.t-dir) { color: #e9d18a; }
	.dark :global(.t-num) { color: #f0b48a; }
	.dark :global(.t-lit) { color: #f0b48a; }
	.dark :global(.t-pipe) { color: #8ff0b8; font-weight: 700; }
	.dark :global(.t-arrow) { color: #8ff0b8; }
	.dark :global(.t-mod) { color: #bfd3c4; }
	.dark :global(.t-fn) { color: #f6efe0; }
	.dark :global(.t-key) { color: #cfe0d3; }
	.dark :global(.t-cm) { color: #8faa96; font-style: italic; }
	.dark :global(.t-p) { color: #bfd3c4; }
</style>
