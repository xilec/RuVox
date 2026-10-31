//! `teratts`-backed [`TtsEngine`] implementation.
//!
//! Loads the verified ONNX bundle from `bundle_dir` and runs TeraTTSv2 fully
//! in-process — no Python sidecar, no PyTorch. Model load and synthesis are
//! synchronous CPU work, so both run in `spawn_blocking`, same as the Piper
//! and Silero Native engines.
//!
//! ## Failure mapping
//! - bundle missing on disk → `TtsError::Ttsd { code: "bundle_not_installed", … }`
//! - bundle corrupt / incompatible / ONNX failure → `bundle_corrupt` /
//!   `bundle_incompatible` / `synthesis_failed`
//! - unknown voice → `voice_unknown`
//! - text the model cannot read (tags, empty alphabet) → `text_invalid_tags`
//! - cancellation between chunks → `teratts_cancelled`
//! - WAV write failure → `TtsError::Ipc(io::Error)`
//!
//! ## Cancellation
//! The crate checks an `AtomicBool` before every chunk. `kill_current` flips
//! it; `synthesize` clears it before starting a new call. A cancelled call
//! returns an error and writes no WAV, which the synthesis worker reports as
//! the entry's cancellation, matching the Piper path.
//!
//! ## Word timestamps
//! The crate returns approximate word boundaries in normalized-text
//! coordinates (sentence boundaries are exact, word boundaries lag an ASR
//! reference by 0.2–0.4 s). They are mapped back to original-text offsets
//! through the pipeline `char_mapping` with the same span-merge logic ttsd and
//! silero-native use (`tts::map_via_spans`).

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use async_trait::async_trait;
use serde_json::json;
use tokio::sync::{Mutex, RwLock};
use tracing::{debug, info, warn};

use crate::pipeline::{Span, SpanKind};
use crate::tts::engine::{EngineKind, TtsEngine};
use crate::tts::map_via_spans;
use crate::tts::profiles;
use crate::tts::supervisor::Emitter;
use crate::tts::{CharMappingEntry, ModelInfo, SynthesizeOutput, TtsError, WordTimestamp};

/// In-process TeraTTSv2 engine (ONNX Runtime, no Python).
pub struct TeraTtsEngine {
    bundle_dir: PathBuf,
    /// Loaded model — behind `Arc<RwLock<...>>` so the detached
    /// `spawn_initial_warmup` task can install it without holding a borrow
    /// of `&self`. `None` until the first successful load.
    loaded: Arc<RwLock<Option<Arc<teratts::Engine>>>>,
    /// Serializes model loads: without it, concurrent `synthesize` / warmup
    /// callers on a cold engine each pay for their own load (the bundle is
    /// verified and the dictionaries are parsed on every load).
    load_lock: Arc<Mutex<()>>,
    /// Set by `kill_current`, checked by the crate between chunks.
    cancel: Arc<AtomicBool>,
    /// Frontend event emitter.
    emitter: Emitter,
}

/// User-facing explanation (Russian) for a missing model bundle. Shared by
/// the load path and the warmup path so both say the same thing.
fn bundle_missing_message(bundle_dir: &Path) -> String {
    format!(
        "Бандл моделей TeraTTS не скачан ({}). \
         Скачайте его в настройках (движок «TeraTTSv2»).",
        bundle_dir.display()
    )
}

impl TeraTtsEngine {
    /// Build a new engine. No I/O — the model is loaded lazily on first
    /// `warmup` / `synthesize`.
    pub fn new(bundle_dir: PathBuf, emitter: Emitter) -> Self {
        Self {
            bundle_dir,
            loaded: Arc::new(RwLock::new(None)),
            load_lock: Arc::new(Mutex::new(())),
            cancel: Arc::new(AtomicBool::new(false)),
            emitter,
        }
    }

    /// Cheap gate before paying for the full bundle verification.
    fn bundle_present(bundle_dir: &Path) -> bool {
        bundle_dir.join(teratts::bundle::MANIFEST_NAME).exists()
    }

    /// Load the model unless already loaded. Single-flighted by `load_lock`:
    /// the first caller loads, concurrent callers wait and then observe the
    /// filled slot. Shared by `ensure_loaded` and `spawn_initial_warmup` so
    /// both walk the same path.
    async fn load_single_flight(
        bundle_dir: &Path,
        loaded: &RwLock<Option<Arc<teratts::Engine>>>,
        load_lock: &Mutex<()>,
    ) -> Result<Arc<teratts::Engine>, TtsError> {
        {
            let guard = loaded.read().await;
            if let Some(engine) = guard.as_ref() {
                return Ok(Arc::clone(engine));
            }
        }

        let _single = load_lock.lock().await;
        {
            let guard = loaded.read().await;
            if let Some(engine) = guard.as_ref() {
                return Ok(Arc::clone(engine));
            }
        }

        if !Self::bundle_present(bundle_dir) {
            return Err(TtsError::Ttsd {
                code: "bundle_not_installed".to_string(),
                message: bundle_missing_message(bundle_dir),
            });
        }

        let dir = bundle_dir.to_path_buf();
        let engine = tokio::task::spawn_blocking(move || teratts::Engine::load(&dir))
            .await
            .map_err(|e| TtsError::Ttsd {
                code: "synthesis_failed".to_string(),
                message: format!("загрузка TeraTTS прервана: {e}"),
            })?
            .map_err(map_error)?;

        let engine = Arc::new(engine);
        let mut guard = loaded.write().await;
        *guard = Some(Arc::clone(&engine));
        info!(target: "tts::teratts", "model bundle loaded");
        Ok(engine)
    }

    async fn ensure_loaded(&self) -> Result<Arc<teratts::Engine>, TtsError> {
        Self::load_single_flight(&self.bundle_dir, &self.loaded, &self.load_lock).await
    }

    /// Reads the bundle manifest for the generation-params snapshot. The
    /// bundle identity is voice-independent. Best-effort: a read/parse failure
    /// yields `None` rather than blocking synthesis or guessing a value.
    fn bundle_model_info(bundle_dir: &Path) -> Option<ModelInfo> {
        let manifest = teratts::bundle::Manifest::load(bundle_dir).ok()?;
        let sha256 =
            teratts::bundle::sha256_file(&bundle_dir.join(teratts::bundle::MANIFEST_NAME)).ok();
        Some(ModelInfo {
            name: format!("{} bundle v{}", manifest.model_id, manifest.bundle_version),
            sha256,
            bundle_version: Some(manifest.bundle_version.to_string()),
            model_id: Some(manifest.model_id),
            upstream_revision: Some(manifest.upstream_revision),
        })
    }

    /// Shared synthesis path for the plain and span-aware entry points.
    #[allow(clippy::too_many_arguments)]
    async fn run_synthesis(
        &self,
        text: String,
        spans: Vec<Span>,
        voice: String,
        _sample_rate: u32,
        out_wav: String,
        char_mapping: Option<Vec<CharMappingEntry>>,
    ) -> Result<SynthesizeOutput, TtsError> {
        let engine = self.ensure_loaded().await?;

        // A new call starts uncancelled; `kill_current` flips this flag and the
        // crate stops before the next chunk.
        self.cancel.store(false, Ordering::SeqCst);
        let cancel = Arc::clone(&self.cancel);
        let policy = profiles::profile_for(EngineKind::TeraTts).chunk;
        let text = neutralize_code_markup(text, &spans);
        let result = tokio::task::spawn_blocking(move || {
            let segments = segments_for(&text, &spans);
            engine.synthesize_segments(
                &segments,
                &voice,
                &policy,
                &cancel,
                policy.pauses.segment_ms,
            )
        })
        .await
        .map_err(|e| TtsError::Ttsd {
            code: "synthesis_failed".to_string(),
            message: format!("задача синтеза TeraTTS прервана: {e}"),
        })?
        .map_err(map_error)?;

        write_wav(&out_wav, &result.wav).await?;
        Ok(SynthesizeOutput {
            timestamps: map_timestamps(result.timestamps, char_mapping.as_deref()),
            duration_sec: result.duration_sec,
        })
    }
}

/// Map a crate error to the user-facing code/message pair. One mapper for
/// load and synthesis: the variants are disjoint, and a second table would
/// drift.
fn map_error(e: teratts::Error) -> TtsError {
    match e {
        teratts::Error::BundleNotInstalled(dir) => TtsError::Ttsd {
            code: "bundle_not_installed".to_string(),
            message: bundle_missing_message(&dir),
        },
        teratts::Error::BundleIncompatible { found, expected } => TtsError::Ttsd {
            code: "bundle_incompatible".to_string(),
            message: format!(
                "Бандл TeraTTS формата {found} не поддерживается этой версией \
                 (нужен формат {expected}). Обновите приложение или перекачайте бандл."
            ),
        },
        teratts::Error::BundleCorrupt(detail) => TtsError::Ttsd {
            code: "bundle_corrupt".to_string(),
            message: format!("Бандл TeraTTS повреждён: {detail}. Перекачайте его в настройках."),
        },
        teratts::Error::VoiceUnknown { voice, available } => TtsError::Ttsd {
            code: "voice_unknown".to_string(),
            message: format!("Голос «{voice}» не найден. Доступны: {available}."),
        },
        teratts::Error::InvalidText(detail) => TtsError::Ttsd {
            code: "text_invalid_tags".to_string(),
            message: format!("Текст не подходит для TeraTTS: {detail}"),
        },
        teratts::Error::Synthesis(detail) => TtsError::Ttsd {
            code: "synthesis_failed".to_string(),
            message: format!("Ошибка синтеза TeraTTS: {detail}"),
        },
        teratts::Error::Cancelled => TtsError::Ttsd {
            code: "teratts_cancelled".to_string(),
            message: "Синтез отменён.".to_string(),
        },
    }
}

/// Split the normalized text into language segments at the pipeline spans.
///
/// Spans are sorted and non-overlapping, so a linear walk suffices: text
/// before, between and after spans is Russian, a span with `lang == "en"` is
/// English, and any other span (a dictionary override, or a `ru` tag) stays
/// Russian. Empty and whitespace-only segments are skipped; `text_start`
/// keeps every timestamp in whole-text codepoint coordinates.
fn segments_for<'a>(text: &'a str, spans: &[Span]) -> Vec<teratts::Segment<'a>> {
    let char_count = text.chars().count();
    let byte_offsets: Vec<usize> = text
        .char_indices()
        .map(|(byte, _)| byte)
        .chain(std::iter::once(text.len()))
        .collect();
    let slice = |start: usize, end: usize| &text[byte_offsets[start]..byte_offsets[end]];

    let mut segments = Vec::new();
    let mut push = |start: usize, end: usize, language: teratts::Language| {
        if start >= end {
            return;
        }
        let part = slice(start, end);
        // Whitespace and punctuation-only gaps carry no words; synthesizing
        // them alone makes the model vocalize the punctuation. A word-bearing
        // segment may still start with the previous sentence's punctuation
        // (an English span ends before its period), so drop that prefix too.
        let Some(prefix) = part
            .char_indices()
            .find(|(_, character)| character.is_alphanumeric())
            .map(|(byte, _)| byte)
        else {
            return;
        };
        let text = &part[prefix..];
        let text_start = start + part[..prefix].chars().count();
        segments.push(teratts::Segment {
            text,
            language,
            text_start,
        });
    };

    let mut cursor = 0usize;
    for span in spans {
        let start = span.start.min(char_count);
        let end = span.end.min(char_count);
        if start > cursor {
            push(cursor, start, teratts::Language::Ru);
        }
        let language = if span.lang == Some("en") {
            teratts::Language::En
        } else {
            teratts::Language::Ru
        };
        push(cursor.max(start), end, language);
        cursor = cursor.max(end);
    }
    if cursor < char_count {
        push(cursor, char_count, teratts::Language::Ru);
    }
    if segments.is_empty() {
        segments.push(teratts::Segment {
            text,
            language: teratts::Language::Ru,
            text_start: 0,
        });
    }
    segments
}

/// Replaces tag delimiters inside native code spans with spaces.
///
/// The model understands `<`/`>` only as tag syntax and the text frontend
/// rejects a stray one, but code legitimately contains both (`Vec<T>`,
/// `a < b`). Spaces keep the neighbouring words apart, which dropping them
/// would not, and the codepoint count stays the same, so spans, timestamps and
/// the char mapping remain valid. Every other segment keeps today's validation.
fn neutralize_code_markup(text: String, spans: &[Span]) -> String {
    let in_code = |index: usize| {
        spans
            .iter()
            .any(|span| span.kind == SpanKind::CodeBlock && span.start <= index && index < span.end)
    };
    let mut chars: Vec<char> = text.chars().collect();
    let mut replaced = 0usize;
    for (index, character) in chars.iter_mut().enumerate() {
        if matches!(character, '<' | '>') && in_code(index) {
            *character = ' ';
            replaced += 1;
        }
    }
    if replaced > 0 {
        debug!(
            count = replaced,
            "replaced tag delimiters inside a native code block"
        );
        chars.into_iter().collect()
    } else {
        text
    }
}

/// Write the engine's WAV bytes to `out_wav`, creating the parent directory.
async fn write_wav(out_wav: &str, wav: &[u8]) -> Result<(), TtsError> {
    let out_path = PathBuf::from(out_wav);
    if let Some(parent) = out_path.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(TtsError::Ipc)?;
    }
    tokio::fs::write(&out_path, wav)
        .await
        .map_err(TtsError::Ipc)
}

/// Map engine word timestamps (offsets in the synthesized text) back to
/// original-text offsets through the pipeline char mapping.
fn map_timestamps(
    engine_ts: Vec<teratts::timestamps::Word>,
    char_mapping: Option<&[CharMappingEntry]>,
) -> Vec<WordTimestamp> {
    engine_ts
        .into_iter()
        .map(|word| WordTimestamp {
            word: word.word,
            start: word.start,
            end: word.end,
            original_pos: match char_mapping {
                Some(spans) => map_via_spans(spans, word.start_char, word.end_char),
                None => (word.start_char, word.end_char),
            },
        })
        .collect()
}

#[async_trait]
impl TtsEngine for TeraTtsEngine {
    fn kind(&self) -> EngineKind {
        EngineKind::TeraTts
    }

    async fn model_info(&self, _voice: &str) -> Option<ModelInfo> {
        Self::bundle_model_info(&self.bundle_dir)
    }

    async fn warmup(&self) -> Result<(), TtsError> {
        let _ = self.ensure_loaded().await?;
        Ok(())
    }

    async fn spawn_initial_warmup(&self) {
        let bundle_dir = self.bundle_dir.clone();
        let emitter = Arc::clone(&self.emitter);
        let slot = Arc::clone(&self.loaded);
        let load_lock = Arc::clone(&self.load_lock);

        tokio::spawn(async move {
            (emitter)("model_loading", json!({ "engine": "teratts" }));

            match TeraTtsEngine::load_single_flight(&bundle_dir, &slot, &load_lock).await {
                Ok(_) => {
                    info!(target: "tts::teratts", "warmup complete");
                    (emitter)("model_loaded", json!({ "engine": "teratts" }));
                }
                Err(err) => {
                    warn!(target: "tts::teratts", "warmup load failed: {err}");
                    let message = match &err {
                        TtsError::Ttsd { code, message } if code == "bundle_not_installed" => {
                            message.clone()
                        }
                        other => other.to_string(),
                    };
                    (emitter)(
                        "model_error",
                        json!({ "engine": "teratts", "message": message }),
                    );
                }
            }
        });
    }

    async fn synthesize(
        &self,
        text: String,
        voice: String,
        sample_rate: u32,
        out_wav: String,
        char_mapping: Option<Vec<CharMappingEntry>>,
    ) -> Result<SynthesizeOutput, TtsError> {
        self.run_synthesis(text, Vec::new(), voice, sample_rate, out_wav, char_mapping)
            .await
    }

    async fn synthesize_spans(
        &self,
        text: String,
        spans: Vec<Span>,
        voice: String,
        sample_rate: u32,
        out_wav: String,
        char_mapping: Option<Vec<CharMappingEntry>>,
    ) -> Result<SynthesizeOutput, TtsError> {
        self.run_synthesis(text, spans, voice, sample_rate, out_wav, char_mapping)
            .await
    }

    async fn kill_current(&self) {
        self.cancel.store(true, Ordering::SeqCst);
    }

    async fn shutdown(&self) -> Result<(), TtsError> {
        // In-process — drop the model so ONNX Runtime releases its sessions
        // (and the accentizer's ~3M entries). The next `warmup` reloads.
        let mut guard = self.loaded.write().await;
        *guard = None;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn segments_follow_prose_spans_and_keep_absolute_offsets() {
        let text = "Привет Kubernetes мир";
        let spans = [Span {
            kind: crate::pipeline::SpanKind::ProseLatin,
            lang: Some("en"),
            start: 7,
            end: 17,
        }];
        let segments = segments_for(text, &spans);
        assert_eq!(segments.len(), 3);
        assert_eq!(segments[0].text, "Привет ");
        assert_eq!(segments[0].language, teratts::Language::Ru);
        assert_eq!(segments[0].text_start, 0);
        assert_eq!(segments[1].text, "Kubernetes");
        assert_eq!(segments[1].language, teratts::Language::En);
        assert_eq!(segments[1].text_start, 7);
        assert_eq!(segments[2].text, "мир");
        assert_eq!(segments[2].text_start, 18);
    }

    #[test]
    fn leading_punctuation_is_trimmed_from_a_segment() {
        let text = "API. Привет";
        let spans = [Span {
            kind: crate::pipeline::SpanKind::ProseLatin,
            lang: Some("en"),
            start: 0,
            end: 3,
        }];
        let segments = segments_for(text, &spans);
        assert_eq!(segments.len(), 2);
        assert_eq!(segments[0].text, "API");
        assert_eq!(segments[1].text, "Привет");
        assert_eq!(segments[1].text_start, 5);
    }

    #[test]
    fn segments_without_spans_are_one_russian_segment() {
        let segments = segments_for("Привет мир", &[]);
        assert_eq!(segments.len(), 1);
        assert_eq!(segments[0].language, teratts::Language::Ru);
        assert_eq!(segments[0].text, "Привет мир");
    }

    #[test]
    fn dictionary_override_spans_stay_russian() {
        let text = "молоко";
        let spans = [Span {
            kind: crate::pipeline::SpanKind::DictionaryOverride,
            lang: None,
            start: 0,
            end: 6,
        }];
        let segments = segments_for(text, &spans);
        assert_eq!(segments.len(), 1);
        assert_eq!(segments[0].language, teratts::Language::Ru);
    }

    #[test]
    fn tag_delimiters_inside_native_code_become_spaces() {
        let text = "Vec<T> и a < b";
        let spans = [Span {
            kind: SpanKind::CodeBlock,
            lang: Some("en"),
            start: 0,
            end: 6,
        }];
        let neutralized = neutralize_code_markup(text.to_string(), &spans);
        assert_eq!(neutralized, "Vec T  и a < b");
        assert_eq!(neutralized.chars().count(), text.chars().count());
    }

    #[test]
    fn tag_delimiters_outside_native_code_stay_for_validation() {
        let text = "привет < мир";
        let neutralized = neutralize_code_markup(text.to_string(), &[]);
        assert_eq!(neutralized, text);
    }

    #[test]
    fn pipeline_code_block_survives_neutralization_at_the_same_offsets() {
        let mut pipeline = crate::pipeline::TTSPipeline::new();
        pipeline
            .set_code_block_mode(crate::pipeline::normalizers::code_blocks::CodeBlockMode::Full);
        let out = pipeline.process_for_engine("```rust\nVec<T>\n```", &profiles::TERATTS_PROFILE);
        assert_eq!(out.text, "Vec<T>");
        let span = out
            .spans
            .iter()
            .find(|span| span.kind == SpanKind::CodeBlock)
            .expect("the native block is annotated");
        let emitted: String = out
            .text
            .chars()
            .skip(span.start)
            .take(span.end - span.start)
            .collect();
        assert_eq!(emitted, "Vec<T>");

        let neutralized = neutralize_code_markup(out.text, &out.spans);
        assert_eq!(neutralized, "Vec T ");
        let segments = segments_for(&neutralized, &out.spans);
        assert_eq!(segments.len(), 1);
        assert_eq!(segments[0].text.trim_end(), "Vec T");
        assert_eq!(segments[0].text_start, span.start);
        assert_eq!(segments[0].language, teratts::Language::En);
    }

    #[test]
    fn error_mapping_keeps_the_codes_stable() {
        let cases = [
            (
                teratts::Error::BundleNotInstalled(PathBuf::from("/b")),
                "bundle_not_installed",
            ),
            (
                teratts::Error::BundleIncompatible {
                    found: 2,
                    expected: 1,
                },
                "bundle_incompatible",
            ),
            (
                teratts::Error::BundleCorrupt("bad".into()),
                "bundle_corrupt",
            ),
            (
                teratts::Error::VoiceUnknown {
                    voice: "ru_m9".into(),
                    available: "ru_m5".into(),
                },
                "voice_unknown",
            ),
            (
                teratts::Error::InvalidText("tag".into()),
                "text_invalid_tags",
            ),
            (teratts::Error::Synthesis("ort".into()), "synthesis_failed"),
            (teratts::Error::Cancelled, "teratts_cancelled"),
        ];
        for (error, expected) in cases {
            let mapped = map_error(error);
            match mapped {
                TtsError::Ttsd { code, message } => {
                    assert_eq!(code, expected);
                    assert!(!message.is_empty());
                }
                other => panic!("unexpected variant: {other:?}"),
            }
        }
    }
}
