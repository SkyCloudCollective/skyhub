// Client-side mirror of the server's embed allowlist (services/api/app/tube.py).
// Used ONLY for the live "paste a link → see the provider + preview" affordance
// before posting. The authoritative validation + stored (provider, video_ref)
// always come from the server; the iframe src for stored videos is the server's
// rebuilt `embed_url`. Keeping this in lockstep means the preview matches what
// the server will accept (no surprise 400 after typing).
import type { VideoProvider } from '$lib/api';

const YOUTUBE_ID = /^[A-Za-z0-9_-]{11}$/;
const VIMEO_ID = /^[0-9]{6,12}$/;
const PT_HOST = /^[A-Za-z0-9.-]{3,253}$/;
const PT_ID = /^[A-Za-z0-9_-]{6,64}$/;

const YOUTUBE_HOSTS = new Set(['youtube.com', 'www.youtube.com', 'm.youtube.com', 'youtu.be']);
const VIMEO_HOSTS = new Set(['vimeo.com', 'www.vimeo.com', 'player.vimeo.com']);

export interface ParsedEmbed {
	provider: VideoProvider;
	videoRef: string;
}

function host(u: URL): string {
	return u.hostname.toLowerCase();
}

/** Mirror of tube.parse_embed → {provider, videoRef} or null if unsupported. */
export function parseEmbed(rawUrl: string): ParsedEmbed | null {
	const raw = (rawUrl ?? '').trim();
	if (!raw) return null;
	let u: URL;
	try {
		u = new URL(raw);
	} catch {
		return null;
	}
	if (u.protocol !== 'https:') return null;
	const h = host(u);
	if (!h) return null;

	if (YOUTUBE_HOSTS.has(h)) {
		let vid = '';
		if (h === 'youtu.be') vid = u.pathname.replace(/^\/+/, '').split('/')[0];
		else if (u.pathname === '/watch') vid = u.searchParams.get('v') ?? '';
		else if (/^\/(embed|shorts|live)\//.test(u.pathname)) vid = u.pathname.split('/')[2] ?? '';
		return YOUTUBE_ID.test(vid) ? { provider: 'youtube', videoRef: vid } : null;
	}

	if (VIMEO_HOSTS.has(h)) {
		const parts = u.pathname.split('/').filter(Boolean);
		const vid = parts[parts.length - 1] ?? '';
		return VIMEO_ID.test(vid) ? { provider: 'vimeo', videoRef: vid } : null;
	}

	// PeerTube: /w/<id> or /videos/watch/<id> or /videos/embed/<id>
	const parts = u.pathname.split('/').filter(Boolean);
	let ptId = '';
	if (parts.length === 2 && parts[0] === 'w') ptId = parts[1];
	else if (parts.length === 3 && parts[0] === 'videos' && (parts[1] === 'watch' || parts[1] === 'embed'))
		ptId = parts[2];
	if (ptId && PT_HOST.test(h) && PT_ID.test(ptId)) return { provider: 'peertube', videoRef: `${h}/${ptId}` };

	return null;
}

/** Mirror of tube.embed_url — build the iframe src from a validated pair. */
export function embedUrl(provider: VideoProvider, videoRef: string): string {
	if (provider === 'youtube') return `https://www.youtube-nocookie.com/embed/${videoRef}`;
	if (provider === 'vimeo') return `https://player.vimeo.com/video/${videoRef}`;
	const [h, id] = videoRef.split('/');
	return `https://${h}/videos/embed/${id}`;
}
