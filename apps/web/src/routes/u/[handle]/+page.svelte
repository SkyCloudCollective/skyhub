<script lang="ts">
	import { onMount } from 'svelte';
	import { page } from '$app/stores';
	import {
		getProfile,
		getMe,
		patchProfile,
		followUser,
		listCollections,
		search,
		getTube,
		type Profile,
		type Collection,
		type Sample,
		type Video
	} from '$lib/api';
	import SampleCard from '$lib/components/SampleCard.svelte';
	import VideoCard from '$lib/components/VideoCard.svelte';

	let profile = $state<Profile | null>(null);
	let collections = $state<Collection[]>([]);
	let samples = $state<Sample[]>([]);
	let videos = $state<Video[]>([]);
	let me = $state('');

	const handle = $derived($page.params.handle ?? '');
	const isMe = $derived(!!handle && handle === me);

	// edit form
	let editing = $state(false);
	let displayName = $state('');
	let bio = $state('');

	async function load() {
		me = (await getMe().catch(() => ({ handle: '' }))).handle;
		profile = await getProfile(handle);
		displayName = profile.display_name ?? '';
		bio = profile.bio ?? '';
		collections = await listCollections(handle).catch(() => []);
		samples = (await search({ contributor: handle, limit: 100 }).catch(() => ({ hits: [] }))).hits;
		videos = await getTube({ handle, limit: 100 }).catch(() => []);
	}
	onMount(load);

	async function save() {
		const p = await patchProfile({ display_name: displayName, bio });
		if (p) profile = { ...profile!, ...p };
		editing = false;
	}

	async function toggleFollow() {
		if (!profile) return;
		const res = await followUser(profile.handle, !profile.you_follow);
		if (res) profile = { ...profile, ...res };
	}
</script>

{#if profile}
	<header class="head glass">
		<div class="avatar" aria-hidden="true">{(profile.display_name ?? profile.handle).slice(0, 1).toUpperCase()}</div>
		<div class="meta">
			<h1>{profile.display_name ?? profile.handle}</h1>
			<p class="muted">
				@{profile.handle} · {profile.sample_count} sample{profile.sample_count === 1 ? '' : 's'}
				· {profile.followers} follower{profile.followers === 1 ? '' : 's'} · {profile.following} following
			</p>
			{#if profile.bio}<p class="bio">{profile.bio}</p>{/if}
		</div>
		{#if isMe}
			<button class="ghost" onclick={() => (editing = !editing)}>Edit</button>
		{:else}
			<button class:accent-fill={!profile.you_follow} class:ghost={profile.you_follow} onclick={toggleFollow}>
				{profile.you_follow ? 'Following' : 'Follow'}
			</button>
		{/if}
	</header>

	{#if editing}
		<form class="edit glass" onsubmit={(e) => { e.preventDefault(); save(); }}>
			<label>Display name<input bind:value={displayName} maxlength="60" /></label>
			<label>Bio<textarea bind:value={bio} rows="3" maxlength="280"></textarea></label>
			<button class="accent-fill" type="submit">Save</button>
		</form>
	{/if}

	{#if collections.length}
		<h2>Crates</h2>
		<ul class="crates">
			{#each collections as c (c.id)}
				<li><a href={`/crate/${c.id}`}>{c.kind === 'favorites' ? '★ ' : '▦ '}{c.name} <span class="muted">({c.count ?? 0})</span></a></li>
			{/each}
		</ul>
	{/if}

	<h2>Samples</h2>
	{#if samples.length}
		<div class="grid">
			{#each samples as s (s.id)}<SampleCard sample={s} />{/each}
		</div>
	{:else}
		<p class="muted">No samples yet.</p>
	{/if}

	{#if videos.length}
		<h2>Channel</h2>
		<div class="videos">
			{#each videos as v (v.id)}<VideoCard video={v} />{/each}
		</div>
	{/if}
{/if}

<style>
	.head {
		display: flex;
		align-items: center;
		gap: var(--space-4);
		padding: var(--space-5);
		margin-bottom: var(--space-5);
	}
	.avatar {
		width: 64px;
		height: 64px;
		border-radius: var(--radius-pill);
		display: grid;
		place-items: center;
		font-family: var(--font-display);
		font-size: var(--text-2xl);
		background: var(--accent);
		color: var(--accent-ink);
	}
	.meta {
		margin-right: auto;
	}
	.meta h1 {
		margin: 0;
	}
	.bio {
		margin: var(--space-2) 0 0;
		max-width: 60ch;
	}
	.edit {
		display: flex;
		flex-direction: column;
		gap: var(--space-3);
		padding: var(--space-5);
		margin-bottom: var(--space-5);
	}
	.edit label {
		display: flex;
		flex-direction: column;
		gap: var(--space-1);
	}
	.crates {
		list-style: none;
		padding: 0;
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-2);
		margin: 0 0 var(--space-5);
	}
	.crates a {
		display: inline-block;
		padding: var(--space-2) var(--space-3);
		border-radius: var(--radius-pill);
		background: var(--surface);
		border: 1px solid var(--line);
		text-decoration: none;
		color: var(--fg);
	}
	.grid {
		display: flex;
		flex-direction: column;
		gap: var(--space-3);
	}
	.videos {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(300px, 1fr));
		gap: var(--space-4);
		margin-top: var(--space-3);
	}
</style>
