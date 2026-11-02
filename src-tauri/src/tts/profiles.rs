//! Per-engine text profiles now live in the `tts-normalize` crate. This module is a
//! re-export plus the drift guard that needs both the profiles and the engine
//! crates in one place.

pub use tts_normalize::profiles::*;

#[cfg(test)]
mod tests {
    use super::*;

    /// The profile owns the number; the engine crate's constant is the value it
    /// must agree with, so drift fails here where both are visible.
    #[test]
    fn silero_chunk_limits_match_their_engines() {
        let expected = silero_native::chunking::MAX_CHUNK_SIZE;
        assert_eq!(SILERO_NATIVE_PROFILE.chunk.char_limit(), Some(expected));
        assert_eq!(SILERO_PROFILE.chunk.char_limit(), Some(expected));
    }
}
