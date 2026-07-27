//! In-process Silero v5 engine backed by the `silero-native` crate, plus the
//! on-demand model bundle downloader.
//!
//! Layout mirrors [`crate::tts::piper`]: `engine` holds the
//! [`TtsEngine`](crate::tts::engine::TtsEngine) implementation, `download`
//! fetches the model bundle from GitHub Releases.

pub mod engine;

pub use engine::SileroNativeEngine;
