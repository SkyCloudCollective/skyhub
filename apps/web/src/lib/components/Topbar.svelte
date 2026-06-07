<script lang="ts">
	import { page } from '$app/stores';
	import { t, locale, setLocale, type Locale } from '$lib/i18n';
	import { theme, toggleTheme } from '$lib/theme';
	import NotificationBell from './NotificationBell.svelte';

	const links: { href: string; key: Parameters<typeof $t>[0] }[] = [
		{ href: '/', key: 'nav.library' },
		{ href: '/studio', key: 'nav.studio' },
		{ href: '/galaxy', key: 'nav.galaxy' },
		{ href: '/projects', key: 'nav.projects' },
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
			{$theme === 'aero-dark' ? '☾' : '☀'}
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
	/* A Bauhaus mark: three primary squares. */
	.mark {
		width: 22px;
		height: 22px;
		border-radius: 5px;
		background: linear-gradient(135deg, var(--bauhaus-red) 0 33%, var(--bauhaus-yellow) 33% 66%, var(--bauhaus-blue) 66% 100%);
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
		background: var(--accent);
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
</style>
