// Tiny API client. In production the static app is same-origin behind the
// reverse proxy (API_BASE = ''); in dev it points at the local FastAPI on :8000.
// Override with VITE_RS_API at build time for the desktop shell / internal builds.
const DEV_DEFAULT = 'http://127.0.0.1:8000';

export const API_BASE: string =
	(import.meta.env.VITE_RS_API as string | undefined) ??
	(import.meta.env.DEV ? DEV_DEFAULT : '');

export async function api<T = unknown>(path: string, init?: RequestInit): Promise<T> {
	const res = await fetch(`${API_BASE}${path}`, init);
	if (!res.ok) throw new Error(`${res.status} ${res.statusText} for ${path}`);
	return (await res.json()) as T;
}

export interface Facets {
	categories: string[];
	kinds: string[];
	instruments: string[];
	contributors: string[];
	packs: string[];
	tags: string[];
}

export interface Health {
	status: string;
	service: string;
	version: string;
}
