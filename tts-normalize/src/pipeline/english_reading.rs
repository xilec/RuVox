//! The user's English reading preference and its resolution against an engine
//! capability.
//!
//! The preference lives in `UIConfig.english_reading`, the capability in
//! `TextProfile::reads_latin_natively`. Neither alone decides what happens to a
//! Latin run in prose: the setting is what the listener wants, the capability
//! is what the engine can honour, and [`EnglishReading::resolve`] is the single
//! place the two meet.

use tts_contract::Reading;

/// How Latin prose is read.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EnglishReading {
    /// Follow the engine: English when it can read it, Russian otherwise. The
    /// default, and the behaviour that predates the setting, so an existing
    /// installation does not change on upgrade.
    #[default]
    Auto,
    /// Ask for English. An engine without the capability falls back to
    /// narration; the stored preference is never rewritten.
    Native,
    /// Always transliterate into Russian.
    Narrated,
}

impl EnglishReading {
    /// Parse an `english_reading` config value. Unknown values fall back to
    /// [`Self::Auto`], the same rule [`CodeBlockMode`] applies to
    /// `code_block_mode`.
    ///
    /// [`CodeBlockMode`]: crate::pipeline::normalizers::code_blocks::CodeBlockMode
    pub fn from_config(value: &str) -> Self {
        match value {
            "native" => Self::Native,
            "narrated" => Self::Narrated,
            _ => Self::Auto,
        }
    }

    /// Canonical config string for this preference.
    pub fn as_config_str(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Native => "native",
            Self::Narrated => "narrated",
        }
    }

    /// The prose route this preference resolves to for an engine.
    pub fn resolve(self, reads_latin_natively: bool) -> Reading {
        match (self, reads_latin_natively) {
            (Self::Narrated, _) | (Self::Auto | Self::Native, false) => Reading::Narrate,
            (Self::Auto | Self::Native, true) => Reading::Native,
        }
    }

    /// Whether the pipeline transliterates prose under this preference.
    pub fn narrates(self, reads_latin_natively: bool) -> bool {
        self.resolve(reads_latin_natively) == Reading::Narrate
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auto_follows_the_engine_capability() {
        assert_eq!(EnglishReading::Auto.resolve(true), Reading::Native);
        assert_eq!(EnglishReading::Auto.resolve(false), Reading::Narrate);
    }

    #[test]
    fn native_falls_back_when_the_engine_cannot_read_latin() {
        assert_eq!(EnglishReading::Native.resolve(true), Reading::Native);
        assert_eq!(EnglishReading::Native.resolve(false), Reading::Narrate);
    }

    #[test]
    fn narrated_always_transliterates() {
        assert_eq!(EnglishReading::Narrated.resolve(true), Reading::Narrate);
        assert_eq!(EnglishReading::Narrated.resolve(false), Reading::Narrate);
        assert!(EnglishReading::Narrated.narrates(true));
        assert!(!EnglishReading::Auto.narrates(true));
    }

    #[test]
    fn config_values_round_trip() {
        for reading in [
            EnglishReading::Auto,
            EnglishReading::Native,
            EnglishReading::Narrated,
        ] {
            assert_eq!(
                EnglishReading::from_config(reading.as_config_str()),
                reading
            );
        }
    }

    #[test]
    fn unknown_and_legacy_values_fall_back_to_auto() {
        for raw in ["", "bilingual", "Auto", "skip", "narrate"] {
            assert_eq!(EnglishReading::from_config(raw), EnglishReading::Auto);
        }
    }

    #[test]
    fn default_is_auto() {
        assert_eq!(EnglishReading::default(), EnglishReading::Auto);
    }
}
