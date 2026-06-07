<script lang="ts">
	import { onMount } from 'svelte';
	import { tubeReactions, toggleTubeReaction, type Video, type Reactions } from '$lib/api';
	import Icon from './Icon.svelte';

	// A small, fixed reaction palette (mirrors the sample page; emoji are
	// decorative — each control has a text aria-label).
	const PALETTE = ['🔥', '💜', '🎧', '✨'];

	let { video }: { video: Video } = $props();

	let reactions = $state<Reactions>({ counts: {}, mine: [] });

	onMount(async () => {
		reactions = await tubeReactions(video.id).catch(() => ({ counts: {}, mine: [] }));
	});

	async function react(emoji: string) {
		const on = !reactions.mine.includes(emoji);
		const res = await toggleTubeReaction(video.id, emoji, on);
		if (res) reactions = res;
	}
</script>

<article class="card glass">
	<div class="frame">
		{#if video.embed_url}
			<!-- src is the server-rebuilt embed URL (from the validated provider+ref),
			     never a raw user string. lazy-loaded, no autoplay, titled for a11y. -->
			<iframe
				src={video.embed_url}
				title={video.title}
				loading="lazy"
				referrerpolicy="strict-origin-when-cross-origin"
				allow="fullscreen; picture-in-picture"
				allowfullscreen
			></iframe>
		{:else}
			<div class="frame-missing"><Icon name="video" size="2em" /></div>
		{/if}
	</div>

	<div class="body">
		<h3 class="title">{video.title}</h3>
		<p class="by">
			<a href={`/u/${video.handle}`}>@{video.handle}</a>
			{#if video.category}<span class="chip">{video.category}</span>{/if}
			<span class="prov">{video.provider}</span>
		</p>
		{#if video.description}<p class="desc">{video.description}</p>{/if}

		<div class="reactions">
			{#each PALETTE as emoji (emoji)}
				<button
					class="rx"
					class:on={reactions.mine.includes(emoji)}
					onclick={() => react(emoji)}
					aria-pressed={reactions.mine.includes(emoji)}
					aria-label={`React ${emoji}`}
				>
					<span class="e" aria-hidden="true">{emoji}</span>
					{#if reactions.counts[emoji]}<span class="n">{reactions.counts[emoji]}</span>{/if}
				</button>
			{/each}
			{#if video.watch_url}
				<a class="watch" href={video.watch_url} target="_blank" rel="noopener noreferrer">
					<Icon name="arrow-up-right" size="0.95em" />
				</a>
			{/if}
		</div>
	</div>
</article>

<style>
	.card {
		display: flex;
		flex-direction: column;
		overflow: hidden;
	}
	.frame {
		position: relative;
		aspect-ratio: 16 / 9;
		background: var(--surface-2, var(--surface));
		border-bottom: 1px solid var(--line);
	}
	.frame iframe,
	.frame-missing {
		position: absolute;
		inset: 0;
		width: 100%;
		height: 100%;
		border: 0;
	}
	.frame-missing {
		display: grid;
		place-items: center;
		color: var(--muted);
	}
	.body {
		padding: var(--space-4);
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
	}
	.title {
		margin: 0;
		font-size: var(--text-md);
		font-weight: 600;
		text-align: left;
	}
	.by {
		margin: 0;
		display: flex;
		align-items: center;
		flex-wrap: wrap;
		gap: var(--space-2);
		font-size: var(--text-sm);
		color: var(--fg-dim);
	}
	.by a {
		font-weight: 600;
		color: var(--fg-dim);
		text-decoration: none;
	}
	.by a:hover {
		color: var(--accent);
	}
	.chip,
	.prov {
		font-size: var(--text-xs);
		padding: 1px var(--space-2);
		border-radius: var(--radius-pill);
		background: var(--surface);
		border: 1px solid var(--line);
		color: var(--fg-dim);
	}
	.prov {
		margin-left: auto;
		text-transform: lowercase;
	}
	.desc {
		margin: 0;
		font-size: var(--text-sm);
		color: var(--fg-dim);
		max-width: 60ch;
	}
	.reactions {
		display: flex;
		gap: var(--space-2);
		flex-wrap: wrap;
		align-items: center;
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
	.watch {
		margin-left: auto;
		display: grid;
		place-items: center;
		width: 34px;
		height: 34px;
		border-radius: var(--radius-sm);
		color: var(--fg-dim);
		text-decoration: none;
		border: 1px solid var(--line);
	}
	.watch:hover {
		color: var(--accent);
		background: var(--surface);
	}
</style>
