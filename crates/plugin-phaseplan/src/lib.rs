//! PhasePlan — CLAP/VST3 instrument.
//!
//! A thin `nih-plug` wrapper: it owns no DSP. MIDI notes drive the `phaseplan`
//! crate's polyphonic `Engine` (the exact code the web studio runs through wasm),
//! and a handful of host-automatable parameters are pushed to the engine each
//! block. Real plugin, byte-identical synthesis to the web build.

use nih_plug::prelude::*;
use phaseplan::{Engine, PhasePlanParams};
use std::sync::Arc;

struct PhasePlanPlugin {
    params: Arc<PluginParams>,
    engine: Engine,
}

#[derive(Params)]
struct PluginParams {
    #[id = "cutoff"]
    cutoff: FloatParam,
    #[id = "reso"]
    resonance: FloatParam,
    #[id = "fenv"]
    filter_env: FloatParam,
    #[id = "drive"]
    drive: FloatParam,
    #[id = "reverb"]
    reverb: FloatParam,
    #[id = "attack"]
    attack: FloatParam,
    #[id = "release"]
    release: FloatParam,
    #[id = "master"]
    master: FloatParam,
}

impl Default for PluginParams {
    fn default() -> Self {
        let secs = |name: &str, def: f32, max: f32| {
            FloatParam::new(
                name,
                def,
                FloatRange::Skewed {
                    min: 0.001,
                    max,
                    factor: FloatRange::skew_factor(-2.0),
                },
            )
            .with_unit(" s")
            .with_value_to_string(formatters::v2s_f32_rounded(3))
        };
        Self {
            cutoff: FloatParam::new(
                "Cutoff",
                2200.0,
                FloatRange::Skewed {
                    min: 20.0,
                    max: 16_000.0,
                    factor: FloatRange::skew_factor(-2.0),
                },
            )
            .with_unit(" Hz")
            .with_value_to_string(formatters::v2s_f32_hz_then_khz(0))
            .with_string_to_value(formatters::s2v_f32_hz_then_khz()),
            resonance: FloatParam::new(
                "Resonance",
                0.18,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            ),
            filter_env: FloatParam::new(
                "Filter env",
                0.5,
                FloatRange::Linear {
                    min: -1.0,
                    max: 1.0,
                },
            ),
            drive: FloatParam::new("Drive", 0.0, FloatRange::Linear { min: 0.0, max: 1.0 }),
            reverb: FloatParam::new("Reverb", 0.12, FloatRange::Linear { min: 0.0, max: 1.0 }),
            attack: secs("Attack", 0.005, 4.0),
            release: secs("Release", 0.25, 6.0),
            master: FloatParam::new("Master", 0.32, FloatRange::Linear { min: 0.0, max: 1.0 }),
        }
    }
}

impl Default for PhasePlanPlugin {
    fn default() -> Self {
        Self {
            params: Arc::new(PluginParams::default()),
            engine: Engine::new(48_000.0),
        }
    }
}

impl PhasePlanPlugin {
    /// Build the engine param snapshot from the host-automatable parameters.
    fn engine_params(&self) -> PhasePlanParams {
        let mut p = PhasePlanParams::default();
        p.voice.cutoff = self.params.cutoff.value();
        p.voice.resonance = self.params.resonance.value();
        p.voice.filter_env = self.params.filter_env.value();
        p.voice.amp_a = self.params.attack.value();
        p.voice.amp_r = self.params.release.value();
        p.drive = self.params.drive.value();
        p.reverb = self.params.reverb.value();
        p.master = self.params.master.value();
        p
    }
}

impl Plugin for PhasePlanPlugin {
    const NAME: &'static str = "PhasePlan";
    const VENDOR: &'static str = "the SKY collective";
    const URL: &'static str = "https://ranchsamples.dev";
    const EMAIL: &'static str = "noreply@ranchsamples.dev";
    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[AudioIOLayout {
        main_input_channels: None,
        main_output_channels: NonZeroU32::new(2),
        ..AudioIOLayout::const_default()
    }];

    const MIDI_INPUT: MidiConfig = MidiConfig::Basic;
    const SAMPLE_ACCURATE_AUTOMATION: bool = true;

    type SysExMessage = ();
    type BackgroundTask = ();

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn initialize(
        &mut self,
        _layout: &AudioIOLayout,
        buffer_config: &BufferConfig,
        _context: &mut impl InitContext<Self>,
    ) -> bool {
        self.engine.set_sample_rate(buffer_config.sample_rate);
        true
    }

    fn reset(&mut self) {
        self.engine.all_notes_off();
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        // push automatable params to the engine once per block (the engine smooths)
        self.engine.set_params(self.engine_params());

        let mut next_event = context.next_event();
        for (sample_id, channel_samples) in buffer.iter_samples().enumerate() {
            while let Some(event) = next_event {
                if event.timing() > sample_id as u32 {
                    break;
                }
                match event {
                    NoteEvent::NoteOn { note, velocity, .. } => {
                        self.engine.note_on(note as f32, velocity)
                    }
                    NoteEvent::NoteOff { note, .. } => self.engine.note_off(note as f32),
                    NoteEvent::Choke { note, .. } => self.engine.note_off(note as f32),
                    _ => {}
                }
                next_event = context.next_event();
            }

            let s = self.engine.render_sample();
            for sample in channel_samples {
                *sample = s;
            }
        }

        // a synth must keep processing so notes ring out after input stops
        ProcessStatus::KeepAlive
    }
}

impl ClapPlugin for PhasePlanPlugin {
    const CLAP_ID: &'static str = "dev.ranchsamples.phaseplan";
    const CLAP_DESCRIPTION: Option<&'static str> =
        Some("Dual-wavetable subtractive synth with an open modulation matrix");
    const CLAP_MANUAL_URL: Option<&'static str> = Some(Self::URL);
    const CLAP_SUPPORT_URL: Option<&'static str> = None;
    const CLAP_FEATURES: &'static [ClapFeature] = &[
        ClapFeature::Instrument,
        ClapFeature::Synthesizer,
        ClapFeature::Stereo,
    ];
}

impl Vst3Plugin for PhasePlanPlugin {
    const VST3_CLASS_ID: [u8; 16] = *b"SKYPhasePlan0001";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] =
        &[Vst3SubCategory::Instrument, Vst3SubCategory::Synth];
}

nih_export_clap!(PhasePlanPlugin);
nih_export_vst3!(PhasePlanPlugin);
