// See https://svelte.dev/docs/kit/types#app.d.ts
// for information about these interfaces
declare global {
	namespace App {
		// interface Error {}
		// interface Locals {}
		// interface PageData {}
		// interface PageState {}
		// interface Platform {}
	}
}

// CSS-only design-system package (no JS/types) — declare so TS resolves the
// side-effect import in +layout.svelte. Vite resolves the actual CSS at build.
declare module '@skyhub/tokens';

export {};
