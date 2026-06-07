//! `dsp-wasm` — a thin C-ABI bridge to the instrument engines.
//!
//! The web studio runs the DSP inside an `AudioWorkletProcessor`. Rather than
//! wasm-bindgen (whose JS glue is awkward in a worklet realm), this crate
//! compiles to a plain `cdylib` exposing `extern "C"` functions over wasm linear
//! memory. The worklet instantiates the module directly with
//! `WebAssembly.instantiate(module, imports)` and calls the exports — the exact
//! same `botanica`/`phaseplan` code the native plugin runs.
//!
//! Ownership: `*_new` returns an opaque heap pointer (a `Box`); the caller must
//! pass it back to `*_free`. Audio buffers live in wasm memory: the host calls
//! [`rs_alloc`] once for a reusable block, fills/reads it around `*_process`,
//! and [`rs_free`]s it on teardown. Parameters are addressed by stable `u32`
//! ids (see `PARAM_IDS.md` / the `params` TS mirror).
//!
//! This is the one crate that needs `unsafe` (raw pointers + `#[no_mangle]`);
//! every actual signal-processing line still lives in the safe DSP crates.

use botanica::{BotanicaParams, Engine as BotanicaEngine};
use dsp_core::{ModRoute, SvfMode, Waveform};
use phaseplan::{Engine as PhasePlanEngine, PhasePlanParams, VoiceParams};

// ── raw audio-buffer allocation in wasm linear memory ────────────────────────

/// Allocate `len` f32s in wasm memory and return the pointer (host-owned).
#[no_mangle]
pub extern "C" fn rs_alloc(len: usize) -> *mut f32 {
    let mut v = vec![0.0f32; len.max(1)];
    let ptr = v.as_mut_ptr();
    core::mem::forget(v); // ownership passes to the host until rs_free
    ptr
}

/// Free a buffer previously returned by [`rs_alloc`] (same `len`).
///
/// # Safety
/// `ptr`/`len` must come from a prior `rs_alloc` and not be used afterwards.
#[no_mangle]
pub unsafe extern "C" fn rs_free(ptr: *mut f32, len: usize) {
    if !ptr.is_null() {
        drop(Vec::from_raw_parts(ptr, len.max(1), len.max(1)));
    }
}

#[inline]
fn wave_from_f32(v: f32) -> Waveform {
    match v as i32 {
        1 => Waveform::Saw,
        2 => Waveform::Square,
        3 => Waveform::Triangle,
        _ => Waveform::Sine,
    }
}

#[inline]
fn svf_from_f32(v: f32) -> SvfMode {
    match v as i32 {
        1 => SvfMode::Highpass,
        2 => SvfMode::Bandpass,
        3 => SvfMode::Notch,
        _ => SvfMode::Lowpass,
    }
}

// ═════════════════════════════════════════════════════════════════════════════
// Botanica
// ═════════════════════════════════════════════════════════════════════════════

/// Opaque Botanica instance (the host only ever holds a pointer to it).
pub struct BotanicaHandle {
    engine: BotanicaEngine,
    params: BotanicaParams,
}

#[no_mangle]
pub extern "C" fn botanica_new(sample_rate: f32) -> *mut BotanicaHandle {
    let params = BotanicaParams::default();
    let mut engine = BotanicaEngine::new(sample_rate);
    engine.set_params(params);
    Box::into_raw(Box::new(BotanicaHandle { engine, params }))
}

/// # Safety
/// `p` must be a live pointer from [`botanica_new`].
#[no_mangle]
pub unsafe extern "C" fn botanica_free(p: *mut BotanicaHandle) {
    if !p.is_null() {
        drop(Box::from_raw(p));
    }
}

/// Set parameter `id` to `val`. Unknown ids are ignored.
///
/// # Safety
/// `p` must be a live pointer from [`botanica_new`].
#[no_mangle]
pub unsafe extern "C" fn botanica_set_param(p: *mut BotanicaHandle, id: u32, val: f32) {
    let h = match p.as_mut() {
        Some(h) => h,
        None => return,
    };
    let q = &mut h.params;
    match id {
        0 => q.blend = val,
        1 => q.intensity = val,
        2 => q.bloom = val,
        3 => q.motion = val,
        4 => q.xy_x = val,
        5 => q.xy_y = val,
        6 => q.freeze = val,
        7 => q.retune_semis = val,
        8 => q.character_q = val,
        9 => q.filter_lfo_rate = val,
        10 => q.xy_macro_mix = val,
        11 => q.resonance = val,
        12 => q.resonance_tilt = val,
        13 => q.arp_amount = val,
        14 => q.arp_density = val,
        15 => q.freeze_size = val,
        16 => q.freeze_spray = val,
        _ => return,
    }
    h.engine.set_params(h.params);
}

/// Load a mono sample (copied from wasm memory) into Botanica's loop voice.
///
/// # Safety
/// `p` must be live; `ptr`/`len` must describe a readable f32 buffer.
#[no_mangle]
pub unsafe extern "C" fn botanica_set_sample(
    p: *mut BotanicaHandle,
    ptr: *const f32,
    len: usize,
    src_rate: f32,
) {
    let h = match p.as_mut() {
        Some(h) => h,
        None => return,
    };
    if ptr.is_null() || len == 0 {
        h.engine.clear_sample();
        return;
    }
    let data = core::slice::from_raw_parts(ptr, len).to_vec();
    h.engine.set_sample(data, src_rate);
}

/// Render `len` mono samples into `out` (in wasm memory).
///
/// # Safety
/// `p` must be live; `out`/`len` must describe a writable f32 buffer.
#[no_mangle]
pub unsafe extern "C" fn botanica_process(p: *mut BotanicaHandle, out: *mut f32, len: usize) {
    let h = match p.as_mut() {
        Some(h) => h,
        None => return,
    };
    if out.is_null() {
        return;
    }
    let buf = core::slice::from_raw_parts_mut(out, len);
    h.engine.process(buf);
}

// ═════════════════════════════════════════════════════════════════════════════
// PhasePlan
// ═════════════════════════════════════════════════════════════════════════════

/// Opaque PhasePlan instance (the host only ever holds a pointer to it).
pub struct PhasePlanHandle {
    engine: PhasePlanEngine,
    params: PhasePlanParams,
    routes: Vec<ModRoute>,
}

#[no_mangle]
pub extern "C" fn phaseplan_new(sample_rate: f32) -> *mut PhasePlanHandle {
    let params = PhasePlanParams::default();
    let mut engine = PhasePlanEngine::new(sample_rate);
    engine.set_params(params);
    Box::into_raw(Box::new(PhasePlanHandle {
        engine,
        params,
        routes: Vec::new(),
    }))
}

/// # Safety
/// `p` must be a live pointer from [`phaseplan_new`].
#[no_mangle]
pub unsafe extern "C" fn phaseplan_free(p: *mut PhasePlanHandle) {
    if !p.is_null() {
        drop(Box::from_raw(p));
    }
}

/// Set parameter `id` to `val`. Unknown ids are ignored.
///
/// # Safety
/// `p` must be a live pointer from [`phaseplan_new`].
#[no_mangle]
pub unsafe extern "C" fn phaseplan_set_param(p: *mut PhasePlanHandle, id: u32, val: f32) {
    let h = match p.as_mut() {
        Some(h) => h,
        None => return,
    };
    let q = &mut h.params;
    let v = &mut q.voice;
    match id {
        // oscillator section
        0 => v.osc.wt_pos_a = val,
        1 => v.osc.wt_pos_b = val,
        2 => v.osc.coarse_b = val,
        3 => v.osc.detune_cents = val,
        4 => v.osc.osc_mix = val,
        5 => v.osc.sub_level = val,
        6 => v.osc.noise_level = val,
        // filter
        7 => v.cutoff = val,
        8 => v.resonance = val,
        9 => v.filter_mode = svf_from_f32(val),
        10 => v.filter_kbd = val,
        11 => v.filter_env = val,
        // amp env
        12 => v.amp_a = val,
        13 => v.amp_d = val,
        14 => v.amp_s = val,
        15 => v.amp_r = val,
        // mod env
        16 => v.mod_a = val,
        17 => v.mod_d = val,
        18 => v.mod_s = val,
        19 => v.mod_r = val,
        // LFOs + glide
        20 => v.lfo1_rate = val,
        21 => v.lfo1_wave = wave_from_f32(val),
        22 => v.lfo2_rate = val,
        23 => v.lfo2_wave = wave_from_f32(val),
        24 => v.glide = val,
        // FX rack + master
        25 => q.drive = val,
        26 => q.chorus = val,
        27 => q.delay_time = val,
        28 => q.delay_feedback = val,
        29 => q.delay_mix = val,
        30 => q.reverb = val,
        31 => q.reverb_size = val,
        32 => q.master = val,
        _ => return,
    }
    h.engine.set_params(h.params);
}

/// # Safety
/// `p` must be a live pointer from [`phaseplan_new`].
#[no_mangle]
pub unsafe extern "C" fn phaseplan_note_on(p: *mut PhasePlanHandle, note: f32, vel: f32) {
    if let Some(h) = p.as_mut() {
        h.engine.note_on(note, vel);
    }
}

/// # Safety
/// `p` must be a live pointer from [`phaseplan_new`].
#[no_mangle]
pub unsafe extern "C" fn phaseplan_note_off(p: *mut PhasePlanHandle, note: f32) {
    if let Some(h) = p.as_mut() {
        h.engine.note_off(note);
    }
}

/// # Safety
/// `p` must be a live pointer from [`phaseplan_new`].
#[no_mangle]
pub unsafe extern "C" fn phaseplan_all_notes_off(p: *mut PhasePlanHandle) {
    if let Some(h) = p.as_mut() {
        h.engine.all_notes_off();
    }
}

/// # Safety
/// `p` must be a live pointer from [`phaseplan_new`].
#[no_mangle]
pub unsafe extern "C" fn phaseplan_active_voices(p: *mut PhasePlanHandle) -> u32 {
    p.as_ref().map_or(0, |h| h.engine.active_voices() as u32)
}

/// Clear the staged route list (call before re-adding).
///
/// # Safety
/// `p` must be a live pointer from [`phaseplan_new`].
#[no_mangle]
pub unsafe extern "C" fn phaseplan_clear_routes(p: *mut PhasePlanHandle) {
    if let Some(h) = p.as_mut() {
        h.routes.clear();
    }
}

/// Stage one modulation route (source/target indices, bipolar depth).
///
/// # Safety
/// `p` must be a live pointer from [`phaseplan_new`].
#[no_mangle]
pub unsafe extern "C" fn phaseplan_add_route(
    p: *mut PhasePlanHandle,
    source: u32,
    target: u32,
    depth: f32,
) {
    if let Some(h) = p.as_mut() {
        h.routes.push(ModRoute {
            source: source as usize,
            target: target as usize,
            depth,
        });
    }
}

/// Commit the staged routes to every voice (off the audio thread).
///
/// # Safety
/// `p` must be a live pointer from [`phaseplan_new`].
#[no_mangle]
pub unsafe extern "C" fn phaseplan_commit_routes(p: *mut PhasePlanHandle) {
    if let Some(h) = p.as_mut() {
        h.engine.set_routes(&h.routes);
    }
}

/// Render `len` mono samples into `out` (in wasm memory).
///
/// # Safety
/// `p` must be live; `out`/`len` must describe a writable f32 buffer.
#[no_mangle]
pub unsafe extern "C" fn phaseplan_process(p: *mut PhasePlanHandle, out: *mut f32, len: usize) {
    let h = match p.as_mut() {
        Some(h) => h,
        None => return,
    };
    if out.is_null() {
        return;
    }
    let buf = core::slice::from_raw_parts_mut(out, len);
    h.engine.process(buf);
}

// keep the VoiceParams import meaningful for downstream readers / future setters
#[allow(dead_code)]
fn _voice_params_marker(_v: VoiceParams) {}

#[cfg(test)]
mod tests {
    use super::*;

    fn rms(b: &[f32]) -> f32 {
        (b.iter().map(|x| x * x).sum::<f32>() / b.len() as f32).sqrt()
    }

    #[test]
    fn buffer_alloc_roundtrip() {
        let n = 256;
        let ptr = rs_alloc(n);
        assert!(!ptr.is_null());
        unsafe {
            let s = core::slice::from_raw_parts_mut(ptr, n);
            assert!(s.iter().all(|&x| x == 0.0));
            s[0] = 1.0;
            rs_free(ptr, n);
        }
    }

    #[test]
    fn phaseplan_abi_renders_audio() {
        let n = 512;
        let out = rs_alloc(n);
        unsafe {
            let h = phaseplan_new(48_000.0);
            phaseplan_set_param(h, 32, 0.4); // master
            phaseplan_set_param(h, 25, 0.2); // drive
            phaseplan_note_on(h, 57.0, 1.0);
            assert_eq!(phaseplan_active_voices(h), 1);
            // render a few blocks
            let mut peak = 0.0f32;
            let mut energy = 0.0f32;
            for _ in 0..32 {
                phaseplan_process(h, out, n);
                let s = core::slice::from_raw_parts(out, n);
                assert!(s.iter().all(|v| v.is_finite()));
                peak = peak.max(s.iter().fold(0.0, |a, &v| a.max(v.abs())));
                energy += rms(s);
            }
            assert!(peak <= 1.0001, "abi output clipped: {peak}");
            assert!(energy > 0.0, "abi produced silence");
            phaseplan_note_off(h, 57.0);
            phaseplan_free(h);
            rs_free(out, n);
        }
    }

    #[test]
    fn phaseplan_routes_abi() {
        unsafe {
            let h = phaseplan_new(48_000.0);
            phaseplan_clear_routes(h);
            phaseplan_add_route(h, 0, 1, 2.0); // LFO1 → cutoff
            phaseplan_add_route(h, 1, 0, 0.5); // LFO2 → pitch
            phaseplan_commit_routes(h); // must not panic
            phaseplan_note_on(h, 60.0, 1.0);
            let n = 256;
            let out = rs_alloc(n);
            phaseplan_process(h, out, n);
            assert!(core::slice::from_raw_parts(out, n)
                .iter()
                .all(|v| v.is_finite()));
            rs_free(out, n);
            phaseplan_free(h);
        }
    }

    #[test]
    fn botanica_abi_renders_audio() {
        let n = 512;
        let out = rs_alloc(n);
        unsafe {
            let h = botanica_new(48_000.0);
            botanica_set_param(h, 1, 0.6); // intensity
            botanica_set_param(h, 4, 0.5); // xy_x
            let mut energy = 0.0f32;
            for _ in 0..16 {
                botanica_process(h, out, n);
                let s = core::slice::from_raw_parts(out, n);
                assert!(s.iter().all(|v| v.is_finite() && v.abs() <= 4.0));
                energy += rms(s);
            }
            assert!(energy > 0.0, "botanica abi silent");
            botanica_free(h);
            rs_free(out, n);
        }
    }
}
