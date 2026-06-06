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

export const previewUrl = (id: number) => `${API_BASE}/v1/preview/${id}`;
export const downloadUrl = (id: number) => `${API_BASE}/v1/download/${id}`;

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

export interface Sample {
	id: number;
	filename: string;
	title: string;
	contributor: string | null;
	pack: string | null;
	category: string | null;
	kind: string | null;
	instrument: string | null;
	bpm: number | null;
	musical_key: string | null;
	duration_ms: number | null;
	brightness: number | null;
	noisiness: number | null;
	percussiveness: number | null;
	loudness: number | null;
	samplerate: number | null;
	channels: number | null;
	bytes: number | null;
	original_format: string | null;
	created_at: string;
}

export interface SearchResult {
	total: number;
	limit: number;
	offset: number;
	hits: Sample[];
}

export interface SearchParams {
	q?: string;
	category?: string;
	kind?: string;
	instrument?: string;
	contributor?: string;
	pack?: string;
	key?: string;
	bpm_min?: number;
	bpm_max?: number;
	tag?: string;
	limit?: number;
	offset?: number;
}

export function search(params: SearchParams = {}): Promise<SearchResult> {
	const qs = new URLSearchParams();
	for (const [k, v] of Object.entries(params)) {
		if (v !== undefined && v !== null && v !== '') qs.set(k, String(v));
	}
	const q = qs.toString();
	return api<SearchResult>(`/v1/search${q ? `?${q}` : ''}`);
}

export const getFacets = () => api<Facets>('/v1/facets');
export const getHealth = () => api<Health>('/v1/health');
export const getPeaks = (id: number) => api<number[][]>(`/v1/peaks/${id}`);
