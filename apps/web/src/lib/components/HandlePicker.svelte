<script lang="ts">
	import { handle, setHandle } from '$lib/handle';
	import { t } from '$lib/i18n';

	// The acting member handle, persisted locally and sent as X-RS-Handle on every
	// request. Small inline control so members can post under their own name.
	let editing = $state(false);
	let draft = $state('');

	function open() {
		draft = $handle;
		editing = true;
	}
	function save() {
		setHandle(draft);
		editing = false;
	}
</script>

<div class="picker">
	{#if editing}
		<form onsubmit={(e) => { e.preventDefault(); save(); }}>
			<label class="lbl" for="rs-handle">{$t('handle.label')}</label>
			<input id="rs-handle" bind:value={draft} maxlength="40" autocomplete="off" spellcheck="false" />
			<button class="accent-fill" type="submit">OK</button>
		</form>
	{:else}
		<button class="current" onclick={open} title={$t('handle.edit')} aria-label={$t('handle.edit')}>
			<span class="lbl">{$t('handle.label')}</span>
			<span class="val">@{$handle}</span>
		</button>
	{/if}
</div>

<style>
	.picker {
		display: flex;
		align-items: center;
	}
	form {
		display: flex;
		align-items: center;
		gap: var(--space-2);
	}
	.lbl {
		font-size: var(--text-xs);
		color: var(--fg-dim);
	}
	input {
		width: 12ch;
	}
	.current {
		display: inline-flex;
		align-items: center;
		gap: var(--space-2);
		padding: var(--space-2) var(--space-3);
		background: var(--surface);
		border: 1px solid var(--line);
		border-radius: var(--radius-pill);
		cursor: pointer;
		text-align: left;
	}
	.current .val {
		font-weight: 600;
		color: var(--fg);
	}
</style>
