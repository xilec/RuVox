//! In-process Silero TTS v5 engine running on ONNX Runtime.
//!
//! Replaces the Python `ttsd` sidecar for Silero synthesis: the model is a
//! pre-exported ONNX bundle (see `export/`) loaded from a local directory,
//! and the text frontend (accentor, homograph solver) is a Rust port of the
//! upstream package code. See `docs/architecture.md` for the pipeline map.

pub mod bundle;
pub mod error;
pub mod frontend;

pub use error::{EngineError, Result};
