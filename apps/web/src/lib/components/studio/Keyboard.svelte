<script lang="ts">
	import { onMount } from 'svelte';

	let {
		noteon,
		noteoff
	}: {
		noteon: (n: number, vel: number) => void;
		noteoff: (n: number) => void;
	} = $props();

	const octaves = 2;
	let base = $state(48); // C3; shift with the octave buttons or z/x
	let held = $state<Set<number>>(new Set());

	const WHITE = [0, 2, 4, 5, 7, 9, 11];
	const BLACK: Record<number, number> = { 1: 0, 3: 1, 6: 3, 8: 4, 10: 5 };

	type Key = { note: number; black: boolean; left?: number };

	const whites = $derived.by(() => {
		const out: Key[] = [];
		for (let o = 0; o < octaves; o++) for (const s of WHITE) out.push({ note: base + o * 12 + s, black: false });
		return out;
	});
	const blacks = $derived.by(() => {
		const out: Key[] = [];
		const total = octaves * 7;
		for (let o = 0; o < octaves; o++)
			for (const s of Object.keys(BLACK).map(Number)) {
				const wi = o * 7 + BLACK[s] + 1;
				out.push({ note: base + o * 12 + s, black: true, left: (wi / total) * 100 });
			}
		return out;
	});

	function on(note: number) {
		if (held.has(note)) return;
		held.add(note);
		held = held;
		noteon(note, 0.9);
	}
	function off(note: number) {
		if (!held.has(note)) return;
		held.delete(note);
		held = held;
		noteoff(note);
	}

	// ── pointer (with glissando while held) ──
	let pointerDown = $state(false);
	function keyDown(e: PointerEvent, note: number) {
		pointerDown = true;
		(e.currentTarget as HTMLElement).releasePointerCapture?.(e.pointerId);
		on(note);
	}
	function keyEnter(note: number) {
		if (pointerDown) on(note);
	}
	function keyLeave(note: number) {
		if (pointerDown) off(note);
	}

	// ── computer keyboard ──
	const KEYMAP: Record<string, number> = {
		a: 0, w: 1, s: 2, e: 3, d: 4, f: 5, t: 6, g: 7, y: 8, h: 9, u: 10, j: 11,
		k: 12, o: 13, l: 14, p: 15
	};

	onMount(() => {
		const isEditable = (el: EventTarget | null) =>
			el instanceof HTMLElement && (el.tagName === 'INPUT' || el.tagName === 'TEXTAREA' || el.isContentEditable);
		const down = (e: KeyboardEvent) => {
			if (e.repeat || e.metaKey || e.ctrlKey || isEditable(e.target)) return;
			if (e.key === 'z') { base = Math.max(12, base - 12); return; }
			if (e.key === 'x') { base = Math.min(96, base + 12); return; }
			const semi = KEYMAP[e.key.toLowerCase()];
			if (semi !== undefined) { on(base + semi); e.preventDefault(); }
		};
		const up = (e: KeyboardEvent) => {
			const semi = KEYMAP[e.key.toLowerCase()];
			if (semi !== undefined) off(base + semi);
		};
		const globalUp = () => {
			pointerDown = false;
			for (const n of [...held]) off(n);
		};
		window.addEventListener('keydown', down);
		window.addEventListener('keyup', up);
		window.addEventListener('pointerup', globalUp);
		return () => {
			window.removeEventListener('keydown', down);
			window.removeEventListener('keyup', up);
			window.removeEventListener('pointerup', globalUp);
		};
	});
</script>

<div class="kbd">
	<div class="controls">
		<button class="ghost" onclick={() => (base = Math.max(12, base - 12))} aria-label="Octave down">−</button>
		<span class="oct">C{Math.floor(base / 12) - 1}</span>
		<button class="ghost" onclick={() => (base = Math.min(96, base + 12))} aria-label="Octave up">+</button>
		<span class="hint muted">play with your keyboard: a w s e d f… · z/x octave</span>
	</div>
	<div class="keys" style={`--whites:${octaves * 7}`}>
		{#each whites as wk (wk.note)}
			<button
				class="white"
				class:held={held.has(wk.note)}
				onpointerdown={(e) => keyDown(e, wk.note)}
				onpointerup={() => off(wk.note)}
				onpointerenter={() => keyEnter(wk.note)}
				onpointerleave={() => keyLeave(wk.note)}
				aria-label={`note ${wk.note}`}
			></button>
		{/each}
		{#each blacks as bk (bk.note)}
			<button
				class="black"
				class:held={held.has(bk.note)}
				style={`left:${bk.left}%`}
				onpointerdown={(e) => keyDown(e, bk.note)}
				onpointerup={() => off(bk.note)}
				onpointerenter={() => keyEnter(bk.note)}
				onpointerleave={() => keyLeave(bk.note)}
				aria-label={`note ${bk.note}`}
			></button>
		{/each}
	</div>
</div>

<style>
	.kbd {
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
	}
	.controls {
		display: flex;
		align-items: center;
		gap: var(--space-2);
	}
	.controls .ghost {
		min-width: 30px;
	}
	.oct {
		font-family: var(--font-mono);
		font-size: var(--text-sm);
		min-width: 30px;
		text-align: center;
	}
	.hint {
		font-size: var(--text-xs);
		margin-left: auto;
	}
	.keys {
		position: relative;
		display: grid;
		grid-template-columns: repeat(var(--whites), 1fr);
		height: 130px;
		user-select: none;
		touch-action: none;
	}
	.white {
		background: linear-gradient(#fff, #eef2f6);
		border: 1px solid var(--line);
		border-radius: 0 0 var(--radius-sm) var(--radius-sm);
		margin: 0;
		padding: 0;
		cursor: pointer;
	}
	.white.held {
		background: linear-gradient(var(--accent), var(--accent));
	}
	.black {
		position: absolute;
		top: 0;
		width: 5.5%;
		height: 62%;
		transform: translateX(-50%);
		background: linear-gradient(#2a2f37, #11151b);
		border: 1px solid #000;
		border-radius: 0 0 var(--radius-sm) var(--radius-sm);
		cursor: pointer;
		z-index: 2;
	}
	.black.held {
		background: linear-gradient(var(--accent-2, var(--accent)), var(--accent));
	}
</style>
