# dsp-wasm — C-ABI reference

The web studio runs the instruments' DSP inside an `AudioWorkletProcessor`. The
`dsp-wasm` crate compiles the **same** `morph` / `phaseplan` engines the
native plugin uses to a plain `cdylib` exposing `extern "C"` functions over wasm
linear memory — **no wasm-bindgen**. The worklet instantiates the module with
`WebAssembly.instantiate(module, {})` (the module is self-contained: no host
imports, math included) and calls the exports directly.

Build + stage: `tools/build-wasm.sh` → `apps/web/static/dsp/dsp_wasm.wasm`
(gitignored build artifact). Headless verification: `tools/wasm-smoke.mjs`.

## Memory & lifetime

- `rs_alloc(len) -> ptr` — allocate `len` f32s in wasm memory (host-owned).
- `rs_free(ptr, len)` — free it.
- `*_new(sample_rate) -> handle` — opaque instance pointer.
- `*_free(handle)` — destroy it.
- `*_process(handle, out_ptr, len)` — render `len` mono f32s into `out_ptr`.

The worklet allocates one reusable output block (128 frames) at construction,
calls `*_process` per render quantum, and copies the block to the output channel.

## PhasePlan exports

`phaseplan_new`, `phaseplan_free`, `phaseplan_set_param(h,id,val)`,
`phaseplan_note_on(h,note,vel)`, `phaseplan_note_off(h,note)`,
`phaseplan_all_notes_off(h)`, `phaseplan_active_voices(h) -> u32`,
`phaseplan_clear_routes(h)`, `phaseplan_add_route(h,source,target,depth)`,
`phaseplan_commit_routes(h)`, `phaseplan_process(h,out,len)`.

### Parameter ids

| id | param | range / notes |
|----|-------|---------------|
| 0  | osc.wt_pos_a | 0..1 morph (sine→tri→saw→square) |
| 1  | osc.wt_pos_b | 0..1 |
| 2  | osc.coarse_b | semitones |
| 3  | osc.detune_cents | cents |
| 4  | osc.osc_mix | 0=A .. 1=B |
| 5  | osc.sub_level | 0..1 |
| 6  | osc.noise_level | 0..1 |
| 7  | cutoff | Hz |
| 8  | resonance | 0..1 |
| 9  | filter_mode | 0 LP, 1 HP, 2 BP, 3 Notch |
| 10 | filter_kbd | 0..1 keyboard tracking |
| 11 | filter_env | -1..1 (octaves = amount·6·mod-env) |
| 12–15 | amp env A/D/S/R | seconds, S in 0..1 |
| 16–19 | mod env A/D/S/R | seconds, S in 0..1 |
| 20 | lfo1_rate | Hz |
| 21 | lfo1_wave | 0 Sine, 1 Saw, 2 Square, 3 Triangle |
| 22 | lfo2_rate | Hz |
| 23 | lfo2_wave | 0..3 |
| 24 | glide | seconds (portamento) |
| 25 | drive | 0..1 |
| 26 | chorus | 0..1 mix |
| 27 | delay_time | seconds |
| 28 | delay_feedback | 0..1 |
| 29 | delay_mix | 0..1 |
| 30 | reverb | 0..1 mix |
| 31 | reverb_size | 0..1 |
| 32 | master | 0..1 |

### Modulation matrix (route source/target indices)

Sources: 0 LFO1, 1 LFO2, 2 mod-env, 3 amp-env, 4 velocity, 5 key.
Targets: 0 pitch (semitones), 1 cutoff (octaves), 2 wt-pos-A, 3 wt-pos-B,
4 amp, 5 resonance, 6 osc-mix.

## Morph exports

`morph_new`, `morph_free`, `morph_set_param(h,id,val)`,
`morph_set_sample(h,ptr,len,src_rate)`, `morph_process(h,out,len)`.

### Parameter ids

| id | param | | id | param |
|----|-------|-|----|-------|
| 0 | blend | | 9 | filter_lfo_rate |
| 1 | intensity | | 10 | xy_macro_mix |
| 2 | bloom | | 11 | resonance |
| 3 | motion | | 12 | resonance_tilt |
| 4 | xy_x (-1..1) | | 13 | arp_amount |
| 5 | xy_y (-1..1) | | 14 | arp_density |
| 6 | freeze (0/1) | | 15 | freeze_size |
| 7 | retune_semis | | 16 | freeze_spray |
| 8 | character_q | | 17 | strings_level (0=off) |
|   |  | | 18 | strings_air |
|   |  | | 19 | strings_density |
|   |  | | 20 | strings_tone |

These ids are mirrored in `apps/web/src/lib/studio/params.ts`; keep both in sync.
