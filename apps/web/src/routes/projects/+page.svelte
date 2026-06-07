<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import {
		listProjects,
		createProject,
		myInvites,
		acceptInvite,
		declineInvite,
		getMe,
		type Project,
		type Invite
	} from '$lib/api';

	let projects = $state<Project[]>([]);
	let invites = $state<Invite[]>([]);
	let offline = $state(false);
	let openUgc = $state(false); // E1 gate — public/open collaboration availability

	// create form
	let title = $state('');
	let daw = $state('ableton');
	let visibility = $state('private');

	async function load() {
		try {
			projects = await listProjects(); // critical call gates the offline state
			offline = false;
		} catch {
			offline = true;
			return;
		}
		try {
			invites = await myInvites(); // best-effort; never blanks the page
		} catch {
			invites = [];
		}
		openUgc = await getMe().then((m) => m.open_ugc).catch(() => false);
	}
	onMount(load);

	async function create() {
		const t = title.trim();
		if (!t) return;
		const p = await createProject({ title: t, daw, visibility });
		title = '';
		if (p?.slug) goto(`/project/${p.slug}`);
		else load();
	}

	async function respond(id: number, accept: boolean) {
		if (accept) await acceptInvite(id);
		else await declineInvite(id);
		load();
	}

	function badge(v: string): string {
		return v === 'open' ? 'open' : v === 'unlisted' ? 'link' : 'private';
	}
</script>

<header class="glass head">
	<div>
		<h1>Projects</h1>
		<p class="muted">Collaborate on tracks, loop packs and full DAW projects — like a GitHub of music.</p>
	</div>
</header>

{#if offline}
	<p class="glass pad warn">Library offline — start the API (just dev-api).</p>
{/if}

{#if invites.length}
	<section class="glass pad invites">
		<h2>Invitations</h2>
		{#each invites as i (i.id)}
			<div class="invite">
				<span><strong>{i.title}</strong> · invited by @{i.invited_by} as {i.role}</span>
				<span class="actions">
					<button class="accent-fill" onclick={() => respond(i.id, true)}>Accept</button>
					<button class="ghost" onclick={() => respond(i.id, false)}>Decline</button>
				</span>
			</div>
		{/each}
	</section>
{/if}

<section class="glass pad create">
	<h2>New project</h2>
	<form onsubmit={(e) => { e.preventDefault(); create(); }}>
		<input placeholder="Project title…" bind:value={title} aria-label="Title" />
		<select bind:value={daw} aria-label="DAW">
			<option value="ableton">Ableton</option>
			<option value="flstudio">FL Studio</option>
			<option value="bitwig">Bitwig</option>
			<option value="other">Other</option>
		</select>
		<select bind:value={visibility} aria-label="Visibility">
			<option value="private">Private (invite only)</option>
			<option value="unlisted">Unlisted (link)</option>
			{#if openUgc}<option value="open">Open (free collab)</option>{/if}
		</select>
		<button class="accent-fill" type="submit">Create</button>
	</form>
</section>

<div class="grid">
	{#each projects as p (p.id)}
		<a class="card glass" href={`/project/${p.slug ?? p.id}`}>
			<div class="row1">
				<span class="t">{p.title}</span>
				<span class="vis {p.visibility}">{badge(p.visibility)}</span>
			</div>
			<p class="muted by">@{p.owner}{p.daw ? ` · ${p.daw}` : ''}{p.role ? ` · ${p.role}` : ''}</p>
			<p class="muted counts">{p.members ?? 0} member{p.members === 1 ? '' : 's'} · {p.files ?? 0} file{p.files === 1 ? '' : 's'}</p>
		</a>
	{:else}
		{#if !offline}<p class="muted">No projects yet. Create one above.</p>{/if}
	{/each}
</div>

<style>
	.head {
		padding: var(--space-5);
		margin-bottom: var(--space-4);
	}
	.head h1 {
		margin-bottom: var(--space-1);
	}
	.pad {
		padding: var(--space-5);
	}
	.warn {
		color: var(--c-warn);
		margin-bottom: var(--space-4);
	}
	.invites,
	.create {
		margin-bottom: var(--space-4);
	}
	.invites h2,
	.create h2 {
		margin-top: 0;
		font-size: var(--text-md);
	}
	.invite {
		display: flex;
		justify-content: space-between;
		align-items: center;
		gap: var(--space-3);
		padding: var(--space-2) 0;
	}
	.invite .actions {
		display: flex;
		gap: var(--space-2);
	}
	form {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-3);
	}
	form input {
		flex: 1;
		min-width: 200px;
	}
	.grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(260px, 1fr));
		gap: var(--space-4);
	}
	.card {
		padding: var(--space-4);
		text-decoration: none;
		color: var(--fg);
	}
	.row1 {
		display: flex;
		justify-content: space-between;
		align-items: baseline;
		gap: var(--space-2);
	}
	.t {
		font-weight: 600;
	}
	.by,
	.counts {
		margin: var(--space-1) 0 0;
		font-size: var(--text-sm);
	}
	.vis {
		font-size: var(--text-xs);
		padding: 1px var(--space-2);
		border-radius: var(--radius-pill);
		border: 1px solid var(--line);
	}
	.vis.open {
		color: var(--c-good);
	}
	.vis.private {
		color: var(--muted);
	}
</style>
