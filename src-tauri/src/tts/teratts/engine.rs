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
use tracing::{info, warn};

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
        let result =
            tokio::task::spawn_blocking(move || engine.synthesize(&text, &voice, &policy, &cancel))
                .await
                .map_err(|e| TtsError::Ttsd {
                    code: "synthesis_failed".to_string(),
                    message: format!("задача синтеза TeraTTS прервана: {e}"),
                })?
                .map_err(map_error)?;

        let out_path = PathBuf::from(&out_wav);
        if let Some(parent) = out_path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(TtsError::Ipc)?;
        }
        tokio::fs::write(&out_path, &result.wav)
            .await
            .map_err(TtsError::Ipc)?;

        Ok(SynthesizeOutput {
            timestamps: map_timestamps(result.timestamps, char_mapping.as_deref()),
            duration_sec: result.duration_sec,
        })
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
