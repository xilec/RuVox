//! TeraTTSv2 engine: in-process ONNX inference through the `teratts` crate.

pub mod download;
pub mod engine;

pub use engine::TeraTtsEngine;
