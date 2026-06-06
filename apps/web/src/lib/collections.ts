// Client-side stores for favourites + the user's crates. Loaded once, mutated
// optimistically, reconciled with the API.
import { writable, get } from 'svelte/store';
import { browser } from '$app/environment';
import * as api from './api';
import type { Collection } from './api';

export const favoriteIds = writable<Set<number>>(new Set());
export const myCollections = writable<Collection[]>([]);

export async function loadCommunity() {
	if (!browser) return;
	try {
		const [ids, cols] = await Promise.all([api.getFavoriteIds(), api.listCollections()]);
		favoriteIds.set(new Set(ids));
		myCollections.set(cols);
	} catch {
		/* offline — leave empty */
	}
}

export function isFavorite(id: number): boolean {
	return get(favoriteIds).has(id);
}

export async function toggleFavorite(id: number) {
	const set = new Set(get(favoriteIds));
	const on = !set.has(id);
	if (on) set.add(id);
	else set.delete(id);
	favoriteIds.set(set); // optimistic
	try {
		if (on) await api.addFavorite(id);
		else await api.removeFavorite(id);
	} catch {
		await loadCommunity(); // revert to server truth on failure
	}
}

export async function newCrate(name: string): Promise<Collection | null> {
	const c = await api.createCollection(name);
	if (c) myCollections.update((l) => [...l, { ...c, count: 0 }]);
	return c;
}

export async function addToCrate(crateId: number, sampleId: number) {
	await api.addItem(crateId, sampleId);
	myCollections.update((l) =>
		l.map((c) => (c.id === crateId ? { ...c, count: (c.count ?? 0) + 1 } : c))
	);
}
