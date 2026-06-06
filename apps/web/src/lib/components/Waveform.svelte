<script lang="ts">
	// Draws compact [[min,max], …] peaks onto a canvas. Cheap, no audio decode.
	let { peaks = [], playing = false }: { peaks?: number[][]; playing?: boolean } = $props();

	let canvas: HTMLCanvasElement | undefined = $state();

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

		const accent = getComputedStyle(canvas).getPropertyValue('--wave') || '#2b6cd4';
		ctx.fillStyle = accent.trim();
		const mid = h / 2;
		const n = peaks.length || 1;
		const bw = w / n;
		for (let i = 0; i < peaks.length; i++) {
			const [lo, hi] = peaks[i];
			const top = mid - hi * mid * 0.92;
			const bot = mid - lo * mid * 0.92;
			ctx.fillRect(i * bw, top, Math.max(1, bw - 0.5), Math.max(1, bot - top));
		}
	}

	$effect(() => {
		// re-draw when peaks change or play-state flips the colour
		void peaks;
		void playing;
		draw();
	});
</script>

<canvas bind:this={canvas} class:playing aria-hidden="true"></canvas>

<style>
	canvas {
		width: 100%;
		height: 48px;
		display: block;
		--wave: color-mix(in srgb, var(--fg-dim) 70%, transparent);
	}
	canvas.playing {
		--wave: var(--accent);
	}
</style>
