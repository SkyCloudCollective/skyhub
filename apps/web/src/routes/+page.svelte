<script lang="ts">
	import { onMount } from 'svelte';
	import { t } from '$lib/i18n';
	import { api, type Facets, type Health } from '$lib/api';

	let health = $state<Health | null>(null);
	let facets = $state<Facets | null>(null);
	let online = $state<boolean | null>(null);

	onMount(async () => {
		try {
			health = await api<Health>('/v1/health');
			facets = await api<Facets>('/v1/facets');
			online = true;
		} catch {
			online = false;
		}
	});

	const stats = $derived(
		facets
			? [
					{ n: facets.categories.length, label: $t('facet.categories') },
					{ n: facets.instruments.length, label: $t('facet.instruments') },
					{ n: facets.contributors.length, label: $t('facet.contributors') },
					{ n: facets.packs.length, label: $t('facet.packs') }
				]
			: []
	);
</script>

<section class="hero glass">
	<h1>{$t('home.welcome')}</h1>
	<p class="lead">{$t('home.intro')}</p>

	{#if online === true}
		<p class="status ok">● {$t('home.status.ok')}{health ? ` (v${health.version})` : ''}</p>
	{:else if online === false}
		<p class="status off">● {$t('home.status.offline')}</p>
	{/if}
</section>

{#if stats.length}
	<h2>{$t('home.facets')}</h2>
	<div class="cards">
		{#each stats as s (s.label)}
			<div class="card glass">
				<div class="num">{s.n}</div>
				<div class="lbl muted">{s.label}</div>
			</div>
		{/each}
	</div>
{/if}

<style>
	.hero {
		padding: var(--space-6);
		margin-bottom: var(--space-6);
	}
	.lead {
		font-size: var(--text-lg);
		color: var(--fg-dim);
		max-width: 60ch;
	}
	.status {
		font-size: var(--text-sm);
		margin: 0;
	}
	.status.ok {
		color: var(--c-good);
	}
	.status.off {
		color: var(--c-warn);
	}
	.cards {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(150px, 1fr));
		gap: var(--space-4);
	}
	.card {
		padding: var(--space-5);
		text-align: left;
	}
	.num {
		font-family: var(--font-display);
		font-size: var(--text-3xl);
		line-height: 1;
		color: var(--accent);
	}
	.lbl {
		margin-top: var(--space-2);
		font-size: var(--text-sm);
	}
</style>
