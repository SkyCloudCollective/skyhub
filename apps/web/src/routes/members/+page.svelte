<script lang="ts">
	import { onMount } from 'svelte';
	import { getMembers, followUser, type Member } from '$lib/api';

	let members = $state<Member[]>([]);
	let offline = $state(false);

	async function load() {
		try {
			members = await getMembers();
			offline = false;
		} catch {
			offline = true;
		}
	}
	onMount(load);

	async function toggleFollow(m: Member) {
		const res = await followUser(m.handle, !m.you_follow);
		if (res)
			members = members.map((x) =>
				x.handle === m.handle ? { ...x, you_follow: res.you_follow, followers: res.followers } : x
			);
	}
</script>

<header class="glass head">
	<h1>Members</h1>
	<p class="muted">Everyone in the collective — follow people whose sounds you like.</p>
</header>

{#if offline}
	<p class="glass pad warn">Library offline — start the API (just dev-api).</p>
{/if}

<div class="grid">
	{#each members as m (m.handle)}
		<article class="card glass">
			<a class="who" href={`/u/${m.handle}`}>
				<span class="avatar" aria-hidden="true">{(m.display_name ?? m.handle).slice(0, 1).toUpperCase()}</span>
				<span class="names">
					<span class="dn">{m.display_name ?? m.handle}</span>
					<span class="handle muted">@{m.handle}</span>
				</span>
			</a>
			{#if m.bio}<p class="bio muted">{m.bio}</p>{/if}
			<p class="counts muted">
				{m.sample_count} sample{m.sample_count === 1 ? '' : 's'} · {m.followers} follower{m.followers === 1 ? '' : 's'}
			</p>
			{#if !m.is_me}
				<button
					class:accent-fill={!m.you_follow}
					class:ghost={m.you_follow}
					onclick={() => toggleFollow(m)}
				>
					{m.you_follow ? 'Following' : 'Follow'}
				</button>
			{/if}
		</article>
	{:else}
		{#if !offline}<p class="muted">No members yet.</p>{/if}
	{/each}
</div>

<style>
	.head {
		padding: var(--space-5);
		border-radius: var(--radius-md);
		margin-bottom: var(--space-4);
	}
	.head h1 {
		margin: 0 0 var(--space-1);
	}
	.pad {
		padding: var(--space-4);
		border-radius: var(--radius-md);
		margin-bottom: var(--space-4);
	}
	.warn {
		color: var(--c-warn);
	}
	.grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(240px, 1fr));
		gap: var(--space-4);
	}
	.card {
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
		padding: var(--space-4);
		border-radius: var(--radius-md);
	}
	.who {
		display: flex;
		align-items: center;
		gap: var(--space-3);
		text-decoration: none;
		color: var(--fg);
	}
	.avatar {
		width: 40px;
		height: 40px;
		border-radius: 50%;
		display: grid;
		place-items: center;
		font-family: var(--font-display);
		font-weight: 600;
		background: var(--accent);
		color: var(--accent-ink);
	}
	.names {
		display: flex;
		flex-direction: column;
	}
	.dn {
		font-weight: 600;
	}
	.handle {
		font-size: var(--text-sm);
	}
	.bio {
		font-size: var(--text-sm);
		margin: 0;
	}
	.counts {
		font-size: var(--text-xs);
		margin: 0;
	}
	.card button {
		align-self: flex-start;
		font-size: var(--text-sm);
	}
</style>
