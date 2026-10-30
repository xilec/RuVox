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
            let audio_offset = samples.len() as f64 / synth::SAMPLE_RATE as f64;
            for word in &mut words {
                word.start += audio_offset;
                word.end += audio_offset;
                word.start_char += segment.text_start;
                word.end_char += segment.text_start;
            }
            samples.append(&mut part);
            timestamps.append(&mut words);
            if index + 1 < segments.len() {
                let count =
                    (segment_pause_ms as f64 / 1000.0 * synth::SAMPLE_RATE as f64).round() as usize;
                samples.extend(std::iter::repeat_n(0.0, count));
            }
        }
        if cancelled.load(Ordering::Relaxed) {
            return Err(Error::Cancelled);
        }
        let wav = synth::encode_wav(&samples)?;
        Ok(Synthesis {
            wav,
            duration_sec: samples.len() as f64 / synth::SAMPLE_RATE as f64,
            samples,
            timestamps,
        })
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
            let prepared = text::prepare(&chunk.text, &self.accentizer, &self.indexer, language)?;
            let ids = self.indexer.encode(&prepared.text);
            let duration_text = prepared.text.replace('+', "");
            let duration_ids = self.indexer.encode(&duration_text);
            let wave = synth::synthesize(
                &self.sessions,
                &styles,
                ids.ids,
                ids.mask,
                duration_ids.ids,
                duration_ids.mask,
                &options,
            )?;

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
                let count = (pause * synth::SAMPLE_RATE as f64).round() as usize;
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
}
