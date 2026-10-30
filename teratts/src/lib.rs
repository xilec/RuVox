//! In-process TeraTTSv2 engine.
//!
//! The crate owns the model bundle, the four ONNX sessions, the Russian text
//! frontend (tags, RUAccent dictionary stress, codepoint gate), sentence
//! chunking under the model's call limit, approximate word timestamps and the
//! 44.1 kHz WAV writer. The application adapter maps [`Engine`] to `TtsEngine`.
//!
//! Behavior is specified in `openspec/specs/teratts-engine/`.

pub mod bundle;
pub mod chunk;
pub mod engine;
pub mod error;
pub mod indexer;
pub mod npy;
pub mod styles;
pub mod synth;
pub mod text;
pub mod timestamps;

pub use engine::{Engine, ModelInfo, Synthesis, default_threads};
pub use error::{Error, Result};
pub use synth::SAMPLE_RATE;
