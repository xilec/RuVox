//! Compatibility shim: the splitter now lives in the `tts-contract` crate.
//!
//! Callers keep their `crate::tts::chunking::…` path while the engines are
//! still wired to the old signature; the module disappears when Piper and
//! `silero-native` read their limit from the text profile instead.

pub use tts_contract::chunk::split_with_limit;
