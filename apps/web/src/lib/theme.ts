// Theme store. Aero light (default) ↔ Aero dark. Persisted; applied as the
// data-theme attribute the tokens read. High-contrast escape hatch separate.
import { writable } from 'svelte/store';
import { browser } from '$app/environment';

export type Theme = 'aero-light' | 'aero-dark';

function detect(): Theme {
	if (!browser) return 'aero-light';
	const saved = localStorage.getItem('rs_theme');
	if (saved === 'aero-light' || saved === 'aero-dark') return saved;
	return window.matchMedia('(prefers-color-scheme: dark)').matches ? 'aero-dark' : 'aero-light';
}

export const theme = writable<Theme>(detect());

if (browser) {
	theme.subscribe((t) => {
		document.documentElement.dataset.theme = t;
		localStorage.setItem('rs_theme', t);
	});
}

export function toggleTheme() {
	theme.update((t) => (t === 'aero-light' ? 'aero-dark' : 'aero-light'));
}
