//! Botanica — CLAP/VST3 generative instrument.
//!
//! A thin `nih-plug` wrapper around the `botanica` crate (the same DSP the web
//! studio runs through wasm). Botanica is generative: it drones from a built-in
//! tone (or a loaded sample) and is steered by the XY character puck + macros, so
//! the plugin is an always-on generator with host-automatable parameters — no
//! MIDI input. Sound design by **Tev**; original implementation.

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
    #[id = "gain"]
    gain: FloatParam,
}

impl Default for PluginParams {
    fn default() -> Self {
        let unit = |name: &str, def: f32| {
            FloatParam::new(name, def, FloatRange::Linear { min: 0.0, max: 1.0 })
        };
        let bipolar = |name: &str| {
            FloatParam::new(name, 0.0, FloatRange::Linear { min: -1.0, max: 1.0 })
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
            ..BotanicaParams::default()
        }
    }
}

impl Plugin for BotanicaPlugin {
    const NAME: &'static str = "Botanica";
    const VENDOR: &'static str = "the SKY collective";
    const URL: &'static str = "https://ranchsamples.dev";
    const EMAIL: &'static str = "noreply@ranchsamples.dev";
    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[AudioIOLayout {
        main_input_channels: None,
        main_output_channels: NonZeroU32::new(2),
        ..AudioIOLayout::const_default()
    }];

    const MIDI_INPUT: MidiConfig = MidiConfig::None;
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
        _context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        self.engine.set_params(self.engine_params());
        let gain = util::db_to_gain(self.params.gain.value());
        for channel_samples in buffer.iter_samples() {
            let s = self.engine.next() * gain;
            for sample in channel_samples {
                *sample = s;
            }
        }
        ProcessStatus::KeepAlive
    }
}

impl ClapPlugin for BotanicaPlugin {
    const CLAP_ID: &'static str = "dev.ranchsamples.botanica";
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
