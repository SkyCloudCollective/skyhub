import adapter from '@sveltejs/adapter-static';

/** @type {import('@sveltejs/kit').Config} */
const config = {
	compilerOptions: {
		// Force runes mode for the project, except for libraries. Can be removed in svelte 6.
		runes: ({ filename }) => (filename.split(/[/\\]/).includes('node_modules') ? undefined : true)
	},
	kit: {
		// Static SPA: Caddy serves plain files; the app fetches the API at runtime
		// (connect-src to the configured host). No Node runtime in production.
		adapter: adapter({
			fallback: 'index.html',
			precompress: true,
			strict: false
		})
	}
};

export default config;
