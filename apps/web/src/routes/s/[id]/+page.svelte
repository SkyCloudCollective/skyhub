<script lang="ts">
	import { onMount } from 'svelte';
	import { page } from '$app/stores';
	import {
		getSample,
		getPeaks,
		downloadUrl,
		sampleComments,
		addSampleComment,
		deleteSampleComment,
		sampleReactions,
		toggleReaction,
		getMe,
		type Sample,
		type Comment,
		type Reactions
	} from '$lib/api';
	import { playingId, toggle } from '$lib/player';
	import { dragOutSample } from '$lib/drag';
	import Waveform from '$lib/components/Waveform.svelte';
	import Icon from '$lib/components/Icon.svelte';

	const PALETTE = ['🔥', '💜', '🎧', '✨', '🥁'];

	let sample = $state<Sample | null>(null);
	let peaks = $state<number[][]>([]);
	let comments = $state<Comment[]>([]);
	let reactions = $state<Reactions>({ counts: {}, mine: [] });
	let me = $state('');
	let error = $state<string | null>(null);
	let draft = $state('');

	const id = $derived(Number($page.params.id));
	const isPlaying = $derived(sample !== null && $playingId === sample.id);

	async function load() {
		error = null;
		try {
			sample = await getSample(id);
		} catch {
			error = 'Sample not found.';
			return;
		}
		try {
			me = (await getMe()).handle;
		} catch {
			me = '';
		}
		peaks = await getPeaks(id).catch(() => []);
		comments = await sampleComments(id).catch(() => []);
		reactions = await sampleReactions(id).catch(() => ({ counts: {}, mine: [] }));
	}
	onMount(load);

	async function react(emoji: string) {
		const on = !reactions.mine.includes(emoji);
		const res = await toggleReaction(id, emoji, on);
		if (res) reactions = res;
	}

	async function post() {
		const b = draft.trim();
		if (!b) return;
		await addSampleComment(id, b);
		draft = '';
		comments = await sampleComments(id);
	}

	async function remove(cid: number) {
		await deleteSampleComment(id, cid);
		comments = await sampleComments(id);
	}

	function fmtDur(ms: number | null): string {
		if (!ms) return '';
		const s = ms / 1000;
		return s >= 1 ? `${s.toFixed(1)}s` : `${ms}ms`;
	}
</script>

{#if error}
	<section class="glass pad"><p class="warn">{error}</p><a href="/">← Library</a></section>
{:else if sample}
	<header
		class="glass head"
		role="group"
		aria-label={`${sample.title} — drag to your DAW`}
		draggable="true"
		ondragstart={(e) => sample && dragOutSample(e, sample)}
		title="Drag to your DAW"
	>
		<button class="play" class:on={isPlaying} onclick={() => sample && toggle(sample.id)} aria-label={isPlaying ? 'Pause' : 'Play'}>
			<Icon name={isPlaying ? 'pause' : 'play'} />
		</button>
		<div class="meta">
			<h1>{sample.title}</h1>
			<p class="muted">
				{#if sample.contributor}<a href={`/u/${sample.contributor}`}>@{sample.contributor}</a>{/if}
				{sample.category ? ` · ${sample.category}` : ''}{sample.instrument ? ` · ${sample.instrument}` : ''}
				{sample.bpm ? ` · ${Math.round(sample.bpm)} BPM` : ''}{sample.musical_key ? ` · ${sample.musical_key}` : ''}
				{sample.duration_ms ? ` · ${fmtDur(sample.duration_ms)}` : ''}
			</p>
			<Waveform {peaks} playing={isPlaying} />
		</div>
		<a class="dl ghost" href={downloadUrl(sample.id)} download={sample.filename} aria-label="Download" title="Download">↓</a>
	</header>

	<section class="glass pad reactions">
		{#each PALETTE as emoji (emoji)}
			<button
				class="rx"
				class:on={reactions.mine.includes(emoji)}
				onclick={() => react(emoji)}
				aria-pressed={reactions.mine.includes(emoji)}
				aria-label={`React ${emoji}`}
			>
				<span class="e">{emoji}</span>
				{#if reactions.counts[emoji]}<span class="n">{reactions.counts[emoji]}</span>{/if}
			</button>
		{/each}
	</section>

	<section class="glass pad">
		<h2>Comments</h2>
		<form onsubmit={(e) => { e.preventDefault(); post(); }}>
			<input placeholder="Leave a comment…" bind:value={draft} aria-label="Comment" />
			<button class="accent-fill" type="submit">Post</button>
		</form>
		<ul class="comments">
			{#each comments as c (c.id)}
				<li class:reply={c.parent_id !== null}>
					<a class="who" href={`/u/${c.handle}`}>@{c.handle}</a>
					<span class="body">{c.body}</span>
					{#if c.handle === me}<button class="rm" onclick={() => remove(c.id)} aria-label="Delete">✕</button>{/if}
				</li>
			{:else}
				<li class="muted empty">No comments yet — say something.</li>
			{/each}
		</ul>
	</section>
{/if}

<style>
	.pad {
		padding: var(--space-5);
		border-radius: var(--radius-md);
		margin-bottom: var(--space-4);
	}
	.head {
		display: grid;
		grid-template-columns: 48px 1fr auto;
		gap: var(--space-4);
		align-items: center;
		padding: var(--space-5);
		border-radius: var(--radius-md);
		margin-bottom: var(--space-4);
	}
	.head h1 {
		margin: 0 0 var(--space-1);
		font-size: var(--text-xl);
	}
	.play {
		width: 44px;
		height: 44px;
		border-radius: 50%;
		background: var(--accent);
		color: var(--accent-ink);
		font-size: var(--text-md);
	}
	.play.on {
		background: var(--accent-2, var(--accent));
	}
	.dl {
		background: var(--surface);
		min-width: 40px;
		text-align: center;
		text-decoration: none;
		color: var(--fg-dim);
		align-self: center;
		border-radius: var(--radius-sm);
		padding: var(--space-2);
	}
	.reactions {
		display: flex;
		gap: var(--space-2);
		flex-wrap: wrap;
	}
	.rx {
		display: flex;
		align-items: center;
		gap: var(--space-1);
		padding: var(--space-1) var(--space-3);
		background: var(--surface);
		border: 1px solid var(--line);
		border-radius: var(--radius-pill);
		cursor: pointer;
	}
	.rx.on {
		background: var(--accent);
		color: var(--accent-ink);
		border-color: transparent;
	}
	.rx .n {
		font-family: var(--font-mono);
		font-size: var(--text-sm);
	}
	h2 {
		margin: 0 0 var(--space-3);
		font-size: var(--text-md);
	}
	form {
		display: flex;
		gap: var(--space-2);
		margin-bottom: var(--space-3);
	}
	form input {
		flex: 1;
	}
	.comments {
		list-style: none;
		margin: 0;
		padding: 0;
	}
	.comments li {
		display: flex;
		gap: var(--space-2);
		align-items: baseline;
		padding: var(--space-2) 0;
		border-top: 1px solid var(--line);
	}
	.comments li.reply {
		padding-left: var(--space-5);
	}
	.who {
		font-weight: 600;
		white-space: nowrap;
	}
	.body {
		flex: 1;
	}
	.rm {
		background: transparent;
		border: 0;
		color: var(--fg-dim);
	}
	.rm:hover {
		color: var(--accent-2, var(--accent));
	}
	.warn {
		color: var(--c-warn);
	}
	.empty {
		padding: var(--space-3) 0;
	}
</style>
