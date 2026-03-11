<script>
	import '../app.css';

	let { children } = $props();
</script>

<div class="app-shell">
	<header class="app-header">
		<a href="/upload" class="logo">
			<svg width="24" height="24" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
				<rect width="24" height="24" rx="6" fill="var(--accent)" />
				<polygon points="10,7 18,12 10,17" fill="white" />
			</svg>
			<span>StreamDrop</span>
		</a>
	</header>

	<main class="app-main">
		{@render children()}
	</main>
</div>

<style>
	.app-shell {
		min-height: 100vh;
		display: flex;
		flex-direction: column;
	}

	.app-header {
		padding: 16px 32px;
		border-bottom: 1px solid var(--border);
		background: var(--bg-surface);
	}

	.logo {
		display: inline-flex;
		align-items: center;
		gap: 10px;
		font-size: 1.1rem;
		font-weight: 700;
		color: var(--text-primary);
		letter-spacing: -0.02em;
	}

	.logo:hover {
		color: var(--text-primary);
	}

	.app-main {
		flex: 1;
		display: flex;
		align-items: center;
		justify-content: center;
		padding: 40px 24px;
	}
</style>
