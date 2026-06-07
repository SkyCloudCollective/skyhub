<script lang="ts">
	import { onMount } from 'svelte';
	import { t } from '$lib/i18n';
	import { getFacets, getFeed, search, type Facets, type Sample, type SearchParams, type SearchResult } from '$lib/api';
	import { loadCommunity } from '$lib/collections';
	import SampleCard from '$lib/components/SampleCard.svelte';
	import FacetSidebar from '$lib/components/FacetSidebar.svelte';
	import Crates from '$lib/components/Crates.svelte';

	let q = $state('');
	let filters = $state<SearchParams>({});
	let facets = $state<Facets | null>(null);
	let result = $state<SearchResult | null>(null);
	let feed = $state<Sample[]>([]);
	let loading = $state(true);
	let offline = $state(false);

	onMount(async () => {
		try {
			facets = await getFacets();
			await loadCommunity();
			feed = await getFeed().catch(() => []);
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

{#if feed.length}
	<section class="feed glass">
		<h2>From people you follow</h2>
		<div class="feed-grid">
			{#each feed.slice(0, 6) as s (s.id)}
				<SampleCard sample={s} />
			{/each}
		</div>
	</section>
{/if}

<div class="layout">
	<div class="rail">
		<FacetSidebar {facets} bind:filters />
		<Crates />
	</div>

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
	.feed {
		padding: var(--space-4);
		margin-bottom: var(--space-4);
		border-radius: var(--radius-md);
	}
	.feed h2 {
		margin: 0 0 var(--space-3);
		font-size: var(--text-sm);
		text-transform: uppercase;
		letter-spacing: var(--tracking-display);
		color: var(--fg-dim);
	}
	.feed-grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
		gap: var(--space-3);
	}
	.layout {
		display: grid;
		grid-template-columns: var(--rail-sidebar) 1fr;
		gap: var(--space-5);
		align-items: start;
	}
	.rail {
		display: flex;
		flex-direction: column;
		gap: var(--space-4);
		position: sticky;
		top: calc(var(--rail-topbar) + var(--space-4));
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
