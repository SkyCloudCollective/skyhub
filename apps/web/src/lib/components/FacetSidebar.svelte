<script lang="ts">
	import type { Facets, SearchParams } from '$lib/api';

	let {
		facets,
		filters = $bindable()
	}: { facets: Facets | null; filters: SearchParams } = $props();

	function reset() {
		filters = {};
	}

	// String-valued facet selects (bpm + key handled separately below).
	type StrKey = 'category' | 'instrument' | 'contributor' | 'pack' | 'tag';
	const selects: { key: StrKey; label: string; opts: () => string[] }[] = [
		{ key: 'category', label: 'category', opts: () => facets?.categories ?? [] },
		{ key: 'instrument', label: 'instrument', opts: () => facets?.instruments ?? [] },
		{ key: 'contributor', label: 'contributor', opts: () => facets?.contributors ?? [] },
		{ key: 'pack', label: 'pack', opts: () => facets?.packs ?? [] },
		{ key: 'tag', label: 'tag', opts: () => facets?.tags ?? [] }
	];
</script>

<aside class="facets glass">
	<div class="head">
		<h2>Filters</h2>
		<button class="ghost" onclick={reset}>Reset</button>
	</div>

	{#each selects as s (s.key)}
		{#if s.opts().length}
			<label>
				<span class="muted">{s.label}</span>
				<select bind:value={filters[s.key]}>
					<option value={undefined}>any</option>
					{#each s.opts() as o (o)}
						<option value={o}>{o}</option>
					{/each}
				</select>
			</label>
		{/if}
	{/each}

	<label>
		<span class="muted">key</span>
		<input type="text" placeholder="e.g. Am" bind:value={filters.key} />
	</label>

	<fieldset class="bpm">
		<span class="muted">BPM</span>
		<div>
			<input type="number" min="0" placeholder="min" bind:value={filters.bpm_min} />
			<input type="number" min="0" placeholder="max" bind:value={filters.bpm_max} />
		</div>
	</fieldset>
</aside>

<style>
	.facets {
		padding: var(--space-4);
		display: flex;
		flex-direction: column;
		gap: var(--space-3);
		align-self: start;
	}
	.head {
		display: flex;
		align-items: center;
		justify-content: space-between;
	}
	.head h2 {
		margin: 0;
		font-size: var(--text-md);
	}
	label,
	.bpm {
		display: flex;
		flex-direction: column;
		gap: var(--space-1);
		border: 0;
		padding: 0;
		margin: 0;
	}
	label span,
	.bpm span {
		font-size: var(--text-xs);
		text-transform: uppercase;
		letter-spacing: 0.04em;
	}
	select,
	input {
		width: 100%;
	}
	.bpm > div {
		display: flex;
		gap: var(--space-2);
	}
	.bpm input {
		width: 50%;
	}
	.ghost {
		font-size: var(--text-xs);
		padding: 2px var(--space-2);
	}
</style>
