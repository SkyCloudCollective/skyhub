<script lang="ts">
	import { goto } from '$app/navigation';
	import { myCollections, newCrate, addToCrate } from '$lib/collections';
	import { readDraggedSampleId, RS_DRAG_MIME } from '$lib/drag';

	let dragOver = $state<number | null>(null);
	let creating = $state(false);
	let name = $state('');

	function canDrop(e: DragEvent): boolean {
		return !!e.dataTransfer && Array.from(e.dataTransfer.types).includes(RS_DRAG_MIME);
	}
	function over(e: DragEvent, cid: number) {
		if (canDrop(e)) {
			e.preventDefault();
			dragOver = cid;
		}
	}
	async function drop(e: DragEvent, cid: number) {
		e.preventDefault();
		dragOver = null;
		const id = readDraggedSampleId(e);
		if (id != null) await addToCrate(cid, id);
	}
	async function create() {
		const n = name.trim();
		if (!n) return;
		await newCrate(n);
		name = '';
		creating = false;
	}
	function icon(kind: string): string {
		return kind === 'favorites' ? '★' : '▦';
	}
</script>

<aside class="crates glass">
	<div class="head">
		<h2>Crates</h2>
		<button class="ghost" onclick={() => (creating = !creating)} aria-label="New crate">＋</button>
	</div>

	{#if creating}
		<form
			onsubmit={(e) => {
				e.preventDefault();
				create();
			}}
		>
			<input bind:value={name} placeholder="Crate name…" aria-label="New crate name" />
		</form>
	{/if}

	<ul>
		{#each $myCollections as c (c.id)}
			<li
				class="crate"
				class:over={dragOver === c.id}
				ondragover={(e) => over(e, c.id)}
				ondragleave={() => (dragOver = null)}
				ondrop={(e) => drop(e, c.id)}
			>
				<button class="open" onclick={() => goto(`/crate/${c.id}`)}>
					<span class="ic" aria-hidden="true">{icon(c.kind)}</span>
					<span class="nm">{c.name}</span>
					<span class="ct muted">{c.count ?? 0}</span>
				</button>
			</li>
		{/each}
		{#if !$myCollections.length}
			<li class="empty muted">Drag a sample here to start a crate.</li>
		{/if}
	</ul>
</aside>

<style>
	.crates {
		padding: var(--space-4);
		display: flex;
		flex-direction: column;
		gap: var(--space-3);
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
	ul {
		list-style: none;
		margin: 0;
		padding: 0;
		display: flex;
		flex-direction: column;
		gap: var(--space-1);
	}
	.crate {
		border-radius: var(--radius-sm);
		border: 1px solid transparent;
	}
	.crate.over {
		border-color: var(--accent);
		background: color-mix(in srgb, var(--accent) 14%, transparent);
	}
	.open {
		width: 100%;
		display: grid;
		grid-template-columns: 20px 1fr auto;
		align-items: center;
		gap: var(--space-2);
		padding: var(--space-2) var(--space-3);
		background: transparent;
		border: 0;
		border-radius: var(--radius-sm);
		text-align: left;
		color: var(--fg);
	}
	.open:hover {
		background: var(--surface);
	}
	.nm {
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}
	.ct {
		font-size: var(--text-xs);
	}
	.empty {
		font-size: var(--text-sm);
		padding: var(--space-2);
	}
	input {
		width: 100%;
	}
	.ghost {
		font-size: var(--text-md);
		padding: 0 var(--space-2);
	}
</style>
