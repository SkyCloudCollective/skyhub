<script lang="ts">
	import { onMount } from 'svelte';
	import { t } from '$lib/i18n';
	import { getFacets, search, type Facets, type SearchParams, type SearchResult } from '$lib/api';
	import SampleCard from '$lib/components/SampleCard.svelte';
	import FacetSidebar from '$lib/components/FacetSidebar.svelte';

	let q = $state('');
	let filters = $state<SearchParams>({});
	let facets = $state<Facets | null>(null);
	let result = $state<SearchResult | null>(null);
	let loading = $state(true);
	let offline = $state(false);

	onMount(async () => {
		try {
			facets = await getFacets();
		} catch {
			offline = true;
		}
	});

	async function runSearch() {
		loading = true;
		try {
			result = await search({ ...filters, q: q || undefined, limit: 100 });
			offline = false;
		} catch {
			offline = true;
			result = null;
		}
		loading = false;
	}

	// debounced search on any query/filter change (the JSON read registers deps)
	let timer: ReturnType<typeof setTimeout>;
	$effect(() => {
		void (q + '|' + JSON.stringify(filters));
		clearTimeout(timer);
		timer = setTimeout(runSearch, 200);
		return () => clearTimeout(timer);
	});
</script>

<header class="bar glass">
	<input
		class="q"
		type="search"
		placeholder="Search samples…"
		bind:value={q}
		aria-label="Search"
	/>
	<span class="count muted">
		{#if loading}…{:else if result}{result.total} result{result.total === 1 ? '' : 's'}{/if}
	</span>
</header>

{#if offline}
	<p class="status off glass">● {$t('home.status.offline')}</p>
{/if}

<div class="layout">
	<FacetSidebar {facets} bind:filters />

	<section class="results">
		{#if result && result.hits.length}
			{#each result.hits as s (s.id)}
				<SampleCard sample={s} />
			{/each}
		{:else if !loading && !offline}
			<p class="empty muted glass">No samples match. Try clearing filters.</p>
		{/if}
	</section>
</div>

<style>
	.bar {
		display: flex;
		align-items: center;
		gap: var(--space-4);
		padding: var(--space-3) var(--space-4);
		margin-bottom: var(--space-4);
	}
	.q {
		flex: 1;
		font-size: var(--text-md);
		background: var(--surface);
	}
	.count {
		font-size: var(--text-sm);
		white-space: nowrap;
	}
	.status.off {
		color: var(--c-warn);
		padding: var(--space-3) var(--space-4);
		margin-bottom: var(--space-4);
	}
	.layout {
		display: grid;
		grid-template-columns: var(--rail-sidebar) 1fr;
		gap: var(--space-5);
		align-items: start;
	}
	.results {
		display: flex;
		flex-direction: column;
		gap: var(--space-3);
	}
	.empty {
		padding: var(--space-6);
		text-align: center;
	}
	@media (max-width: 760px) {
		.layout {
			grid-template-columns: 1fr;
		}
	}
</style>
