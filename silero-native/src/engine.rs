//! Synthesis engine: text frontend → tts_main → istft → (pqmf for 24k/8k).
//!
//! The audio path mirrors the upstream v5 pipeline: everything is synthesized
//! at 48 kHz (`tts_main` → `istft`), then the PQMF analysis filterbank
//! downsamples to 24 kHz or 8 kHz when requested. Durations and pitch are
//! neutral (`durs_rate` = `pitch_coefs` = 1); the zero-duration clamp lives
//! inside the exported `tts_main` graph (`repeat_interleave(dur + 0.5)`,
//! truncated), exactly as upstream — the engine does not duplicate it.

use std::collections::HashSet;
use std::path::Path;
use std::sync::Mutex;

use ort::session::Session;
use ort::value::Tensor;
use tracing::{debug, info, instrument};

use crate::bundle::{Manifest, Sessions};
use crate::error::{EngineError, Result};
use crate::frontend::accentor::Accentor;
use crate::frontend::homosolver::HomoSolver;
use crate::frontend::text::{build_sequence, prepare_text_input, strip_unsupported_markup};
use crate::frontend::FrontendConfig;
use crate::lock_session;

/// Raw synthesis output (pre-WAV).
pub struct EngineOutput {
    /// f32 samples in [-1, 1] at `sample_rate`.
    pub samples: Vec<f32>,
    pub sample_rate: u32,
    pub duration_sec: f32,
    /// Text after the full frontend (what was actually spoken).
    pub spoken_text: String,
}

/// The in-process Silero v5 engine.
pub struct Engine {
    config: FrontendConfig,
    homosolver: HomoSolver,
    accentor: Accentor,
    /// Keep-set for text filtering, precomputed from `config.symbols` at
    /// load so the per-chunk `prepare` does not rebuild it every call.
    symbols_tail: HashSet<char>,
    /// Shared empty skip-sets for the accentor call (we never skip words),
    /// precomputed for the same reason.
    empty_skip: HashSet<String>,
    tts_main: Mutex<Session>,
    istft: Mutex<Session>,
    pqmf_24k: Mutex<Session>,
    pqmf_8k: Mutex<Session>,
}

impl Engine {
    /// Verify the bundle, open all sessions, load the frontend.
    #[instrument(skip_all, fields(dir = %bundle_dir.display()))]
    pub fn load(bundle_dir: &Path) -> Result<Self> {
        let manifest = Manifest::load(bundle_dir)?;
        manifest.verify(bundle_dir)?;
        let sessions = Sessions::open(bundle_dir, &manifest)?;
        let config = FrontendConfig::load(bundle_dir)?;
        let homosolver = HomoSolver::load(bundle_dir, &config.homosolver, sessions.homosolver)?;
        let accentor = Accentor::load(bundle_dir, &config.accentor, sessions.accentor_tensor)?;
        info!(model = %manifest.model_id, "engine loaded");
        Ok(Self {
            symbols_tail: config.symbols_tail(),
            empty_skip: HashSet::new(),
            config,
            homosolver,
            accentor,
            tts_main: Mutex::new(sessions.tts_main),
            istft: Mutex::new(sessions.istft),
            pqmf_24k: Mutex::new(sessions.pqmf_24k),
            pqmf_8k: Mutex::new(sessions.pqmf_8k),
        })
    }

    /// Run the text frontend: strip markup → sanitize newlines → normalize →
    /// homosolver → accentor → symbol ids.
    fn prepare(&self, text: &str) -> Result<(Vec<i64>, String)> {
        let stripped = strip_unsupported_markup(text);
        // ttsd parity (`sanitize_for_silero`): the pipeline keeps `\n\n` in
        // the normalized text, and the symbol filter below would drop `\n`
        // silently, gluing the surrounding words into one.
        let sanitized = crate::chunking::sanitize_for_silero(&stripped);
        let prepared = prepare_text_input(&sanitized, &self.symbols_tail);
        if !prepared.has_text {
            return Err(EngineError::BadInput(
                "text has no speakable content after normalization".to_string(),
            ));
        }
        let homosolved = self
            .homosolver
            .resolve(&prepared.sentence, true, true, true)?;
        let accented = self.accentor.accentuate(
            &homosolved,
            true,
            true,
            true,
            &self.empty_skip,
            &self.empty_skip,
        )?;
        let sequence = build_sequence(
            &accented,
            &self.config.symbol_to_id,
            &self.config.sos_token,
            &self.config.eos_token,
        )?;
        Ok((sequence, accented))
    }

    /// Extract a (1, …, N) f32 output tensor as a flat owned vec.
    fn take_f32(
        outputs: &ort::session::SessionOutputs,
        name: &str,
    ) -> Result<(Vec<i64>, Vec<f32>)> {
        let (shape, data) = outputs[name]
            .try_extract_tensor::<f32>()
            .map_err(|e| EngineError::Synthesis(format!("cannot read output {name}: {e}")))?;
        Ok((shape.to_vec(), data.to_vec()))
    }

    /// Synthesize one chunk of text. Validation errors (`BadInput`) are
    /// returned before any ONNX session runs.
    #[instrument(skip_all, fields(speaker, sample_rate, text_len = text.len()))]
    pub fn synthesize(&self, text: &str, speaker: &str, sample_rate: u32) -> Result<EngineOutput> {
        let speaker_id = self.config.speaker_id(speaker).ok_or_else(|| {
            EngineError::BadInput(format!(
                "unknown speaker {speaker:?}, expected one of: {}",
                self.config.speakers.join(", ")
            ))
        })?;
        if !self.config.sample_rates.contains(&sample_rate) {
            return Err(EngineError::BadInput(format!(
                "unsupported sample rate {sample_rate}, expected one of: {:?}",
                self.config.sample_rates
            )));
        }
        if text.trim().is_empty() {
            return Err(EngineError::BadInput("empty text".to_string()));
        }
        let (sequence, spoken_text) = self.prepare(text)?;

        // tts_main: sequence + speaker + neutral dur/pitch → mag/x/y.
        let len = sequence.len();
        let seq_t = Tensor::<i64>::from_array((vec![1usize, len], sequence))?;
        let spk_t = Tensor::<i64>::from_array((vec![1usize], vec![speaker_id]))?;
        let durs_t = Tensor::<f32>::from_array((vec![1usize, len], vec![1.0f32; len]))?;
        let pitch_t = Tensor::<f32>::from_array((vec![1usize, len], vec![1.0f32; len]))?;
        let (mag_shape, mag, x, y) = {
            let mut session = lock_session(&self.tts_main);
            let outputs = session.run(
                ort::inputs!["sequence" => seq_t, "speaker_ids" => spk_t, "durs_rate" => durs_t, "pitch_coefs" => pitch_t],
            )?;
            let (mag_shape, mag) = Self::take_f32(&outputs, "mag")?;
            let (_, x) = Self::take_f32(&outputs, "x")?;
            let (_, y) = Self::take_f32(&outputs, "y")?;
            (mag_shape, mag, x, y)
        };
        debug!(mel_shape = ?mag_shape, "tts_main done");

        // istft: mag/x/y → 48 kHz waveform.
        let mag_t = Tensor::<f32>::from_array((mag_shape.clone(), mag))?;
        let x_t = Tensor::<f32>::from_array((mag_shape.clone(), x))?;
        let y_t = Tensor::<f32>::from_array((mag_shape, y))?;
        let audio_48k = {
            let mut session = lock_session(&self.istft);
            let outputs = session.run(ort::inputs!["mag" => mag_t, "x" => x_t, "y" => y_t])?;
            let (_, audio) = Self::take_f32(&outputs, "audio")?;
            audio
        };
        debug!(samples_48k = audio_48k.len(), "istft done");

        // PQMF downsample for 24k/8k; 48k passes through.
        let (samples, out_rate) = match sample_rate {
            r if r == self.config.native_sample_rate => (audio_48k, r),
            r => {
                let n = audio_48k.len();
                let audio_t = Tensor::<f32>::from_array((vec![1usize, 1, n], audio_48k))?;
                let pqmf = if r == 24000 {
                    &self.pqmf_24k
                } else {
                    &self.pqmf_8k
                };
                let mut session = lock_session(pqmf);
                let outputs = session.run(ort::inputs!["audio" => audio_t])?;
                let (_, band) = Self::take_f32(&outputs, "band0")?;
                debug!(samples = band.len(), rate = r, "pqmf done");
                (band, r)
            }
        };

        Ok(EngineOutput {
            duration_sec: samples.len() as f32 / out_rate as f32,
            samples,
            sample_rate: out_rate,
            spoken_text,
        })
    }
}
