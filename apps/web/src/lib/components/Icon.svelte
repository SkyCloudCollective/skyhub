<script lang="ts">
	// One tiny inline-SVG icon set — crisp on every platform (no emoji-glyph
	// roulette). Sized in em so it scales with the surrounding text; inherits
	// currentColor. Decorative by default; pass `label` for a standalone control.
	let { name, size = '1.15em', label }: { name: string; size?: string; label?: string } = $props();
</script>

<svg
	class="icon"
	viewBox="0 0 24 24"
	width={size}
	height={size}
	role={label ? 'img' : undefined}
	aria-label={label}
	aria-hidden={label ? undefined : 'true'}
	focusable="false"
>
	{#if name === 'bell'}
		<g fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
			<path d="M18 8a6 6 0 1 0-12 0c0 7-3 9-3 9h18s-3-2-3-9Z" />
			<path d="M13.7 21a2 2 0 0 1-3.4 0" />
		</g>
	{:else if name === 'sun'}
		<g fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
			<circle cx="12" cy="12" r="4" />
			<path d="M12 2v2M12 20v2M4 12H2M22 12h-2M5 5l1.5 1.5M17.5 17.5L19 19M19 5l-1.5 1.5M6.5 17.5L5 19" />
		</g>
	{:else if name === 'moon'}
		<path fill="currentColor" d="M21 12.8A9 9 0 1 1 11.2 3a7 7 0 0 0 9.8 9.8Z" />
	{:else if name === 'play'}
		<path fill="currentColor" d="M8 5.5v13a1 1 0 0 0 1.5.87l11-6.5a1 1 0 0 0 0-1.74l-11-6.5A1 1 0 0 0 8 5.5Z" />
	{:else if name === 'pause'}
		<path fill="currentColor" d="M8 4h3v16H8zM13 4h3v16h-3z" />
	{:else if name === 'star'}
		<path fill="none" stroke="currentColor" stroke-width="1.7" stroke-linejoin="round" d="M12 3.6l2.6 5.3 5.8.85-4.2 4.1 1 5.8L12 17l-5.2 2.65 1-5.8-4.2-4.1 5.8-.85Z" />
	{:else if name === 'star-fill'}
		<path fill="currentColor" d="M12 3.6l2.6 5.3 5.8.85-4.2 4.1 1 5.8L12 17l-5.2 2.65 1-5.8-4.2-4.1 5.8-.85Z" />
	{:else if name === 'download'}
		<g fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
			<path d="M12 4v11" /><path d="M8 11l4 4 4-4" /><path d="M5 20h14" />
		</g>
	{:else if name === 'power'}
		<g fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
			<path d="M12 3v8.5" /><path d="M6.6 7.2a8 8 0 1 0 10.8 0" />
		</g>
	{:else if name === 'dice'}
		<g fill="none" stroke="currentColor" stroke-width="1.7">
			<rect x="3.5" y="3.5" width="17" height="17" rx="4.5" />
		</g>
		<g fill="currentColor">
			<circle cx="8.5" cy="8.5" r="1.4" /><circle cx="15.5" cy="8.5" r="1.4" />
			<circle cx="12" cy="12" r="1.4" />
			<circle cx="8.5" cy="15.5" r="1.4" /><circle cx="15.5" cy="15.5" r="1.4" />
		</g>
	{:else if name === 'caret-up'}
		<path fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" d="M6 15l6-6 6 6" />
	{:else if name === 'arrow-up-right'}
		<g fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
			<path d="M7 17L17 7" /><path d="M8 7h9v9" />
		</g>
	{/if}
</svg>

<style>
	.icon {
		display: inline-block;
		vertical-align: -0.15em;
		flex: 0 0 auto;
	}
</style>
