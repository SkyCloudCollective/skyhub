//! PhasePlan — a modular subtractive + wavetable synth with an open modulation
//! matrix.
//!
//! Inspired by Phase Plant; original implementation. Built entirely on
//! `dsp-core`, so the web (wasm) build and the native plugin run the same DSP.
//! v1's PhasePlan was a thin, aliasing, un-modulated stub; v2 rebuilds it as a
//! real instrument: anti-aliased dual wavetable oscillators, a TPT state-variable
//! filter, two ADSRs and two LFOs feeding an open modulation matrix, polyphony,
//! and a shared FX rack — each landing in its own commit.
//!
//! Real-time contract (see `dsp-core` crate docs): no alloc/lock/IO/panic in any
//! `process`/`next`; every user parameter that reaches the audio thread is
//! smoothed; modulation routes are configured off the audio thread.

#![forbid(unsafe_code)]

pub mod osc_section;
pub mod voice;

pub use osc_section::{OscParams, OscSection};
pub use voice::{Voice, VoiceParams};
