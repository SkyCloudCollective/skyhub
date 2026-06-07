// The acting member handle, persisted locally and sent on every API request as
// the X-RS-Handle header (see api.ts). This is the app identity that coexists
// with the optional HTTP Basic preview gate (which uses Authorization), so the
// two never collide. Default 'guest' until the member picks a handle.
import { writable } from 'svelte/store';
import { browser } from '$app/environment';
import { clean } from '$lib/identity';

const KEY = 'rs_handle';
const DEFAULT = 'guest';

function detect(): string {
	if (!browser) return DEFAULT;
	return clean(localStorage.getItem(KEY) ?? '') || DEFAULT;
}

export const handle = writable<string>(detect());

if (browser) {
	handle.subscribe((h) => localStorage.setItem(KEY, h || DEFAULT));
}

/** The current handle, readable synchronously from non-Svelte code (api.ts). */
export function currentHandle(): string {
	if (!browser) return DEFAULT;
	return clean(localStorage.getItem(KEY) ?? '') || DEFAULT;
}

export function setHandle(raw: string) {
	handle.set(clean(raw) || DEFAULT);
}
