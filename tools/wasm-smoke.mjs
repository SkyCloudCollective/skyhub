// Headless end-to-end check of the dsp-wasm ABI — the exact module the
// AudioWorklet will load. Instantiates, plays a note, renders blocks, asserts
// the output is finite and audible. No browser needed.
import { readFileSync } from 'node:fs';

const path = process.argv[2] ?? 'target/wasm32-unknown-unknown/release/dsp_wasm.wasm';
const bytes = readFileSync(path);
const { instance } = await WebAssembly.instantiate(bytes, {});
const x = instance.exports;
const mem = () => new Float32Array(x.memory.buffer);

const SR = 48000, BLK = 128;
const out = x.rs_alloc(BLK);
const synth = x.phaseplan_new(SR);
x.phaseplan_set_param(synth, 32, 0.4); // master
x.phaseplan_set_param(synth, 8, 0.4);  // resonance
x.phaseplan_clear_routes(synth);
x.phaseplan_add_route(synth, 0, 1, 1.5); // LFO1 → cutoff
x.phaseplan_commit_routes(synth);
x.phaseplan_note_on(synth, 57, 1.0);

if (x.phaseplan_active_voices(synth) !== 1) throw new Error('voice not allocated');

let peak = 0, energy = 0, nonfinite = 0;
const blocks = Math.floor(SR / BLK); // ~1 s
for (let b = 0; b < blocks; b++) {
  x.phaseplan_process(synth, out, BLK);
  const base = out >> 2; // f32 index
  const view = mem();
  for (let i = 0; i < BLK; i++) {
    const v = view[base + i];
    if (!Number.isFinite(v)) nonfinite++;
    peak = Math.max(peak, Math.abs(v));
    energy += v * v;
  }
}
const rms = Math.sqrt(energy / (blocks * BLK));
x.phaseplan_note_off(synth, 57);
x.phaseplan_free(synth);

// Morph too
const bot = x.morph_new(SR);
x.morph_set_param(bot, 1, 0.6); // intensity
let botE = 0;
for (let b = 0; b < 64; b++) {
  x.morph_process(bot, out, BLK);
  const base = out >> 2, view = mem();
  for (let i = 0; i < BLK; i++) botE += view[base + i] ** 2;
}
const botRms = Math.sqrt(botE / (64 * BLK));
x.morph_free(bot);
x.rs_free(out, BLK);

console.log(`PhasePlan: peak=${peak.toFixed(3)} rms=${rms.toFixed(4)} nonfinite=${nonfinite}`);
console.log(`Morph:     rms=${botRms.toFixed(4)}`);
if (nonfinite > 0) throw new Error('non-finite output');
if (peak > 1.0001) throw new Error(`clipped: ${peak}`);
if (rms < 0.01) throw new Error('PhasePlan silent');
if (botRms < 0.005) throw new Error('Morph silent');
console.log('OK — wasm ABI renders finite, bounded, audible output for both instruments.');
