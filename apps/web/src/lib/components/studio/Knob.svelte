<script lang="ts">
	import type { Knob } from '$lib/studio/params';

	let {
		ctl,
		value,
		oninput
	}: { ctl: Knob; value: number; oninput: (v: number) => void } = $props();

	// normalised position 0..1 (respecting the curve)
	function toT(v: number): number {
		if (ctl.curve === 'exp' && ctl.min > 0) {
			return Math.log(Math.max(v, ctl.min) / ctl.min) / Math.log(ctl.max / ctl.min);
		}
		return (v - ctl.min) / (ctl.max - ctl.min);
	}
	function fromT(t: number): number {
		const c = Math.min(1, Math.max(0, t));
		if (ctl.curve === 'exp' && ctl.min > 0) {
			return ctl.min * Math.pow(ctl.max / ctl.min, c);
		}
		return ctl.min + (ctl.max - ctl.min) * c;
	}

	const t = $derived(Math.min(1, Math.max(0, toT(value))));

	function fmt(v: number): string {
		const u = ctl.unit ?? '';
		if (u === 'Hz') return v >= 1000 ? `${(v / 1000).toFixed(1)}k` : v.toFixed(v < 10 ? 2 : 0);
		if (u === 's') return v < 1 ? `${Math.round(v * 1000)}ms` : `${v.toFixed(2)}s`;
		if (u === '¢' || u === 'st') return `${v >= 0 ? '+' : ''}${Math.round(v)}${u}`;
		return Math.abs(v) >= 100 ? v.toFixed(0) : v.toFixed(2);
	}

	function set(nt: number) {
		oninput(fromT(nt));
	}

	// ── pointer drag (vertical) ──
	let dragging = $state(false);
	function onpointerdown(e: PointerEvent) {
		dragging = true;
		(e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
		let lastY = e.clientY;
		const move = (ev: PointerEvent) => {
			const dy = lastY - ev.clientY;
			lastY = ev.clientY;
			const speed = ev.shiftKey ? 0.0015 : 0.006; // fine with Shift
			set(t + dy * speed);
		};
		const up = (ev: PointerEvent) => {
			dragging = false;
			(e.currentTarget as HTMLElement).releasePointerCapture?.(ev.pointerId);
			window.removeEventListener('pointermove', move);
			window.removeEventListener('pointerup', up);
		};
		window.addEventListener('pointermove', move);
		window.addEventListener('pointerup', up);
	}

	function ondblclick() {
		oninput(ctl.def);
	}

	function onkeydown(e: KeyboardEvent) {
		const step = e.shiftKey ? 0.1 : 0.02;
		if (e.key === 'ArrowUp' || e.key === 'ArrowRight') set(t + step);
		else if (e.key === 'ArrowDown' || e.key === 'ArrowLeft') set(t - step);
		else if (e.key === 'Home') set(0);
		else if (e.key === 'End') set(1);
		else return;
		e.preventDefault();
	}

	// ── gauge geometry (270° sweep, gap at the bottom) ──
	const R = 16;
	function polar(deg: number) {
		const a = (deg * Math.PI) / 180;
		return [24 + R * Math.cos(a), 24 + R * Math.sin(a)];
	}
	function arc(t0: number, t1: number): string {
		const a0 = 135 + 270 * t0;
		const a1 = 135 + 270 * t1;
		const [x0, y0] = polar(a0);
		const [x1, y1] = polar(a1);
		const large = a1 - a0 > 180 ? 1 : 0;
		return `M ${x0} ${y0} A ${R} ${R} 0 ${large} 1 ${x1} ${y1}`;
	}
	const indicator = $derived(polar(135 + 270 * t));
</script>

<div class="knob" class:dragging>
	<div
		class="dial"
		role="slider"
		tabindex="0"
		aria-label={ctl.label}
		aria-valuemin={ctl.min}
		aria-valuemax={ctl.max}
		aria-valuenow={Math.round(value * 100) / 100}
		aria-valuetext={`${fmt(value)} ${ctl.unit ?? ''}`.trim()}
		{onpointerdown}
		{onkeydown}
		{ondblclick}
		title={`${ctl.label} — drag, or arrows; double-click resets`}
	>
		<svg viewBox="0 0 48 48" aria-hidden="true">
			<path class="track" d={arc(0, 1)} />
			<path class="fill" d={arc(0, t)} />
			<line class="needle" x1="24" y1="24" x2={indicator[0]} y2={indicator[1]} />
		</svg>
	</div>
	<span class="label">{ctl.label}</span>
	<span class="value">{fmt(value)}</span>
</div>

<style>
	.knob {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 2px;
		width: 64px;
	}
	.dial {
		width: 48px;
		height: 48px;
		border-radius: 50%;
		cursor: ns-resize;
		touch-action: none;
		outline: none;
	}
	.dial:focus-visible {
		box-shadow: var(--focus-ring);
		border-radius: 50%;
	}
	svg {
		width: 48px;
		height: 48px;
	}
	.track {
		fill: none;
		stroke: var(--line);
		stroke-width: 3;
		stroke-linecap: round;
	}
	.fill {
		fill: none;
		stroke: var(--accent);
		stroke-width: 3;
		stroke-linecap: round;
	}
	.dragging .fill {
		stroke: var(--accent-2, var(--accent));
	}
	.needle {
		stroke: var(--fg);
		stroke-width: 2;
		stroke-linecap: round;
	}
	.label {
		font-size: var(--text-xs);
		color: var(--fg-dim);
	}
	.value {
		font-size: var(--text-xs);
		font-family: var(--font-mono);
		color: var(--muted);
	}
</style>
