//! In-process Silero TTS v5 engine running on ONNX Runtime.
//!
//! Replaces the Python `ttsd` sidecar for Silero synthesis: the model is a
//! pre-exported ONNX bundle (see `export/`) loaded from a local directory,
//! and the text frontend (accentor, homograph solver) is a Rust port of the
//! upstream package code. See `docs/architecture.md` for the pipeline map.

pub mod bundle;
pub mod engine;
pub mod error;
pub mod frontend;
pub mod timestamps;

use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::Path;
use std::sync::{Mutex, MutexGuard};

use ort::session::Session;
use tracing::instrument;

pub use engine::Engine;
pub use error::{EngineError, Result};
pub use timestamps::WordTimestamp;

/// Lock an engine session mutex, recovering from poisoning.
///
/// Poisoning here means a previous inference panicked while holding the
/// lock — which the public API already converts into a typed error via
/// `catch_unwind`. ONNX Runtime sessions carry no Rust-side state between
/// `run` calls, so reusing a session after a panic is safe; returning the
/// inner guard keeps the engine usable for subsequent calls (spec:
/// "engine panic is contained").
pub(crate) fn lock_session(m: &Mutex<Session>) -> MutexGuard<'_, Session> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

/// Result of [`SileroNative::synthesize`], mirroring the ttsd
/// `OkSynthesize` contract so callers are engine-agnostic.
pub struct SynthesisResult {
    /// 16-bit PCM mono WAV at the requested sample rate.
    pub wav: Vec<u8>,
    pub timestamps: Vec<WordTimestamp>,
    pub duration_sec: f32,
}

impl std::fmt::Debug for SynthesisResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SynthesisResult")
            .field("wav_len", &self.wav.len())
            .field("timestamps", &self.timestamps)
            .field("duration_sec", &self.duration_sec)
            .finish()
    }
}

/// Public engine handle: load once, synthesize many.
pub struct SileroNative {
    engine: Engine,
}

impl SileroNative {
    /// Load and verify the model bundle, open all ONNX sessions.
    pub fn load(bundle_dir: impl AsRef<Path>) -> Result<Self> {
        Ok(Self {
            engine: Engine::load(bundle_dir.as_ref())?,
        })
    }

    /// Synthesize one chunk of text at `sample_rate` (8000/24000/48000;
    /// the engine default used by callers is 24000).
    ///
    /// The inference core runs under `catch_unwind`: an ONNX Runtime panic
    /// becomes a typed `Synthesis` error instead of crossing the engine
    /// boundary. `AssertUnwindSafe` is sound here because all mutable state
    /// lives behind mutexes that [`lock_session`] recovers from poisoning,
    /// and ort sessions hold no cross-run Rust state.
    #[instrument(skip_all, fields(speaker, sample_rate, text_len = text.len()))]
    pub fn synthesize(&self, text: &str, speaker: &str, sample_rate: u32) -> Result<SynthesisResult> {
        let output = catch_unwind(AssertUnwindSafe(|| {
            self.engine.synthesize(text, speaker, sample_rate)
        }))
        .map_err(|payload| {
            let msg = payload
                .downcast_ref::<&str>()
                .map(|s| (*s).to_string())
                .or_else(|| payload.downcast_ref::<String>().cloned())
                .unwrap_or_else(|| "unknown panic".to_string());
            EngineError::Synthesis(format!("inference panicked: {msg}"))
        })??;
        let wav = encode_wav(&output.samples, output.sample_rate)?;
        let stripped = frontend::text::strip_unsupported_markup(text);
        let timestamps = timestamps::estimate_timestamps(&stripped, output.duration_sec);
        Ok(SynthesisResult {
            wav,
            timestamps,
            duration_sec: output.duration_sec,
        })
    }
}

/// Encode f32 samples as a 16-bit PCM mono WAV.
///
/// Upstream `save_wav` truncates (`(audio * 32767).astype(int16)`); we round
/// instead — at most 1 LSB difference, far below the parity threshold.
fn encode_wav(samples: &[f32], sample_rate: u32) -> Result<Vec<u8>> {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut cursor = std::io::Cursor::new(Vec::new());
    {
        let mut writer = hound::WavWriter::new(&mut cursor, spec)
            .map_err(|e| EngineError::Internal(format!("wav writer init: {e}")))?;
        for &s in samples {
            let v = (s.clamp(-1.0, 1.0) * 32767.0).round() as i16;
            writer
                .write_sample(v)
                .map_err(|e| EngineError::Internal(format!("wav write: {e}")))?;
        }
        writer
            .finalize()
            .map_err(|e| EngineError::Internal(format!("wav finalize: {e}")))?;
    }
    Ok(cursor.into_inner())
}
