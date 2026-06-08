//! Botanica — CLAP/VST3 generative instrument.
//!
//! A thin `nih-plug` wrapper around the `botanica` crate (the same DSP the web
//! studio runs through wasm). Botanica drones from a built-in tone (or a loaded
//! sample) steered by the XY character puck + macros, and (A1) is playable:
//! incoming MIDI notes pitch the loop and set the root the generative layers
//! lock to (mono, last-note priority). With no note held it falls back to the
//! drone. Sound design by **Tev**; original implementation.

use botanica::{BotanicaParams, Engine};
use nih_plug::prelude::*;
use std::sync::Arc;

struct BotanicaPlugin {
    params: Arc<PluginParams>,
    engine: Engine,
}

#[derive(Params)]
struct PluginParams {
    #[id = "xyx"]
    xy_x: FloatParam,
    #[id = "xyy"]
    xy_y: FloatParam,
    #[id = "inten"]
    intensity: FloatParam,
    #[id = "bloom"]
    bloom: FloatParam,
    #[id = "motion"]
    motion: FloatParam,
    #[id = "reso"]
    resonance: FloatParam,
    #[id = "arp"]
    arp: FloatParam,
    #[id = "strngs"]
    strings: FloatParam,
    #[id = "spark"]
    spark: BoolParam,
    #[id = "gain"]
    gain: FloatParam,
}

impl Default for PluginParams {
    fn default() -> Self {
        let unit = |name: &str, def: f32| {
            FloatParam::new(name, def, FloatRange::Linear { min: 0.0, max: 1.0 })
        };
        let bipolar = |name: &str| {
            FloatParam::new(
                name,
                0.0,
                FloatRange::Linear {
                    min: -1.0,
                    max: 1.0,
                },
            )
        };
        Self {
            xy_x: bipolar("Character X"),
            xy_y: bipolar("Character Y"),
            intensity: unit("Intensity", 0.55),
            bloom: unit("Bloom", 0.65),
            motion: unit("Motion", 0.55),
            resonance: unit("Resonance", 0.25),
            arp: unit("Arp", 0.3),
            strings: unit("Strings", 0.0),
            // Spark (pluck) layer — on by default; toggling it off ramps the
            // layer to silence in the DSP (no click). Sound design by Tev.
            spark: BoolParam::new("Spark", true),
            gain: FloatParam::new(
                "Output",
                -3.0,
                FloatRange::Linear {
                    min: -40.0,
                    max: 6.0,
                },
            )
            .with_unit(" dB")
            .with_value_to_string(formatters::v2s_f32_rounded(1)),
        }
    }
}

impl Default for BotanicaPlugin {
    fn default() -> Self {
        Self {
            params: Arc::new(PluginParams::default()),
            engine: Engine::new(48_000.0),
        }
    }
}

impl BotanicaPlugin {
    fn engine_params(&self) -> BotanicaParams {
        BotanicaParams {
            xy_x: self.params.xy_x.value(),
            xy_y: self.params.xy_y.value(),
            intensity: self.params.intensity.value(),
            bloom: self.params.bloom.value(),
            motion: self.params.motion.value(),
            resonance: self.params.resonance.value(),
            arp_amount: self.params.arp.value(),
            strings_level: self.params.strings.value(),
            spark_on: if self.params.spark.value() { 1.0 } else { 0.0 },
            ..BotanicaParams::default()
        }
    }
}

impl Plugin for BotanicaPlugin {
    const NAME: &'static str = "Botanica";
    const VENDOR: &'static str = "the SkyCloudCollective";
    const URL: &'static str = "https://skyhub.dev";
    const EMAIL: &'static str = "noreply@skyhub.dev";
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
        // Botanica has no set_sample_rate; rebuild at the host rate.
        self.engine = Engine::new(buffer_config.sample_rate);
        true
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        self.engine.set_params(self.engine_params());
        let gain = util::db_to_gain(self.params.gain.value());

        let mut next_event = context.next_event();
        for (sample_id, channel_samples) in buffer.iter_samples().enumerate() {
            // sample-accurate MIDI → mono last-note pitch + key (A1)
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

            let s = self.engine.next() * gain;
            for sample in channel_samples {
                *sample = s;
            }
        }
        ProcessStatus::KeepAlive
    }
}

impl ClapPlugin for BotanicaPlugin {
    const CLAP_ID: &'static str = "dev.skyhub.botanica";
    const CLAP_DESCRIPTION: Option<&'static str> =
        Some("Granular sample-morph generative instrument (sound design by Tev)");
    const CLAP_MANUAL_URL: Option<&'static str> = Some(Self::URL);
    const CLAP_SUPPORT_URL: Option<&'static str> = None;
    const CLAP_FEATURES: &'static [ClapFeature] = &[
        ClapFeature::Instrument,
        ClapFeature::Synthesizer,
        ClapFeature::Stereo,
    ];
}

impl Vst3Plugin for BotanicaPlugin {
    const VST3_CLASS_ID: [u8; 16] = *b"SKYBotanica00001";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] =
        &[Vst3SubCategory::Instrument, Vst3SubCategory::Synth];
}

nih_export_clap!(BotanicaPlugin);
nih_export_vst3!(BotanicaPlugin);
