<script lang="ts">
	let {
		x,
		y,
		label = 'XY',
		oninput
	}: { x: number; y: number; label?: string; oninput: (x: number, y: number) => void } = $props();

	// values are bipolar [-1, 1]; the puck maps +1 to the top/right.
	let pad = $state<HTMLDivElement>();
	let active = $state(false);

	const px = $derived(((x + 1) / 2) * 100);
	const py = $derived(((1 - y) / 2) * 100);

	function fromEvent(e: PointerEvent) {
		if (!pad) return;
		const r = pad.getBoundingClientRect();
		const nx = Math.min(1, Math.max(0, (e.clientX - r.left) / r.width));
		const ny = Math.min(1, Math.max(0, (e.clientY - r.top) / r.height));
		oninput(nx * 2 - 1, 1 - ny * 2);
	}
	function down(e: PointerEvent) {
		active = true;
		pad?.setPointerCapture(e.pointerId);
		fromEvent(e);
	}
	function move(e: PointerEvent) {
		if (active) fromEvent(e);
	}
	function up(e: PointerEvent) {
		active = false;
		pad?.releasePointerCapture?.(e.pointerId);
	}
</script>

<div class="xy">
	<div
		class="pad"
		class:active
		bind:this={pad}
		onpointerdown={down}
		onpointermove={move}
		onpointerup={up}
		role="application"
		aria-label={`${label} pad — use the X and Y sliders below for precise control`}
	>
		<span class="grid-x" aria-hidden="true"></span>
		<span class="grid-y" aria-hidden="true"></span>
		<span class="puck" style={`left:${px}%;top:${py}%`} aria-hidden="true"></span>
	</div>
	<div class="sliders">
		<label>
			<span>X</span>
			<input
				type="range"
				min="-1"
				max="1"
				step="0.01"
				value={x}
				oninput={(e) => oninput(+e.currentTarget.value, y)}
				aria-label={`${label} X`}
			/>
		</label>
		<label>
			<span>Y</span>
			<input
				type="range"
				min="-1"
				max="1"
				step="0.01"
				value={y}
				oninput={(e) => oninput(x, +e.currentTarget.value)}
				aria-label={`${label} Y`}
			/>
		</label>
	</div>
</div>

<style>
	.xy {
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
	}
	.pad {
		position: relative;
		aspect-ratio: 1;
		width: 100%;
		max-width: 220px;
		border: 1px solid var(--glass-border, var(--line));
		border-radius: var(--radius-md);
		background:
			radial-gradient(120% 120% at 50% 0%, var(--surface-2), var(--surface));
		overflow: hidden;
		touch-action: none;
		cursor: crosshair;
	}
	.pad.active {
		box-shadow: var(--focus-ring);
	}
	.grid-x,
	.grid-y {
		position: absolute;
		background: var(--line);
		opacity: 0.5;
	}
	.grid-x {
		left: 0;
		right: 0;
		top: 50%;
		height: 1px;
	}
	.grid-y {
		top: 0;
		bottom: 0;
		left: 50%;
		width: 1px;
	}
	.puck {
		position: absolute;
		width: 18px;
		height: 18px;
		border-radius: 50%;
		transform: translate(-50%, -50%);
		background: var(--accent);
		box-shadow: 0 0 0 4px color-mix(in srgb, var(--accent) 30%, transparent);
	}
	.sliders {
		display: grid;
		gap: 2px;
	}
	label {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		font-size: var(--text-xs);
		color: var(--fg-dim);
	}
	label span {
		width: 12px;
		font-family: var(--font-mono);
	}
	input[type='range'] {
		flex: 1;
	}
</style>
