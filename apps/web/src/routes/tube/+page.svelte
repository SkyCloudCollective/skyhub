<script lang="ts">
	import { onMount } from 'svelte';
	import { getTube, postVideo, type Video } from '$lib/api';
	import { parseEmbed, embedUrl } from '$lib/embed';
	import { t } from '$lib/i18n';
	import Icon from '$lib/components/Icon.svelte';
	import VideoCard from '$lib/components/VideoCard.svelte';
	import HandlePicker from '$lib/components/HandlePicker.svelte';

	let videos = $state<Video[]>([]);
	let offline = $state(false);

	// share form
	let url = $state('');
	let title = $state('');
	let description = $state('');
	let category = $state('');
	let posting = $state(false);
	let formError = $state<string | null>(null);

	// live, client-side preview (mirrors the server allowlist; authoritative
	// validation still happens server-side on submit).
	const parsed = $derived(parseEmbed(url));
	const previewSrc = $derived(parsed ? embedUrl(parsed.provider, parsed.videoRef) : null);
	const urlLooksBad = $derived(url.trim().length > 0 && parsed === null);

	// filters
	let filterCategory = $state('');
	let filterContributor = $state('');

	const categories = $derived([...new Set(videos.map((v) => v.category).filter(Boolean))] as string[]);
	const contributors = $derived([...new Set(videos.map((v) => v.handle))]);
	const shown = $derived(
		videos.filter(
			(v) =>
				(!filterCategory || v.category === filterCategory) &&
				(!filterContributor || v.handle === filterContributor)
		)
	);

	async function load() {
		try {
			videos = await getTube({ limit: 100 });
			offline = false;
		} catch {
			offline = true;
		}
	}
	onMount(load);

	async function share() {
		formError = null;
		const u = url.trim();
		const ti = title.trim();
		if (!u || !ti || posting) return;
		posting = true;
		try {
			const v = await postVideo({
				url: u,
				title: ti,
				description: description.trim() || undefined,
				category: category.trim() || undefined
			});
			if (v) {
				videos = [v, ...videos];
				url = title = description = category = '';
			}
		} catch {
			formError = $t('tube.invalid');
		} finally {
			posting = false;
		}
	}
</script>

<header class="head glass">
	<div class="head-text">
		<h1>{$t('tube.title')}</h1>
		<p class="muted">{$t('tube.intro')}</p>
	</div>
	<HandlePicker />
</header>

{#if offline}
	<p class="glass pad warn">{$t('tube.offline')}</p>
{/if}

<section class="glass pad share">
	<h2><Icon name="plus" size="1em" /> {$t('tube.post')}</h2>
	<form onsubmit={(e) => { e.preventDefault(); share(); }}>
		<div class="fields">
			<label>
				<span>{$t('tube.url')}</span>
				<input
					bind:value={url}
					placeholder="https://…"
					inputmode="url"
					aria-invalid={urlLooksBad}
					autocomplete="off"
					spellcheck="false"
				/>
			</label>
			<label>
				<span>{$t('tube.video_title')}</span>
				<input bind:value={title} maxlength="160" />
			</label>
			<div class="two">
				<label>
					<span>{$t('tube.category')}</span>
					<input bind:value={category} maxlength="40" />
				</label>
				<label>
					<span>{$t('tube.description')}</span>
					<input bind:value={description} maxlength="2000" />
				</label>
			</div>
			{#if urlLooksBad}<p class="warn small" role="alert">{$t('tube.invalid')}</p>{/if}
			{#if formError}<p class="warn small" role="alert">{formError}</p>{/if}
			<button class="accent-fill" type="submit" disabled={!parsed || !title.trim() || posting}>
				{$t('tube.submit')}
			</button>
		</div>

		<div class="preview" aria-live="polite">
			{#if previewSrc && parsed}
				<p class="small muted">{$t('tube.preview')} · {$t('tube.detected')}: {parsed.provider}</p>
				<div class="frame">
					<iframe
						src={previewSrc}
						title={title.trim() || $t('tube.preview')}
						loading="lazy"
						referrerpolicy="strict-origin-when-cross-origin"
						allow="fullscreen; picture-in-picture"
						allowfullscreen
					></iframe>
				</div>
			{:else}
				<div class="frame frame-empty"><Icon name="video" size="2em" /></div>
			{/if}
		</div>
	</form>
</section>

<section class="filters">
	<label>
		<span class="small muted">{$t('tube.filter.category')}</span>
		<select bind:value={filterCategory}>
			<option value="">{$t('tube.filter.all')}</option>
			{#each categories as c (c)}<option value={c}>{c}</option>{/each}
		</select>
	</label>
	<label>
		<span class="small muted">{$t('tube.filter.contributor')}</span>
		<select bind:value={filterContributor}>
			<option value="">{$t('tube.filter.all')}</option>
			{#each contributors as c (c)}<option value={c}>@{c}</option>{/each}
		</select>
	</label>
</section>

{#if shown.length}
	<div class="grid">
		{#each shown as v (v.id)}<VideoCard video={v} />{/each}
	</div>
{:else if !offline}
	<p class="glass pad muted">{$t('tube.empty')}</p>
{/if}

<style>
	.head {
		display: flex;
		align-items: center;
		gap: var(--space-4);
		padding: var(--space-5);
		margin-bottom: var(--space-4);
	}
	.head-text {
		margin-right: auto;
	}
	.head h1 {
		margin-bottom: var(--space-1);
	}
	.pad {
		padding: var(--space-5);
	}
	.warn {
		color: var(--c-warn);
	}
	.small {
		font-size: var(--text-xs);
	}
	.share {
		margin-bottom: var(--space-5);
	}
	.share h2 {
		margin: 0 0 var(--space-4);
		font-size: var(--text-md);
		display: flex;
		align-items: center;
		gap: var(--space-2);
	}
	.share form {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: var(--space-5);
		align-items: start;
	}
	.fields {
		display: flex;
		flex-direction: column;
		gap: var(--space-3);
	}
	.two {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: var(--space-3);
	}
	label {
		display: flex;
		flex-direction: column;
		gap: var(--space-1);
		text-align: left;
	}
	label span {
		font-size: var(--text-sm);
		color: var(--fg-dim);
	}
	input,
	select {
		width: 100%;
	}
	.fields button {
		align-self: flex-start;
	}
	.preview {
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
	}
	.frame {
		position: relative;
		aspect-ratio: 16 / 9;
		border-radius: var(--radius-md);
		overflow: hidden;
		border: 1px solid var(--line);
		background: var(--surface);
	}
	.frame iframe {
		position: absolute;
		inset: 0;
		width: 100%;
		height: 100%;
		border: 0;
	}
	.frame-empty {
		display: grid;
		place-items: center;
		color: var(--muted);
	}
	.filters {
		display: flex;
		gap: var(--space-4);
		flex-wrap: wrap;
		margin-bottom: var(--space-4);
	}
	.filters label {
		flex-direction: row;
		align-items: center;
		gap: var(--space-2);
	}
	.grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(320px, 1fr));
		gap: var(--space-4);
	}
	@media (max-width: 820px) {
		.share form {
			grid-template-columns: 1fr;
		}
		.preview {
			order: -1;
		}
	}
</style>
