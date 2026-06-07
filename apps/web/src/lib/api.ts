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

// ── community: profiles + collections + favourites ──────────────────────────
export interface Collection {
	id: number;
	owner: string;
	name: string;
	slug: string | null;
	parent_id: number | null;
	kind: string; // 'crate' | 'favorites' | 'smart'
	visibility: string; // 'private' | 'unlisted' | 'public'
	share_token: string | null;
	position: number;
	count?: number;
}

export interface CollectionItem {
	id: number;
	filename: string;
	title: string;
	contributor: string | null;
	category: string | null;
	kind: string | null;
	instrument: string | null;
	bpm: number | null;
	musical_key: string | null;
	duration_ms: number | null;
	position: number;
}

export interface CollectionFull {
	collection: Collection;
	items: CollectionItem[];
}

export interface Profile {
	handle: string;
	display_name: string | null;
	bio: string | null;
	avatar_path: string | null;
	links: { label: string; url: string }[];
	sample_count: number;
}

async function send<T>(path: string, method: string, body?: unknown): Promise<T | null> {
	const res = await fetch(`${API_BASE}${path}`, {
		method,
		headers: body !== undefined ? { 'content-type': 'application/json' } : undefined,
		body: body !== undefined ? JSON.stringify(body) : undefined
	});
	if (!res.ok) throw new Error(`${res.status} ${res.statusText} for ${path}`);
	if (res.status === 204) return null;
	return (await res.json()) as T;
}

export const listCollections = (owner?: string) =>
	api<Collection[]>(`/v1/collections${owner ? `?owner=${encodeURIComponent(owner)}` : ''}`);
export const createCollection = (name: string, visibility = 'private') =>
	send<Collection>('/v1/collections', 'POST', { name, visibility });
export const getCollection = (id: number, token?: string) =>
	api<CollectionFull>(`/v1/collections/${id}${token ? `?token=${encodeURIComponent(token)}` : ''}`);
export const patchCollection = (id: number, body: Partial<Pick<Collection, 'name' | 'visibility' | 'position'>>) =>
	send<Collection>(`/v1/collections/${id}`, 'PATCH', body);
export const deleteCollection = (id: number) => send<null>(`/v1/collections/${id}`, 'DELETE');
export const addItem = (id: number, sample_id: number) =>
	send<{ ok: boolean }>(`/v1/collections/${id}/items`, 'POST', { sample_id });
export const removeItem = (id: number, sample_id: number) =>
	send<null>(`/v1/collections/${id}/items/${sample_id}`, 'DELETE');

export const getFavoriteIds = () => api<number[]>('/v1/favorites/ids');
export const addFavorite = (sample_id: number) => send('/v1/favorites', 'POST', { sample_id });
export const removeFavorite = (sample_id: number) => send<null>(`/v1/favorites/${sample_id}`, 'DELETE');

export const getProfile = (handle: string) => api<Profile>(`/v1/profile/${encodeURIComponent(handle)}`);
export const patchProfile = (body: Partial<Pick<Profile, 'display_name' | 'bio' | 'links'>>) =>
	send<Profile>('/v1/me/profile', 'PATCH', body);

export interface Me {
	auth_enabled: boolean;
	handle: string;
}
export const getMe = () => api<Me>('/v1/me');

// ── collaboration: projects ("a GitHub of music") ───────────────────────────
export interface Project {
	id: number;
	owner: string;
	title: string;
	slug: string | null;
	description: string | null;
	kind: string | null;
	daw: string | null;
	visibility: string; // 'private' | 'unlisted' | 'open'
	storage: string;
	license: string | null;
	updated_at: string;
	role?: string | null;
	members?: number;
	files?: number;
}
export interface ProjectMember {
	handle: string;
	role: string;
	accepted_at: string | null;
}
export interface ProjectFile {
	rel_path: string;
	kind: string | null;
	bytes: number | null;
	sha256: string | null;
	version: number;
	updated_by: string | null;
	updated_at: string;
}
export interface ProjectActivity {
	actor: string | null;
	action: string;
	target: string | null;
	created_at: string;
}
export interface ProjectFull {
	project: Project;
	role: string | null;
	members: ProjectMember[];
	files: ProjectFile[];
	activity: ProjectActivity[];
}
export interface Invite {
	id: number;
	project_id: number;
	role: string;
	invited_by: string | null;
	title: string;
	slug: string | null;
}

export const listProjects = () => api<Project[]>('/v1/projects');
export const createProject = (body: { title: string; kind?: string; daw?: string; visibility?: string }) =>
	send<Project>('/v1/projects', 'POST', body);
export const getProject = (idOrSlug: string | number) =>
	api<ProjectFull>(`/v1/projects/${encodeURIComponent(String(idOrSlug))}`);
export const patchProject = (id: number, body: Partial<Pick<Project, 'title' | 'description' | 'kind' | 'daw' | 'license' | 'visibility'>>) =>
	send<Project>(`/v1/projects/${id}`, 'PATCH', body);
export const deleteProject = (id: number) => send<null>(`/v1/projects/${id}`, 'DELETE');

export const inviteMember = (id: number, handle: string, role = 'editor') =>
	send<{ id: number; status: string }>(`/v1/projects/${id}/invite`, 'POST', { handle, role });
export const myInvites = () => api<Invite[]>('/v1/invites');
export const acceptInvite = (id: number) => send(`/v1/invites/${id}/accept`, 'POST', {});
export const declineInvite = (id: number) => send(`/v1/invites/${id}/decline`, 'POST', {});
export const joinProject = (id: number) => send(`/v1/projects/${id}/join`, 'POST', {});
export const removeMember = (id: number, handle: string) =>
	send<null>(`/v1/projects/${id}/members/${encodeURIComponent(handle)}`, 'DELETE');

export const projectFileUrl = (id: number, relPath: string) =>
	`${API_BASE}/v1/projects/${id}/files/${relPath.split('/').map(encodeURIComponent).join('/')}`;
export async function uploadProjectFile(id: number, relPath: string, file: Blob, kind?: string) {
	const path = relPath.split('/').map(encodeURIComponent).join('/');
	const res = await fetch(`${API_BASE}/v1/projects/${id}/files/${path}${kind ? `?kind=${kind}` : ''}`, {
		method: 'PUT',
		body: file
	});
	if (!res.ok) throw new Error(`${res.status} for upload`);
	return res.json();
}
export const deleteProjectFile = (id: number, relPath: string) => {
	const path = relPath.split('/').map(encodeURIComponent).join('/');
	return send<null>(`/v1/projects/${id}/files/${path}`, 'DELETE');
};
export const projectComments = (id: number) =>
	api<{ id: number; rel_path: string | null; handle: string; body: string; created_at: string }[]>(`/v1/projects/${id}/comments`);
export const addProjectComment = (id: number, body: string, rel_path?: string) =>
	send(`/v1/projects/${id}/comments`, 'POST', { body, rel_path });
