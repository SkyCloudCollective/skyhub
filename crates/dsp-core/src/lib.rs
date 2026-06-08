//! `dsp-core` — real-time-safe DSP primitives.
//!
//! This crate is the ONE source of truth for signal processing. Instrument
//! crates (`morph`, `phaseplan`) build on it; the wasm bindings and the
//! `nih-plug` wrappers are thin and call into the same code, so the web studio
//! and the native plugin run byte-identical DSP.
//!
//! # Real-time contract (enforced by review + the golden-render tests)
//! Anything reachable from an audio callback MUST NOT:
//!
//! - allocate, lock, perform I/O, or panic;
//! - read a parameter without smoothing (no zipper noise / clicks).
//!
//! All state is sized once at construction. `#![forbid(unsafe_code)]` keeps the
//! hot path memory-safe.

#![forbid(unsafe_code)]

pub mod env;
pub mod filter;
pub mod fx;
pub mod grain;
pub mod lfo;
pub mod modmatrix;
pub mod osc;
pub mod smooth;
pub mod util;
pub mod wavetable;

pub use env::{Adsr, EnvStage};
pub use filter::{Svf, SvfMode};
pub use fx::{Chorus, Delay, DelayLine, Drive, Reverb};
pub use grain::{GrainCloud, MAX_GRAINS};
pub use lfo::Lfo;
pub use modmatrix::{ModMatrix, ModRoute};
pub use osc::{poly_blep, Oscillator, Waveform};
pub use smooth::Smoothed;
pub use util::{clampf, db_to_gain, flush_denormal, TWO_PI};
pub use wavetable::Wavetable;
