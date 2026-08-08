use nih_plug::prelude::*;
use std::sync::Arc;
use nih_plug_egui::EguiState;
mod editor;

// Delay architecture: a fixed base delay keeps the effect in chorus territory
// (the sweep never collapses to zero / flange range), and the Depth knob sets
// how much modulation rides on top of it. Buffer sizing derives from the same
// constants so the knobs and the buffer can never disagree.
const BASE_DELAY_SECONDS: f32 = 0.006; // 6 ms: Juno chorus zone, keeps some comb color
const MIN_MOD_DEPTH_SECONDS: f32 = 0.0005; // 0.5 ms
const MAX_MOD_DEPTH_SECONDS: f32 = 0.006; // 6 ms
const MIN_CUTOFF_HZ: f32 = 1000.0;
const MAX_CUTOFF_HZ: f32 = 7000.0;
const HP_CUTOFF_HZ: f32 = 160.0; // fixed mud filter on the wet; doesn't breathe
const DEFAULT_RATE_HZ: f32 = 1.60;
const DEFAULT_DEPTH_SECONDS: f32 = 0.0041; // 4.1 ms — stored in seconds, not ms
const DEFAULT_FEEDBACK: f32 = 0.44;
const DEFAULT_MIX: f32 = 0.40;
const INTERP_MARGIN: usize = 2; // slack past the interpolator's furthest read

#[derive(Enum, Debug, PartialEq, Clone, Copy)]
enum WaveType {
    #[name = "Sine"]
    Sine,
    #[name = "Triangle"]
    Triangle,
    #[name = "Square"]
    Square,
    #[name = "Sawtooth"]
    Sawtooth,
}

#[derive(Params)]
struct ChorusParams {
    #[persist = "editor-state"]
    editor_state: Arc<EguiState>,

    #[id = "rate"]
    pub rate: FloatParam,

    #[id = "depth"]
    pub depth: FloatParam,

    #[id = "mix"]
    pub mix: FloatParam,

    #[id = "feedback"]
    pub feedback: FloatParam,

    #[id = "wave_type"]
    pub wave_type: EnumParam<WaveType>,
}

impl Default for ChorusParams {
    fn default() -> Self {
        
        let rate_default = DEFAULT_RATE_HZ;
        let rate_min = 0.01;
        let rate_max = 3.0; // vocal-friendly cap; faster reads as seasick warble

        let depth_default = DEFAULT_DEPTH_SECONDS;
        let depth_min = MIN_MOD_DEPTH_SECONDS;
        let depth_max = MAX_MOD_DEPTH_SECONDS;

        let mix_default = DEFAULT_MIX;
        let mix_min = 0.0;
        let mix_max = 1.0;

        let feedback_default = DEFAULT_FEEDBACK;
        let feedback_min = 0.0;
        let feedback_max = 0.9;

        Self {
            editor_state: editor::default_state(),

            rate: FloatParam::new("Rate", rate_default, FloatRange::Linear { min: rate_min, max: rate_max })
                .with_unit(" Hz")
                .with_value_to_string(Arc::new(|val| format!("{:.2}", val)))
                .with_smoother(SmoothingStyle::Linear(40.0)),

            depth: FloatParam::new("Depth", depth_default, FloatRange::Linear { min: depth_min, max: depth_max })
                .with_unit(" ms")
                .with_value_to_string(Arc::new(|val| format!("{:.1}", val * 1000.0)))
                .with_smoother(SmoothingStyle::Linear(50.0)),

            mix: FloatParam::new("Mix", mix_default, FloatRange::Linear { min: mix_min, max: mix_max })
                .with_unit("%")
                .with_value_to_string(Arc::new(|val| format!("{:.0}", val * 100.0)))
                .with_smoother(SmoothingStyle::Linear(50.0)),

            feedback: FloatParam::new("Feedback", feedback_default, FloatRange::Linear { min: feedback_min, max: feedback_max })
                .with_unit("%")
                .with_value_to_string(Arc::new(|val| format!("{:.0}", val * 100.0)))
                .with_smoother(SmoothingStyle::Linear(50.0)),

            wave_type: EnumParam::new("Wave", WaveType::Sine),
        }
    }
}

// Per-channel processing state. Each channel owns an independent delay line and
// filter state so the stereo image stays intact.
#[derive(Clone)]
struct ChorusVoice {
    delay_buffer: Vec<f32>,
    write_pos: usize,
    lp_state: f32,
    hp_state: f32,
    fb_state: f32,
}

impl ChorusVoice {
    fn new(max_delay_samples: usize) -> Self {
        Self {
            delay_buffer: vec![0.0; max_delay_samples],
            write_pos: 0,
            lp_state: 0.0,
            hp_state: 0.0,
            fb_state: 0.0,
        }
    }

    fn reset(&mut self) {
        self.write_pos = 0;
        self.lp_state = 0.0;
        self.hp_state = 0.0;
        self.fb_state = 0.0;
        self.delay_buffer.fill(0.0);
    }

    /// Process one sample for this channel; returns the wet (pre-mix) signal.
    #[inline]
    fn process_sample(&mut self, input: f32, mod_delay: f32, smooth_amount: f32, hp_amount: f32, feedback: f32) -> f32 {
        let len = self.delay_buffer.len();

        // Write before read. With the base delay the read taps always land at
        // least BASE_DELAY behind the write head, but writing first stays the
        // safe convention (a zero-delay read would still be well-defined if
        // the architecture ever changes).
        self.delay_buffer[self.write_pos] = input + feedback * self.fb_state;

        let read_pos = (self.write_pos as f32 - mod_delay + len as f32) % len as f32;
        let index_a = (read_pos.floor() as usize) % len;

        // Cubic Lagrange interpolation on 4 taps. 
        // Coefficient denominators (-6, 2, -2, 6) come from the fixed tap positions at -1, 0, 1, 2
        let frac = read_pos - index_a as f32;

        let a = frac + 1.0;
        let b = frac;
        let c = frac - 1.0;
        let d = frac - 2.0;

        let c0 = (b*c*d)/-6.0;
        let c1 = (a*c*d)/2.0;
        let c2 = (a*b*d)/-2.0;
        let c3 = (a*b*c)/6.0;

        let x0 = self.delay_buffer[(index_a + len - 1) % len];
        let x1 = self.delay_buffer[index_a];
        let x2 = self.delay_buffer[(index_a + len + 1) % len];
        let x3 = self.delay_buffer[(index_a + len + 2) % len];

        let delayed = c0*x0 + c1*x1 + c2*x2 + c3*x3;

        // One-pole low-pass on the wet. Coefficient is computed by the caller
        // (it needs the sample rate); this is just the filter.
        self.lp_state += smooth_amount * (delayed - self.lp_state);

        // Flush denormals so the IIR state can't stall the CPU during silence.
        if self.lp_state.abs() < 1e-25 {
            self.lp_state = 0.0;
        }

        self.hp_state += hp_amount * (self.lp_state - self.hp_state);
        
        if self.hp_state.abs() < 1e-25 {
            self.hp_state = 0.0;
        }

        self.write_pos = if self.write_pos + 1 >= len { 0 } else { self.write_pos + 1 };

        let final_state = self.lp_state - self.hp_state;

        self.fb_state = final_state;

        final_state
    }
}

struct Chorus {
    voices: Vec<ChorusVoice>,
    lfo_phase: f32,
    sample_rate: f32,
    params: Arc<ChorusParams>,
}

impl Default for Chorus {
    fn default() -> Self {
        let default_sr = 48_000.0_f32;
        let max_delay_samples = ((BASE_DELAY_SECONDS + MAX_MOD_DEPTH_SECONDS) * default_sr).ceil() as usize + INTERP_MARGIN;
        Self {
            voices: vec![ChorusVoice::new(max_delay_samples); 2],
            lfo_phase: 0.0,
            sample_rate: default_sr,
            params: Arc::new(ChorusParams::default()),
        }
    }
}

// Band-limited LFO shaper (up to the 7th harmonic) — smooth, which is all an
// LFO needs. The scale factors normalize each truncated series so every shape
// peaks at ~±1 (measured peaks of the partial sums: tri 1.17, sq 0.93,
// saw 1.53). Without this, switching waveforms silently changed the effective
// modulation depth.
#[inline]
fn lfo_value(phase: f32, wave: WaveType) -> f32 {
    match wave {
        WaveType::Sine => phase.sin(),
        WaveType::Triangle => {
            let s1 = phase.sin();
            let s3 = (3.0 * phase).sin() / 9.0;
            let s5 = (5.0 * phase).sin() / 25.0;
            let s7 = (7.0 * phase).sin() / 49.0;
            (s1 - s3 + s5 - s7) * 0.85
        }
        WaveType::Square => {
            let s1 = phase.sin();
            let s3 = (3.0 * phase).sin() / 3.0;
            let s5 = (5.0 * phase).sin() / 5.0;
            let s7 = (7.0 * phase).sin() / 7.0;
            (s1 + s3 + s5 + s7) * 1.07
        }
        WaveType::Sawtooth => {
            let s1 = phase.sin();
            let s2 = (2.0 * phase).sin() / 2.0;
            let s3 = (3.0 * phase).sin() / 3.0;
            let s4 = (4.0 * phase).sin() / 4.0;
            (s1 - s2 + s3 - s4) * 0.65
        }
    }
}

impl Plugin for Chorus {
    const NAME: &'static str = "Crimson";
    const VENDOR: &'static str = "Pyfessional";
    const URL: &'static str = "https://pyfessional.tech";
    const EMAIL: &'static str = "contact@pyfessional.tech";
    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    // Stereo first — nih-plug uses the first entry as the default when the host
    // can't or doesn't pick. Mono is declared so hosts with explicit mono
    // channel types can instantiate directly, instead of trying to do the work on its own

    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[
        AudioIOLayout {
            main_input_channels: NonZeroU32::new(2),
            main_output_channels: NonZeroU32::new(2),
            aux_input_ports: &[],
            aux_output_ports: &[],
            ..AudioIOLayout::const_default()
        },
        AudioIOLayout {
            main_input_channels: NonZeroU32::new(1),
            main_output_channels: NonZeroU32::new(1),
            aux_input_ports: &[],
            aux_output_ports: &[],
            ..AudioIOLayout::const_default()
        },
    ];

    type SysExMessage = ();
    type BackgroundTask = ();

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn editor(&mut self, _async_executor: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        editor::create(self.params.clone(), self.params.editor_state.clone())
    }

    fn initialize(
        &mut self,
        audio_io_layout: &AudioIOLayout,
        buffer_config: &BufferConfig,
        _context: &mut impl InitContext<Self>,
    ) -> bool {
        self.sample_rate = buffer_config.sample_rate;

        let max_delay_samples =
            ((BASE_DELAY_SECONDS + MAX_MOD_DEPTH_SECONDS) * self.sample_rate).ceil() as usize + INTERP_MARGIN;
        let channels = audio_io_layout
            .main_output_channels
            .map(|c| c.get() as usize)
            .unwrap_or(2);

        self.voices = (0..channels)
            .map(|_| ChorusVoice::new(max_delay_samples))
            .collect();

        true
    }

    fn reset(&mut self) {
        self.lfo_phase = 0.0;
        for voice in &mut self.voices {
            voice.reset();
        }
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        _context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {

        let hp_amount = 1.0 - (-std::f32::consts::TAU * HP_CUTOFF_HZ / self.sample_rate).exp();

        for channel_samples in buffer.iter_samples() {
            // Read smoothed params once per frame so the smoother advances at the
            // real sample rate regardless of channel count.
            let rate = self.params.rate.smoothed.next();
            let depth = self.params.depth.smoothed.next();
            let mix = self.params.mix.smoothed.next();
            let feedback = self.params.feedback.smoothed.next();
            let wave = self.params.wave_type.value();

            let lfo_increment = rate * std::f32::consts::TAU / self.sample_rate;

            for (ch, sample) in channel_samples.into_iter().enumerate() {
                if ch >= self.voices.len() {
                    break;
                }

                // 90-degree phase offset per channel gives mono-safe stereo width.
                let phase = self.lfo_phase + ch as f32 * std::f32::consts::FRAC_PI_2;
                let lfo = lfo_value(phase, wave);

                let unipolar = 0.5 * (lfo + 1.0);

                // Delay rides base..base+depth: a true chorus, never through-zero.
                let mod_delay = (BASE_DELAY_SECONDS + depth * unipolar) * self.sample_rate;

                // Cutoff sweeps geometrically (perceptually even), brightest at
                // the longest delay — deliberately the opposite of vintage BBD.
                // For the vintage direction: powf(1.0 - unipolar).
                // Deriving the coefficient from Hz + sample_rate keeps the
                // voicing identical across host sample rates.
                let cutoff_hz = MIN_CUTOFF_HZ * (MAX_CUTOFF_HZ / MIN_CUTOFF_HZ).powf(unipolar);
                let smooth_amount = 1.0 - (-std::f32::consts::TAU * cutoff_hz / self.sample_rate).exp();

                let dry = *sample;
                let wet = self.voices[ch].process_sample(dry, mod_delay, smooth_amount, hp_amount, feedback);

                // No clamp: resonant boost from feedback is intended character; Mix controls level.
                *sample = mix * wet + (1.0 - mix) * dry;
            }

            // LFO advances once per frame, after all channels.
            self.lfo_phase = (self.lfo_phase + lfo_increment) % std::f32::consts::TAU;
        }

        ProcessStatus::Normal
    }
}

impl ClapPlugin for Chorus {
    const CLAP_ID: &'static str = "tech.pyfessional.crimson";
    const CLAP_DESCRIPTION: Option<&'static str> =
        Some("Lightweight vocal chorus with a warm metallic shimmer");
    const CLAP_MANUAL_URL: Option<&'static str> = None;
    const CLAP_SUPPORT_URL: Option<&'static str> = None;
    const CLAP_FEATURES: &'static [ClapFeature] = &[
        ClapFeature::AudioEffect,
        ClapFeature::Stereo,
        ClapFeature::Mono,
        ClapFeature::Custom("chorus"),
    ];
}

impl Vst3Plugin for Chorus {
    const VST3_CLASS_ID: [u8; 16] = *b"PyfessCrimsonV01";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] =
        &[Vst3SubCategory::Fx, Vst3SubCategory::Modulation];
}

nih_export_clap!(Chorus);
nih_export_vst3!(Chorus);