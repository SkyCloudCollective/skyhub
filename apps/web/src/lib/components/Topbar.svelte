<script lang="ts">
	import { page } from '$app/stores';
	import { t, locale, setLocale, type Locale } from '$lib/i18n';
	import { theme, toggleTheme } from '$lib/theme';
	import NotificationBell from './NotificationBell.svelte';
	import Icon from './Icon.svelte';

	const links: { href: string; key: Parameters<typeof $t>[0] }[] = [
		{ href: '/', key: 'nav.library' },
		{ href: '/studio', key: 'nav.studio' },
		{ href: '/galaxy', key: 'nav.galaxy' },
		{ href: '/projects', key: 'nav.projects' },
		{ href: '/members', key: 'nav.members' },
		{ href: '/board', key: 'nav.board' },
		{ href: '/download', key: 'nav.downloads' }
	];

	function nextLang(l: Locale): Locale {
		return l === 'en' ? 'fr' : 'en';
	}
</script>

<header class="topbar glass-2">
	<a class="brand" href="/">
		<span class="mark" aria-hidden="true"></span>
		<span class="name">{$t('app.name')}</span>
	</a>

	<nav aria-label="Primary">
		{#each links as l (l.href)}
			<a
				href={l.href}
				class="navlink"
				aria-current={$page.url.pathname === l.href ? 'page' : undefined}
			>
				{$t(l.key)}
			</a>
		{/each}
	</nav>

	<div class="actions">
		<NotificationBell />
		<a class="ghost acct" href="/u/me" title="Profile" aria-label="Profile">me</a>
		<button class="ghost" onclick={toggleTheme} title={$t('theme.toggle')} aria-label={$t('theme.toggle')}>
			<Icon name={$theme === 'aero-dark' ? 'moon' : 'sun'} />
		</button>
		<button
			class="ghost"
			onclick={() => setLocale(nextLang($locale))}
			title={$t('lang.toggle')}
			aria-label={$t('lang.toggle')}
		>
			{$locale.toUpperCase()}
		</button>
	</div>
</header>

<style>
	.topbar {
		display: flex;
		align-items: center;
		gap: var(--space-5);
		height: var(--rail-topbar);
		padding: 0 var(--space-5);
		position: sticky;
		top: 0;
		z-index: 20;
		border-radius: 0;
		border-left: 0;
		border-right: 0;
		border-top: 0;
	}
	.brand {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		text-decoration: none;
		color: var(--fg);
		font-family: var(--font-display);
		font-weight: 600;
		font-size: var(--text-lg);
	}
	/* A Bauhaus mark: three primaries, rendered as a glossy 3D chiclet. */
	.mark {
		position: relative;
		width: 28px;
		height: 28px;
		border-radius: 9px;
		background: linear-gradient(135deg, var(--bauhaus-red) 0 33%, var(--bauhaus-yellow) 33% 66%, var(--bauhaus-blue) 66% 100%);
		box-shadow:
			inset 0 1.5px 0 rgba(255, 255, 255, 0.75),
			inset 0 -4px 7px -2px rgba(0, 0, 0, 0.45),
			0 4px 9px -2px rgba(0, 0, 0, 0.45);
	}
	.mark::after {
		content: '';
		position: absolute;
		inset: 0;
		border-radius: inherit;
		background: linear-gradient(to bottom, rgba(255, 255, 255, 0.6), transparent 48%);
		pointer-events: none;
	}
	nav {
		display: flex;
		gap: var(--space-2);
		margin-right: auto;
	}
	.navlink {
		color: var(--fg-dim);
		text-decoration: none;
		padding: var(--space-1) var(--space-3);
		border-radius: var(--radius-pill);
		font-size: var(--text-sm);
	}
	.navlink:hover {
		color: var(--fg);
		background: var(--surface);
	}
	.navlink[aria-current='page'] {
		color: var(--accent-ink);
		background:
			linear-gradient(
				to bottom,
				color-mix(in srgb, var(--accent) 72%, white),
				var(--accent) 55%,
				color-mix(in srgb, var(--accent) 88%, black)
			);
		box-shadow:
			inset 0 1px 0 color-mix(in srgb, white 55%, transparent),
			0 4px 12px -5px var(--glow);
		text-shadow: 0 1px 0 color-mix(in srgb, black 18%, transparent);
	}
	.actions {
		display: flex;
		gap: var(--space-2);
	}
	.ghost {
		background: var(--surface);
		min-width: 38px;
		text-align: center;
	}
	.acct {
		text-decoration: none;
		color: var(--fg-dim);
		line-height: 26px;
	}

	/* Mobile: wrap the bar; the nav becomes a horizontally-scrollable strip so
	   every destination stays one swipe away (no overflow, no hamburger). */
	@media (max-width: 680px) {
		.topbar {
			flex-wrap: wrap;
			height: auto;
			gap: var(--space-2);
			padding: var(--space-2) var(--space-3);
		}
		nav {
			order: 3;
			width: 100%;
			margin-right: 0;
			overflow-x: auto;
			flex-wrap: nowrap;
			-webkit-overflow-scrolling: touch;
			scrollbar-width: none;
			padding-bottom: 2px;
		}
		nav::-webkit-scrollbar {
			display: none;
		}
		.navlink {
			white-space: nowrap;
		}
		.actions {
			margin-left: auto;
		}
	}
	@media (max-width: 420px) {
		.brand .name {
			display: none;
		}
	}
</style>
