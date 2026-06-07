// Control specs for the studio UI — mirrors the wasm param ids in
// docs/dsp-wasm-abi.md. Keep the ids in sync with crates/dsp-wasm/src/lib.rs.

export type Knob = {
	kind: 'knob';
	id: number;
	label: string;
	min: number;
	max: number;
	def: number;
	unit?: string;
	curve?: 'lin' | 'exp';
};
export type Switch = {
	kind: 'switch';
	id: number;
	label: string;
	options: string[];
	def: number;
};
export type Ctl = Knob | Switch;
export type Group = { title: string; ctls: Ctl[] };

const k = (
	id: number,
	label: string,
	min: number,
	max: number,
	def: number,
	unit = '',
	curve: 'lin' | 'exp' = 'lin'
): Knob => ({ kind: 'knob', id, label, min, max, def, unit, curve });

const sw = (id: number, label: string, options: string[], def: number): Switch => ({
	kind: 'switch',
	id,
	label,
	options,
	def
});

const WAVES = ['Sine', 'Saw', 'Square', 'Tri'];

export const PHASEPLAN_GROUPS: Group[] = [
	{
		title: 'Oscillators',
		ctls: [
			k(0, 'Morph A', 0, 1, 0.5),
			k(1, 'Morph B', 0, 1, 0.66),
			k(4, 'A · B', 0, 1, 0.4),
			k(3, 'Detune', 0, 50, 8, '¢'),
			k(2, 'Coarse B', -24, 24, 0, 'st'),
			k(5, 'Sub', 0, 1, 0.3),
			k(6, 'Noise', 0, 1, 0)
		]
	},
	{
		title: 'Filter',
		ctls: [
			k(7, 'Cutoff', 20, 16000, 2200, 'Hz', 'exp'),
			k(8, 'Reso', 0, 1, 0.18),
			sw(9, 'Mode', ['LP', 'HP', 'BP', 'Notch'], 0),
			k(11, 'Env', -1, 1, 0.5),
			k(10, 'Key', 0, 1, 0.3)
		]
	},
	{
		title: 'Amp envelope',
		ctls: [
			k(12, 'Attack', 0.001, 4, 0.005, 's', 'exp'),
			k(13, 'Decay', 0.002, 4, 0.15, 's', 'exp'),
			k(14, 'Sustain', 0, 1, 0.7),
			k(15, 'Release', 0.002, 6, 0.25, 's', 'exp')
		]
	},
	{
		title: 'Mod envelope',
		ctls: [
			k(16, 'Attack', 0.001, 4, 0.002, 's', 'exp'),
			k(17, 'Decay', 0.002, 4, 0.2, 's', 'exp'),
			k(18, 'Sustain', 0, 1, 0),
			k(19, 'Release', 0.002, 6, 0.2, 's', 'exp')
		]
	},
	{
		title: 'LFOs',
		ctls: [
			k(20, 'LFO 1', 0.01, 20, 4, 'Hz', 'exp'),
			sw(21, 'Wave 1', WAVES, 0),
			k(22, 'LFO 2', 0.01, 20, 0.6, 'Hz', 'exp'),
			sw(23, 'Wave 2', WAVES, 3),
			k(24, 'Glide', 0, 1, 0, 's')
		]
	},
	{
		title: 'FX rack',
		ctls: [
			k(25, 'Drive', 0, 1, 0),
			k(26, 'Chorus', 0, 1, 0),
			k(29, 'Delay', 0, 1, 0),
			k(28, 'Feedback', 0, 0.95, 0.3),
			k(30, 'Reverb', 0, 1, 0.12),
			k(31, 'Size', 0, 1, 0.5),
			k(32, 'Master', 0, 1, 0.32)
		]
	}
];

export const BOTANICA_GROUPS: Group[] = [
	{
		title: 'Character',
		ctls: [
			k(1, 'Intensity', 0, 1, 0.55),
			k(2, 'Bloom', 0, 1, 0.65),
			k(3, 'Motion', 0, 1, 0.55),
			k(0, 'Blend', 0, 1, 0.4),
			k(10, 'Orb mix', 0, 1, 0.45)
		]
	},
	{
		title: 'Resonance',
		ctls: [k(11, 'Amount', 0, 1, 0.25), k(12, 'Tilt', -1, 1, 0), k(8, 'Q', 0, 1, 0.3)]
	},
	{
		title: 'Transient arp',
		ctls: [k(13, 'Amount', 0, 1, 0.3), k(14, 'Density', 0, 1, 0.4)]
	},
	{
		title: 'Granular freeze',
		ctls: [sw(6, 'Freeze', ['Off', 'On'], 0), k(15, 'Size', 0, 1, 0.5), k(16, 'Spray', 0, 1, 0.3)]
	},
	{
		title: 'Tone',
		ctls: [k(7, 'Retune', -12, 12, 0, 'st'), k(9, 'Filter LFO', 0, 1, 0.12)]
	},
	{
		title: 'String bed',
		ctls: [
			k(17, 'Level', 0, 1, 0),
			k(18, 'Air', 0, 1, 0.4),
			k(19, 'Density', 0, 1, 0.4),
			k(20, 'Tone', 0, 1, 0.5)
		]
	}
];

// Modulation-matrix vocabulary (PhasePlan) — source/target indices match
// crates/phaseplan/src/voice.rs.
export const MOD_SOURCES = ['LFO 1', 'LFO 2', 'Mod env', 'Amp env', 'Velocity', 'Key'];
export const MOD_TARGETS = ['Pitch', 'Cutoff', 'Morph A', 'Morph B', 'Amp', 'Resonance', 'A·B mix'];

/** Default knob/switch values for an instrument, keyed by param id. */
export function defaults(groups: Group[]): Map<number, number> {
	const m = new Map<number, number>();
	for (const g of groups) for (const c of g.ctls) m.set(c.id, c.def);
	return m;
}
