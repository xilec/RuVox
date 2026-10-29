//! Shared contract between the text pipeline and the TTS engines.
//!
//! Two independent cargo workspaces need the same two things: the data-only
//! description of what an engine can read ([`profile::TextProfile`]) and the
//! splitter that turns normalized text into bounded synthesis calls
//! ([`chunk::split_with_limit`]). They live below both — the application
//! (`src-tauri`) depends on the engine crate (`silero-native`), so a shared
//! home cannot be either of them.
//!
//! Nothing here knows about a concrete engine: mapping an engine identity to a
//! profile is the application's job.

pub mod chunk;
pub mod profile;

pub use chunk::split_with_limit;
pub use profile::{
    Charset, Chunk, Digits, Limit, Markup, Pauses, ProfileError, Reading, Route, Stress,
    TextProfile, Units,
};
