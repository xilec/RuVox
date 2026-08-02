//! Synthesis engine: text frontend → tts_main → istft → (pqmf for 24k/8k).
//!
//! The audio path mirrors the upstream v5 pipeline: everything is synthesized
//! at 48 kHz (`tts_main` → `istft`), then the PQMF analysis filterbank
//! downsamples to 24 kHz or 8 kHz when requested. Durations and pitch are
//! neutral (`durs_rate` = `pitch_coefs` = 1); the zero-duration clamp lives
//! inside the exported `tts_main` graph (`repeat_interleave(dur + 0.5)`,
//! truncated), exactly as upstream — the engine does not duplicate it.

use std::collections::HashSet;
use std::ops::AddAssign;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use ort::session::Session;
use ort::value::Tensor;
use tracing::{debug, info, instrument};

use crate::bundle::{Manifest, Sessions};
use crate::error::{EngineError, Result};
use crate::frontend::FrontendConfig;
use crate::frontend::accentor::Accentor;
use crate::frontend::homosolver::HomoSolver;
use crate::frontend::text::{BuiltSequence, build_sequence, prepare_text_input};
use crate::lock_session;
use crate::timestamps::SymbolDuration;

/// Per-stage wall times of one synthesis, collected with `Instant` around
/// each pipeline stage (a handful of clock reads per call — negligible
/// against a ~35 ms synthesis). Drives the per-stage breakdown printed by
/// `examples/bench.rs` and recorded in `docs/benchmarks.md` (issue #164).
///
/// `wav_encode` / `concat_timestamps` live outside [`Engine`] (in
/// `SileroNative::synthesize`), so they stay zero here.
#[derive(Debug, Clone, Copy, Default)]
pub struct StageTimings {
    /// `prepare_text_input` (normalize + symbol filter).
    pub frontend_text: Duration,
    pub homosolver: Duration,
    pub accentor: Duration,
    pub build_sequence: Duration,
    pub tts_main: Duration,
    pub istft: Duration,
    /// PQMF downsample (24k/8k only; zero for 48k pass-through).
    pub pqmf: Duration,
    /// 16-bit PCM WAV encode of the concatenated samples.
    pub wav_encode: Duration,
    /// Chunk concat + word-timestamp computation.
    pub concat_timestamps: Duration,
}

impl AddAssign for StageTimings {
    fn add_assign(&mut self, rhs: Self) {
        self.frontend_text += rhs.frontend_text;
        self.homosolver += rhs.homosolver;
        self.accentor += rhs.accentor;
        self.build_sequence += rhs.build_sequence;
        self.tts_main += rhs.tts_main;
        self.istft += rhs.istft;
        self.pqmf += rhs.pqmf;
        self.wav_encode += rhs.wav_encode;
        self.concat_timestamps += rhs.concat_timestamps;
    }
}

/// Raw synthesis output (pre-WAV).
pub struct EngineOutput {
    /// f32 samples in [-1, 1] at `sample_rate`.
    pub samples: Vec<f32>,
    pub sample_rate: u32,
    pub duration_sec: f32,
    /// Text after the full frontend (what was actually spoken).
    pub spoken_text: String,
    /// Exact frame count the model rendered per input symbol (sos/eos
    /// included), from the `dur_hat` output of `tts_main`. Aligned 1:1 with
    /// the built sequence; the timestamp layer's letter-level anchor.
    pub durations: Vec<SymbolDuration>,
    /// Per-stage wall times of this chunk's synthesis.
    pub stage_timings: StageTimings,
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
    fn prepare(&self, text: &str, timings: &mut StageTimings) -> Result<(BuiltSequence, String)> {
        // ttsd parity (`sanitize_for_silero`): the pipeline keeps `\n\n` in
        // the normalized text, and the symbol filter below would drop `\n`
        // silently, gluing the surrounding words into one.
        let sanitized = crate::chunking::sanitize_for_silero(text);
        let t = Instant::now();
        let prepared = prepare_text_input(&sanitized, &self.symbols_tail);
        timings.frontend_text += t.elapsed();
        if !prepared.has_text {
            return Err(EngineError::BadInput(
                "text has no speakable content after normalization".to_string(),
            ));
        }
        let t = Instant::now();
        let homosolved = self.homosolver.resolve(&prepared.sentence)?;
        timings.homosolver += t.elapsed();
        let t = Instant::now();
        let accented = self.accentor.accentuate(&homosolved)?;
        timings.accentor += t.elapsed();
        let t = Instant::now();
        let sequence = build_sequence(
            &accented,
            &self.config.symbol_to_id,
            &self.config.sos_token,
            &self.config.eos_token,
        )?;
        timings.build_sequence += t.elapsed();
        Ok((sequence, accented))
    }

    /// Extract a (1, …, N) f32 output tensor as a flat owned vec. A missing
    /// output is a typed error, not a panic: `dur_hat` is absent from bundles
    /// exported before it was wired through, and that must surface as a
    /// bundle-contract failure rather than an `Index` panic.
    fn take_f32(
        outputs: &ort::session::SessionOutputs,
        name: &str,
    ) -> Result<(Vec<i64>, Vec<f32>)> {
        let value = outputs
            .get(name)
            .ok_or_else(|| EngineError::Synthesis(format!("model output {name} missing")))?;
        let (shape, data) = value
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
        let mut timings = StageTimings::default();
        let (sequence, spoken_text) = self.prepare(text, &mut timings)?;

        // tts_main: sequence + speaker + neutral dur/pitch → mag/x/y.
        let len = sequence.ids.len();
        let seq_t = Tensor::<i64>::from_array((vec![1usize, len], sequence.ids.clone()))?;
        let spk_t = Tensor::<i64>::from_array((vec![1usize], vec![speaker_id]))?;
        let durs_t = Tensor::<f32>::from_array((vec![1usize, len], vec![1.0f32; len]))?;
        let pitch_t = Tensor::<f32>::from_array((vec![1usize, len], vec![1.0f32; len]))?;
        let t = Instant::now();
        let (mag_shape, mag, x, y, dur_hat) = {
            let mut session = lock_session(&self.tts_main);
            let outputs = session.run(
                ort::inputs!["sequence" => seq_t, "speaker_ids" => spk_t, "durs_rate" => durs_t, "pitch_coefs" => pitch_t],
            )?;
            let (mag_shape, mag) = Self::take_f32(&outputs, "mag")?;
            let (_, x) = Self::take_f32(&outputs, "x")?;
            let (_, y) = Self::take_f32(&outputs, "y")?;
            let (_, dur_hat) = Self::take_f32(&outputs, "dur_hat")?;
            (mag_shape, mag, x, y, dur_hat)
        };
        timings.tts_main += t.elapsed();
        debug!(mel_shape = ?mag_shape, "tts_main done");

        // dur_hat: per-symbol durations in 600-sample frames @48kHz. The
        // exported graph renders audio via repeat_interleave(trunc(dur + 0.5))
        // after the baked-in sos/eos clamps, so truncating here reproduces
        // the exact per-symbol frame counts of the rendered waveform.
        if dur_hat.len() != len {
            return Err(EngineError::Synthesis(format!(
                "dur_hat length {} does not match sequence length {len}",
                dur_hat.len()
            )));
        }
        let durations: Vec<SymbolDuration> = sequence
            .chars
            .iter()
            .zip(dur_hat.iter())
            .map(|(&ch, &dur)| SymbolDuration {
                ch,
                frames: (dur + 0.5).trunc().max(0.0) as u32,
            })
            .collect();

        // istft: mag/x/y → 48 kHz waveform.
        let mag_t = Tensor::<f32>::from_array((mag_shape.clone(), mag))?;
        let x_t = Tensor::<f32>::from_array((mag_shape.clone(), x))?;
        let y_t = Tensor::<f32>::from_array((mag_shape, y))?;
        let t = Instant::now();
        let audio_48k = {
            let mut session = lock_session(&self.istft);
            let outputs = session.run(ort::inputs!["mag" => mag_t, "x" => x_t, "y" => y_t])?;
            let (_, audio) = Self::take_f32(&outputs, "audio")?;
            audio
        };
        timings.istft += t.elapsed();
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
                // Timer placement matches the other ORT stages: around
                // run + output extraction, after input tensor construction.
                let t = Instant::now();
                let outputs = session.run(ort::inputs!["audio" => audio_t])?;
                let (_, band) = Self::take_f32(&outputs, "band0")?;
                timings.pqmf += t.elapsed();
                debug!(samples = band.len(), rate = r, "pqmf done");
                (band, r)
            }
        };

        Ok(EngineOutput {
            duration_sec: samples.len() as f32 / out_rate as f32,
            samples,
            sample_rate: out_rate,
            spoken_text,
            durations,
            stage_timings: timings,
        })
    }
}

#[cfg(test)]
mod stage_timings_tests {
    use super::*;

    /// Full-literal field coverage: adding a stage to `StageTimings` fails
    /// compilation here, forcing the author to also extend `AddAssign` and
    /// the bench breakdown array — otherwise the new stage would silently
    /// vanish into the bench's "(unaccounted)" row.
    #[test]
    fn add_assign_covers_every_field() {
        let a = StageTimings {
            frontend_text: Duration::from_millis(1),
            homosolver: Duration::from_millis(2),
            accentor: Duration::from_millis(3),
            build_sequence: Duration::from_millis(4),
            tts_main: Duration::from_millis(5),
            istft: Duration::from_millis(6),
            pqmf: Duration::from_millis(7),
            wav_encode: Duration::from_millis(8),
            concat_timestamps: Duration::from_millis(9),
        };
        let mut acc = a;
        acc += a;
        assert_eq!(acc.frontend_text, Duration::from_millis(2));
        assert_eq!(acc.homosolver, Duration::from_millis(4));
        assert_eq!(acc.accentor, Duration::from_millis(6));
        assert_eq!(acc.build_sequence, Duration::from_millis(8));
        assert_eq!(acc.tts_main, Duration::from_millis(10));
        assert_eq!(acc.istft, Duration::from_millis(12));
        assert_eq!(acc.pqmf, Duration::from_millis(14));
        assert_eq!(acc.wav_encode, Duration::from_millis(16));
        assert_eq!(acc.concat_timestamps, Duration::from_millis(18));
    }
}
