//! The public engine: bundle, sessions, text frontend, chunking, synthesis and
//! timestamps behind one call.
//!
//! The application adapter owns the `TtsEngine` trait, error translation and
//! file writing; this type owns everything that is model-specific.

use std::collections::{BTreeMap, HashMap};
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex, MutexGuard};

use tracing::{info, instrument};
use tts_contract::Chunk as ChunkPolicy;

use crate::bundle::{Bundle, Manifest, Voice};
use crate::chunk;
use crate::error::{Error, Result};
use crate::indexer::Indexer;
use crate::styles::Styles;
use crate::synth::{self, SessionPool, Sessions, SynthOptions};
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
    sessions: SessionPool,
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

/// Lanes the engine opens by default when the thread budget allows.
pub const DEFAULT_LANES: usize = 4;

/// Split a total intra-op thread budget into lanes: four lanes when the budget
/// allows it, never fewer than one thread per lane.
pub fn lane_split(total_threads: usize) -> (usize, usize) {
    let total = total_threads.max(1);
    let lanes = DEFAULT_LANES.min(total);
    (lanes, (total / lanes).max(1))
}

/// One chunk of work for the lane pool: everything a lane needs to synthesize
/// it, plus where it belongs in the assembled utterance.
struct PlannedChunk {
    text: String,
    /// Absolute codepoint offset in the normalized utterance.
    text_start: usize,
    language: Language,
    options: SynthOptions,
    /// Whether a paragraph break follows this chunk in the source text.
    paragraph_break_after: bool,
    pause_after: PauseAfter,
}

/// What separates this chunk from the next one in the assembled audio.
#[derive(Clone, Copy, PartialEq, Eq)]
enum PauseAfter {
    /// The chunk ends the utterance.
    None,
    /// Another chunk of the same segment follows.
    Chunk { paragraph: bool },
    /// Another segment follows; the caller's segment pause separates them.
    Segment,
}

/// One synthesized chunk with its word timestamps in chunk-local coordinates:
/// audio seconds from the chunk's first sample, codepoints from its first
/// character. The assembler shifts both.
struct ChunkAudio {
    samples: Vec<f32>,
    timestamps: Vec<Word>,
}

/// The pause that follows `chunk`, if any: inside a segment the profile's
/// paragraph or segment pause, between segments the caller's value.
fn chunk_pause(
    chunk: &PlannedChunk,
    policy: &ChunkPolicy,
    segment_pause_ms: u32,
) -> Option<(f64, bool)> {
    match chunk.pause_after {
        PauseAfter::None => None,
        PauseAfter::Chunk { paragraph } => Some((
            if paragraph {
                policy.paragraph_pause_sec()
            } else {
                policy.segment_pause_sec()
            },
            paragraph,
        )),
        PauseAfter::Segment => Some((f64::from(segment_pause_ms) / 1000.0, false)),
    }
}

impl Engine {
    /// Load and verify a bundle with the default lane pool.
    #[instrument(skip_all, fields(dir = %dir.display()))]
    pub fn load(dir: &Path) -> Result<Self> {
        let (lanes, threads) = lane_split(default_threads());
        Self::load_with_lanes(dir, lanes, threads)
    }

    /// Load and verify a bundle with one lane of `threads` intra-op threads.
    ///
    /// The parity fixtures and the benchmark example pin the thread count this
    /// way: ONNX Runtime's intra-op reduction order changes the samples, so the
    /// multi-lane default would silently move the reference.
    pub fn load_with_threads(dir: &Path, threads: usize) -> Result<Self> {
        Self::load_with_lanes(dir, 1, threads)
    }

    /// Load and verify a bundle with `lanes` independent session sets of
    /// `threads` intra-op threads each.
    pub fn load_with_lanes(dir: &Path, lanes: usize, threads: usize) -> Result<Self> {
        let started = std::time::Instant::now();
        let bundle = Bundle::load(dir)?;
        let indexer = Indexer::load(&bundle.file("unicode_indexer.json"))?;
        let accentizer = Accentizer::load(&bundle.dir().join("ruaccent/dictionary"))?;
        let sessions = SessionPool::open(&bundle, lanes, threads)?;
        info!(
            elapsed_ms = started.elapsed().as_secs_f64() * 1e3,
            lanes = sessions.lanes().len(),
            threads = sessions.threads_per_lane(),
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
        let segments = [Segment {
            text,
            language,
            text_start: 0,
        }];
        self.synthesize_segments(&segments, voice, policy, cancelled, 0)
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
        let (styles, plan) = self.plan_chunks(segments, voice, policy, cancelled)?;
        let mut samples: Vec<f32> = Vec::new();
        let mut timestamps: Vec<Word> = Vec::new();
        self.run_plan(&plan, &styles, true, cancelled, |chunk, audio| {
            let ChunkAudio {
                samples: chunk_samples,
                timestamps: words,
            } = audio;
            let audio_start = samples_to_secs(samples.len());
            timestamps.extend(words.into_iter().map(|mut word| {
                word.start += audio_start;
                word.end += audio_start;
                word.start_char += chunk.text_start;
                word.end_char += chunk.text_start;
                word
            }));
            samples.extend_from_slice(&chunk_samples);
            if let Some((pause, _)) = chunk_pause(chunk, policy, segment_pause_ms) {
                samples.extend(std::iter::repeat_n(0.0, pause_samples(pause)));
            }
            Ok(())
        })?;
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
        let (styles, plan) = self.plan_chunks(segments, voice, policy, cancelled)?;
        let mut summary = StreamSummary::default();
        let emit = &mut *emit;
        self.run_plan(&plan, &styles, false, cancelled, |chunk, audio| {
            let start_sec = samples_to_secs(summary.samples);
            summary.parts += 1;
            summary.samples += audio.samples.len();
            emit(AudioPart {
                samples: &audio.samples,
                text_start: Some(chunk.text_start),
                paragraph_break_after: chunk.paragraph_break_after,
                start_sec,
            })?;
            if let Some((pause, paragraph)) = chunk_pause(chunk, policy, segment_pause_ms) {
                emit_pause(&mut summary, emit, pause, paragraph)?;
            }
            Ok(())
        })?;
        // Same cancellation contract as the buffered path: a cancel raised
        // during the last chunk still surfaces as `Cancelled`.
        if cancelled.load(Ordering::Relaxed) {
            return Err(Error::Cancelled);
        }
        summary.duration_sec = samples_to_secs(summary.samples);
        Ok(summary)
    }

    /// Split every segment into chunks under the policy and record where each
    /// chunk goes, so the lanes only have to synthesize.
    fn plan_chunks(
        &self,
        segments: &[Segment<'_>],
        voice: &str,
        policy: &ChunkPolicy,
        cancelled: &AtomicBool,
    ) -> Result<(Arc<Styles>, Vec<PlannedChunk>)> {
        let styles = self.styles_for(voice)?;
        let mut plan = Vec::new();
        for (index, segment) in segments.iter().enumerate() {
            if cancelled.load(Ordering::Relaxed) {
                return Err(Error::Cancelled);
            }
            // An isolated one- or two-letter Russian word (a conjunction or a
            // preposition between English fragments) makes the duration
            // predictor return a nearly fixed ~0.9 s, which sounds stretched.
            // Scale only such short Russian segments down; longer text keeps
            // the measured optimum.
            let mut options = self.options;
            if segment.language == Language::Ru && segment.text.trim().chars().count() <= 2 {
                options.duration_scale *= SHORT_RU_SEGMENT_SCALE;
            }
            let estimate = |part: &str| self.estimate(part, &styles, segment.language);
            let chunks = chunk::split(segment.text, policy, estimate)?;
            let last_segment = index + 1 == segments.len();
            let chunk_count = chunks.len();
            for (position, chunk) in chunks.iter().enumerate() {
                let pause_after = if position + 1 < chunk_count {
                    PauseAfter::Chunk {
                        paragraph: chunk.paragraph_break_after,
                    }
                } else if last_segment {
                    PauseAfter::None
                } else {
                    PauseAfter::Segment
                };
                plan.push(PlannedChunk {
                    text: chunk.text.clone(),
                    text_start: segment.text_start + chunk.start,
                    language: segment.language,
                    options,
                    paragraph_break_after: chunk.paragraph_break_after,
                    pause_after,
                });
            }
        }
        Ok((styles, plan))
    }

    /// Run the plan across the lane pool, delivering chunks in text order.
    ///
    /// `with_timestamps` is off for the streaming path, which never returns
    /// timestamps and would pay a duration-predictor call per word for nothing.
    fn run_plan(
        &self,
        plan: &[PlannedChunk],
        styles: &Styles,
        with_timestamps: bool,
        cancelled: &AtomicBool,
        mut emit: impl FnMut(&PlannedChunk, ChunkAudio) -> Result<()>,
    ) -> Result<()> {
        if plan.is_empty() {
            return Ok(());
        }
        let next = AtomicUsize::new(0);
        // Set when the consumer stops early (an error or a cancellation): the
        // lanes then stop taking new chunks instead of draining the plan.
        let stop = AtomicBool::new(false);
        let (sender, receiver) = mpsc::channel();
        std::thread::scope(|scope| -> Result<()> {
            let next = &next;
            let stop = &stop;
            for lane in self.sessions.lanes() {
                let sender = sender.clone();
                scope.spawn(move || {
                    while !stop.load(Ordering::Relaxed) && !cancelled.load(Ordering::Relaxed) {
                        let index = next.fetch_add(1, Ordering::Relaxed);
                        if index >= plan.len() {
                            break;
                        }
                        let result = self.synthesize_chunk(
                            lane,
                            styles,
                            &plan[index],
                            with_timestamps,
                            cancelled,
                        );
                        if sender.send((index, result)).is_err() {
                            break;
                        }
                    }
                });
            }
            drop(sender);

            let mut pending: BTreeMap<usize, Result<ChunkAudio>> = BTreeMap::new();
            let mut expected = 0;
            while expected < plan.len() {
                if cancelled.load(Ordering::Relaxed) {
                    stop.store(true, Ordering::Relaxed);
                    return Err(Error::Cancelled);
                }
                while !pending.contains_key(&expected) {
                    match receiver.recv() {
                        Ok((index, result)) => {
                            pending.insert(index, result);
                        }
                        Err(_) => break,
                    }
                }
                let Some(result) = pending.remove(&expected) else {
                    if cancelled.load(Ordering::Relaxed) {
                        return Err(Error::Cancelled);
                    }
                    return Err(Error::Synthesis(
                        "chunk lanes stopped before delivering the next chunk".into(),
                    ));
                };
                let audio = match result {
                    Ok(audio) => audio,
                    Err(error) => {
                        stop.store(true, Ordering::Relaxed);
                        return Err(error);
                    }
                };
                if let Err(error) = emit(&plan[expected], audio) {
                    stop.store(true, Ordering::Relaxed);
                    return Err(error);
                }
                expected += 1;
            }
            Ok(())
        })
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
        let raw = synth::predict_raw(self.sessions.lane(0), encoded.ids, encoded.mask, &styles.dp)?;
        Ok(raw * self.options.duration_scale as f64 / synth::SPEED as f64)
    }

    /// Prepare one chunk and run the four sessions on it, on its own lane.
    fn synthesize_chunk(
        &self,
        lane: &Sessions,
        styles: &Styles,
        chunk: &PlannedChunk,
        with_timestamps: bool,
        cancelled: &AtomicBool,
    ) -> Result<ChunkAudio> {
        if cancelled.load(Ordering::Relaxed) {
            return Err(Error::Cancelled);
        }
        let prepared = text::prepare(&chunk.text, &self.accentizer, &self.indexer, chunk.language)?;
        let ids = self.indexer.encode(&prepared.text);
        let duration_text = prepared.text.replace('+', "");
        let duration_ids = self.indexer.encode(&duration_text);
        let wave = synth::synthesize(
            lane,
            styles,
            ids.ids,
            ids.mask,
            duration_ids.ids,
            duration_ids.mask,
            &chunk.options,
        )?;
        let timestamps = if with_timestamps {
            self.chunk_timestamps(lane, styles, chunk, &prepared, &wave)?
        } else {
            Vec::new()
        };
        Ok(ChunkAudio {
            samples: wave.samples,
            timestamps,
        })
    }

    /// Word timestamps of one chunk in chunk-local coordinates; the assembler
    /// shifts the audio and codepoint origins to their place in the utterance.
    fn chunk_timestamps(
        &self,
        lane: &Sessions,
        styles: &Styles,
        chunk: &PlannedChunk,
        prepared: &text::PreparedText,
        wave: &synth::Waveform,
    ) -> Result<Vec<Word>> {
        let open = format!("<{}>", chunk.language.tag());
        let close = format!("</{}>", chunk.language.tag());
        let inner = prepared
            .text
            .strip_prefix(&open)
            .and_then(|rest| rest.strip_suffix(&close))
            .unwrap_or(&prepared.text);
        let predict = |prefix: &str| {
            let encoded = self.indexer.encode(&prefix.replace('+', ""));
            synth::predict_raw(lane, encoded.ids, encoded.mask, &styles.dp)
        };
        timestamps::chunk_timestamps(
            &chunk.text,
            inner,
            &prepared.dropped,
            timestamps::ChunkTiming {
                chunk_start: 0,
                audio_start: 0.0,
                audio_end: wave.duration_sec,
                leading_silence: timestamps::leading_silence(&wave.samples),
            },
            predict,
        )
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

    #[test]
    fn lane_split_caps_the_pool_and_never_starves_a_lane() {
        assert_eq!(lane_split(12), (4, 3));
        assert_eq!(lane_split(24), (4, 6));
        assert_eq!(lane_split(4), (4, 1));
        assert_eq!(lane_split(3), (3, 1));
        assert_eq!(lane_split(1), (1, 1));
        assert_eq!(lane_split(0), (1, 1), "a zero budget still yields one lane");
    }

    fn planned(pause_after: PauseAfter) -> PlannedChunk {
        PlannedChunk {
            text: String::new(),
            text_start: 0,
            language: Language::Ru,
            options: SynthOptions::default(),
            paragraph_break_after: false,
            pause_after,
        }
    }

    #[test]
    fn the_pause_after_a_chunk_follows_its_kind() {
        let policy = ChunkPolicy {
            limit: tts_contract::Limit::Chars(40),
            pauses: tts_contract::Pauses {
                paragraph_ms: 250,
                segment_ms: 200,
            },
        };

        assert_eq!(chunk_pause(&planned(PauseAfter::None), &policy, 120), None);
        assert_eq!(
            chunk_pause(
                &planned(PauseAfter::Chunk { paragraph: false }),
                &policy,
                120
            ),
            Some((0.2, false)),
            "a plain chunk boundary uses the profile's segment pause"
        );
        assert_eq!(
            chunk_pause(
                &planned(PauseAfter::Chunk { paragraph: true }),
                &policy,
                120
            ),
            Some((0.25, true)),
            "a paragraph break uses the profile's paragraph pause"
        );
        assert_eq!(
            chunk_pause(&planned(PauseAfter::Segment), &policy, 120),
            Some((0.12, false)),
            "between segments the caller's pause wins"
        );
    }
}
