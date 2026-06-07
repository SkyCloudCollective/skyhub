<script lang="ts">
	import { onMount } from 'svelte';
	import { page } from '$app/stores';
	import { goto } from '$app/navigation';
	import {
		getProject,
		patchProject,
		deleteProject,
		inviteMember,
		joinProject,
		removeMember,
		uploadProjectFile,
		deleteProjectFile,
		projectFileUrl,
		projectComments,
		addProjectComment,
		type ProjectFull
	} from '$lib/api';

	let data = $state<ProjectFull | null>(null);
	let comments = $state<{ id: number; handle: string; body: string; created_at: string }[]>([]);
	let error = $state<string | null>(null);

	const slug = $derived($page.params.slug ?? '');
	const canEdit = $derived(!!data && (data.role === 'owner' || data.role === 'editor'));
	const isOwner = $derived(!!data && data.role === 'owner');
	const canJoin = $derived(!!data && data.role == null && data.project.visibility === 'open');

	let inviteHandle = $state('');
	let inviteRole = $state('editor');
	let newComment = $state('');

	async function load() {
		error = null;
		try {
			data = await getProject(slug);
			comments = await projectComments(data.project.id);
		} catch (e) {
			const m = (e as Error).message;
			error = m.startsWith('403') ? 'This project is private.' : 'Project not found.';
			data = null;
		}
	}
	onMount(load);

	function kindOf(name: string): string {
		const n = name.toLowerCase();
		if (n.endsWith('.als') || n.endsWith('.flp') || n.endsWith('.bwproject')) return 'project';
		if (n.endsWith('.mid') || n.endsWith('.midi')) return 'midi';
		if (n.includes('stem')) return 'stem';
		if (n.includes('loop')) return 'loop';
		return 'sample';
	}

	async function onFiles(e: Event) {
		const input = e.target as HTMLInputElement;
		if (!data || !input.files) return;
		for (const f of Array.from(input.files)) {
			await uploadProjectFile(data.project.id, f.name, f, kindOf(f.name));
		}
		input.value = '';
		load();
	}

	async function invite() {
		if (!data || !inviteHandle.trim()) return;
		await inviteMember(data.project.id, inviteHandle.trim(), inviteRole).catch(() => {});
		inviteHandle = '';
		load();
	}
	async function join() {
		if (!data) return;
		await joinProject(data.project.id);
		load();
	}
	async function setVisibility(v: string) {
		if (!data) return;
		await patchProject(data.project.id, { visibility: v });
		load();
	}
	async function destroy() {
		if (!data) return;
		await deleteProject(data.project.id);
		goto('/projects');
	}
	async function removeFile(rel: string) {
		if (!data) return;
		await deleteProjectFile(data.project.id, rel);
		load();
	}
	async function kick(handle: string) {
		if (!data) return;
		await removeMember(data.project.id, handle);
		load();
	}
	async function comment() {
		if (!data || !newComment.trim()) return;
		await addProjectComment(data.project.id, newComment.trim());
		newComment = '';
		comments = await projectComments(data.project.id);
	}
	function fmtBytes(b: number | null): string {
		if (!b) return '';
		return b > 1e6 ? `${(b / 1e6).toFixed(1)} MB` : `${(b / 1e3).toFixed(0)} kB`;
	}
</script>

{#if error}
	<section class="glass pad"><p class="warn">{error}</p><a href="/projects">← Projects</a></section>
{:else if data}
	<header class="glass head">
		<div>
			<a class="back" href="/projects">← Projects</a>
			<h1>{data.project.title}</h1>
			<p class="muted">
				@{data.project.owner}
				· <span class="vis {data.project.visibility}">{data.project.visibility}</span>
				{data.project.daw ? ` · ${data.project.daw}` : ''}
				{data.role ? ` · you are ${data.role}` : ''}
			</p>
		</div>
		<div class="head-actions">
			{#if canJoin}
				<button class="accent-fill" onclick={join}>Join</button>
			{/if}
			{#if isOwner}
				<select value={data.project.visibility} onchange={(e) => setVisibility(e.currentTarget.value)} aria-label="Visibility">
					<option value="private">private</option>
					<option value="unlisted">unlisted</option>
					<option value="open">open</option>
				</select>
				<button class="ghost" onclick={destroy}>Delete</button>
			{/if}
		</div>
	</header>

	<div class="cols">
		<section class="main">
			<div class="glass pad">
				<div class="files-head">
					<h2>Files</h2>
					{#if canEdit}
						<label class="upload accent-fill">
							+ Add files
							<input type="file" multiple onchange={onFiles} hidden />
						</label>
					{/if}
				</div>
				<ul class="files">
					{#each data.files as f (f.rel_path)}
						<li>
							<span class="fk">{f.kind ?? 'file'}</span>
							<a class="fn" href={projectFileUrl(data.project.id, f.rel_path)} download>{f.rel_path}</a>
							<span class="muted fmeta">v{f.version}{f.updated_by ? ` · @${f.updated_by}` : ''} · {fmtBytes(f.bytes)}</span>
							{#if canEdit}<button class="rm" onclick={() => removeFile(f.rel_path)} aria-label="Remove">✕</button>{/if}
						</li>
					{:else}
						<li class="muted empty">No files yet.{canEdit ? ' Add stems, loops, or your full project.' : ''}</li>
					{/each}
				</ul>
			</div>

			<div class="glass pad">
				<h2>Discussion</h2>
				{#if data.role}
					<form onsubmit={(e) => { e.preventDefault(); comment(); }}>
						<input placeholder="Leave a note…" bind:value={newComment} />
						<button class="accent-fill" type="submit">Post</button>
					</form>
				{/if}
				<ul class="comments">
					{#each comments as c (c.id)}
						<li><strong>@{c.handle}</strong> {c.body}</li>
					{:else}
						<li class="muted">No comments.</li>
					{/each}
				</ul>
			</div>
		</section>

		<aside class="side">
			<div class="glass pad">
				<h2>Members</h2>
				<ul class="members">
					{#each data.members as m (m.handle)}
						<li>
							<a href={`/u/${m.handle}`}>@{m.handle}</a>
							<span class="muted">{m.role}</span>
							{#if isOwner && m.role !== 'owner'}<button class="rm" onclick={() => kick(m.handle)} aria-label="Remove">✕</button>{/if}
						</li>
					{/each}
				</ul>
				{#if canEdit}
					<form class="invite" onsubmit={(e) => { e.preventDefault(); invite(); }}>
						<input placeholder="invite @handle" bind:value={inviteHandle} />
						<select bind:value={inviteRole}>
							<option value="editor">editor</option>
							<option value="viewer">viewer</option>
						</select>
						<button class="ghost" type="submit">Invite</button>
					</form>
				{/if}
			</div>

			<div class="glass pad">
				<h2>Activity</h2>
				<ul class="activity">
					{#each data.activity as a (a.created_at + a.action + (a.target ?? ''))}
						<li class="muted"><strong>@{a.actor}</strong> {a.action}{a.target ? ` ${a.target}` : ''}</li>
					{:else}
						<li class="muted">No activity yet.</li>
					{/each}
				</ul>
			</div>
		</aside>
	</div>
{/if}

<style>
	.pad { padding: var(--space-5); }
	.head {
		display: flex;
		justify-content: space-between;
		align-items: flex-start;
		gap: var(--space-4);
		padding: var(--space-5);
		margin-bottom: var(--space-5);
	}
	.head h1 { margin: var(--space-2) 0 var(--space-1); }
	.back { font-size: var(--text-sm); }
	.head-actions { display: flex; gap: var(--space-2); align-items: center; }
	.vis { padding: 0 var(--space-2); border-radius: var(--radius-pill); border: 1px solid var(--line); }
	.vis.open { color: var(--c-good); }
	.cols { display: grid; grid-template-columns: 1fr 300px; gap: var(--space-5); align-items: start; }
	.main { display: flex; flex-direction: column; gap: var(--space-4); }
	.side { display: flex; flex-direction: column; gap: var(--space-4); }
	h2 { margin-top: 0; font-size: var(--text-md); }
	.files-head { display: flex; justify-content: space-between; align-items: center; }
	.upload { cursor: pointer; padding: var(--space-1) var(--space-3); border-radius: var(--radius-sm); }
	ul { list-style: none; margin: 0; padding: 0; }
	.files li {
		display: grid;
		grid-template-columns: auto 1fr auto auto;
		align-items: center;
		gap: var(--space-2);
		padding: var(--space-2) 0;
		border-top: 1px solid var(--line);
	}
	.fk { font-size: var(--text-xs); color: var(--accent); text-transform: uppercase; }
	.fn { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
	.fmeta { font-size: var(--text-xs); }
	.comments li, .members li, .activity li { padding: var(--space-1) 0; }
	.members li { display: flex; gap: var(--space-2); align-items: center; }
	.members li .muted { margin-left: auto; }
	form { display: flex; gap: var(--space-2); margin-top: var(--space-3); }
	form input { flex: 1; }
	.invite input { min-width: 0; }
	.rm { background: transparent; border: 0; color: var(--fg-dim); }
	.rm:hover { color: var(--accent-2); }
	.warn { color: var(--c-warn); }
	.empty { padding: var(--space-3) 0; }
	@media (max-width: 820px) { .cols { grid-template-columns: 1fr; } }
</style>
