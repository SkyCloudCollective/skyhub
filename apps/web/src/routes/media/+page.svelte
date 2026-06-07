<script lang="ts">
	import { onMount } from 'svelte';
	import { t } from '$lib/i18n';
	import { getHub, type HubService } from '$lib/api';

	let services = $state<HubService[]>([]);
	let loaded = $state(false);

	onMount(async () => {
		try {
			services = (await getHub()).services;
		} catch {
			services = [];
		}
		loaded = true;
	});
</script>

<svelte:head><title>{$t('media.title')} · RanchSamples</title></svelte:head>

<section class="head glass-2">
	<h1>{$t('media.title')}</h1>
	<p>{$t('media.intro')}</p>
</section>

{#if loaded && !services.length}
	<p class="empty muted">{$t('media.empty')}</p>
{:else}
	<div class="grid">
		{#each services as s (s.key)}
			<a class="card glass-2" href={s.url} target="_blank" rel="noopener noreferrer" style="--card-accent:{s.accent}">
				<span class="badge" aria-hidden="true">{s.name.slice(0, 1)}</span>
				<span class="body">
					<span class="name">{s.name}</span>
					<span class="tool">{s.tool}</span>
					<span class="desc">{s.desc}</span>
				</span>
				<span class="open">{$t('media.open')} ↗</span>
			</a>
		{/each}
	</div>
{/if}

<style>
	.head {
		padding: var(--space-5) var(--space-6);
		border-radius: var(--radius-lg);
		margin-bottom: var(--space-6);
	}
	.head h1 {
		margin: 0 0 var(--space-2);
	}
	.head p {
		margin: 0;
		color: var(--fg-dim);
		max-width: 60ch;
	}
	.grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(248px, 1fr));
		gap: var(--space-4);
	}
	.card {
		display: flex;
		flex-direction: column;
		gap: var(--space-3);
		padding: var(--space-5);
		border-radius: var(--radius-lg);
		text-decoration: none;
		color: var(--fg);
		border-top: 3px solid var(--card-accent);
		transition:
			transform var(--dur-fast) var(--ease-out),
			box-shadow var(--dur-fast) var(--ease-out);
	}
	.card:hover {
		transform: translateY(-3px);
	}
	.badge {
		width: 44px;
		height: 44px;
		border-radius: 13px;
		display: inline-flex;
		align-items: center;
		justify-content: center;
		font-family: var(--font-display);
		font-weight: 700;
		font-size: var(--text-lg);
		color: #fff;
		background: linear-gradient(150deg, var(--card-accent), color-mix(in srgb, var(--card-accent) 60%, #000));
		box-shadow:
			inset 0 1px 0 rgba(255, 255, 255, 0.6),
			0 4px 10px -4px var(--glass-lo);
	}
	.body {
		display: flex;
		flex-direction: column;
		gap: 2px;
	}
	.name {
		font-family: var(--font-display);
		font-weight: 600;
		font-size: var(--text-lg);
	}
	.tool {
		font-size: var(--text-xs);
		color: var(--muted);
		text-transform: uppercase;
		letter-spacing: 0.04em;
	}
	.desc {
		font-size: var(--text-sm);
		color: var(--fg-dim);
		margin-top: var(--space-1);
	}
	.open {
		margin-top: auto;
		font-size: var(--text-sm);
		color: var(--accent);
	}
	.empty {
		padding: var(--space-8) var(--space-5);
		text-align: center;
	}
</style>
