<script lang="ts">
	import { onMount } from 'svelte';
	import { getGalaxy, type GalaxyPoint, type Sample } from '$lib/api';
	import { toggle, playingId } from '$lib/player';
	import { dragOutSample } from '$lib/drag';

	type Axis = 'brightness' | 'percussiveness' | 'noisiness' | 'loudness' | 'bpm';
	const AXES: { key: Axis; label: string }[] = [
		{ key: 'brightness', label: 'Brightness' },
		{ key: 'percussiveness', label: 'Percussiveness' },
		{ key: 'noisiness', label: 'Noisiness' },
		{ key: 'loudness', label: 'Loudness' },
		{ key: 'bpm', label: 'Tempo (BPM)' }
	];

	let points = $state<GalaxyPoint[]>([]);
	let offline = $state(false);
	let axisX = $state<Axis>('brightness');
	let axisY = $state<Axis>('percussiveness');
	let canvas = $state<HTMLCanvasElement>();
	let hover = $state<{ p: GalaxyPoint; x: number; y: number } | null>(null);

	// screen positions cached for hit-testing
	let positions: { p: GalaxyPoint; x: number; y: number }[] = [];

	const categories = $derived([...new Set(points.map((p) => p.category ?? 'other'))].sort());
	function colorFor(cat: string | null): string {
		const i = Math.max(0, categories.indexOf(cat ?? 'other'));
		return `hsl(${(i * 47) % 360} 70% 55%)`;
	}
	const asSample = (p: GalaxyPoint): Sample =>
		({ id: p.id, title: p.title, filename: null }) as unknown as Sample;

	function range(axis: Axis): [number, number] {
		let lo = Infinity;
		let hi = -Infinity;
		for (const p of points) {
			const v = p[axis];
			if (v != null) {
				lo = Math.min(lo, v);
				hi = Math.max(hi, v);
			}
		}
		if (!isFinite(lo)) return [0, 1];
		return lo === hi ? [lo - 1, hi + 1] : [lo, hi];
	}

	function draw() {
		if (!canvas) return;
		const ctx = canvas.getContext('2d');
		if (!ctx) return;
		const dpr = window.devicePixelRatio || 1;
		const w = canvas.clientWidth;
		const h = canvas.clientHeight;
		canvas.width = w * dpr;
		canvas.height = h * dpr;
		ctx.scale(dpr, dpr);
		ctx.clearRect(0, 0, w, h);

		const pad = 28;
		const [xlo, xhi] = range(axisX);
		const [ylo, yhi] = range(axisY);
		const sx = (v: number) => pad + ((v - xlo) / (xhi - xlo)) * (w - 2 * pad);
		const sy = (v: number) => h - pad - ((v - ylo) / (yhi - ylo)) * (h - 2 * pad);

		const line = (getComputedStyle(canvas).getPropertyValue('--line') || '#ccc').trim();
		ctx.strokeStyle = line;
		ctx.lineWidth = 1;
		ctx.strokeRect(pad, pad, w - 2 * pad, h - 2 * pad);

		positions = [];
		const fg = (getComputedStyle(canvas).getPropertyValue('--fg') || '#111').trim();
		for (const p of points) {
			const vx = p[axisX];
			const vy = p[axisY];
			if (vx == null || vy == null) continue;
			const x = sx(vx);
			const y = sy(vy);
			positions.push({ p, x, y });
			const playing = $playingId === p.id;
			ctx.beginPath();
			ctx.arc(x, y, playing ? 6 : 4, 0, Math.PI * 2);
			ctx.fillStyle = colorFor(p.category);
			ctx.globalAlpha = playing ? 1 : 0.78;
			ctx.fill();
			if (playing) {
				ctx.globalAlpha = 1;
				ctx.lineWidth = 2;
				ctx.strokeStyle = fg;
				ctx.stroke();
			}
		}
		ctx.globalAlpha = 1;
	}

	onMount(() => {
		(async () => {
			try {
				points = await getGalaxy();
				offline = false;
			} catch {
				offline = true;
			}
		})();
		const ro = new ResizeObserver(() => draw());
		if (canvas) ro.observe(canvas);
		return () => ro.disconnect();
	});

	// redraw whenever the data, axes, or play-state change
	$effect(() => {
		void axisX;
		void axisY;
		void points;
		void $playingId;
		draw();
	});

	function onmove(e: PointerEvent) {
		if (!canvas) return;
		const r = canvas.getBoundingClientRect();
		const mx = e.clientX - r.left;
		const my = e.clientY - r.top;
		let best: { p: GalaxyPoint; x: number; y: number } | null = null;
		let bd = 14 * 14;
		for (const pos of positions) {
			const dx = pos.x - mx;
			const dy = pos.y - my;
			const d = dx * dx + dy * dy;
			if (d < bd) {
				bd = d;
				best = pos;
			}
		}
		hover = best; // moving onto the card (on top) halts these events, so it persists
	}

	function fmtBpm(b: number | null): string {
		return b ? `${Math.round(b)} BPM` : '';
	}
</script>

<header class="glass head">
	<div>
		<h1>Galaxy</h1>
		<p class="muted">
			Samples plotted by timbre. Hover to inspect, click to audition, drag a point into your DAW.
		</p>
	</div>
	<div class="axes">
		<label>X
			<select bind:value={axisX} aria-label="X axis">
				{#each AXES as a (a.key)}<option value={a.key}>{a.label}</option>{/each}
			</select>
		</label>
		<label>Y
			<select bind:value={axisY} aria-label="Y axis">
				{#each AXES as a (a.key)}<option value={a.key}>{a.label}</option>{/each}
			</select>
		</label>
	</div>
</header>

{#if offline}
	<p class="glass pad warn">Library offline — start the API (just dev-api).</p>
{:else if points.length === 0}
	<p class="glass pad muted">No analysed samples yet — the timbre features power this view.</p>
{/if}

<div class="plot glass">
	<canvas bind:this={canvas} onpointermove={onmove} aria-hidden="true"></canvas>

	{#if hover}
		<div
			class="card glass-2"
			style={`left:${hover.x}px; top:${hover.y}px`}
			draggable="true"
			ondragstart={(e) => hover && dragOutSample(e, asSample(hover.p))}
			role="group"
			aria-label={`${hover.p.title} — drag to your DAW`}
		>
			<div class="t">{hover.p.title}</div>
			<div class="meta muted">
				{hover.p.contributor ? `@${hover.p.contributor}` : ''}{hover.p.category
					? ` · ${hover.p.category}`
					: ''}{hover.p.bpm ? ` · ${fmtBpm(hover.p.bpm)}` : ''}
			</div>
			<div class="actions">
				<button class="accent-fill" onclick={() => hover && toggle(hover.p.id)}>
					{$playingId === hover.p.id ? '❚❚' : '▶'} Audition
				</button>
				<a class="ghost" href={`/s/${hover.p.id}`}>Open ↗</a>
			</div>
		</div>
	{/if}
</div>

<div class="bottom">
	<div class="legend" aria-hidden="true">
		{#each categories as c (c)}
			<span class="swatch"><i style={`background:${colorFor(c)}`}></i>{c}</span>
		{/each}
	</div>
	<p class="a11y muted">
		This is a visual map. The same {points.length} samples are listed, searchable and
		keyboard-navigable, in the <a href="/">Library</a>.
	</p>
</div>

<style>
	.head {
		display: flex;
		justify-content: space-between;
		align-items: flex-start;
		gap: var(--space-4);
		padding: var(--space-5);
		border-radius: var(--radius-md);
		margin-bottom: var(--space-4);
	}
	.head h1 {
		margin: 0 0 var(--space-1);
	}
	.axes {
		display: flex;
		gap: var(--space-3);
	}
	.axes label {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		font-size: var(--text-sm);
		color: var(--fg-dim);
	}
	.pad {
		padding: var(--space-4);
		border-radius: var(--radius-md);
		margin-bottom: var(--space-4);
	}
	.warn {
		color: var(--c-warn);
	}
	.plot {
		position: relative;
		border-radius: var(--radius-md);
		overflow: hidden;
		height: 62vh;
		min-height: 380px;
	}
	canvas {
		width: 100%;
		height: 100%;
		display: block;
		touch-action: none;
		cursor: crosshair;
	}
	.card {
		position: absolute;
		transform: translate(12px, 12px);
		max-width: 240px;
		padding: var(--space-3);
		border-radius: var(--radius-md);
		box-shadow: var(--glass-shadow);
		z-index: 5;
		cursor: grab;
	}
	.card .t {
		font-weight: 600;
	}
	.card .meta {
		font-size: var(--text-xs);
		margin: 2px 0 var(--space-2);
	}
	.card .actions {
		display: flex;
		gap: var(--space-2);
	}
	.card .actions a,
	.card .actions button {
		font-size: var(--text-sm);
		padding: var(--space-1) var(--space-3);
		border-radius: var(--radius-sm);
		text-decoration: none;
	}
	.card .ghost {
		background: var(--surface);
		color: var(--fg-dim);
	}
	.bottom {
		margin-top: var(--space-3);
	}
	.legend {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-3);
		font-size: var(--text-xs);
		color: var(--fg-dim);
	}
	.swatch {
		display: inline-flex;
		align-items: center;
		gap: var(--space-1);
	}
	.swatch i {
		width: 10px;
		height: 10px;
		border-radius: 50%;
		display: inline-block;
	}
	.a11y {
		font-size: var(--text-xs);
		margin: var(--space-2) 0 0;
	}
</style>
