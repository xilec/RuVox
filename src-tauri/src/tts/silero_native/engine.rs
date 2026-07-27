//! `silero-native`-backed [`TtsEngine`] implementation.
//!
//! Loads the pre-exported ONNX bundle from `bundle_dir` (see
//! `silero-native/src/bundle.rs` for the manifest format) and runs Silero v5
//! inference fully in-process — no Python sidecar. Model load and synthesis
//! are synchronous CPU work, so both run in `spawn_blocking`, same as the
//! Piper engine.
//!
//! ## Failure mapping
//! - bundle missing on disk → `TtsError::Ttsd { code: "bundle_not_installed", … }`
//! - bundle corrupt / ONNX session failure → `TtsError::Ttsd { code: "silero_native_load_failed", … }`
//! - bad synthesize input (unknown speaker / sample rate) → `TtsError::Ttsd { code: "bad_input", … }`
//! - inference failure → `TtsError::Ttsd { code: "silero_native_synthesis_failed", … }`
//! - WAV write failure → `TtsError::Ipc(io::Error)`
//!
//! ## Word timestamps
//! The engine returns char-proportional timestamps whose `original_pos` are
//! char offsets into the text it synthesized (after stripping unsupported
//! markup). The app pipeline never emits `[[...]]` / SSML markup, so those
//! offsets line up with the normalized text; we then map them back to
//! original-text offsets through the pipeline `char_mapping` with the same
//! span-merge logic ttsd uses (`tts::piper::timestamps::map_via_spans`). When
//! markup *is* present the positions degrade to an approximation — the same
//! class of drift the ttsd path has.

use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use serde_json::json;
use silero_native::{EngineError, SileroNative};
use tokio::sync::RwLock;
use tracing::{info, warn};

use crate::tts::engine::{EngineKind, TtsEngine};
use crate::tts::piper::timestamps::map_via_spans;
use crate::tts::supervisor::Emitter;
use crate::tts::{CharMappingEntry, SynthesizeOutput, TtsError, WordTimestamp};

/// In-process Silero v5 engine (ONNX Runtime, no Python).
pub struct SileroNativeEngine {
    bundle_dir: PathBuf,
    /// Loaded model — behind `Arc<RwLock<...>>` so the detached
    /// `spawn_initial_warmup` task can install it without holding a borrow
    /// of `&self`. `None` until the first successful load.
    loaded: Arc<RwLock<Option<Arc<SileroNative>>>>,
    /// Frontend event emitter.
    emitter: Emitter,
}

/// User-facing explanation (Russian) for a missing model bundle. Shared by
/// the load path and the warmup path so both say the same thing.
fn bundle_missing_message(bundle_dir: &std::path::Path) -> String {
    format!(
        "Бандл моделей Silero не скачан ({}). \
         Скачайте его в настройках (движок «Silero (нативный)»).",
        bundle_dir.display()
    )
}

impl SileroNativeEngine {
    /// Build a new engine. No I/O — the model is loaded lazily on first
    /// `warmup` / `synthesize`.
    pub fn new(bundle_dir: PathBuf, emitter: Emitter) -> Self {
        Self {
            bundle_dir,
            loaded: Arc::new(RwLock::new(None)),
            emitter,
        }
    }

    /// Whether the bundle directory looks loadable (manifest present). The
    /// full manifest + per-file probe lives in `tts::availability`; here we
    /// only need a cheap gate before paying for `SileroNative::load`.
    fn bundle_present(bundle_dir: &std::path::Path) -> bool {
        bundle_dir.join("manifest.json").exists()
    }

    /// Load the model unless already loaded. Concurrent callers serialize on
    /// the write lock; the second one observes the filled slot.
    async fn ensure_loaded(&self) -> Result<Arc<SileroNative>, TtsError> {
        {
            let guard = self.loaded.read().await;
            if let Some(engine) = guard.as_ref() {
                return Ok(Arc::clone(engine));
            }
        }

        if !Self::bundle_present(&self.bundle_dir) {
            return Err(TtsError::Ttsd {
                code: "bundle_not_installed".to_string(),
                message: bundle_missing_message(&self.bundle_dir),
            });
        }

        let bundle_dir = self.bundle_dir.clone();
        let engine = tokio::task::spawn_blocking(move || SileroNative::load(&bundle_dir))
            .await
            .map_err(|e| TtsError::Ttsd {
                code: "silero_native_load_panic".to_string(),
                message: format!("silero-native load task panicked: {e}"),
            })?
            .map_err(map_load_error)?;

        let engine = Arc::new(engine);
        let mut guard = self.loaded.write().await;
        *guard = Some(Arc::clone(&engine));
        info!(target: "tts::silero_native", "model bundle loaded");
        Ok(engine)
    }
}

fn map_load_error(e: EngineError) -> TtsError {
    TtsError::Ttsd {
        code: "silero_native_load_failed".to_string(),
        message: format!("не удалось загрузить бандл Silero: {e}"),
    }
}

fn map_synthesis_error(e: EngineError) -> TtsError {
    match e {
        EngineError::BadInput(msg) => TtsError::Ttsd {
            code: "bad_input".to_string(),
            message: msg,
        },
        other => TtsError::Ttsd {
            code: "silero_native_synthesis_failed".to_string(),
            message: format!("silero-native synthesis failed: {other}"),
        },
    }
}

/// Map engine word timestamps (offsets in the synthesized text) back to
/// original-text offsets through the pipeline char mapping — the ttsd
/// contract (`_map_via_spans` in `ttsd/timestamps.py`).
fn map_timestamps(
    engine_ts: Vec<silero_native::WordTimestamp>,
    char_mapping: Option<&[CharMappingEntry]>,
) -> Vec<WordTimestamp> {
    engine_ts
        .into_iter()
        .map(|w| WordTimestamp {
            word: w.word,
            start: w.start as f64,
            end: w.end as f64,
            original_pos: match char_mapping {
                Some(spans) => map_via_spans(spans, w.original_pos.0, w.original_pos.1),
                None => w.original_pos,
            },
        })
        .collect()
}

#[async_trait]
impl TtsEngine for SileroNativeEngine {
    fn kind(&self) -> EngineKind {
        EngineKind::SileroNative
    }

    async fn warmup(&self) -> Result<(), TtsError> {
        let _ = self.ensure_loaded().await?;
        Ok(())
    }

    async fn spawn_initial_warmup(&self) {
        let bundle_dir = self.bundle_dir.clone();
        let emitter = Arc::clone(&self.emitter);
        let slot = Arc::clone(&self.loaded);

        tokio::spawn(async move {
            (emitter)("model_loading", json!({ "engine": "silero_native" }));

            if !SileroNativeEngine::bundle_present(&bundle_dir) {
                let msg = bundle_missing_message(&bundle_dir);
                warn!(target: "tts::silero_native", "warmup skipped: {msg}");
                (emitter)(
                    "model_error",
                    json!({ "engine": "silero_native", "message": msg }),
                );
                return;
            }

            let load_result =
                tokio::task::spawn_blocking(move || SileroNative::load(&bundle_dir)).await;

            match load_result {
                Ok(Ok(engine)) => {
                    let mut guard = slot.write().await;
                    *guard = Some(Arc::new(engine));
                    info!(target: "tts::silero_native", "warmup complete");
                    (emitter)("model_loaded", json!({ "engine": "silero_native" }));
                }
                Ok(Err(e)) => {
                    let err = map_load_error(e);
                    warn!(target: "tts::silero_native", "warmup load failed: {err}");
                    (emitter)(
                        "model_error",
                        json!({ "engine": "silero_native", "message": err.to_string() }),
                    );
                }
                Err(e) => {
                    warn!(target: "tts::silero_native", "warmup task panicked: {e}");
                    (emitter)(
                        "model_error",
                        json!({ "engine": "silero_native", "message": e.to_string() }),
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
        let engine = self.ensure_loaded().await?;

        let text_for_blocking = text.clone();
        let result = tokio::task::spawn_blocking(move || {
            engine.synthesize(&text_for_blocking, &voice, sample_rate)
        })
        .await
        .map_err(|e| TtsError::Ttsd {
            code: "silero_native_synthesis_panic".to_string(),
            message: format!("synthesis task panicked: {e}"),
        })?
        .map_err(map_synthesis_error)?;

        let out_path = PathBuf::from(&out_wav);
        if let Some(parent) = out_path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(TtsError::Ipc)?;
        }
        tokio::fs::write(&out_path, &result.wav)
            .await
            .map_err(TtsError::Ipc)?;

        let timestamps = map_timestamps(result.timestamps, char_mapping.as_deref());

        Ok(SynthesizeOutput {
            timestamps,
            duration_sec: result.duration_sec as f64,
        })
    }

    async fn shutdown(&self) -> Result<(), TtsError> {
        // In-process — drop the model so onnxruntime releases its sessions.
        // The next `warmup` will reload.
        let mut guard = self.loaded.write().await;
        *guard = None;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tts::supervisor::test_helpers::recording_emitter;

    fn engine_at(dir: &std::path::Path) -> SileroNativeEngine {
        let (emitter, _) = recording_emitter();
        SileroNativeEngine::new(dir.to_path_buf(), emitter)
    }

    #[tokio::test]
    async fn synthesize_without_bundle_is_bundle_not_installed() {
        let dir = tempfile::TempDir::new().unwrap();
        let engine = engine_at(dir.path());
        let err = engine
            .synthesize(
                "привет".to_string(),
                "xenia".to_string(),
                24000,
                dir.path().join("out.wav").to_string_lossy().into_owned(),
                None,
            )
            .await
            .unwrap_err();
        match err {
            TtsError::Ttsd { code, message } => {
                assert_eq!(code, "bundle_not_installed");
                // User-facing errors must be Russian.
                assert!(
                    message.chars().any(|c| matches!(c, 'А'..='я' | 'ё' | 'Ё')),
                    "message should be Russian: {message}"
                );
            }
            other => panic!("expected bundle_not_installed, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn warmup_without_bundle_emits_model_error() {
        let dir = tempfile::TempDir::new().unwrap();
        let (emitter, log) = recording_emitter();
        let engine = SileroNativeEngine::new(dir.path().to_path_buf(), emitter);
        engine.spawn_initial_warmup().await;
        // Wait for the detached warmup task to finish its (failing) load.
        for _ in 0..100 {
            if log.lock().unwrap().iter().any(|(n, _)| n == "model_error") {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        let log = log.lock().unwrap();
        let names: Vec<&str> = log.iter().map(|(n, _)| n.as_str()).collect();
        assert!(names.contains(&"model_loading"));
        assert!(names.contains(&"model_error"));
        assert!(!names.contains(&"model_loaded"));
        let (_, payload) = log.iter().find(|(n, _)| n == "model_error").unwrap();
        assert_eq!(payload["engine"], "silero_native");
    }

    #[test]
    fn map_timestamps_applies_char_mapping() {
        // Normalized "эй пи ай" (8 chars) came from original "API" (3 chars).
        let spans = vec![CharMappingEntry {
            norm_start: 0,
            norm_end: 8,
            orig_start: 0,
            orig_end: 3,
        }];
        let engine_ts = vec![
            silero_native::WordTimestamp {
                word: "эй".to_string(),
                start: 0.0,
                end: 0.3,
                original_pos: (0, 2),
            },
            silero_native::WordTimestamp {
                word: "пи".to_string(),
                start: 0.3,
                end: 0.6,
                original_pos: (3, 5),
            },
        ];
        let mapped = map_timestamps(engine_ts, Some(&spans));
        assert_eq!(mapped.len(), 2);
        for w in &mapped {
            assert_eq!(w.original_pos, (0, 3));
        }
        assert!((mapped[1].end - 0.6).abs() < 1e-6);
    }

    #[test]
    fn map_timestamps_without_mapping_keeps_engine_offsets() {
        let engine_ts = vec![silero_native::WordTimestamp {
            word: "мир".to_string(),
            start: 0.0,
            end: 0.5,
            original_pos: (7, 10),
        }];
        let mapped = map_timestamps(engine_ts, None);
        assert_eq!(mapped[0].original_pos, (7, 10));
    }

    #[test]
    fn kind_is_silero_native() {
        let dir = tempfile::TempDir::new().unwrap();
        assert_eq!(engine_at(dir.path()).kind(), EngineKind::SileroNative);
        assert_eq!(EngineKind::SileroNative.as_str(), "silero_native");
    }
}
