//! Text normalization for TTS engines: the rules that turn source text into
//! exactly what an engine will read.
//!
//! Owns the normalization pipeline (numbers, units, dates, URLs, code, English
//! reading, stress), the user dictionary that overrides the built-in tables,
//! the per-engine text profiles that state what each engine reads itself, and
//! the engine identity both are keyed by.
//!
//! Scope: text in, text out. Synthesis, timings, bundle loading and audio belong
//! to the engine crates; nothing in that list fits this name, which is the
//! point of it. The dictionary module also owns the user dictionary's TOML file
//! format, so a consumer reads exactly the file the app writes — that is the
//! one place this crate touches the filesystem.
//!
//! Depends only on [`tts_contract`] — no ONNX Runtime, no engine crate — so
//! every consumer that speaks the contract gets exactly the behaviour the
//! others ship.

pub mod dictionary;
mod engine_kind;
pub mod pipeline;
pub mod profiles;

pub use engine_kind::EngineKind;
