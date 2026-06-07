<script lang="ts">
	import { onMount } from 'svelte';
	import { getNotifications, readNotifications, type Notification } from '$lib/api';
	import Icon from './Icon.svelte';

	let items = $state<Notification[]>([]);
	let unread = $state(0);
	let open = $state(false);
	let root = $state<HTMLDivElement>();

	async function refresh() {
		try {
			const inbox = await getNotifications();
			items = inbox.items;
			unread = inbox.unread;
		} catch {
			/* offline — leave as is */
		}
	}

	async function toggle() {
		open = !open;
		if (open && unread > 0) {
			await readNotifications().catch(() => {});
			unread = 0;
		}
	}

	function verb(n: Notification): string {
		if (n.kind === 'comment') return 'commented on';
		if (n.kind === 'reaction') return 'reacted to';
		if (n.kind === 'follow') return 'followed you';
		return n.kind;
	}
	function subjectLabel(n: Notification): string {
		const d = n.data as { title?: string } | null;
		return d?.title ?? '';
	}
	function href(n: Notification): string {
		if (n.subject_type === 'sample' && n.subject_id) return `/s/${n.subject_id}`;
		if (n.kind === 'follow') return `/u/${n.actor}`;
		return '#';
	}

	onMount(() => {
		refresh();
		const id = setInterval(refresh, 25_000); // calm poll
		const onClick = (e: MouseEvent) => {
			if (open && root && !root.contains(e.target as Node)) open = false;
		};
		window.addEventListener('click', onClick);
		return () => {
			clearInterval(id);
			window.removeEventListener('click', onClick);
		};
	});
</script>

<div class="bell-wrap" bind:this={root}>
	<button class="ghost bell" onclick={toggle} aria-label={`Notifications${unread ? ` (${unread} new)` : ''}`} aria-expanded={open}>
		<Icon name="bell" />
		{#if unread > 0}<span class="badge" aria-hidden="true">{unread > 9 ? '9+' : unread}</span>{/if}
	</button>

	{#if open}
		<div class="panel glass-2" role="menu">
			<h3>Notifications</h3>
			<ul>
				{#each items as n (n.id)}
					<li class:unread={!n.read}>
						<a href={href(n)} role="menuitem">
							<strong>@{n.actor}</strong> {verb(n)}{subjectLabel(n) ? ` “${subjectLabel(n)}”` : ''}
						</a>
					</li>
				{:else}
					<li class="empty muted">Nothing yet.</li>
				{/each}
			</ul>
		</div>
	{/if}
</div>

<style>
	.bell-wrap {
		position: relative;
	}
	.bell {
		position: relative;
		min-width: 38px;
	}
	.badge {
		position: absolute;
		top: -4px;
		right: -4px;
		min-width: 16px;
		height: 16px;
		padding: 0 3px;
		border-radius: var(--radius-pill);
		background: var(--accent);
		color: var(--accent-ink);
		font-size: 10px;
		line-height: 16px;
		text-align: center;
		font-family: var(--font-mono);
	}
	.panel {
		position: absolute;
		right: 0;
		top: calc(100% + var(--space-2));
		width: 300px;
		max-height: 60vh;
		overflow: auto;
		padding: var(--space-3);
		border-radius: var(--radius-md);
		box-shadow: var(--glass-shadow);
		z-index: 30;
	}
	.panel h3 {
		margin: 0 0 var(--space-2);
		font-size: var(--text-sm);
		color: var(--fg-dim);
	}
	ul {
		list-style: none;
		margin: 0;
		padding: 0;
	}
	li {
		padding: var(--space-2) 0;
		border-top: 1px solid var(--line);
		font-size: var(--text-sm);
	}
	li:first-child {
		border-top: 0;
	}
	li a {
		color: var(--fg);
		text-decoration: none;
	}
	li a:hover {
		color: var(--accent);
	}
	li.unread a strong {
		color: var(--accent);
	}
	.empty {
		padding: var(--space-3) 0;
	}
</style>
