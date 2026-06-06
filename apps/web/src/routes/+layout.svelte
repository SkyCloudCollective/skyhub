<script lang="ts">
	import '@ranchsamples/tokens'; // design system: tokens + glass skin + base reset
	import favicon from '$lib/assets/favicon.svg';
	import { theme } from '$lib/theme'; // subscribing applies data-theme to <html>
	import Topbar from '$lib/components/Topbar.svelte';

	let { children } = $props();

	// touch the store so the subscription (which sets data-theme) stays live
	$effect(() => {
		void $theme;
	});
</script>

<svelte:head>
	<link rel="icon" href={favicon} />
</svelte:head>

<Topbar />

<main>
	{@render children()}
</main>

<footer>
	<hr class="bauhaus-rule" />
	<p class="muted">
		RanchSamples · community-owned · AGPL-3.0 · sound design by Tev · meters by polarity
	</p>
</footer>

<style>
	main {
		max-width: var(--content-max);
		margin: 0 auto;
		padding: var(--space-6) var(--space-5) var(--space-10);
	}
	footer {
		max-width: var(--content-max);
		margin: 0 auto;
		padding: var(--space-5);
	}
	footer p {
		font-size: var(--text-xs);
		margin-top: var(--space-3);
	}
	.bauhaus-rule {
		margin: 0;
	}
</style>
