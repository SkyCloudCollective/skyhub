// Single shared audio element: only one sample plays at a time. The current id
// is a store so cards can show their own play/pause state.
import { writable } from 'svelte/store';
import { browser } from '$app/environment';
import { previewUrl } from './api';

export const playingId = writable<number | null>(null);

let audio: HTMLAudioElement | null = null;

function el(): HTMLAudioElement {
	if (!audio) {
		audio = new Audio();
		audio.addEventListener('ended', () => playingId.set(null));
		audio.addEventListener('pause', () => {
			if (audio && audio.currentTime === 0) playingId.set(null);
		});
	}
	return audio;
}

export function toggle(id: number) {
	if (!browser) return;
	const a = el();
	let current: number | null = null;
	playingId.update((v) => ((current = v), v));
	if (current === id) {
		a.pause();
		a.currentTime = 0;
		playingId.set(null);
		return;
	}
	a.src = previewUrl(id);
	a.currentTime = 0;
	void a.play().then(() => playingId.set(id)).catch(() => playingId.set(null));
}

export function stop() {
	if (audio) {
		audio.pause();
		audio.currentTime = 0;
	}
	playingId.set(null);
}
