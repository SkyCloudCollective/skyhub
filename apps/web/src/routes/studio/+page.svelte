<script lang="ts">
	import { onDestroy } from 'svelte';
	import { StudioEngine, type Instrument, type Route } from '$lib/studio/engine';
	import {
		PHASEPLAN_GROUPS,
		BOTANICA_GROUPS,
		MOD_SOURCES,
		MOD_TARGETS,
		defaults,
		type Group
	} from '$lib/studio/params';
	import Knob from '$lib/components/studio/Knob.svelte';
	import Keyboard from '$lib/components/studio/Keyboard.svelte';
	import XYPad from '$lib/components/studio/XYPad.svelte';
	import PresetBar from '$lib/components/studio/PresetBar.svelte';
	import { makeState, type PresetState } from '$lib/studio/presets';

	const engine = new StudioEngine();

	let instrument = $state<Instrument>('phaseplan');
	let started = $state(false);
	let voices = $state(0);

	// per-instrument param values (id → value)
	let pp = $state<Record<number, number>>(Object.fromEntries(defaults(PHASEPLAN_GROUPS)));
	let bot = $state<Record<number, number>>(Object.fromEntries(defaults(BOTANICA_GROUPS)));
	// Botanica XY puck (-1..1), not part of the knob grid
	let bx = $state(0);
	let by = $state(0);

	// PhasePlan modulation matrix — 3 slots
	let slots = $state<Route[]>([
		{ source: 0, target: 1, depth: 0 },
		{ source: 2, target: 1, depth: 0 },
		{ source: 1, target: 0, depth: 0 }
	]);

	const groups = $derived<Group[]>(instrument === 'phaseplan' ? PHASEPLAN_GROUPS : BOTANICA_GROUPS);
	const vals = $derived(instrument === 'phaseplan' ? pp : bot);

	engine.onVoices = (n) => (voices = n);
	engine.onReady = () => flushAll();

	function flushAll() {
		const v = instrument === 'phaseplan' ? pp : bot;
		for (const [id, value] of Object.entries(v)) engine.setParam(+id, value);
		if (instrument === 'botanica') {
			engine.setParam(4, bx);
			engine.setParam(5, by);
		} else {
			engine.setRoutes(activeRoutes());
		}
	}

	function setVal(id: number, value: number) {
		if (instrument === 'phaseplan') pp = { ...pp, [id]: value };
		else bot = { ...bot, [id]: value };
		if (started) engine.setParam(id, value);
	}

	function setXY(x: number, y: number) {
		bx = x;
		by = y;
		if (started) {
			engine.setParam(4, x);
			engine.setParam(5, y);
		}
	}

	function activeRoutes(): Route[] {
		return slots.filter((s) => Math.abs(s.depth) > 0.001);
	}
	function updateSlot(i: number, patch: Partial<Route>) {
		slots = slots.map((s, j) => (j === i ? { ...s, ...patch } : s));
		if (started && instrument === 'phaseplan') engine.setRoutes(activeRoutes());
	}

	async function power() {
		if (started) return;
		await engine.start(instrument);
		started = true;
	}

	function pick(which: Instrument) {
		if (which === instrument) return;
		instrument = which;
		voices = 0;
		if (started) {
			engine.allNotesOff();
			engine.setInstrument(which);
			flushAll();
		}
	}

	// ── presets ──
	function currentState(): PresetState {
		return makeState(instrument, instrument === 'phaseplan' ? pp : bot, slots, { x: bx, y: by });
	}
	function applyPreset(s: PresetState) {
		if (s.instrument !== instrument) return;
		if (instrument === 'phaseplan') {
			pp = { ...pp, ...s.params };
			if (s.routes) slots = s.routes.map((r) => ({ ...r }));
		} else {
			bot = { ...bot, ...s.params };
			if (s.xy) {
				bx = s.xy.x;
				by = s.xy.y;
			}
		}
		if (started) flushAll();
	}

	function noteOn(n: number, vel: number) {
		if (!started) void power();
		engine.noteOn(n, vel);
	}
	function noteOff(n: number) {
		engine.noteOff(n);
	}

	onDestroy(() => void engine.dispose());
</script>

<div class="studio">
	<header class="bar glass-2">
		<div class="tabs" role="tablist" aria-label="Instrument">
			<button
				role="tab"
				aria-selected={instrument === 'phaseplan'}
				class:active={instrument === 'phaseplan'}
				onclick={() => pick('phaseplan')}
			>
				PhasePlan
			</button>
			<button
				role="tab"
				aria-selected={instrument === 'botanica'}
				class:active={instrument === 'botanica'}
				onclick={() => pick('botanica')}
			>
				Botanica
			</button>
		</div>
		<p class="blurb muted">
			{#if instrument === 'phaseplan'}
				Dual-wavetable subtractive synth with an open modulation matrix.
			{:else}
				Granular sample-morph instrument. Sound design by Tev.
			{/if}
		</p>
		<div class="status">
			{#if instrument === 'phaseplan' && started}
				<span class="voices" title="active voices">{voices}/8</span>
			{/if}
			<button class="power" class:on={started} onclick={power} aria-pressed={started}>
				{started ? '● live' : '▶ power'}
			</button>
		</div>
	</header>

	<PresetBar {instrument} getState={currentState} onapply={applyPreset} />

	{#if !started}
		<p class="hint glass">
			Click <strong>power</strong> (or play a key) to start the audio engine — browsers require a gesture first.
		</p>
	{/if}

	<div class="rack">
		{#each groups as g (g.title)}
			<section class="panel glass">
				<h2>{g.title}</h2>
				<div class="ctls">
					{#each g.ctls as c (c.id)}
						{#if c.kind === 'knob'}
							<Knob ctl={c} value={vals[c.id]} oninput={(v) => setVal(c.id, v)} />
						{:else}
							<div class="seg">
								<span class="seglabel">{c.label}</span>
								<div class="segbtns" role="group" aria-label={c.label}>
									{#each c.options as opt, oi (opt)}
										<button class:active={Math.round(vals[c.id]) === oi} onclick={() => setVal(c.id, oi)}>
											{opt}
										</button>
									{/each}
								</div>
							</div>
						{/if}
					{/each}
				</div>
			</section>
		{/each}

		{#if instrument === 'botanica'}
			<section class="panel glass xy-panel">
				<h2>Character XY</h2>
				<XYPad x={bx} y={by} label="Character" oninput={setXY} />
			</section>
		{/if}

		{#if instrument === 'phaseplan'}
			<section class="panel glass matrix">
				<h2>Mod matrix</h2>
				<div class="rows">
					{#each slots as s, i (i)}
						<div class="row">
							<select
								aria-label="source"
								value={s.source}
								onchange={(e) => updateSlot(i, { source: +e.currentTarget.value })}
							>
								{#each MOD_SOURCES as src, si (src)}<option value={si}>{src}</option>{/each}
							</select>
							<span class="arrow" aria-hidden="true">→</span>
							<select
								aria-label="target"
								value={s.target}
								onchange={(e) => updateSlot(i, { target: +e.currentTarget.value })}
							>
								{#each MOD_TARGETS as tg, ti (tg)}<option value={ti}>{tg}</option>{/each}
							</select>
							<input
								type="range"
								min="-1"
								max="1"
								step="0.01"
								value={s.depth}
								oninput={(e) => updateSlot(i, { depth: +e.currentTarget.value })}
								aria-label="depth"
							/>
							<span class="depth">{s.depth.toFixed(2)}</span>
						</div>
					{/each}
				</div>
			</section>
		{/if}
	</div>

	{#if instrument === 'phaseplan'}
		<section class="keyboard glass">
			<Keyboard noteon={noteOn} noteoff={noteOff} />
		</section>
	{:else}
		<p class="play-bot muted">
			Botanica drones from a built-in tone (or a loaded sample). Use the Character XY pad and the knobs
			above — sound starts on power.
		</p>
	{/if}
</div>

<style>
	.studio {
		display: flex;
		flex-direction: column;
		gap: var(--space-4);
		padding-bottom: var(--space-6);
	}
	.bar {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: var(--space-3) var(--space-4);
		padding: var(--space-3) var(--space-4);
		border-radius: var(--radius-md);
	}
	.blurb {
		min-width: 0;
	}
	.tabs {
		display: flex;
		gap: var(--space-1);
	}
	.tabs button {
		font-family: var(--font-display);
		font-size: var(--text-md);
		padding: var(--space-2) var(--space-4);
		background: transparent;
		border-radius: var(--radius-pill);
		color: var(--fg-dim);
	}
	.tabs button.active {
		background: var(--accent);
		color: var(--accent-ink);
	}
	.blurb {
		font-size: var(--text-sm);
		margin: 0;
		margin-right: auto;
	}
	.status {
		display: flex;
		align-items: center;
		gap: var(--space-3);
	}
	.voices {
		font-family: var(--font-mono);
		font-size: var(--text-sm);
		color: var(--muted);
	}
	.power {
		font-family: var(--font-mono);
		padding: var(--space-2) var(--space-4);
		border-radius: var(--radius-pill);
		background: var(--surface);
	}
	.power.on {
		background: color-mix(in srgb, var(--c-good) 22%, transparent);
		color: var(--c-good);
	}
	.hint {
		padding: var(--space-3) var(--space-4);
		border-radius: var(--radius-md);
		font-size: var(--text-sm);
		color: var(--fg-dim);
	}
	.rack {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
		gap: var(--space-4);
		align-items: start;
	}
	.panel {
		padding: var(--space-4);
		border-radius: var(--radius-md);
	}
	.panel h2 {
		margin: 0 0 var(--space-3);
		font-size: var(--text-sm);
		text-transform: uppercase;
		letter-spacing: var(--tracking-display);
		color: var(--fg-dim);
	}
	.ctls {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-3);
		align-items: flex-start;
	}
	.seg {
		display: flex;
		flex-direction: column;
		gap: var(--space-1);
	}
	.seglabel {
		font-size: var(--text-xs);
		color: var(--fg-dim);
	}
	.segbtns {
		display: flex;
		gap: 2px;
	}
	.segbtns button {
		font-size: var(--text-xs);
		padding: var(--space-1) var(--space-2);
		background: var(--surface);
		border-radius: var(--radius-sm);
		color: var(--fg-dim);
	}
	.segbtns button.active {
		background: var(--accent);
		color: var(--accent-ink);
	}
	.matrix .rows {
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
	}
	.matrix .row {
		display: grid;
		grid-template-columns: 1fr auto 1fr 1.4fr auto;
		align-items: center;
		gap: var(--space-2);
	}
	.matrix select {
		min-width: 0;
		font-size: var(--text-xs);
	}
	.matrix input[type='range'] {
		min-width: 0;
		width: 100%;
	}
	.matrix .arrow {
		color: var(--muted);
	}
	.matrix .depth {
		font-family: var(--font-mono);
		font-size: var(--text-xs);
		color: var(--muted);
		width: 34px;
		text-align: right;
	}
	.xy-panel {
		max-width: 280px;
	}
	.keyboard {
		padding: var(--space-4);
		border-radius: var(--radius-md);
		position: sticky;
		bottom: var(--space-3);
	}
	.play-bot {
		font-size: var(--text-sm);
	}
</style>
