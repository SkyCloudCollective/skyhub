// Mirror of the server's auth.clean_handle: '@Alice:server.tld' / 'Alice' -> 'alice'.
// Keep this in lockstep with services/api/app/auth.py so the X-RS-Handle the
// client sends is exactly what the server would normalise it to.
export function clean(raw: string | null | undefined): string {
	if (!raw) return '';
	let s = raw.trim().replace(/^@+/, '');
	s = s.split(':', 1)[0];
	s = s.replace(/[^A-Za-z0-9._-]/g, '');
	return s.toLowerCase();
}
