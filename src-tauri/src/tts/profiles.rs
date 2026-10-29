//! Static text profiles, one per engine kind — the single home for every
//! per-engine text decision.
//!
//! A profile states what the engine reads itself; the pipeline reads it to
//! decide what to rewrite, and the engine adapter reads it to decide what to do
//! with the fragments the pipeline left alone. Values that only one engine
//! knows (chunk limit, paragraph pause) live here rather than as constants in
//! the engine module, so there is exactly one place to change them.

use super::EngineKind;
use tts_contract::{
    Charset, Chunk, Digits, Limit, Markup, Pauses, Reading, Route, Stress, TextProfile, Units,
};

/// Silero, both the `ttsd` sidecar and the in-process crate: Russian only, so
/// every Latin fragment is transliterated; digits and units are expanded by the
/// pipeline; stress is applied by the engine's own frontend; codepoints outside
/// the model alphabet are dropped, which is why the alphabet is declared here.
pub static SILERO_PROFILE: TextProfile = TextProfile {
    languages: &["ru"],
    route: Route {
        latin_prose: Reading::Narrate,
        identifier: Reading::Narrate,
    },
    digits: Digits::Expanded,
    units: Units::Expanded,
    stress: Stress::Auto,
    charset: Charset::ModelAlphabet,
    chunk: Chunk {
        limit: Limit::Chars(900),
        pauses: Pauses {
            paragraph_ms: 0,
            segment_ms: 0,
        },
    },
    markup: Markup::None,
};

/// The in-process Silero engine. Identical to the sidecar profile: same model
/// family, same frontend, same limits. Kept as a distinct value so the two can
/// diverge (the native frontend ports the Python one) without touching callers.
pub static SILERO_NATIVE_PROFILE: TextProfile = TextProfile {
    languages: &["ru"],
    route: Route {
        latin_prose: Reading::Narrate,
        identifier: Reading::Narrate,
    },
    digits: Digits::Expanded,
    units: Units::Expanded,
    stress: Stress::Auto,
    charset: Charset::ModelAlphabet,
    chunk: Chunk {
        limit: Limit::Chars(900),
        pauses: Pauses {
            paragraph_ms: 0,
            segment_ms: 0,
        },
    },
    markup: Markup::None,
};

/// Piper: Russian voice set, eSpeak-ng frontend, no stress control of its own.
/// The chunk limit is a memory bound rather than a model rule (see
/// `tts::piper::engine`: VITS activation memory grows quadratically, measured
/// at ~0.5 GB per 300-codepoint chunk), and the paragraph pause exists because
/// eSpeak-ng reads a blank line as a plain space.
pub static PIPER_PROFILE: TextProfile = TextProfile {
    languages: &["ru"],
    route: Route {
        latin_prose: Reading::Narrate,
        identifier: Reading::Narrate,
    },
    digits: Digits::Expanded,
    units: Units::Expanded,
    stress: Stress::None,
    charset: Charset::Any,
    chunk: Chunk {
        limit: Limit::Chars(500),
        pauses: Pauses {
            paragraph_ms: 450,
            segment_ms: 0,
        },
    },
    markup: Markup::None,
};

/// A profile for an engine that reads Latin, digits and units natively.
///
/// Deliberately absent from [`profile_for`]: no engine ships with it. It exists
/// so tests and the golden harness can exercise every gated path — a profile is
/// data, so proving a gated block works needs no engine to implement it.
pub static NATIVE_EN_PROFILE: TextProfile = TextProfile {
    languages: &["ru", "en"],
    route: Route {
        latin_prose: Reading::Native,
        identifier: Reading::Native,
    },
    digits: Digits::Native,
    units: Units::Native,
    stress: Stress::Manual,
    charset: Charset::Any,
    chunk: Chunk {
        limit: Limit::Secs(25),
        pauses: Pauses {
            paragraph_ms: 0,
            segment_ms: 200,
        },
    },
    markup: Markup::RuEnTags,
};

/// The profile of an engine kind.
///
/// Total over [`EngineKind::ALL`]: the registry test asserts that every variant
/// resolves, so adding an engine without a profile fails a test instead of
/// falling back to a guess.
pub fn profile_for(kind: EngineKind) -> &'static TextProfile {
    match kind {
        EngineKind::Silero => &SILERO_PROFILE,
        EngineKind::Piper => &PIPER_PROFILE,
        EngineKind::SileroNative => &SILERO_NATIVE_PROFILE,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_en_profile_satisfies_its_invariants() {
        assert_eq!(NATIVE_EN_PROFILE.validate(), Ok(()));
    }

    #[test]
    fn native_en_profile_is_not_reachable_from_the_registry() {
        for kind in EngineKind::ALL {
            assert_ne!(
                profile_for(*kind),
                &NATIVE_EN_PROFILE,
                "no shipping engine may use the test-only profile"
            );
        }
    }

    #[test]
    fn every_engine_kind_has_a_profile() {
        for kind in EngineKind::ALL {
            let profile = profile_for(*kind);
            assert_eq!(
                profile.validate(),
                Ok(()),
                "profile of {kind:?} violates its invariants"
            );
        }
    }

    #[test]
    fn every_engine_kind_profile_differs_only_where_engines_differ() {
        assert_eq!(
            SILERO_PROFILE, SILERO_NATIVE_PROFILE,
            "the two Silero engines share a model family and a frontend"
        );
        assert_ne!(
            PIPER_PROFILE.chunk, SILERO_PROFILE.chunk,
            "Piper's memory bound differs from Silero's model bound"
        );
        assert_ne!(
            PIPER_PROFILE.stress, SILERO_PROFILE.stress,
            "Piper has no stress control of its own"
        );
    }

    #[test]
    fn no_current_engine_routes_latin_natively() {
        for kind in EngineKind::ALL {
            assert!(
                !profile_for(*kind).route.any_native(),
                "{kind:?} cannot read Latin yet, so it must not route it Native"
            );
        }
    }

    #[test]
    fn piper_chunk_limit_is_the_measured_memory_bound() {
        assert_eq!(PIPER_PROFILE.chunk.char_limit(), Some(500));
        assert_eq!(PIPER_PROFILE.chunk.paragraph_pause_sec(), 0.45);
    }
}
