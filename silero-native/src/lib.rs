//! In-process Silero TTS v5 engine running on ONNX Runtime.
//!
//! Replaces the Python `ttsd` sidecar for Silero synthesis: the model is a
//! pre-exported ONNX bundle (see `export/`) loaded from a local directory,
//! and the text frontend (accentor, homograph solver) is a Rust port of the
//! upstream package code. See `docs/architecture.md` for the pipeline map.

pub mod bundle;
pub mod chunking;
pub mod engine;
pub mod error;
pub mod frontend;
pub mod timestamps;

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::Path;
use std::sync::{Mutex, MutexGuard};

use tracing::{debug, instrument};

pub use engine::Engine;
pub use error::{EngineError, Result};
pub use timestamps::WordTimestamp;

/// Lock an engine mutex, recovering from poisoning.
///
/// Poisoning here means a previous inference panicked while holding the
/// lock — which the public API already converts into a typed error via
/// `catch_unwind`. ONNX Runtime sessions carry no Rust-side state between
/// `run` calls, so reusing a session after a panic is safe; returning the
/// inner guard keeps the engine usable for subsequent calls (spec:
/// "engine panic is contained").
pub(crate) fn lock_session<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
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

/// One synthesized (sub-)chunk: its text, char offset in the full input and
/// the raw engine output.
struct ChunkOutput {
    text: String,
    offset: usize,
    output: engine::EngineOutput,
}

/// The exported `tts_main` has the decoder's positional table baked in at a
/// fixed size (5000 frames; the torch reference grows it dynamically, so
/// ttsd never hit this). A chunk whose predicted duration exceeds the table
/// fails inside ONNX Runtime with a broadcast error on the `pos_encoder`
/// Add node — the signal to re-split the chunk and retry.
fn is_positional_overflow(e: &EngineError) -> bool {
    match e {
        EngineError::Ort(err) => {
            let msg = err.to_string();
            msg.contains("pos_encoder") || msg.contains("Attempting to broadcast")
        }
        _ => false,
    }
}

impl SileroNative {
    /// Load and verify the model bundle, open the always-needed ONNX
    /// sessions (PQMF opens lazily on first use).
    pub fn load(bundle_dir: impl AsRef<Path>) -> Result<Self> {
        Ok(Self {
            engine: Engine::load(bundle_dir.as_ref())?,
        })
    }

    /// Synthesize one chunk, re-splitting it in half when the exported
    /// decoder's positional table overflows. Sub-chunks are appended to
    /// `outputs` in order, with char offsets into the full input text.
    ///
    /// The inference core runs under `catch_unwind`: an ONNX Runtime panic
    /// becomes a typed `Synthesis` error instead of crossing the engine
    /// boundary. `AssertUnwindSafe` is sound here because all mutable state
    /// lives behind mutexes that [`lock_session`] recovers from poisoning,
    /// and ort sessions hold no cross-run Rust state.
    fn synthesize_with_fallback(
        &self,
        text: &str,
        offset: usize,
        speaker: &str,
        sample_rate: u32,
        outputs: &mut Vec<ChunkOutput>,
    ) -> Result<()> {
        let result = catch_unwind(AssertUnwindSafe(|| {
            self.engine.synthesize(text, speaker, sample_rate)
        }))
        .map_err(|payload| {
            let msg = payload
                .downcast_ref::<&str>()
                .map(|s| (*s).to_string())
                .or_else(|| payload.downcast_ref::<String>().cloned())
                .unwrap_or_else(|| "unknown panic".to_string());
            EngineError::Synthesis(format!("inference panicked: {msg}"))
        });

        match result {
            Ok(Ok(output)) => {
                outputs.push(ChunkOutput {
                    text: text.to_string(),
                    offset,
                    output,
                });
                Ok(())
            }
            Ok(Err(e)) if is_positional_overflow(&e) && text.chars().count() > 1 => {
                // ~7 frames/char of Russian speech against the 5000-frame
                // table means a 900-char ttsd-sized chunk can be too long;
                // halve until it fits.
                let half = (text.chars().count() / 2).max(1);
                debug!(
                    chars = text.chars().count(),
                    "chunk overflows the decoder positional table; re-splitting"
                );
                for (sub, sub_offset) in chunking::split_with_limit(text, half) {
                    self.synthesize_with_fallback(
                        &sub,
                        offset + sub_offset,
                        speaker,
                        sample_rate,
                        outputs,
                    )?;
                }
                Ok(())
            }
            Ok(Err(e)) => Err(e),
            Err(e) => Err(e),
        }
    }

    /// Synthesize text at `sample_rate` (8000/24000/48000; the engine
    /// default used by callers is 24000).
    ///
    /// Text longer than [`chunking::MAX_CHUNK_SIZE`] chars is split into
    /// sentence-boundary chunks (ttsd `split_into_chunks` parity), each chunk
    /// is synthesized separately, and the audio is concatenated; word
    /// timestamps are shifted by the accumulated chunk durations. A chunk
    /// that still overflows the exported decoder's fixed positional table
    /// is re-split in half until it fits.
    #[instrument(skip_all, fields(speaker, sample_rate, text_len = text.len()))]
    pub fn synthesize(
        &self,
        text: &str,
        speaker: &str,
        sample_rate: u32,
    ) -> Result<SynthesisResult> {
        // The single strip point for the whole crate: markup is removed
        // before chunking so chunk offsets and word timestamps are in
        // stripped-text coordinates. `Engine::prepare` does NOT strip again.
        let stripped = frontend::text::strip_unsupported_markup(text);
        let chunks = chunking::split_into_chunks(&stripped);

        let mut outputs: Vec<ChunkOutput> = Vec::new();
        for (chunk_text, chunk_start) in &chunks {
            self.synthesize_with_fallback(
                chunk_text,
                *chunk_start,
                speaker,
                sample_rate,
                &mut outputs,
            )?;
        }

        let mut samples: Vec<f32> = Vec::new();
        let mut timings: Vec<(&str, usize, f32)> = Vec::with_capacity(outputs.len());
        for co in &outputs {
            // The engine always returns audio at the requested rate — take
            // it from the argument, not from "the last chunk wins".
            debug_assert_eq!(co.output.sample_rate, sample_rate);
            timings.push((co.text.as_str(), co.offset, co.output.duration_sec));
            samples.extend_from_slice(&co.output.samples);
        }

        let duration_sec = samples.len() as f32 / sample_rate as f32;
        let wav = encode_wav(&samples, sample_rate)?;
        let timestamps = timestamps::estimate_timestamps_chunked(&timings);
        Ok(SynthesisResult {
            wav,
            timestamps,
            duration_sec,
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
