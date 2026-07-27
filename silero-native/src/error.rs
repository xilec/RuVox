//! Domain errors for the silero-native engine.

/// Errors surfaced by the silero-native engine.
#[derive(Debug, thiserror::Error)]
pub enum EngineError {
    /// The model bundle is missing, malformed, or fails checksum
    /// verification.
    #[error("model bundle error: {0}")]
    Bundle(String),

    /// The caller passed invalid input (empty text, unknown speaker,
    /// unsupported sample rate).
    #[error("bad input: {0}")]
    BadInput(String),

    /// ONNX Runtime failed during model loading or inference.
    #[error("onnx runtime error: {0}")]
    Ort(#[from] ort::Error),

    /// Synthesis failed inside the engine.
    #[error("synthesis failed: {0}")]
    Synthesis(String),

    /// Anything else (I/O, manifest JSON, etc.).
    #[error("internal error: {0}")]
    Internal(String),
}

/// Convenient alias for engine results.
pub type Result<T> = std::result::Result<T, EngineError>;
