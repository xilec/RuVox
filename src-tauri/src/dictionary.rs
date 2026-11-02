//! The user dictionary now lives in the `tts-normalize` crate (it shares
//! [`EngineKind`](crate::tts::EngineKind) scopes and the pipeline's lookup
//! tables). This module is a re-export only.

pub use tts_normalize::dictionary::*;
