// Theme: two independent axes + per-user customisation, all persisted and
// applied to <html> (the tokens read data-theme / data-palette / inline --accent
// / data-contrast). The light/dark MODE stays its own store (`theme`) so the
// one-tap toggle keeps working; PALETTE, a CUSTOM accent, a flat-glass switch
// and user-saved themes layer on top.
import { writable } from 'svelte/store';
import { browser } from '$app/environment';

export type Theme = 'aero-light' | 'aero-dark'; // mode
export type Palette = 'aero' | 'ember' | 'violet' | 'aqua' | 'amber' | 'rose';

export const PALETTES: { id: Palette; label: string; swatch: string }[] = [
	{ id: 'aero', label: 'Aero', swatch: '#1f86e8' },
	{ id: 'ember', label: 'Ember', swatch: '#e2473b' },
	{ id: 'violet', label: 'Violet', swatch: '#8b7fe8' },
	{ id: 'aqua', label: 'Aqua', swatch: '#14b8a6' },
	{ id: 'amber', label: 'Amber', swatch: '#f59e0b' },
	{ id: 'rose', label: 'Rose', swatch: '#ec4899' }
];

/** A theme a user composed and saved (name + the four axes). */
export interface SavedTheme {
	name: string;
	mode: Theme;
	palette: Palette;
	accent: string | null;
	flat: boolean;
}

function lsget(key: string): string | null {
	return browser ? localStorage.getItem(key) : null;
}

function detectMode(): Theme {
	const saved = lsget('rs_theme');
	if (saved === 'aero-light' || saved === 'aero-dark') return saved;
	if (browser && window.matchMedia('(prefers-color-scheme: dark)').matches) return 'aero-dark';
	return 'aero-light';
}
function detectPalette(): Palette {
	const saved = lsget('rs_palette') as Palette | null;
	return PALETTES.some((p) => p.id === saved) ? (saved as Palette) : 'aero';
}
function detectSaved(): SavedTheme[] {
	try {
		const raw = lsget('rs_themes');
		return raw ? (JSON.parse(raw) as SavedTheme[]) : [];
	} catch {
		return [];
	}
}

export const theme = writable<Theme>(detectMode());
export const palette = writable<Palette>(detectPalette());
export const customAccent = writable<string | null>(lsget('rs_accent'));
export const flatGlass = writable<boolean>(lsget('rs_flat') === '1');
export const savedThemes = writable<SavedTheme[]>(detectSaved());

/** Pick a legible ink colour (dark/light) for text on an arbitrary accent. */
function readableInk(hex: string): string {
	const c = hex.replace('#', '');
	if (c.length < 6) return '#ffffff';
	const r = parseInt(c.slice(0, 2), 16);
	const g = parseInt(c.slice(2, 4), 16);
	const b = parseInt(c.slice(4, 6), 16);
	const L = (0.2126 * r + 0.7152 * g + 0.0722 * b) / 255;
	return L > 0.62 ? '#06121d' : '#ffffff';
}

if (browser) {
	const el = document.documentElement;
	theme.subscribe((t) => {
		el.dataset.theme = t;
		localStorage.setItem('rs_theme', t);
	});
	palette.subscribe((p) => {
		if (p === 'aero') delete el.dataset.palette;
		else el.dataset.palette = p;
		localStorage.setItem('rs_palette', p);
	});
	customAccent.subscribe((c) => {
		if (c) {
			el.style.setProperty('--accent', c);
			el.style.setProperty('--accent-ink', readableInk(c));
			localStorage.setItem('rs_accent', c);
		} else {
			el.style.removeProperty('--accent');
			el.style.removeProperty('--accent-ink');
			localStorage.removeItem('rs_accent');
		}
	});
	flatGlass.subscribe((f) => {
		if (f) el.dataset.contrast = 'high';
		else if (el.dataset.contrast === 'high') delete el.dataset.contrast;
		localStorage.setItem('rs_flat', f ? '1' : '0');
	});
	savedThemes.subscribe((list) => localStorage.setItem('rs_themes', JSON.stringify(list)));
}

export function toggleTheme() {
	theme.update((t) => (t === 'aero-light' ? 'aero-dark' : 'aero-light'));
}
/** Choosing a preset palette clears any custom accent. */
export function setPalette(p: Palette) {
	customAccent.set(null);
	palette.set(p);
}
export function setCustomAccent(hex: string) {
	customAccent.set(hex);
}
export function clearCustomAccent() {
	customAccent.set(null);
}
export function toggleFlatGlass() {
	flatGlass.update((f) => !f);
}

/** Capture the current look as a named theme the user can re-apply later. */
export function saveCurrentTheme(name: string) {
	const clean = name.trim().slice(0, 40);
	if (!clean) return;
	// read current store values synchronously (subscribe fires once, then unsub)
	let mode: Theme = 'aero-light',
		pal: Palette = 'aero',
		acc: string | null = null,
		flat = false;
	theme.subscribe((v) => (mode = v))();
	palette.subscribe((v) => (pal = v))();
	customAccent.subscribe((v) => (acc = v))();
	flatGlass.subscribe((v) => (flat = v))();
	const snap: SavedTheme = { name: clean, mode, palette: pal, accent: acc, flat };
	savedThemes.update((list) => [...list.filter((t) => t.name !== clean), snap]);
}

export function applySavedTheme(t: SavedTheme) {
	theme.set(t.mode);
	palette.set(t.palette);
	customAccent.set(t.accent);
	flatGlass.set(t.flat);
}

export function deleteSavedTheme(name: string) {
	savedThemes.update((list) => list.filter((t) => t.name !== name));
}
