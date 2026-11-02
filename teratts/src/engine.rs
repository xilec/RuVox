//! The public engine: bundle, sessions, text frontend, chunking, synthesis and
//! timestamps behind one call.
//!
//! The application adapter owns the `TtsEngine` trait, error translation and
//! file writing; this type owns everything that is model-specific.

use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};

use tracing::{info, instrument};
use tts_contract::Chunk as ChunkPolicy;

use crate::bundle::{Bundle, Manifest, Voice};
use crate::chunk;
use crate::error::{Error, Result};
use crate::indexer::Indexer;
use crate::styles::Styles;
use crate::synth::{self, Sessions, SynthOptions};
use crate::text::{self, Accentizer, Language};
use crate::timestamps::{self, Word};

/// Duration-scale multiplier for isolated one- or two-letter Russian words.
/// The predictor treats them as a whole utterance and returns ~0.9 s; 0.3
/// brings a conjunction to a natural ~0.25 s.
const SHORT_RU_SEGMENT_SCALE: f32 = 0.3;

/// Model identity for the generation snapshot.
#[derive(Debug, Clone)]
pub struct ModelInfo {
    pub name: String,
    pub sample_rate: u32,
}

/// One complete synthesis result.
#[derive(Debug)]
pub struct Synthesis {
    /// Mono PCM16 WAV at 44 100 Hz.
    pub wav: Vec<u8>,
    /// Raw mono samples (44 100 Hz, -1.0..=1.0) before WAV encoding; used to
    /// concatenate language segments without re-parsing the WAV.
    pub samples: Vec<f32>,
    pub timestamps: Vec<Word>,
    pub duration_sec: f64,
}

/// One language segment of a mixed utterance.
#[derive(Debug, Clone, Copy)]
pub struct Segment<'a> {
    /// Segment text in the normalized utterance.
    pub text: &'a str,
    pub language: Language,
    /// Codepoint offset of the segment in the normalized utterance; added to
    /// every timestamp so the caller maps ranges against the whole text.
    pub text_start: usize,
}

/// One piece of audio produced by [`Engine::synthesize_stream`].
#[derive(Debug)]
pub struct AudioPart<'a> {
    /// Mono f32 samples at [`synth::SAMPLE_RATE`]. Silence inserted between
    /// chunks and language segments is emitted as its own part (zeros).
    pub samples: &'a [f32],
    /// Codepoint offset of the chunk's first character in the *normalized*
    /// utterance — the same coordinate space as `Word::start_char`; `None`
    /// for a pause between parts.
    pub text_start: Option<usize>,
    /// Whether the source had a paragraph break right after this chunk.
    pub paragraph_break_after: bool,
    /// Offset of the part's first sample from the start of the stream, seconds.
    pub start_sec: f64,
}

/// Totals of a streamed synthesis, known once it returns.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct StreamSummary {
    /// Emitted parts, pauses included.
    pub parts: usize,
    /// Total samples across all parts.
    pub samples: usize,
    /// Total duration, seconds.
    pub duration_sec: f64,
}

/// In-process TeraTTSv2 engine.
pub struct Engine {
    bundle: Bundle,
    indexer: Indexer,
    accentizer: Accentizer,
    sessions: Sessions,
    styles: Mutex<HashMap<String, Arc<Styles>>>,
    options: SynthOptions,
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Half the logical cores, at least one: ONNX Runtime oversubscribes badly on
/// SMT and the model is memory-bandwidth bound at full width.
pub fn default_threads() -> usize {
    std::thread::available_parallelism()
        .map(|parallelism| (parallelism.get() / 2).max(1))
        .unwrap_or(1)
}

impl Engine {
    /// Load and verify a bundle with the default thread count.
    #[instrument(skip_all, fields(dir = %dir.display()))]
    pub fn load(dir: &Path) -> Result<Self> {
        Self::load_with_threads(dir, default_threads())
    }

    /// Load and verify a bundle with an explicit intra-op thread count.
    pub fn load_with_threads(dir: &Path, threads: usize) -> Result<Self> {
        let started = std::time::Instant::now();
        let bundle = Bundle::load(dir)?;
        let indexer = Indexer::load(&bundle.file("unicode_indexer.json"))?;
        let accentizer = Accentizer::load(&bundle.dir().join("ruaccent/dictionary"))?;
        let sessions = Sessions::open(&bundle, threads.max(1))?;
        info!(
            elapsed_ms = started.elapsed().as_secs_f64() * 1e3,
            threads = threads.max(1),
            model = %bundle.manifest().model_id,
            "teratts engine loaded"
        );
        Ok(Self {
            bundle,
            indexer,
            accentizer,
            sessions,
            styles: Mutex::new(HashMap::new()),
            options: SynthOptions::default(),
        })
    }

    /// Bundle manifest (voices, model id, upstream revision).
    pub fn manifest(&self) -> &Manifest {
        self.bundle.manifest()
    }

    /// All voices shipped in the bundle.
    pub fn voices(&self) -> &[Voice] {
        &self.bundle.manifest().voices
    }

    /// Model identity for the generation snapshot.
    pub fn model_info(&self) -> ModelInfo {
        let manifest = self.bundle.manifest();
        let short = &self.bundle.manifest_sha256()[..12];
        ModelInfo {
            name: format!(
                "{} bundle v{} ({short})",
                manifest.model_id, manifest.bundle_version
            ),
            sample_rate: synth::SAMPLE_RATE,
        }
    }

    /// Override sampling options (tests and the parity fixture).
    pub fn with_options(mut self, options: SynthOptions) -> Self {
        self.options = options;
        self
    }

    /// Synthesize `text` in `voice`, honoring the profile's chunk policy.
    ///
    /// Cancellation is checked before every chunk and before the WAV is
    /// produced; a cancelled call returns [`Error::Cancelled`] and no audio.
    pub fn synthesize(
        &self,
        text: &str,
        voice: &str,
        policy: &ChunkPolicy,
        cancelled: &AtomicBool,
    ) -> Result<Synthesis> {
        self.synthesize_lang(text, voice, Language::Ru, policy, cancelled)
    }

    /// Synthesize one segment in an explicit language.
    pub fn synthesize_lang(
        &self,
        text: &str,
        voice: &str,
        language: Language,
        policy: &ChunkPolicy,
        cancelled: &AtomicBool,
    ) -> Result<Synthesis> {
        let (samples, timestamps) =
            self.synthesize_samples(text, voice, language, policy, cancelled)?;
        let wav = synth::encode_wav(&samples)?;
        Ok(Synthesis {
            wav,
            duration_sec: samples.len() as f64 / synth::SAMPLE_RATE as f64,
            samples,
            timestamps,
        })
    }

    /// Synthesize a mixed utterance segment by segment, joining them with
    /// `segment_pause_ms` of silence. Timestamps stay in one timeline and in
    /// codepoint coordinates of the whole normalized text.
    pub fn synthesize_segments(
        &self,
        segments: &[Segment<'_>],
        voice: &str,
        policy: &ChunkPolicy,
        cancelled: &AtomicBool,
        segment_pause_ms: u32,
    ) -> Result<Synthesis> {
        if segments.is_empty() {
            return Err(Error::InvalidText("empty text".into()));
        }
        let mut samples: Vec<f32> = Vec::new();
        let mut timestamps: Vec<Word> = Vec::new();
        for (index, segment) in segments.iter().enumerate() {
            if cancelled.load(Ordering::Relaxed) {
                return Err(Error::Cancelled);
            }
            let (mut part, mut words) =
                self.synthesize_samples(segment.text, voice, segment.language, policy, cancelled)?;
            let audio_offset = samples_to_secs(samples.len());
            for word in &mut words {
                word.start += audio_offset;
                word.end += audio_offset;
                word.start_char += segment.text_start;
                word.end_char += segment.text_start;
            }
            samples.append(&mut part);
            timestamps.append(&mut words);
            if index + 1 < segments.len() {
                let count = pause_samples(f64::from(segment_pause_ms) / 1000.0);
                samples.extend(std::iter::repeat_n(0.0, count));
            }
        }
        if cancelled.load(Ordering::Relaxed) {
            return Err(Error::Cancelled);
        }
        let wav = synth::encode_wav(&samples)?;
        Ok(Synthesis {
            wav,
            duration_sec: samples_to_secs(samples.len()),
            samples,
            timestamps,
        })
    }

    /// Synthesize `segments`, handing every ready piece of audio to `emit`
    /// instead of returning one buffer.
    ///
    /// `emit` runs on the calling thread between model chunks: a consumer that
    /// must not block (a socket writer) enqueues and returns. Word timestamps
    /// are not produced on this path — they cost a duration-predictor call per
    /// word and no streaming consumer needs them.
    ///
    /// Cancellation is checked between chunks; a cancelled stream returns
    /// [`Error::Cancelled`] after the parts it already handed out.
    pub fn synthesize_stream(
        &self,
        segments: &[Segment<'_>],
        voice: &str,
        policy: &ChunkPolicy,
        cancelled: &AtomicBool,
        segment_pause_ms: u32,
        emit: &mut dyn FnMut(AudioPart<'_>) -> Result<()>,
    ) -> Result<StreamSummary> {
        if segments.is_empty() {
            return Err(Error::InvalidText("empty text".into()));
        }
        let styles = self.styles_for(voice)?;
        let mut summary = StreamSummary::default();

        for (index, segment) in segments.iter().enumerate() {
            if cancelled.load(Ordering::Relaxed) {
                return Err(Error::Cancelled);
            }
            let mut options = self.options;
            if segment.language == Language::Ru && segment.text.trim().chars().count() <= 2 {
                options.duration_scale *= SHORT_RU_SEGMENT_SCALE;
            }
            let estimate = |part: &str| self.estimate(part, &styles, segment.language);
            let chunks = chunk::split(segment.text, policy, estimate)?;

            for (chunk_index, chunk) in chunks.iter().enumerate() {
                if cancelled.load(Ordering::Relaxed) {
                    return Err(Error::Cancelled);
                }
                let (_, wave) =
                    self.synth_chunk(&styles, &chunk.text, segment.language, &options)?;
                let start_sec = samples_to_secs(summary.samples);
                summary.parts += 1;
                summary.samples += wave.samples.len();
                emit(AudioPart {
                    samples: &wave.samples,
                    text_start: Some(segment.text_start + chunk.start),
                    paragraph_break_after: chunk.paragraph_break_after,
                    start_sec,
                })?;

                if chunk_index + 1 < chunks.len() {
                    let pause_sec = if chunk.paragraph_break_after {
                        policy.paragraph_pause_sec()
                    } else {
                        policy.segment_pause_sec()
                    };
                    emit_pause(&mut summary, emit, pause_sec, chunk.paragraph_break_after)?;
                }
            }

            if index + 1 < segments.len() {
                let pause_sec = f64::from(segment_pause_ms) / 1000.0;
                emit_pause(&mut summary, emit, pause_sec, false)?;
            }
        }

        // Same cancellation contract as the buffered path: a cancel raised
        // during the last chunk still surfaces as `Cancelled`.
        if cancelled.load(Ordering::Relaxed) {
            return Err(Error::Cancelled);
        }
        summary.duration_sec = samples_to_secs(summary.samples);
        Ok(summary)
    }

    fn synthesize_samples(
        &self,
        text: &str,
        voice: &str,
        language: Language,
        policy: &ChunkPolicy,
        cancelled: &AtomicBool,
    ) -> Result<(Vec<f32>, Vec<Word>)> {
        let styles = self.styles_for(voice)?;
        // An isolated one- or two-letter Russian word (a conjunction or a
        // preposition between English fragments) makes the duration predictor
        // return a nearly fixed ~0.9 s, which sounds stretched. Scale only
        // such short Russian segments down; longer text keeps the measured
        // optimum.
        let mut options = self.options;
        if language == Language::Ru && text.trim().chars().count() <= 2 {
            options.duration_scale *= SHORT_RU_SEGMENT_SCALE;
        }
        let estimate = |part: &str| self.estimate(part, &styles, language);
        let chunks = chunk::split(text, policy, estimate)?;

        let mut samples: Vec<f32> = Vec::new();
        let mut timestamps: Vec<Word> = Vec::new();
        for (index, chunk) in chunks.iter().enumerate() {
            if cancelled.load(Ordering::Relaxed) {
                return Err(Error::Cancelled);
            }
            let (prepared, wave) = self.synth_chunk(&styles, &chunk.text, language, &options)?;

            let audio_start = samples.len() as f64 / synth::SAMPLE_RATE as f64;
            let audio_end = audio_start + wave.duration_sec;
            let silence = timestamps::leading_silence(&wave.samples);
            let open = format!("<{}>", language.tag());
            let close = format!("</{}>", language.tag());
            let inner = prepared
                .text
                .strip_prefix(&open)
                .and_then(|rest| rest.strip_suffix(&close))
                .unwrap_or(&prepared.text);
            let predict = |prefix: &str| {
                let encoded = self.indexer.encode(&prefix.replace('+', ""));
                synth::predict_raw(&self.sessions, encoded.ids, encoded.mask, &styles.dp)
            };
            let words = timestamps::chunk_timestamps(
                &chunk.text,
                inner,
                &prepared.dropped,
                timestamps::ChunkTiming {
                    chunk_start: chunk.start,
                    audio_start,
                    audio_end,
                    leading_silence: silence,
                },
                predict,
            )?;
            timestamps.extend(words);
            samples.extend_from_slice(&wave.samples);

            if index + 1 < chunks.len() {
                let pause = if chunk.paragraph_break_after {
                    policy.paragraph_pause_sec()
                } else {
                    policy.segment_pause_sec()
                };
                let count = pause_samples(pause);
                samples.extend(std::iter::repeat_n(0.0, count));
            }
        }

        if cancelled.load(Ordering::Relaxed) {
            return Err(Error::Cancelled);
        }
        Ok((samples, timestamps))
    }

    fn styles_for(&self, voice: &str) -> Result<Arc<Styles>> {
        let directory = self.bundle.voice_dir(voice)?;
        let mut cache = lock(&self.styles);
        if let Some(styles) = cache.get(voice) {
            return Ok(styles.clone());
        }
        let styles = Arc::new(Styles::load(&directory)?);
        cache.insert(voice.to_string(), styles.clone());
        Ok(styles)
    }

    fn estimate(&self, part: &str, styles: &Styles, language: Language) -> Result<f64> {
        let prepared = text::prepare(part, &self.accentizer, &self.indexer, language)?;
        let duration_text = prepared.text.replace('+', "");
        let encoded = self.indexer.encode(&duration_text);
        let raw = synth::predict_raw(&self.sessions, encoded.ids, encoded.mask, &styles.dp)?;
        Ok(raw * self.options.duration_scale as f64 / synth::SPEED as f64)
    }

    /// Prepare one chunk and run the four sessions on it.
    ///
    /// Shared by the buffered and the streaming paths so a fix to the model
    /// call cannot land in only one of them.
    fn synth_chunk(
        &self,
        styles: &Styles,
        chunk_text: &str,
        language: Language,
        options: &SynthOptions,
    ) -> Result<(text::PreparedText, synth::Waveform)> {
        let prepared = text::prepare(chunk_text, &self.accentizer, &self.indexer, language)?;
        let ids = self.indexer.encode(&prepared.text);
        let duration_text = prepared.text.replace('+', "");
        let duration_ids = self.indexer.encode(&duration_text);
        let wave = synth::synthesize(
            &self.sessions,
            styles,
            ids.ids,
            ids.mask,
            duration_ids.ids,
            duration_ids.mask,
            options,
        )?;
        Ok((prepared, wave))
    }
}

/// Samples of silence for `seconds` of pause. One rounding rule shared by the
/// buffered and the streaming path — the two must produce identical audio.
fn pause_samples(seconds: f64) -> usize {
    (seconds * synth::SAMPLE_RATE as f64).round() as usize
}

/// Seconds at `samples` samples into the stream. Offsets come from the integer
/// sample count, so `StreamSummary::duration_sec` equals `samples / rate`
/// instead of a sum of rounded per-part quotients that can drift in the last
/// bits.
fn samples_to_secs(samples: usize) -> f64 {
    samples as f64 / synth::SAMPLE_RATE as f64
}

/// Emit `seconds` of silence as its own stream part, updating the totals.
fn emit_pause(
    summary: &mut StreamSummary,
    emit: &mut dyn FnMut(AudioPart<'_>) -> Result<()>,
    seconds: f64,
    paragraph_break_after: bool,
) -> Result<()> {
    let count = pause_samples(seconds);
    if count == 0 {
        return Ok(());
    }
    let silence = vec![0.0f32; count];
    let start_sec = samples_to_secs(summary.samples);
    summary.parts += 1;
    summary.samples += count;
    emit(AudioPart {
        samples: &silence,
        text_start: None,
        paragraph_break_after,
        start_sec,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// What the streaming callbacks received, so the pause accounting is
    /// checked without a model bundle.
    #[derive(Default)]
    struct Parts {
        lens: Vec<usize>,
        starts: Vec<f64>,
        paragraph_after: Vec<bool>,
        all_zero: Vec<bool>,
    }

    impl Parts {
        fn record(&mut self, part: AudioPart<'_>) {
            self.lens.push(part.samples.len());
            self.starts.push(part.start_sec);
            self.paragraph_after.push(part.paragraph_break_after);
            self.all_zero
                .push(part.samples.iter().all(|sample| *sample == 0.0));
        }
    }

    #[test]
    fn pause_samples_is_the_shared_rounding_rule() {
        assert_eq!(pause_samples(0.0), 0);
        assert_eq!(pause_samples(0.2), 8_820);
        assert_eq!(pause_samples(0.25), 11_025);
        assert_eq!(pause_samples(0.001), 44, "rounds, never truncates to zero");
    }

    #[test]
    fn sample_offsets_do_not_drift_like_a_float_sum() {
        // Summing per-part seconds and dividing the total sample count
        // disagree in the last bits for these sizes; the stream derives every
        // offset from the sample count, so it cannot drift.
        assert_ne!(
            samples_to_secs(30_640) + samples_to_secs(57_895),
            samples_to_secs(30_640 + 57_895)
        );
    }

    #[test]
    fn a_zero_length_pause_emits_no_part() {
        let mut summary = StreamSummary::default();
        let mut parts = Parts::default();
        let mut emit = |part: AudioPart<'_>| {
            parts.record(part);
            Ok(())
        };

        emit_pause(&mut summary, &mut emit, 0.0, true).expect("a zero pause cannot fail");

        assert!(parts.lens.is_empty(), "no callback for an empty pause");
        assert_eq!(summary, StreamSummary::default(), "totals stay untouched");
    }

    #[test]
    fn consecutive_pauses_are_offset_by_the_samples_before_them() {
        let mut summary = StreamSummary {
            parts: 1,
            samples: 1_000,
            ..StreamSummary::default()
        };
        let mut parts = Parts::default();
        let mut emit = |part: AudioPart<'_>| {
            parts.record(part);
            Ok(())
        };

        emit_pause(&mut summary, &mut emit, 0.2, false).expect("first pause");
        emit_pause(&mut summary, &mut emit, 0.25, true).expect("second pause");

        assert_eq!(parts.lens, vec![8_820, 11_025]);
        assert_eq!(
            parts.starts,
            vec![samples_to_secs(1_000), samples_to_secs(9_820)],
            "each pause starts where the previous audio ended"
        );
        assert_eq!(parts.paragraph_after, vec![false, true]);
        assert!(
            parts.all_zero.iter().all(|zero| *zero),
            "a pause is silence"
        );
        assert_eq!(summary.parts, 3);
        assert_eq!(summary.samples, 20_845);
    }
}
