export const en = {
	'app.name': 'RanchSamples',
	'app.tagline': 'A community-owned sample library.',
	'nav.library': 'Library',
	'nav.studio': 'Studio',
	'nav.galaxy': 'Galaxy',
	'nav.projects': 'Projects',
	'nav.members': 'Members',
	'nav.board': 'Board',
	'nav.downloads': 'Get the app',
	'theme.toggle': 'Theme',
	'lang.toggle': 'Language',
	'home.welcome': 'Welcome',
	'home.intro': 'Browse, preview, organise, and drag straight into your DAW.',
	'home.status.ok': 'Connected to the library.',
	'home.status.offline': 'Library offline — start the API (just dev-api).',
	'home.facets': 'Catalog at a glance',
	'facet.categories': 'categories',
	'facet.instruments': 'instruments',
	'facet.contributors': 'contributors',
	'facet.packs': 'packs'
} as const;

// Keys come from the English dict; values are plain strings so translations
// (fr.ts) aren't forced to the English literal types that `as const` produces.
export type Dict = Record<keyof typeof en, string>;
