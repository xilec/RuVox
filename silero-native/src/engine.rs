//! Synthesis engine: text frontend → tts_main → istft → (pqmf for 24k/8k).
//!
//! The audio path mirrors the upstream v5 pipeline: everything is synthesized
//! at 48 kHz (`tts_main` → `istft`), then the PQMF analysis filterbank
//! downsamples to 24 kHz or 8 kHz when requested. Durations and pitch are
//! neutral (`durs_rate` = `pitch_coefs` = 1); the zero-duration clamp lives
//! inside the exported `tts_main` graph (`repeat_interleave(dur + 0.5)`,
//! truncated), exactly as upstream — the engine does not duplicate it.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Instant;

use ort::session::Session;
use ort::value::Tensor;
use tracing::{debug, info, instrument};

use crate::bundle::{Manifest, Sessions};
use crate::error::{EngineError, Result};
use crate::frontend::FrontendConfig;
use crate::frontend::accentor::Accentor;
use crate::frontend::homosolver::HomoSolver;
use crate::frontend::text::{build_sequence, prepare_text_input};
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
    tts_main: Mutex<Session>,
    istft: Mutex<Session>,
    /// Rate-specific PQMF downsamplers, lazy-opened on first synthesis at
    /// that rate: they are only needed for 24k/8k output, so an engine that
    /// only ever serves 48 kHz (or is never used at all) does not pay for
    /// them at load. The paths were verified by `Manifest::verify` at load.
    pqmf_24k: Mutex<Option<Session>>,
    pqmf_24k_path: PathBuf,
    pqmf_8k: Mutex<Option<Session>>,
    pqmf_8k_path: PathBuf,
}

impl Engine {
    /// Verify the bundle, open the always-needed sessions (the rate-specific
    /// PQMF filters are lazy-opened on first use), load the frontend.
    #[instrument(skip_all, fields(dir = %bundle_dir.display()))]
    pub fn load(bundle_dir: &Path) -> Result<Self> {
        let total = Instant::now();
        let manifest = Manifest::load(bundle_dir)?;
        let t = Instant::now();
        manifest.verify(bundle_dir)?;
        info!(
            elapsed_ms = t.elapsed().as_secs_f64() * 1e3,
            "bundle verified"
        );
        let sessions = Sessions::open(bundle_dir, &manifest)?;
        let t = Instant::now();
        let config = FrontendConfig::load(bundle_dir)?;
        let homosolver = HomoSolver::load(bundle_dir, &config.homosolver, sessions.homosolver)?;
        let accentor = Accentor::load(bundle_dir, &config.accentor, sessions.accentor_tensor)?;
        info!(
            elapsed_ms = t.elapsed().as_secs_f64() * 1e3,
            "frontend loaded"
        );
        let pqmf_24k_path = manifest.file_path(bundle_dir, crate::bundle::PQMF_24K)?;
        let pqmf_8k_path = manifest.file_path(bundle_dir, crate::bundle::PQMF_8K)?;
        // The PQMF sessions open lazily, but a bundle that lacks the files
        // must still fail at load — pre-lazy behavior surfaced a missing
        // model here, not mid-synthesis.
        for path in [&pqmf_24k_path, &pqmf_8k_path] {
            if !path
                .try_exists()
                .map_err(|e| EngineError::Bundle(format!("cannot stat {}: {e}", path.display())))?
            {
                return Err(EngineError::Bundle(format!(
                    "missing PQMF model {}",
                    path.display()
                )));
            }
        }
        info!(
            model = %manifest.model_id,
            elapsed_ms = total.elapsed().as_secs_f64() * 1e3,
            "engine loaded"
        );
        Ok(Self {
            symbols_tail: config.symbols_tail(),
            config,
            homosolver,
            accentor,
            tts_main: Mutex::new(sessions.tts_main),
            istft: Mutex::new(sessions.istft),
            pqmf_24k: Mutex::new(None),
            pqmf_24k_path,
            pqmf_8k: Mutex::new(None),
            pqmf_8k_path,
        })
    }

    /// Run the text frontend: sanitize newlines → normalize → homosolver →
    /// accentor → symbol ids.
    ///
    /// The input must already be free of `[[...]]` / SSML markup:
    /// [`crate::frontend::text::strip_unsupported_markup`] runs exactly once, in
    /// `SileroNative::synthesize`, before chunking (chunk offsets and word
    /// timestamps are in stripped-text coordinates, so a second strip here
    /// would be both redundant and contractually confusing). Direct
    /// `Engine` callers with markup-bearing text must strip first.
    fn prepare(&self, text: &str) -> Result<(Vec<i64>, String)> {
        // ttsd parity (`sanitize_for_silero`): the pipeline keeps `\n\n` in
        // the normalized text, and the symbol filter below would drop `\n`
        // silently, gluing the surrounding words into one.
        let sanitized = crate::chunking::sanitize_for_silero(text);
        let prepared = prepare_text_input(&sanitized, &self.symbols_tail);
        if !prepared.has_text {
            return Err(EngineError::BadInput(
                "text has no speakable content after normalization".to_string(),
            ));
        }
        let homosolved = self.homosolver.resolve(&prepared.sentence)?;
        let accented = self.accentor.accentuate(&homosolved)?;
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
    ///
    /// The input must already be free of `[[...]]` / SSML markup: the strip
    /// runs exactly once, in
    /// [`SileroNative::synthesize`](crate::SileroNative::synthesize), before
    /// chunking. Direct `Engine` callers with markup-bearing text must apply
    /// [`crate::frontend::text::strip_unsupported_markup`] first, or word
    /// timestamps and spoken text degrade.
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
                let (slot, path) = if r == 24000 {
                    (&self.pqmf_24k, &self.pqmf_24k_path)
                } else {
                    (&self.pqmf_8k, &self.pqmf_8k_path)
                };
                // Lazy open: first synthesis at this rate pays the (~2 ms)
                // session creation here instead of every engine load paying
                // it up front.
                let mut slot = lock_session(slot);
                if slot.is_none() {
                    let t = Instant::now();
                    *slot = Some(crate::bundle::open_session(path)?);
                    info!(
                        model = %path.file_name().unwrap_or_default().to_string_lossy(),
                        elapsed_ms = t.elapsed().as_secs_f64() * 1e3,
                        "lazy PQMF session opened"
                    );
                }
                let session = slot.as_mut().expect("just initialized");
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
