<script lang="ts">
	import { onMount } from 'svelte';
	import { page } from '$app/stores';
	import { goto } from '$app/navigation';
	import {
		getCollection,
		getMe,
		patchCollection,
		removeItem,
		deleteCollection,
		downloadUrl,
		type CollectionFull
	} from '$lib/api';
	import { toggle, playingId } from '$lib/player';

	let data = $state<CollectionFull | null>(null);
	let me = $state<string>('');
	let error = $state<string | null>(null);

	const id = $derived(parseInt($page.params.id ?? '0', 10));
	const token = $derived($page.url.searchParams.get('token') ?? undefined);
	const isOwner = $derived(!!data && data.collection.owner === me);

	async function load() {
		error = null;
		try {
			me = (await getMe()).handle;
			data = await getCollection(id, token);
		} catch (e) {
			error = (e as Error).message.startsWith('403') ? 'This crate is private.' : 'Crate not found.';
			data = null;
		}
	}
	onMount(load);

	async function setVisibility(v: string) {
		if (!data) return;
		const c = await patchCollection(id, { visibility: v });
		if (c) data = { ...data, collection: c };
	}
	async function remove(sampleId: number) {
		await removeItem(id, sampleId);
		if (data) data = { ...data, items: data.items.filter((i) => i.id !== sampleId) };
	}
	async function destroy() {
		if (!data || data.collection.kind === 'favorites') return;
		await deleteCollection(id);
		goto('/');
	}
	function shareLink(): string {
		if (!data?.collection.share_token) return '';
		return `${location.origin}/crate/${id}?token=${data.collection.share_token}`;
	}
</script>

{#if error}
	<section class="glass pad">
		<p class="warn">{error}</p>
		<a href="/">← Library</a>
	</section>
{:else if data}
	<header class="head glass pad">
		<div>
			<a class="back" href="/">← Library</a>
			<h1>{data.collection.kind === 'favorites' ? '★ ' : ''}{data.collection.name}</h1>
			<p class="muted">@{data.collection.owner} · {data.items.length} sample{data.items.length === 1 ? '' : 's'}</p>
		</div>
		{#if isOwner}
			<div class="owner">
				<label>
					<span class="muted">visibility</span>
					<select value={data.collection.visibility} onchange={(e) => setVisibility(e.currentTarget.value)}>
						<option value="private">private</option>
						<option value="unlisted">unlisted (link)</option>
						<option value="public">public</option>
					</select>
				</label>
				{#if data.collection.kind !== 'favorites'}
					<button class="ghost" onclick={destroy}>Delete crate</button>
				{/if}
			</div>
		{/if}
	</header>

	{#if isOwner && data.collection.visibility === 'unlisted' && data.collection.share_token}
		<p class="share glass pad">Share link: <code>{shareLink()}</code></p>
	{/if}

	<ul class="items">
		{#each data.items as it (it.id)}
			<li class="row glass">
				<button class="play" class:on={$playingId === it.id} onclick={() => toggle(it.id)} aria-label="Play">
					{$playingId === it.id ? '❚❚' : '▶'}
				</button>
				<span class="t">{it.title}</span>
				<span class="meta muted">
					{it.category ?? ''}{it.musical_key ? ` · ${it.musical_key}` : ''}{it.bpm ? ` · ${Math.round(it.bpm)} BPM` : ''}
				</span>
				<a class="dl" href={downloadUrl(it.id)} download={it.filename} aria-label="Download">↓</a>
				{#if isOwner}
					<button class="rm" onclick={() => remove(it.id)} aria-label="Remove">✕</button>
				{/if}
			</li>
		{:else}
			<li class="empty muted glass pad">Empty. Drag samples here from the library.</li>
		{/each}
	</ul>
{/if}

<style>
	.pad {
		padding: var(--space-5);
	}
	.head {
		display: flex;
		justify-content: space-between;
		align-items: flex-start;
		gap: var(--space-4);
		margin-bottom: var(--space-4);
	}
	.head h1 {
		margin: var(--space-2) 0 var(--space-1);
	}
	.back {
		font-size: var(--text-sm);
	}
	.owner {
		display: flex;
		gap: var(--space-3);
		align-items: center;
	}
	.owner label {
		display: flex;
		flex-direction: column;
		gap: var(--space-1);
		font-size: var(--text-xs);
	}
	.share {
		margin-bottom: var(--space-4);
		font-size: var(--text-sm);
		word-break: break-all;
	}
	.items {
		list-style: none;
		margin: 0;
		padding: 0;
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
	}
	.row {
		display: grid;
		grid-template-columns: 40px 1fr auto auto auto;
		align-items: center;
		gap: var(--space-3);
		padding: var(--space-2) var(--space-4);
	}
	.play {
		width: 36px;
		height: 36px;
		border-radius: var(--radius-pill);
	}
	.play.on {
		background: var(--accent);
		color: var(--accent-ink);
	}
	.t {
		font-weight: 600;
	}
	.meta {
		font-size: var(--text-sm);
	}
	.dl,
	.rm {
		background: transparent;
		border: 0;
		color: var(--fg-dim);
		text-decoration: none;
		font-size: 16px;
	}
	.rm:hover {
		color: var(--accent-2);
	}
	.warn {
		color: var(--c-warn);
	}
</style>
