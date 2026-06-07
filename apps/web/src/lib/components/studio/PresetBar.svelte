<script lang="ts">
	import Icon from '$lib/components/Icon.svelte';
	import type { Instrument } from '$lib/studio/engine';
	import {
		FACTORY,
		listUser,
		saveUser,
		deleteUser,
		dice,
		type PresetState,
		type NamedPreset
	} from '$lib/studio/presets';

	let {
		instrument,
		getState,
		onapply
	}: {
		instrument: Instrument;
		getState: () => PresetState;
		onapply: (s: PresetState) => void;
	} = $props();

	const factory = $derived<NamedPreset[]>(FACTORY[instrument]);
	let user = $state<NamedPreset[]>([]);
	let selected = $state(''); // composite key "f:Name" / "u:Name"
	let amount = $state(0.25);
	let seedctr = 1;

	// in-memory A/B compare slots
	let slotA = $state<PresetState | null>(null);
	let slotB = $state<PresetState | null>(null);

	$effect(() => {
		user = listUser(instrument);
	});

	function applyByKey(key: string) {
		const [kind, ...rest] = key.split(':');
		const name = rest.join(':');
		const src = kind === 'f' ? factory : user;
		const found = src.find((p) => p.name === name);
		if (found) onapply(found.state);
	}

	function onselect(e: Event) {
		const v = (e.target as HTMLSelectElement).value;
		if (v) applyByKey(v);
		selected = ''; // reset so re-picking the same preset re-applies
	}

	function save() {
		const name = prompt('Preset name')?.trim();
		if (!name) return;
		user = saveUser(instrument, name, getState());
	}

	function removeSelectedUser() {
		const name = user[0] && prompt('Delete which preset? (exact name)')?.trim();
		if (!name) return;
		user = deleteUser(instrument, name);
	}

	function roll() {
		onapply(dice(getState(), amount, (seedctr++ * 2654435761) >>> 0));
	}

	function exportJson() {
		const state = getState();
		const blob = new Blob([JSON.stringify(state, null, 2)], { type: 'application/json' });
		const url = URL.createObjectURL(blob);
		const a = document.createElement('a');
		a.href = url;
		a.download = `${instrument}-preset.json`;
		a.click();
		URL.revokeObjectURL(url);
	}

	async function importJson(e: Event) {
		const input = e.target as HTMLInputElement;
		const file = input.files?.[0];
		input.value = '';
		if (!file) return;
		try {
			const state = JSON.parse(await file.text()) as PresetState;
			if (state?.instrument === instrument && state.params) onapply(state);
		} catch {
			/* ignore malformed files */
		}
	}
</script>

<div class="presets glass">
	<label class="field">
		<span class="lbl">Preset</span>
		<select value={selected} onchange={onselect} aria-label="Load preset">
			<option value="">Load…</option>
			<optgroup label="Factory">
				{#each factory as p (p.name)}<option value={`f:${p.name}`}>{p.name}</option>{/each}
			</optgroup>
			{#if user.length}
				<optgroup label="Yours">
					{#each user as p (p.name)}<option value={`u:${p.name}`}>{p.name}</option>{/each}
				</optgroup>
			{/if}
		</select>
	</label>

	<button class="ghost" onclick={save} title="Save the current sound as a preset">Save</button>
	{#if user.length}
		<button class="ghost" onclick={removeSelectedUser} title="Delete one of your presets">Delete</button>
	{/if}

	<span class="sep" aria-hidden="true"></span>

	<div class="dice">
		<button class="accent-fill dice-btn" onclick={roll} title="Randomise within range (seeded)"><Icon name="dice" /> Dice</button>
		<input
			type="range"
			min="0.02"
			max="0.6"
			step="0.01"
			bind:value={amount}
			aria-label="Dice amount"
			title="Dice amount"
		/>
	</div>

	<span class="sep" aria-hidden="true"></span>

	<div class="ab" role="group" aria-label="A/B compare">
		<button class="ghost" onclick={() => (slotA = getState())} title="Copy current sound to A">→A</button>
		<button class="ghost" disabled={!slotA} onclick={() => slotA && onapply(slotA)} title="Recall A">A</button>
		<button class="ghost" disabled={!slotB} onclick={() => slotB && onapply(slotB)} title="Recall B">B</button>
		<button class="ghost" onclick={() => (slotB = getState())} title="Copy current sound to B">→B</button>
	</div>

	<span class="sep" aria-hidden="true"></span>

	<button class="ghost" onclick={exportJson} title="Download this preset as JSON">Export</button>
	<label class="import ghost" title="Load a preset from a JSON file">
		Import
		<input type="file" accept="application/json,.json" onchange={importJson} hidden />
	</label>
</div>

<style>
	.presets {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: var(--space-2);
		padding: var(--space-2) var(--space-3);
		border-radius: var(--radius-md);
	}
	.field {
		display: flex;
		align-items: center;
		gap: var(--space-2);
	}
	.lbl {
		font-size: var(--text-xs);
		color: var(--fg-dim);
	}
	select {
		font-size: var(--text-sm);
		min-width: 130px;
	}
	.ghost {
		background: var(--surface);
		font-size: var(--text-sm);
		padding: var(--space-1) var(--space-3);
		border-radius: var(--radius-sm);
	}
	.ghost:disabled {
		opacity: 0.4;
	}
	.sep {
		width: 1px;
		align-self: stretch;
		background: var(--line);
		margin: 0 var(--space-1);
	}
	.dice {
		display: flex;
		align-items: center;
		gap: var(--space-2);
	}
	.dice input {
		width: 90px;
	}
	.ab {
		display: flex;
		gap: 2px;
	}
	.import {
		cursor: pointer;
	}
</style>
