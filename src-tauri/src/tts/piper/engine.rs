//! `piper-rs`-backed [`TtsEngine`] implementation.
//!
//! Loads a `.onnx` voice + `.onnx.json` config from
//! `<voices_dir>/<voice_id>/`. Inference runs in `spawn_blocking` because
//! piper-rs's synthesis is synchronous CPU work.
//!
//! ## Why a `Mutex<Piper>` instead of a shared read-only handle
//!
//! piper-rs 0.2.0's `Piper::create` takes `&mut self` (the ONNX Runtime
//! session is mutated during inference), so concurrent synthesis calls for
//! the same loaded voice must serialize on a lock. `parking_lot::Mutex` is
//! used (not `tokio::sync::Mutex`) because the lock is only ever held inside
//! a `spawn_blocking` closure — a synchronous context, never across an
//! `.await`.
//!
//! ## Failure mapping
//! - voice files missing → `TtsError::Ttsd { code: "voice_not_installed", … }`
//! - config JSON parse / load failure → `TtsError::Ttsd { code: "piper_load_failed", … }`
//! - phonemizer / ONNX inference failure → `TtsError::Ttsd { code: "piper_*_failed", … }`
//! - WAV write failure → `TtsError::Ipc(io::Error)`

use std::path::{Path, PathBuf};
use std::sync::Arc;

use async_trait::async_trait;
use parking_lot::Mutex;
use piper_rs::{ModelConfig, Piper};
use serde_json::json;
use tokio::sync::RwLock;
use tracing::{info, warn};

/// VITS `length_scale` applied at synthesis time. The Piper voice configs
/// ship `length_scale: 1.0` which sounds slow on Russian voices; 0.8 cuts
/// audio length by ~20% while keeping the natural prosody (we change the
/// model's own duration prediction, not a post-process resample).
const PIPER_LENGTH_SCALE: f32 = 0.8;

use super::timestamps::estimate_timestamps_single_chunk;
use crate::tts::engine::{EngineKind, TtsEngine};
use crate::tts::supervisor::Emitter;
use crate::tts::{CharMappingEntry, SynthesizeOutput, TtsError};

type LoadedSlot = Arc<RwLock<Option<LoadedVoice>>>;

/// In-process Piper engine.
pub struct PiperEngine {
    voices_dir: PathBuf,
    /// Currently loaded voice — behind `Arc<RwLock<...>>` so the detached
    /// `spawn_initial_warmup` task can install the loaded voice without
    /// holding a borrow of `&self`.
    loaded: LoadedSlot,
    /// Default voice id, used by `warmup` and as a fallback.
    default_voice: String,
    /// Frontend event emitter.
    emitter: Emitter,
}

struct LoadedVoice {
    id: String,
    piper: Arc<Mutex<Piper>>,
    /// Per-voice tuning read from the `.onnx.json` config, kept alongside
    /// our own `PIPER_LENGTH_SCALE` override at synthesis time.
    noise_scale: f32,
    noise_w: f32,
}

/// Lightweight handle returned from `ensure_loaded` — keeps the model alive
/// while a synthesize call runs without holding the engine's RwLock.
struct LoadedHandle {
    piper: Arc<Mutex<Piper>>,
    noise_scale: f32,
    noise_w: f32,
}

impl PiperEngine {
    /// Build a new engine. No I/O — the model is loaded lazily on first
    /// `warmup` / `synthesize`.
    pub fn new(voices_dir: PathBuf, default_voice: String, emitter: Emitter) -> Self {
        Self {
            voices_dir,
            loaded: Arc::new(RwLock::new(None)),
            default_voice,
            emitter,
        }
    }

    /// Resolve `<voices_dir>/<voice_id>/ru_RU-<voice_id>-medium.onnx.json`.
    /// Matches the rhasspy file naming convention.
    fn config_path_for(voices_dir: &Path, voice_id: &str) -> PathBuf {
        voices_dir
            .join(voice_id)
            .join(format!("ru_RU-{voice_id}-medium.onnx.json"))
    }

    /// Load (or reload, if voice changed) the Piper model.
    async fn ensure_loaded(&self, voice_id: &str) -> Result<LoadedHandle, TtsError> {
        // Fast path — voice already loaded.
        {
            let guard = self.loaded.read().await;
            if let Some(loaded) = guard.as_ref() {
                if loaded.id == voice_id {
                    return Ok(LoadedHandle {
                        piper: Arc::clone(&loaded.piper),
                        noise_scale: loaded.noise_scale,
                        noise_w: loaded.noise_w,
                    });
                }
            }
        }

        let config_path = Self::config_path_for(&self.voices_dir, voice_id);
        if !config_path.exists() {
            return Err(TtsError::Ttsd {
                code: "voice_not_installed".to_string(),
                message: format!(
                    "Piper voice \"{voice_id}\" не установлен ({}). \
                     Загрузка по требованию будет добавлена в Phase 4.",
                    config_path.display()
                ),
            });
        }

        let voice_id_owned = voice_id.to_string();
        let cfg = config_path.clone();
        let (piper, noise_scale, noise_w, sample_rate) =
            tokio::task::spawn_blocking(move || load_voice_blocking(&cfg))
                .await
                .map_err(|e| TtsError::Ttsd {
                    code: "piper_load_panic".to_string(),
                    message: format!("piper-rs load task panicked: {e}"),
                })??;

        let piper = Arc::new(Mutex::new(piper));
        let mut guard = self.loaded.write().await;
        *guard = Some(LoadedVoice {
            id: voice_id_owned,
            piper: Arc::clone(&piper),
            noise_scale,
            noise_w,
        });
        info!(target: "tts::piper", "loaded voice \"{voice_id}\" (sr={sample_rate})");
        Ok(LoadedHandle {
            piper,
            noise_scale,
            noise_w,
        })
    }
}

#[async_trait]
impl TtsEngine for PiperEngine {
    fn kind(&self) -> EngineKind {
        EngineKind::Piper
    }

    async fn warmup(&self) -> Result<(), TtsError> {
        let _ = self.ensure_loaded(&self.default_voice).await?;
        Ok(())
    }

    async fn spawn_initial_warmup(&self) {
        let voices_dir = self.voices_dir.clone();
        let voice_id = self.default_voice.clone();
        let emitter = Arc::clone(&self.emitter);
        let slot = Arc::clone(&self.loaded);

        tokio::spawn(async move {
            (emitter)("model_loading", json!({ "engine": "piper" }));

            let config_path = PiperEngine::config_path_for(&voices_dir, &voice_id);
            if !config_path.exists() {
                let msg = format!(
                    "Piper voice \"{voice_id}\" не установлен ({}). \
                     Загрузка по требованию будет добавлена в Phase 4.",
                    config_path.display()
                );
                warn!(target: "tts::piper", "warmup skipped: {msg}");
                (emitter)("model_error", json!({ "engine": "piper", "message": msg }));
                return;
            }

            let cfg = config_path.clone();
            let load_result = tokio::task::spawn_blocking(move || load_voice_blocking(&cfg)).await;

            match load_result {
                Ok(Ok((piper, noise_scale, noise_w, sample_rate))) => {
                    let piper = Arc::new(Mutex::new(piper));
                    let mut guard = slot.write().await;
                    *guard = Some(LoadedVoice {
                        id: voice_id,
                        piper,
                        noise_scale,
                        noise_w,
                    });
                    info!(target: "tts::piper", "warmup complete (sr={sample_rate})");
                    (emitter)("model_loaded", json!({ "engine": "piper" }));
                }
                Ok(Err(e)) => {
                    warn!(target: "tts::piper", "warmup load failed: {e}");
                    (emitter)(
                        "model_error",
                        json!({ "engine": "piper", "message": e.to_string() }),
                    );
                }
                Err(e) => {
                    warn!(target: "tts::piper", "warmup task panicked: {e}");
                    (emitter)(
                        "model_error",
                        json!({ "engine": "piper", "message": e.to_string() }),
                    );
                }
            }
        });
    }

    async fn synthesize(
        &self,
        text: String,
        voice: String,
        _sample_rate: u32, // Piper output is fixed by the voice; mpv handles SR mismatch.
        out_wav: String,
        char_mapping: Option<Vec<CharMappingEntry>>,
    ) -> Result<SynthesizeOutput, TtsError> {
        let handle = self.ensure_loaded(&voice).await?;
        let piper = Arc::clone(&handle.piper);
        let noise_scale = handle.noise_scale;
        let noise_w = handle.noise_w;
        let text_for_blocking = text.clone();

        let (samples, sample_rate) =
            tokio::task::spawn_blocking(move || -> Result<(Vec<f32>, u32), TtsError> {
                piper
                    .lock()
                    .create(
                        &text_for_blocking,
                        false,
                        None,
                        Some(PIPER_LENGTH_SCALE),
                        Some(noise_scale),
                        Some(noise_w),
                    )
                    .map_err(|e| TtsError::Ttsd {
                        code: "piper_synthesis_failed".to_string(),
                        message: format!("Piper::create failed: {e}"),
                    })
            })
            .await
            .map_err(|e| TtsError::Ttsd {
                code: "piper_synthesis_panic".to_string(),
                message: format!("synthesis task panicked: {e}"),
            })??;

        let duration_sec = if sample_rate == 0 {
            0.0
        } else {
            samples.len() as f64 / sample_rate as f64
        };

        let out_path = PathBuf::from(&out_wav);
        if let Some(parent) = out_path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(TtsError::Ipc)?;
        }
        let samples_for_write = samples;
        let out_wav_for_write = out_wav.clone();
        tokio::task::spawn_blocking(move || {
            write_wav_i16(&out_wav_for_write, sample_rate, &samples_for_write)
        })
        .await
        .map_err(|e| {
            TtsError::Ipc(std::io::Error::other(format!(
                "wav write task panicked: {e}"
            )))
        })??;

        let timestamps =
            estimate_timestamps_single_chunk(&text, duration_sec, char_mapping.as_deref());

        Ok(SynthesizeOutput {
            timestamps,
            duration_sec,
        })
    }

    async fn shutdown(&self) -> Result<(), TtsError> {
        // In-process — drop the model so onnxruntime releases its session.
        // The next `warmup` will reload.
        let mut guard = self.loaded.write().await;
        *guard = None;
        Ok(())
    }
}

/// Synchronous helper for the blocking thread: load the model and read the
/// per-voice tuning (`noise_scale`, `noise_w`, `sample_rate`) from the
/// `.onnx.json` config that ships alongside the `.onnx` model.
fn load_voice_blocking(config_path: &Path) -> Result<(Piper, f32, f32, u32), TtsError> {
    // `Piper::new` wants the `.onnx` model path and the `.onnx.json` config
    // path separately; rhasspy's naming convention is the config path with
    // its trailing `.json` extension stripped.
    let model_path = config_path.with_extension("");

    let cfg_text = std::fs::read_to_string(config_path).map_err(|e| TtsError::Ttsd {
        code: "piper_load_failed".to_string(),
        message: format!("failed to read piper config {}: {e}", config_path.display()),
    })?;
    let cfg: ModelConfig = serde_json::from_str(&cfg_text).map_err(|e| TtsError::Ttsd {
        code: "piper_load_failed".to_string(),
        message: format!("failed to parse piper config: {e}"),
    })?;

    let piper = Piper::new(&model_path, config_path).map_err(|e| TtsError::Ttsd {
        code: "piper_load_failed".to_string(),
        message: format!("piper-rs Piper::new failed: {e}"),
    })?;

    Ok((
        piper,
        cfg.inference.noise_scale,
        cfg.inference.noise_w,
        cfg.audio.sample_rate,
    ))
}

/// Write `samples` (f32 in -1.0..1.0) as a mono i16 PCM WAV at `sample_rate`.
fn write_wav_i16(path: &str, sample_rate: u32, samples: &[f32]) -> Result<(), TtsError> {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create(path, spec).map_err(map_hound_err)?;
    for s in samples {
        let clipped = s.clamp(-1.0, 1.0);
        let i = (clipped * 32767.0) as i16;
        writer.write_sample(i).map_err(map_hound_err)?;
    }
    writer.finalize().map_err(map_hound_err)
}

fn map_hound_err(e: hound::Error) -> TtsError {
    match e {
        hound::Error::IoError(io) => TtsError::Ipc(io),
        other => TtsError::Ipc(std::io::Error::other(other.to_string())),
    }
}
