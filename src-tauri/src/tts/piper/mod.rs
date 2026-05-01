//! Piper TTS engine — in-process ONNX inference via the `piper-rs` crate.
//!
//! See `tmp/issue_42_piper_plan.md` and `tmp/piper_rs_spike/findings.md` for
//! the design rationale (native runtime, no Python sidecar, voices on demand).

pub mod catalog;
pub mod download;
pub mod engine;
pub mod timestamps;

pub use engine::PiperEngine;
