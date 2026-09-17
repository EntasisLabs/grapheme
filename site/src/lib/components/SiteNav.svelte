<script lang="ts">
	import { resolve } from '$app/paths';

	let {
		compact = false
	}: {
		compact?: boolean;
	} = $props();

	let open = $state(false);
</script>

<header class="nav" class:compact>
	<div class="inner">
	<a class="brand" href={resolve('/')} aria-label="Grapheme home">
		<span class="mark" aria-hidden="true">|&gt;</span>
		<span class="name">grapheme</span>
	</a>

	<button
		class="menu"
		type="button"
		aria-expanded={open}
		aria-controls="site-nav"
		onclick={() => (open = !open)}
	>
		{open ? 'Close' : 'Menu'}
	</button>

	<nav id="site-nav" class:open>
		<a href={resolve('/docs/why-grapheme')}>Docs</a>
		<a href={resolve('/docs/tutorials')}>Tutorials</a>
		<a href={resolve('/playground')}>Playground</a>
		<a class="ext" href="https://github.com/EntasisLabs/grapheme" rel="noreferrer" target="_blank"
			>GitHub</a
		>
	</nav>
	</div>
</header>

<style>
	.nav {
		position: sticky;
		top: 0;
		z-index: 40;
		background: var(--paper);
		border-bottom: 1px solid var(--line);
	}

	.inner {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 1rem;
		height: var(--nav-h);
		max-width: 76rem;
		margin: 0 auto;
		padding: 0 clamp(1rem, 4vw, 3rem);
	}

	.brand {
		display: inline-flex;
		align-items: center;
		gap: 0.55rem;
		text-decoration: none;
		font-weight: 600;
		letter-spacing: -0.01em;
	}

	.mark {
		display: inline-grid;
		place-items: center;
		width: 1.5rem;
		height: 1.5rem;
		border-radius: var(--radius);
		background: var(--ink);
		color: var(--paper);
		font-family: var(--font-mono);
		font-size: 0.7rem;
		font-weight: 600;
		line-height: 1;
	}

	.name {
		font-size: 1rem;
		color: var(--ink);
	}

	nav {
		display: flex;
		align-items: center;
		gap: 1.5rem;
		font-weight: 500;
		font-size: 0.88rem;
		color: var(--ink-soft);
	}

	nav a {
		text-decoration: none;
	}

	nav a:hover {
		color: var(--ink);
	}

	.menu {
		display: none;
		min-height: 2.25rem;
		border: 1px solid var(--line);
		background: transparent;
		padding: 0.35rem 0.8rem;
		border-radius: var(--radius);
		cursor: pointer;
		font-weight: 500;
		color: var(--ink);
	}

	@media (max-width: 720px) {
		.menu {
			display: inline-flex;
		}

		nav {
			display: none;
			position: absolute;
			inset: var(--nav-h) 0 auto 0;
			flex-direction: column;
			align-items: stretch;
			gap: 0;
			padding: 0.25rem 1.25rem 0.75rem;
			background: var(--paper);
			border-bottom: 1px solid var(--line);
			font-size: 1rem;
			color: var(--ink);
		}

		nav.open {
			display: flex;
		}

		nav a {
			padding: 0.8rem 0;
			border-bottom: 1px solid var(--line);
		}

		nav a:last-child {
			border-bottom: 0;
		}
	}
</style>
