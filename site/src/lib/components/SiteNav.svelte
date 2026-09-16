<script lang="ts">
	let {
		compact = false
	}: {
		compact?: boolean;
	} = $props();

	let open = $state(false);
</script>

<header class="nav" class:compact>
	<a class="brand" href="/" aria-label="Grapheme home">
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
		<a href="/docs/why-grapheme">Docs</a>
		<a href="/docs/tutorials">Tutorials</a>
		<a href="/playground">Playground</a>
		<a class="external" href="https://github.com/EntasisLabs/grapheme" rel="noreferrer" target="_blank"
			>GitHub</a
		>
	</nav>
</header>

<style>
	.nav {
		position: sticky;
		top: 0;
		z-index: 40;
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 1rem;
		height: var(--nav-h);
		padding: 0 clamp(1rem, 3vw, 2.5rem);
		backdrop-filter: blur(12px);
		background: color-mix(in srgb, var(--mist) 82%, transparent);
		border-bottom: 1px solid var(--line);
	}

	.brand {
		display: inline-flex;
		align-items: baseline;
		gap: 0.55rem;
		text-decoration: none;
		font-family: var(--font-display);
		font-weight: 800;
		letter-spacing: -0.04em;
	}

	.mark {
		display: inline-grid;
		place-items: center;
		width: 1.7rem;
		height: 1.7rem;
		border-radius: 0.35rem;
		background: var(--sage-deep);
		color: var(--signal-bright);
		font-family: var(--font-mono);
		font-size: 0.7rem;
		font-weight: 700;
		line-height: 1;
	}

	.name {
		font-size: 1.35rem;
	}

	nav {
		display: flex;
		align-items: center;
		gap: 1.4rem;
		font-family: var(--font-display);
		font-weight: 600;
		font-size: 0.95rem;
	}

	nav a {
		text-decoration: none;
	}

	.external::after {
		content: ' ↗';
		font-size: 0.8em;
		opacity: 0.7;
	}

	.menu {
		display: none;
		border: 1px solid var(--line);
		background: var(--mist);
		padding: 0.4rem 0.75rem;
		border-radius: var(--radius);
		cursor: pointer;
		font-family: var(--font-display);
		font-weight: 600;
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
			padding: 0.5rem 1rem 1rem;
			background: var(--mist);
			border-bottom: 1px solid var(--line);
		}

		nav.open {
			display: flex;
		}

		nav a {
			padding: 0.75rem 0;
			border-bottom: 1px solid var(--line);
		}
	}
</style>
