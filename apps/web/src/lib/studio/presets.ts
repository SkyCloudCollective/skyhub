// Studio presets — client-side (localStorage) save/load, a seeded "dice"
// mutator, and factory presets for both instruments. No backend: a preset is a
// plain snapshot of the studio's parameter state, shareable by JSON export.
//
// A PresetState captures exactly what the studio holds: the per-id param record,
// plus PhasePlan's mod-matrix routes OR Botanica's XY puck. `dice()` reads the
// control specs (params.ts) so a mutation always stays within each knob's range.
import { browser } from '$app/environment';
import type { Instrument, Route } from './engine';
import { PHASEPLAN_GROUPS, BOTANICA_GROUPS, defaults, type Ctl, type Group } from './params';

export type PresetState = {
	instrument: Instrument;
	version: number;
	params: Record<number, number>;
	routes?: Route[]; // PhasePlan
	xy?: { x: number; y: number }; // Botanica
};

export type NamedPreset = { name: string; state: PresetState };

const VERSION = 1;

function groupsFor(instrument: Instrument): Group[] {
	return instrument === 'phaseplan' ? PHASEPLAN_GROUPS : BOTANICA_GROUPS;
}

function ctlMap(instrument: Instrument): Map<number, Ctl> {
	const m = new Map<number, Ctl>();
	for (const g of groupsFor(instrument)) for (const c of g.ctls) m.set(c.id, c);
	return m;
}

/** A defaults param record for the instrument (id -> default value). */
export function baseParams(instrument: Instrument): Record<number, number> {
	return Object.fromEntries(defaults(groupsFor(instrument)));
}

/** Package the studio's live state into a portable preset. */
export function makeState(
	instrument: Instrument,
	params: Record<number, number>,
	routes: Route[],
	xy: { x: number; y: number }
): PresetState {
	const s: PresetState = { instrument, version: VERSION, params: { ...params } };
	if (instrument === 'phaseplan') s.routes = routes.map((r) => ({ ...r }));
	else s.xy = { ...xy };
	return s;
}

// ── localStorage manager (mirrors theme.ts) ──────────────────────────────────
const storeKey = (i: Instrument) => `rs_presets_${i}`;

export function listUser(instrument: Instrument): NamedPreset[] {
	if (!browser) return [];
	try {
		const raw = localStorage.getItem(storeKey(instrument));
		const arr = raw ? (JSON.parse(raw) as NamedPreset[]) : [];
		return Array.isArray(arr) ? arr : [];
	} catch {
		return [];
	}
}

export function saveUser(instrument: Instrument, name: string, state: PresetState): NamedPreset[] {
	const list = listUser(instrument).filter((p) => p.name !== name);
	list.push({ name, state });
	list.sort((a, b) => a.name.localeCompare(b.name));
	if (browser) localStorage.setItem(storeKey(instrument), JSON.stringify(list));
	return list;
}

export function deleteUser(instrument: Instrument, name: string): NamedPreset[] {
	const list = listUser(instrument).filter((p) => p.name !== name);
	if (browser) localStorage.setItem(storeKey(instrument), JSON.stringify(list));
	return list;
}

// ── dice / mutate (seeded, range-respecting) ─────────────────────────────────
function lcg(seed: number): () => number {
	let s = seed >>> 0 || 1;
	return () => {
		s = (Math.imul(s, 1664525) + 1013904223) >>> 0;
		return s / 4294967296;
	};
}

const clamp = (v: number, lo: number, hi: number) => Math.min(hi, Math.max(lo, v));

/** Mutate every diceable param by ±amount of its range (deterministic per seed). */
export function dice(state: PresetState, amount: number, seed: number): PresetState {
	const rng = lcg(seed);
	const map = ctlMap(state.instrument);
	// leave the output level fixed so the mutation never jumps the volume
	const fixed = state.instrument === 'phaseplan' ? new Set([32]) : new Set<number>();

	const params: Record<number, number> = { ...state.params };
	for (const [idStr, val] of Object.entries(state.params)) {
		const id = +idStr;
		const c = map.get(id);
		if (!c || fixed.has(id)) continue;
		if (c.kind === 'knob') {
			params[id] = clamp(val + (rng() * 2 - 1) * amount * (c.max - c.min), c.min, c.max);
		} else if (rng() < amount * 0.5) {
			params[id] = Math.floor(rng() * c.options.length);
		}
	}

	const out = makeState(
		state.instrument,
		params,
		state.routes ?? [],
		state.xy ?? { x: 0, y: 0 }
	);
	if (state.instrument === 'botanica' && state.xy) {
		out.xy = {
			x: clamp(state.xy.x + (rng() * 2 - 1) * amount, -1, 1),
			y: clamp(state.xy.y + (rng() * 2 - 1) * amount, -1, 1)
		};
	}
	if (state.instrument === 'phaseplan' && state.routes) {
		out.routes = state.routes.map((r) => ({
			...r,
			depth: clamp(r.depth + (rng() * 2 - 1) * amount, -1, 1)
		}));
	}
	return out;
}

// ── factory presets ──────────────────────────────────────────────────────────
function preset(
	instrument: Instrument,
	overrides: Record<number, number>,
	extra: { routes?: Route[]; xy?: { x: number; y: number } } = {}
): PresetState {
	const params = { ...baseParams(instrument), ...overrides };
	return makeState(instrument, params, extra.routes ?? [], extra.xy ?? { x: 0, y: 0 });
}

export const FACTORY: Record<Instrument, NamedPreset[]> = {
	phaseplan: [
		{ name: 'Init', state: preset('phaseplan', {}) },
		{
			name: 'Soft Pad',
			state: preset(
				'phaseplan',
				{ 4: 0.5, 5: 0.4, 7: 1200, 11: 0.3, 12: 0.6, 15: 1.6, 30: 0.4, 31: 0.6 },
				{ routes: [{ source: 1, target: 1, depth: 0.35 }] }
			)
		},
		{
			name: 'Pluck Bass',
			state: preset('phaseplan', {
				2: -12, 5: 0.7, 7: 850, 8: 0.25, 11: 0.8, 13: 0.18, 14: 0.2, 15: 0.16
			})
		},
		{
			name: 'Bright Lead',
			state: preset('phaseplan', { 0: 0.66, 3: 14, 7: 6500, 8: 0.3, 24: 0.04, 25: 0.3 })
		},
		{
			name: 'Wobble',
			state: preset(
				'phaseplan',
				{ 7: 700, 8: 0.45, 20: 4 },
				{ routes: [{ source: 0, target: 1, depth: 0.8 }] }
			)
		}
	],
	botanica: [
		{ name: 'Init', state: preset('botanica', {}) },
		{
			name: 'Glass Drift',
			state: preset('botanica', { 11: 0.4, 17: 0.4, 18: 0.5 }, { xy: { x: 0.6, y: 0.3 } })
		},
		{
			name: 'Pollen',
			state: preset('botanica', { 13: 0.6, 14: 0.6 }, { xy: { x: -0.5, y: 0.5 } })
		},
		{
			name: 'Frozen Bloom',
			state: preset('botanica', { 6: 1, 2: 0.8, 15: 0.6 }, { xy: { x: 0.2, y: -0.4 } })
		},
		{
			name: 'Choir',
			state: preset('botanica', { 17: 0.7, 18: 0.6, 1: 0.6 }, { xy: { x: 0.1, y: 0.6 } })
		}
	]
};
