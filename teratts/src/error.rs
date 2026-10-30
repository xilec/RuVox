//! Typed engine errors.
//!
//! The application adapter maps these to the user-facing error codes; the
//! messages here are developer-facing (English) and carry the concrete file or
//! tensor that failed.

use std::path::PathBuf;

/// Everything the engine can fail with. Variants are deliberately narrow: the
/// adapter needs a stable code per variant, and a catch-all `String` would lose
/// the difference between "download the bundle" and "the model is broken".
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The bundle directory exists but has no manifest, or does not exist.
    #[error("model bundle is not installed at {}", .0.display())]
    BundleNotInstalled(PathBuf),
    /// A manifest entry is missing, truncated, or has the wrong checksum.
    #[error("model bundle is corrupt: {0}")]
    BundleCorrupt(String),
    /// The manifest is readable but its format version is not supported.
    #[error("model bundle format {found} is not supported (expected {expected})")]
    BundleIncompatible { found: u32, expected: u32 },
    /// The requested voice id is not in the bundle.
    #[error("unknown voice {voice:?}; available: {available}")]
    VoiceUnknown { voice: String, available: String },
    /// The text cannot be represented by the model (tags, empty result).
    #[error("text cannot be synthesized: {0}")]
    InvalidText(String),
    /// An ONNX Runtime call or a shape check failed.
    #[error("synthesis failed: {0}")]
    Synthesis(String),
    /// The caller asked to stop between chunks.
    #[error("synthesis cancelled")]
    Cancelled,
}

/// Crate result alias.
pub type Result<T> = std::result::Result<T, Error>;
