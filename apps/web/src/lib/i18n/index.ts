// Minimal i18n: a locale store + a derived translator. No dependency, no build
// magic — the v1 "i18n hardcoded across files" clutter becomes one keyed dict.
import { derived, writable } from 'svelte/store';
import { browser } from '$app/environment';
import { en, type Dict } from './en';
import { fr } from './fr';

export type Locale = 'en' | 'fr';
const dicts: Record<Locale, Dict> = { en, fr };

function detect(): Locale {
	if (!browser) return 'en';
	const saved = localStorage.getItem('rs_lang');
	if (saved === 'en' || saved === 'fr') return saved;
	return navigator.language.toLowerCase().startsWith('fr') ? 'fr' : 'en';
}

export const locale = writable<Locale>(detect());

if (browser) {
	locale.subscribe((l) => localStorage.setItem('rs_lang', l));
}

export function setLocale(l: Locale) {
	locale.set(l);
}

/** Usage in markup: {$t('home.welcome')} */
export const t = derived(
	locale,
	($l) =>
		(key: keyof Dict): string =>
			dicts[$l][key] ?? en[key] ?? (key as string)
);
