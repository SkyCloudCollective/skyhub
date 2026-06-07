<script lang="ts">
	import { t } from '$lib/i18n';
	import {
		theme,
		toggleTheme,
		palette,
		PALETTES,
		setPalette,
		customAccent,
		setCustomAccent,
		clearCustomAccent,
		flatGlass,
		toggleFlatGlass,
		savedThemes,
		saveCurrentTheme,
		applySavedTheme,
		deleteSavedTheme
	} from '$lib/theme';
	import Icon from './Icon.svelte';

	let open = $state(false);
	let newName = $state('');
	let root: HTMLDivElement;

	function onWindowClick(e: MouseEvent) {
		if (open && root && !root.contains(e.target as Node)) open = false;
	}
	function onKey(e: KeyboardEvent) {
		if (e.key === 'Escape') open = false;
	}

	function pickColor(e: Event) {
		setCustomAccent((e.currentTarget as HTMLInputElement).value);
	}
	function save() {
		if (!newName.trim()) return;
		saveCurrentTheme(newName);
		newName = '';
	}
</script>

<svelte:window onclick={onWindowClick} onkeydown={onKey} />

<div class="picker" bind:this={root}>
	<button
		class="ghost trigger"
		onclick={() => (open = !open)}
		aria-expanded={open}
		aria-haspopup="dialog"
		title={$t('appearance.toggle')}
		aria-label={$t('appearance.toggle')}
	>
		<span class="dot" style="background:var(--accent)"></span>
	</button>

	{#if open}
		<div class="panel" role="dialog" aria-label={$t('appearance.toggle')}>
			<!-- mode -->
			<div class="row">
				<span class="lbl">{$t('appearance.mode')}</span>
				<button class="mode" onclick={toggleTheme}>
					<Icon name={$theme === 'aero-dark' ? 'moon' : 'sun'} />
					<span>{$theme === 'aero-dark' ? 'Dark' : 'Light'}</span>
				</button>
			</div>

			<!-- palettes -->
			<div class="block">
				<span class="lbl">{$t('appearance.palette')}</span>
				<div class="swatches">
					{#each PALETTES as p (p.id)}
						<button
							class="sw"
							class:active={$palette === p.id && !$customAccent}
							style="--sw:{p.swatch}"
							title={p.label}
							aria-label={p.label}
							aria-pressed={$palette === p.id && !$customAccent}
							onclick={() => setPalette(p.id)}
						></button>
					{/each}
				</div>
			</div>

			<!-- custom accent -->
			<div class="row">
				<span class="lbl">{$t('appearance.custom')}</span>
				<span class="custom">
					<input
						type="color"
						value={$customAccent ?? '#1f86e8'}
						oninput={pickColor}
						aria-label={$t('appearance.custom')}
					/>
					{#if $customAccent}
						<button class="link" onclick={clearCustomAccent}>{$t('appearance.reset')}</button>
					{/if}
				</span>
			</div>

			<!-- flat glass -->
			<label class="row toggle">
				<span class="lbl">{$t('appearance.flat')}</span>
				<input type="checkbox" checked={$flatGlass} onchange={toggleFlatGlass} />
			</label>

			<!-- save / saved themes -->
			<div class="block save">
				<div class="saverow">
					<input
						type="text"
						bind:value={newName}
						placeholder={$t('appearance.name')}
						maxlength="40"
						onkeydown={(e) => e.key === 'Enter' && save()}
					/>
					<button class="primary" onclick={save} disabled={!newName.trim()}
						>{$t('appearance.save')}</button
					>
				</div>
				{#if $savedThemes.length}
					<span class="lbl">{$t('appearance.saved')}</span>
					<ul class="saved">
						{#each $savedThemes as s (s.name)}
							<li>
								<button class="chip" onclick={() => applySavedTheme(s)} title={s.name}>
									<span class="chipdot" style="background:{s.accent ?? PALETTES.find((p) => p.id === s.palette)?.swatch}"
									></span>
									{s.name}
								</button>
								<button class="x" onclick={() => deleteSavedTheme(s.name)} aria-label="Delete">×</button>
							</li>
						{/each}
					</ul>
				{/if}
			</div>
		</div>
	{/if}
</div>

<style>
	.picker {
		position: relative;
		display: inline-flex;
	}
	.trigger {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		width: 38px;
		height: 38px;
		min-width: 38px;
		padding: 0;
		border-radius: 50%;
	}
	.dot {
		width: 16px;
		height: 16px;
		border-radius: 50%;
		box-shadow:
			inset 0 1px 0 rgba(255, 255, 255, 0.7),
			0 1px 3px rgba(0, 0, 0, 0.35);
	}
	.panel {
		position: absolute;
		top: calc(100% + var(--space-2));
		right: 0;
		z-index: 40;
		width: 264px;
		padding: var(--space-4);
		border-radius: var(--radius-lg);
		display: flex;
		flex-direction: column;
		gap: var(--space-3);
		/* Legible floating panel — near-opaque + blur (no content bleed-through). */
		background: color-mix(in srgb, var(--surface-solid) 90%, transparent);
		-webkit-backdrop-filter: blur(var(--glass-blur)) saturate(150%);
		backdrop-filter: blur(var(--glass-blur)) saturate(150%);
		border: 1px solid var(--glass-border);
		box-shadow: var(--glass-shadow);
		transform-origin: top right;
		animation: rs-pop var(--dur-fast) var(--ease-out);
	}
	.row {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: var(--space-3);
	}
	.block {
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
	}
	.lbl {
		font-size: var(--text-xs);
		color: var(--fg-dim);
	}
	.mode {
		display: inline-flex;
		align-items: center;
		gap: var(--space-2);
		padding: var(--space-1) var(--space-3);
	}
	.swatches {
		display: flex;
		gap: var(--space-2);
		flex-wrap: wrap;
	}
	.sw {
		width: 30px;
		height: 30px;
		min-width: 0;
		padding: 0;
		border-radius: 50%;
		background: var(--sw);
		border: 2px solid var(--glass-border);
		box-shadow: 0 2px 6px -2px var(--glass-lo);
		cursor: pointer;
	}
	.sw.active {
		outline: 2px solid var(--fg);
		outline-offset: 2px;
	}
	.custom {
		display: inline-flex;
		align-items: center;
		gap: var(--space-2);
	}
	.custom input[type='color'] {
		width: 38px;
		height: 28px;
		padding: 0;
		border-radius: var(--radius-sm);
		cursor: pointer;
	}
	.link {
		background: none;
		border: none;
		box-shadow: none;
		padding: 0;
		color: var(--accent);
		font-size: var(--text-xs);
		text-decoration: underline;
	}
	.toggle {
		cursor: pointer;
	}
	.save {
		border-top: 1px solid var(--line);
		padding-top: var(--space-3);
	}
	.saverow {
		display: flex;
		gap: var(--space-2);
	}
	.saverow input {
		flex: 1;
		min-width: 0;
	}
	.saved {
		list-style: none;
		margin: 0;
		padding: 0;
		display: flex;
		flex-direction: column;
		gap: var(--space-1);
	}
	.saved li {
		display: flex;
		align-items: center;
		gap: var(--space-2);
	}
	.chip {
		flex: 1;
		display: inline-flex;
		align-items: center;
		gap: var(--space-2);
		justify-content: flex-start;
		padding: var(--space-1) var(--space-3);
		font-size: var(--text-sm);
	}
	.chipdot {
		width: 12px;
		height: 12px;
		border-radius: 50%;
		flex: none;
	}
	.x {
		min-width: 0;
		padding: var(--space-1) var(--space-2);
		line-height: 1;
	}
</style>
