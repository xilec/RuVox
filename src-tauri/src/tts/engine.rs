//! Engine-agnostic interface for the TTS layer.
//!
//! Three concrete impls exist:
//! - [`crate::tts::supervisor::TtsSupervisor`] — Silero, runs as a Python
//!   `ttsd` sidecar that can die and respawn.
//! - [`crate::tts::piper::PiperEngine`] — Piper, runs in-process via the
//!   `piper-rs` ONNX wrapper.
//! - [`crate::tts::silero_native::SileroNativeEngine`] — Silero v5, runs
//!   in-process via the `silero-native` crate on ONNX Runtime.
//!
//! [`crate::state::AppState::tts`] holds a `Arc<dyn TtsEngine>` so the rest of
//! the codebase (commands, synthesis worker, tray) is engine-agnostic.

use std::sync::Arc;

use async_trait::async_trait;

use super::{CharMappingEntry, ModelInfo, SynthesizeOutput, TtsError};

/// Identifies which engine implementation is currently active. Used for
/// logging, telemetry, and (future) UI events that need to differentiate
/// between Silero and Piper lifecycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EngineKind {
    Silero,
    Piper,
    SileroNative,
}

impl EngineKind {
    pub fn as_str(self) -> &'static str {
        match self {
            EngineKind::Silero => "silero",
            EngineKind::Piper => "piper",
            EngineKind::SileroNative => "silero_native",
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
    /// Identifies the concrete engine. Cheap. Called for logs/events and for
    /// engine-aware decisions (the input length limit gates on it), so it must
    /// reflect the actually-active engine at call time.
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

    /// Identity of the loaded model/voice, when the engine can report it
    /// cheaply (no file hashing on the synthesis path). `voice` is the same
    /// engine-specific voice id `synthesize` receives; engines whose model
    /// identity is voice-independent (silero-native bundle) ignore it. `None`
    /// means the engine cannot report an identity — the generation-params
    /// snapshot renders it as absent. Default impl: `None` (engines with no
    /// exposed model identity, and test stubs).
    ///
    /// Async so engine wrappers (the switcher) can delegate to the engine
    /// that is current at call time; concrete engines resolve theirs without
    /// blocking awaits.
    async fn model_info(&self, voice: &str) -> Option<ModelInfo> {
        let _ = voice;
        None
    }

    /// Forcibly terminate the engine's current in-flight work (for Silero,
    /// the ttsd subprocess). Default is a no-op: engines with no external
    /// subprocess (Piper, test stubs) have nothing to kill. Called by
    /// `cancel_synthesis` when the cancelled entry had entered the TTS stage.
    async fn kill_current(&self) {}

    /// Graceful shutdown. After this call the engine should release model
    /// memory / subprocess handles and refuse subsequent requests.
    async fn shutdown(&self) -> Result<(), TtsError>;
}

/// Convenience alias for a shared dynamic-dispatch engine handle.
pub type SharedEngine = Arc<dyn TtsEngine>;
