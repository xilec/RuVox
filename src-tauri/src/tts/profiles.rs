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

/// The `ttsd` sidecar (Silero v5 through Python): Russian only, so every Latin
/// fragment is transliterated; digits and units are expanded by the pipeline;
/// codepoints outside the model alphabet are dropped, which is why the alphabet
/// is declared here. Stress is [`Stress::None`]: the Python frontend accentuates
/// on its own, but whether it honours an explicit `+` is not verified yet, so
/// the dictionary mark is stripped until the probe (#311) says otherwise.
pub static SILERO_PROFILE: TextProfile = TextProfile {
    languages: &["ru"],
    route: Route {
        latin_prose: Reading::Narrate,
        identifier: Reading::Narrate,
        code_block: Reading::Narrate,
    },
    digits: Digits::Expanded,
    units: Units::Expanded,
    stress: Stress::None,
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

/// The in-process Silero engine. Same model family and limits as the sidecar
/// profile, but the accentor is our code: it predicts stress and keeps an
/// explicit `+` mark where the user put it, which is [`Stress::Auto`]. Kept as a
/// distinct value so the two can diverge further without touching callers.
pub static SILERO_NATIVE_PROFILE: TextProfile = TextProfile {
    languages: &["ru"],
    route: Route {
        latin_prose: Reading::Narrate,
        identifier: Reading::Narrate,
        code_block: Reading::Narrate,
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
///
/// The #311 probe accepted a raw code block read by the Russian voice, while
/// transliteration still sounds better for prose, so only the code-block route
/// is `Native`. That route needs a language to annotate its span with, hence
/// `en` in the language list; the list means "the engine accepts the
/// language", not "the pipeline routes every fragment to it".
pub static PIPER_PROFILE: TextProfile = TextProfile {
    languages: &["ru", "en"],
    route: Route {
        latin_prose: Reading::Narrate,
        identifier: Reading::Narrate,
        code_block: Reading::Native,
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

/// TeraTTSv2: Russian and English in one engine. Latin prose, code identifiers
/// and fenced code blocks stay for the engine (the #311 probe accepted all
/// three), while numbers, units, URLs and emails stay with the pipeline's
/// Russian narration. The engine owns stress: RUAccent dictionary marks fill the
/// gaps while an explicit `+` from the user dictionary always wins, which is
/// [`Stress::Manual`]. The 20-second limit is the model's decoder context;
/// 250 ms and 200 ms separate paragraphs and chunks. `RuEnTags` is the markup
/// the adapter wraps around each language segment; the profile never emits
/// tags.
pub static TERATTS_PROFILE: TextProfile = TextProfile {
    languages: &["ru", "en"],
    route: Route {
        latin_prose: Reading::Native,
        identifier: Reading::Native,
        code_block: Reading::Native,
    },
    digits: Digits::Expanded,
    units: Units::Expanded,
    stress: Stress::Manual,
    charset: Charset::ModelAlphabet,
    chunk: Chunk {
        limit: Limit::Secs(20),
        pauses: Pauses {
            paragraph_ms: 250,
            segment_ms: 200,
        },
    },
    markup: Markup::RuEnTags,
};

/// A profile for an engine that reads Latin, digits, units and code blocks
/// natively.
///
/// Deliberately absent from [`profile_for`]: no engine ships with it. It exists
/// so tests and the golden harness can exercise every gated path — a profile is
/// data, so proving a gated block works needs no engine to implement it. It also
/// declares [`Stress::Manual`] so the explicit-only stress path is covered.
pub static NATIVE_EN_PROFILE: TextProfile = TextProfile {
    languages: &["ru", "en"],
    route: Route {
        latin_prose: Reading::Native,
        identifier: Reading::Native,
        code_block: Reading::Native,
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
        EngineKind::TeraTts => &TERATTS_PROFILE,
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
        // Stress is the only intended difference between the two Silero
        // profiles: everything else (route, digits, units, charset, chunk,
        // markup) stays in lockstep, so a silent drift in one of them fails
        // here rather than in a fixture nobody reads.
        assert_eq!(
            TextProfile {
                stress: Stress::None,
                ..SILERO_NATIVE_PROFILE
            },
            SILERO_PROFILE,
            "the two Silero engines differ only in stress"
        );
        assert_ne!(
            PIPER_PROFILE.chunk, SILERO_PROFILE.chunk,
            "Piper's memory bound differs from Silero's model bound"
        );
        assert_ne!(
            PIPER_PROFILE.charset, SILERO_PROFILE.charset,
            "Piper is sanitized by its adapter while Silero is bound to its model alphabet"
        );
        assert_eq!(
            SILERO_NATIVE_PROFILE.stress,
            Stress::Auto,
            "the native accentor honours an explicit mark, so the dictionary mark reaches it"
        );
        assert_eq!(
            SILERO_PROFILE.stress,
            Stress::None,
            "the sidecar's handling of an explicit mark is unverified (#311)"
        );
        assert_eq!(
            PIPER_PROFILE.stress,
            Stress::None,
            "eSpeak reads '+' as 'plus', so the mark must be stripped"
        );
        assert_eq!(
            NATIVE_EN_PROFILE.stress,
            Stress::Manual,
            "the test profile covers the explicit-only path"
        );
    }

    #[test]
    fn routes_follow_the_probe_verdicts() {
        // #311: TeraTTS reads prose, identifiers and code natively; Piper reads
        // only code natively; the Silero model family reads none of them.
        for kind in EngineKind::ALL {
            let profile = profile_for(*kind);
            match kind {
                EngineKind::TeraTts => {
                    assert_eq!(profile.route.latin_prose, Reading::Native);
                    assert_eq!(profile.route.identifier, Reading::Native);
                    assert_eq!(profile.route.code_block, Reading::Native);
                    assert!(profile.languages.contains(&"en"));
                }
                EngineKind::Piper => {
                    assert_eq!(profile.route.latin_prose, Reading::Narrate);
                    assert_eq!(profile.route.identifier, Reading::Narrate);
                    assert_eq!(profile.route.code_block, Reading::Native);
                    assert!(profile.languages.contains(&"en"));
                }
                EngineKind::Silero | EngineKind::SileroNative => {
                    assert!(
                        !profile.route.any_native(),
                        "{kind:?} cannot read Latin or code, so it must not route either Native"
                    );
                }
            }
        }
    }

    #[test]
    fn silero_chunk_limits_match_their_engines() {
        // The profile owns the number; the engine crate's constant and the ttsd
        // default are the values it must agree with, so drift fails here where
        // every engine is visible at once.
        let expected = silero_native::chunking::MAX_CHUNK_SIZE;
        assert_eq!(SILERO_NATIVE_PROFILE.chunk.char_limit(), Some(expected));
        assert_eq!(SILERO_PROFILE.chunk.char_limit(), Some(expected));
    }

    #[test]
    fn teratts_profile_declares_its_model_limits() {
        assert_eq!(TERATTS_PROFILE.chunk.limit, Limit::Secs(20));
        assert_eq!(TERATTS_PROFILE.chunk.paragraph_pause_sec(), 0.25);
        assert_eq!(TERATTS_PROFILE.chunk.segment_pause_sec(), 0.2);
        assert_eq!(TERATTS_PROFILE.stress, Stress::Manual);
        assert_eq!(TERATTS_PROFILE.markup, Markup::RuEnTags);
        assert_eq!(TERATTS_PROFILE.charset, Charset::ModelAlphabet);
    }

    #[test]
    fn piper_chunk_limit_is_the_measured_memory_bound() {
        assert_eq!(PIPER_PROFILE.chunk.char_limit(), Some(500));
        assert_eq!(PIPER_PROFILE.chunk.paragraph_pause_sec(), 0.45);
    }
}
