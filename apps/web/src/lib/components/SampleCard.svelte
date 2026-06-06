<script lang="ts">
	import { onMount } from 'svelte';
	import { type Sample, downloadUrl, getPeaks } from '$lib/api';
	import { playingId, toggle } from '$lib/player';
	import { dragOutSample } from '$lib/drag';
	import Waveform from './Waveform.svelte';

	let { sample }: { sample: Sample } = $props();

	let peaks = $state<number[][]>([]);
	const isPlaying = $derived($playingId === sample.id);

	onMount(async () => {
		try {
			peaks = await getPeaks(sample.id);
		} catch {
			peaks = [];
		}
	});

	function fmtDur(ms: number | null): string {
		if (!ms) return '';
		const s = ms / 1000;
		return s >= 1 ? `${s.toFixed(1)}s` : `${ms}ms`;
	}
</script>

<article
	class="card glass"
	draggable="true"
	ondragstart={(e) => dragOutSample(e, sample)}
	title="Drag to your DAW"
>
	<button
		class="play"
		class:on={isPlaying}
		onclick={() => toggle(sample.id)}
		aria-label={isPlaying ? 'Pause' : 'Play'}
	>
		{isPlaying ? '❚❚' : '▶'}
	</button>

	<div class="body">
		<div class="row1">
			<span class="title">{sample.title}</span>
			{#if sample.contributor}<span class="by muted">@{sample.contributor}</span>{/if}
		</div>

		<Waveform {peaks} playing={isPlaying} />

		<div class="chips">
			{#if sample.category}<span class="chip">{sample.category}</span>{/if}
			{#if sample.bpm}<span class="chip">{Math.round(sample.bpm)} BPM</span>{/if}
			{#if sample.musical_key}<span class="chip">{sample.musical_key}</span>{/if}
			{#if sample.instrument}<span class="chip">{sample.instrument}</span>{/if}
			<span class="chip dur">{fmtDur(sample.duration_ms)}</span>
		</div>
	</div>

	<a class="dl" href={downloadUrl(sample.id)} download={sample.filename} aria-label="Download" title="Download">↓</a>
</article>

<style>
	.card {
		display: grid;
		grid-template-columns: 44px 1fr 36px;
		align-items: center;
		gap: var(--space-3);
		padding: var(--space-3) var(--space-4);
		cursor: grab;
	}
	.card:active {
		cursor: grabbing;
	}
	.play {
		width: 44px;
		height: 44px;
		border-radius: var(--radius-pill);
		background: var(--surface-2);
		border: 1px solid var(--line);
		font-size: 14px;
		display: grid;
		place-items: center;
	}
	.play.on {
		background: var(--accent);
		color: var(--accent-ink);
		border-color: var(--accent);
	}
	.body {
		min-width: 0;
	}
	.row1 {
		display: flex;
		align-items: baseline;
		gap: var(--space-2);
		margin-bottom: var(--space-1);
	}
	.title {
		font-weight: 600;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}
	.by {
		font-size: var(--text-xs);
	}
	.chips {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-1);
		margin-top: var(--space-2);
	}
	.chip {
		font-size: var(--text-xs);
		padding: 1px var(--space-2);
		border-radius: var(--radius-pill);
		background: var(--surface);
		border: 1px solid var(--line);
		color: var(--fg-dim);
	}
	.chip.dur {
		margin-left: auto;
	}
	.dl {
		text-decoration: none;
		color: var(--fg-dim);
		font-size: 18px;
		width: 36px;
		height: 36px;
		display: grid;
		place-items: center;
		border-radius: var(--radius-sm);
	}
	.dl:hover {
		color: var(--accent);
		background: var(--surface);
	}
</style>
