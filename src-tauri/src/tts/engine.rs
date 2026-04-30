//! Engine-agnostic interface for the TTS layer.
//!
//! Two concrete impls exist:
//! - [`crate::tts::supervisor::TtsSupervisor`] — Silero, runs as a Python
//!   `ttsd` sidecar that can die and respawn.
//! - [`crate::tts::piper::PiperEngine`] — Piper, runs in-process via the
//!   `piper-rs` ONNX wrapper.
//!
//! [`crate::state::AppState::tts`] holds a `Arc<dyn TtsEngine>` so the rest of
//! the codebase (commands, synthesis worker, tray) is engine-agnostic.

use std::sync::Arc;

use async_trait::async_trait;

use super::{CharMappingEntry, SynthesizeOutput, TtsError};

/// Identifies which engine implementation is currently active. Used for
/// logging, telemetry, and (future) UI events that need to differentiate
/// between Silero and Piper lifecycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EngineKind {
    Silero,
    Piper,
}

impl EngineKind {
    pub fn as_str(self) -> &'static str {
        match self {
            EngineKind::Silero => "silero",
            EngineKind::Piper => "piper",
        }
    }
}

/// Engine-agnostic TTS interface.
///
/// All methods are async and return [`TtsError`]. Implementations must be
/// `Send + Sync` so the engine can be shared via `Arc<dyn TtsEngine>` across
/// the Tokio runtime.
#[async_trait]
pub trait TtsEngine: Send + Sync {
    /// Identifies the concrete engine. Cheap; called for logs/events only.
    fn kind(&self) -> EngineKind;

    /// Load the model. Idempotent — calling it twice should be a no-op.
    /// Implementations should emit `model_loading` → `model_loaded` /
    /// `model_error` events themselves if they want UI feedback.
    async fn warmup(&self) -> Result<(), TtsError>;

    /// Run the warmup in the background, mirroring the
    /// `model_loading` → `model_loaded` / `model_error` lifecycle that the
    /// frontend expects on startup. Returns immediately; the warmup runs in
    /// a detached task.
    async fn spawn_initial_warmup(&self);

    /// Synthesize `text` and write the WAV file to `out_wav`.
    ///
    /// `voice` is the engine-specific voice id (`xenia` for Silero,
    /// `ruslan` for Piper, etc.). `char_mapping` is the optional pipeline
    /// bridge for mapping normalized text positions back to original-text
    /// offsets in the returned word timestamps.
    async fn synthesize(
        &self,
        text: String,
        voice: String,
        sample_rate: u32,
        out_wav: String,
        char_mapping: Option<Vec<CharMappingEntry>>,
    ) -> Result<SynthesizeOutput, TtsError>;

    /// Graceful shutdown. After this call the engine should release model
    /// memory / subprocess handles and refuse subsequent requests.
    async fn shutdown(&self) -> Result<(), TtsError>;
}

/// Convenience alias for a shared dynamic-dispatch engine handle.
pub type SharedEngine = Arc<dyn TtsEngine>;
