//! Data-only description of what a TTS engine can read.
//!
//! The profile is the whole per-engine contract for text preparation: the
//! pipeline reads it to decide which fragments it must rewrite, and the engine
//! adapter reads it to decide what to do with the fragments the pipeline left
//! alone. It holds data only — no callbacks, no trait objects — so it can be
//! shared by two independent crates, copied, logged and asserted in tests.

use std::borrow::Cow;
use std::fmt;

/// What happens to one kind of fragment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reading {
    /// The pipeline rewrites the fragment into Russian — the compensation a
    /// Russian-only engine needs.
    Narrate,
    /// The fragment survives untouched and the adapter reads it in its own
    /// language.
    Native,
}

/// Per-kind routing decisions.
///
/// Only kinds with a decided answer are routed here. A kind gains its field
/// when evidence for a `Native` route exists, never as an unimplemented
/// placeholder: a `Native` value that no phase honours would silently produce
/// narrate behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Route {
    /// A run of Latin letters in prose, e.g. "Kubernetes" inside a Russian
    /// sentence.
    pub latin_prose: Reading,
    /// A code identifier or a part of one, e.g. "getUserData", "snake_case".
    pub identifier: Reading,
}

impl Route {
    /// Whether any routed kind is left to the engine.
    pub fn any_native(&self) -> bool {
        self.latin_prose == Reading::Native || self.identifier == Reading::Native
    }
}

/// Digits: expanded into words by the pipeline, or read by the engine itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Digits {
    /// The pipeline expands every number, date, time, range and version.
    Expanded,
    /// The pipeline leaves digits untouched; the engine reads them.
    Native,
}

/// Units attached to a number (`%`, `км/ч`, `₽`, `ГБ`): expanded into words by
/// the pipeline, or read by the engine itself.
///
/// Separate from [`Digits`] because the two decisions differ: a number reading
/// is engine data, while a unit reading is language data — "пятьдесят
/// процентов" and "fifty percent" are different words for the same fragment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Units {
    /// The pipeline expands percentages, sizes and currency into words.
    Expanded,
    /// The pipeline leaves units untouched; the engine reads them.
    Native,
}

/// Stress handling.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stress {
    /// No stress control: the adapter adds nothing and an explicit mark from
    /// the user dictionary is not rendered.
    None,
    /// Automatic stress; an explicit mark from the user dictionary overtakes
    /// automatic stress for that word.
    Auto,
    /// Only explicit marks are rendered — no automatic stress.
    Manual,
}

/// What the engine accepts as codepoints.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Charset {
    /// Any codepoint the pipeline emits.
    Any,
    /// The engine filters against the model's own symbol table (a codepoint
    /// outside it is dropped silently). The adapter MUST filter; the pipeline
    /// never does, and must not rely on the engine having dropped anything.
    ModelAlphabet,
}

/// Markup the adapter wraps around a [`Reading::Native`] span.
///
/// The pipeline never emits markup: tags belong to the engine call, not to the
/// text. Variants stay parameterless so a new syntax becomes an explicit
/// choice instead of a configuration string.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Markup {
    /// No tags: the engine reads mixed-language text as it arrives.
    None,
    /// A tag per language code, e.g. `<ru>…</ru>` and `<en>…</en>`.
    RuEnTags,
    /// One generic tag carrying the language as an attribute, e.g.
    /// `<lang="en">…</lang>`.
    LangTag,
}

impl Markup {
    /// Wraps `text` for the given language code, or returns it unchanged when
    /// the engine needs no markup.
    pub fn wrap<'a>(&self, language: &str, text: &'a str) -> Cow<'a, str> {
        match self {
            Markup::None => Cow::Borrowed(text),
            Markup::RuEnTags => Cow::Owned(format!("<{language}>{text}</{language}>")),
            Markup::LangTag => Cow::Owned(format!("<lang=\"{language}\">{text}</lang>")),
        }
    }
}

/// The unit of the per-call synthesis limit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Limit {
    /// Codepoints of normalized text.
    Chars(usize),
    /// Seconds of produced audio (for engines limited by decoder context).
    Secs(u32),
}

/// Silences the engine inserts, in milliseconds; `0` means "insert nothing".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pauses {
    /// Inserted where the source had a paragraph break.
    pub paragraph_ms: u32,
    /// Inserted at a language-segment boundary.
    pub segment_ms: u32,
}

/// Chunking limits and pauses for one engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Chunk {
    pub limit: Limit,
    pub pauses: Pauses,
}

impl Chunk {
    /// The codepoint limit, when the engine declares one.
    pub fn char_limit(&self) -> Option<usize> {
        match self.limit {
            Limit::Chars(chars) => Some(chars),
            Limit::Secs(_) => None,
        }
    }

    /// Silence inserted after a paragraph break, in seconds.
    pub fn paragraph_pause_sec(&self) -> f64 {
        f64::from(self.pauses.paragraph_ms) / 1000.0
    }

    /// Silence inserted at a language-segment boundary, in seconds.
    pub fn segment_pause_sec(&self) -> f64 {
        f64::from(self.pauses.segment_ms) / 1000.0
    }
}

/// Everything the pipeline and the adapter need to know about one engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextProfile {
    /// Language codes the engine accepts natively. Every voice of the engine
    /// inherits this list: a voice-level capability is deliberately not
    /// modelled until a voice catalogue can supply one.
    pub languages: &'static [&'static str],
    pub route: Route,
    pub digits: Digits,
    pub units: Units,
    pub stress: Stress,
    pub charset: Charset,
    pub chunk: Chunk,
    pub markup: Markup,
}

impl TextProfile {
    /// Whether the engine reads `language` natively.
    pub fn reads_natively(&self, language: &str) -> bool {
        self.languages.contains(&language)
    }

    /// Checks the cross-field invariants a profile must satisfy.
    pub fn validate(&self) -> Result<(), ProfileError> {
        if self.route.any_native() && !self.languages.iter().any(|lang| *lang != "ru") {
            return Err(ProfileError::NativeLatinWithoutLanguage);
        }
        // A unit reading spells the number out ("пятьдесят процентов"), so the
        // pipeline cannot expand units while the engine reads the number.
        if self.units == Units::Expanded && self.digits == Digits::Native {
            return Err(ProfileError::ExpandedUnitsWithoutDigits);
        }
        // An engine that reads identifiers natively reads Latin by definition;
        // routing identifiers Native while transliterating prose would hand it
        // a Latin token in one place and a transliteration in another.
        if self.route.identifier == Reading::Native && self.route.latin_prose == Reading::Narrate {
            return Err(ProfileError::NativeIdentifierWithoutNativeProse);
        }
        Ok(())
    }
}

/// A violation of a profile invariant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProfileError {
    /// A fragment is routed `Native` while the engine lists no language other
    /// than Russian — there would be no language to read it in.
    NativeLatinWithoutLanguage,
    /// Units are expanded by the pipeline while digits are left to the engine:
    /// a unit reading needs the number in words.
    ExpandedUnitsWithoutDigits,
    /// Identifiers are routed `Native` while prose is transliterated — the same
    /// engine would receive both a Latin token and its Russian reading.
    NativeIdentifierWithoutNativeProse,
}

impl fmt::Display for ProfileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProfileError::NativeLatinWithoutLanguage => f.write_str(
                "profile routes Latin fragments Native but lists no language other than Russian",
            ),
            ProfileError::ExpandedUnitsWithoutDigits => f.write_str(
                "profile expands units while leaving digits Native; a unit reading needs words",
            ),
            ProfileError::NativeIdentifierWithoutNativeProse => {
                f.write_str("profile routes identifiers Native while transliterating prose")
            }
        }
    }
}

impl std::error::Error for ProfileError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn native_profile() -> TextProfile {
        TextProfile {
            languages: &["ru", "en"],
            route: Route {
                latin_prose: Reading::Native,
                identifier: Reading::Native,
            },
            digits: Digits::Native,
            units: Units::Native,
            stress: Stress::None,
            charset: Charset::Any,
            chunk: Chunk {
                limit: Limit::Chars(300),
                pauses: Pauses {
                    paragraph_ms: 0,
                    segment_ms: 200,
                },
            },
            markup: Markup::RuEnTags,
        }
    }

    #[test]
    fn markup_none_returns_the_text_unchanged() {
        let wrapped = Markup::None.wrap("en", "Kubernetes");
        assert_eq!(wrapped, "Kubernetes");
        assert!(matches!(wrapped, Cow::Borrowed(_)));
    }

    #[test]
    fn markup_ru_en_tags_wraps_with_the_language_code() {
        assert_eq!(
            Markup::RuEnTags.wrap("en", "Kubernetes"),
            "<en>Kubernetes</en>"
        );
        assert_eq!(Markup::RuEnTags.wrap("ru", "кластер"), "<ru>кластер</ru>");
    }

    #[test]
    fn markup_lang_tag_wraps_with_a_language_attribute() {
        assert_eq!(
            Markup::LangTag.wrap("en", "Kubernetes"),
            "<lang=\"en\">Kubernetes</lang>"
        );
    }

    #[test]
    fn native_route_without_a_non_russian_language_is_rejected() {
        let mut profile = native_profile();
        profile.languages = &["ru"];
        assert_eq!(
            profile.validate(),
            Err(ProfileError::NativeLatinWithoutLanguage)
        );
    }

    #[test]
    fn russian_only_profile_narrating_every_fragment_is_valid() {
        let mut profile = native_profile();
        profile.languages = &["ru"];
        profile.route = Route {
            latin_prose: Reading::Narrate,
            identifier: Reading::Narrate,
        };
        assert_eq!(profile.validate(), Ok(()));
    }

    #[test]
    fn languages_are_matched_exactly() {
        let profile = native_profile();
        assert!(profile.reads_natively("en"));
        assert!(!profile.reads_natively("de"));
    }

    #[test]
    fn expanding_units_while_leaving_digits_native_is_rejected() {
        let mut profile = native_profile();
        profile.digits = Digits::Native;
        profile.units = Units::Expanded;
        assert_eq!(
            profile.validate(),
            Err(ProfileError::ExpandedUnitsWithoutDigits)
        );
    }

    #[test]
    fn native_identifiers_require_native_prose() {
        let mut profile = native_profile();
        profile.route.latin_prose = Reading::Narrate;
        assert_eq!(
            profile.validate(),
            Err(ProfileError::NativeIdentifierWithoutNativeProse)
        );
    }

    #[test]
    fn expanded_digits_with_native_units_is_valid() {
        let mut profile = native_profile();
        profile.digits = Digits::Expanded;
        profile.units = Units::Native;
        assert_eq!(profile.validate(), Ok(()));
    }
}
