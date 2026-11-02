//! The normalization pipeline now lives in the `tts-normalize` crate so the DSH
//! sidecar and this application normalize text with the same code. This module
//! is a re-export only: add behaviour there, not here.

pub use tts_normalize::pipeline::*;
