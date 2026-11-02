pub mod constants;
pub mod english_reading;
pub mod normalizers;
pub mod spans;
pub mod tracked_text;

pub use english_reading::EnglishReading;
pub use spans::{Span, SpanKind};

use std::sync::OnceLock;

use regex::Regex;

use crate::dictionary::{KeyScript, UserDictionary};
use crate::pipeline::constants::{ARROW_SYMBOLS, GREEK_LETTERS, MATH_SYMBOLS};
use crate::pipeline::normalizers::abbreviations::AbbreviationNormalizer;
use crate::pipeline::normalizers::code::CodeIdentifierNormalizer;
use crate::pipeline::normalizers::code_blocks::{CodeBlockHandler, CodeBlockMode};
use crate::pipeline::normalizers::english::EnglishNormalizer;
use crate::pipeline::normalizers::numbers::NumberNormalizer;
use crate::pipeline::normalizers::symbols::SymbolNormalizer;
use crate::pipeline::normalizers::urls::{URLPathNormalizer, is_known_tld};
use crate::pipeline::spans::{
    RecordedSpan, byte_to_char_offsets, debug_assert_spans, record_byte_ranges, record_native,
    resolve_spans,
};
use crate::pipeline::tracked_text::{CharMapping, TrackedText};
use crate::profiles;
use tts_contract::{Digits, Reading, Stress, TextProfile, Units};

// ── Static compiled regexes ───────────────────────────────────────────────────

fn re_url() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(
            r#"https?://[^\s<>"'\)]+|ftp://[^\s<>"'\)]+|ssh://[^\s<>"'\)]+|git://[^\s<>"'\)]+"#,
        )
        .expect("valid regex")
    })
}

fn re_schemeless_url() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    // Generic dotted-labels candidate ("example.com", "example.com/path").
    // The www/TLD validation happens in the phase-7 closure, which rejects
    // versions, dates, filenames, and path-internal segments.
    RE.get_or_init(|| {
        Regex::new(r#"\b(?:[a-zA-Z0-9-]+\.)+[a-zA-Z0-9-]+(?:/[^\s<>"'\)]*)?"#).expect("valid regex")
    })
}

/// Split trailing sentence punctuation off a matched URL so it is neither
/// parsed nor read as part of it ("смотри example.com." keeps its dot).
fn split_trailing_punct(url: &str) -> (&str, &str) {
    let core = url.trim_end_matches(['.', ',', ';', ':', '!', '?']);
    (core, &url[core.len()..])
}

fn re_email() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"[a-zA-Z0-9._%+\-]+@[a-zA-Z0-9.\-]+\.[a-zA-Z]{2,}").expect("valid regex")
    })
}

fn re_ip() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"\b(\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3})\b").expect("valid regex"))
}

fn re_path() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    // Lookbehind is not supported; the closure filters out false positives.
    RE.get_or_init(|| {
        Regex::new(r"([~/][a-zA-Z0-9_./ \-]+\.[a-zA-Z0-9]+|[~/][a-zA-Z0-9_./\-]+)")
            .expect("valid regex")
    })
}

fn re_size() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(
            r"(?i)\b(\d+(?:\.\d+)?)\s*(KB|MB|GB|TB|ms|sec|min|hr|px|em|rem|vh|vw|кб|мб|гб|тб)\b",
        )
        .expect("valid regex")
    })
}

fn re_date_iso() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"\b(\d{4})-(\d{2})-(\d{2})\b").expect("valid regex"))
}

fn re_date_dot() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    // European DD.MM.YYYY. A 4-digit year is required so semver-like "1.2.3"
    // and truncated forms like "10.04." are left for the version/number phases.
    RE.get_or_init(|| Regex::new(r"\b(\d{1,2})\.(\d{1,2})\.(\d{4})\b").expect("valid regex"))
}

fn re_time() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    // HH:MM or HH:MM:SS. Two-digit minutes avoid catching ratios like "1:2".
    RE.get_or_init(|| Regex::new(r"\b(\d{1,2}):(\d{2})(?::(\d{2}))?\b").expect("valid regex"))
}

fn re_version() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"(?i)\bv?(\d+\.\d+(?:\.\d+)?(?:-(?:alpha|beta|rc|dev|stable|release)\d*)?)\b")
            .expect("valid regex")
    })
}

/// Bare decimal fraction without the integer part (".5"). The dot must be
/// at text start or preceded by anything except a letter, digit,
/// underscore, dot, or path separator — otherwise it is a float tail
/// ("1.5"), dotted label ("example.5"), version chain, ellipsis, or path
/// fragment owned by earlier phases. Group 1 is the boundary context kept
/// verbatim, group 2 the fractional digits.
fn re_leading_dot_decimal() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"(^|[^\p{L}\p{N}_./\\])\.(\d+)").expect("valid regex"))
}

fn re_range() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"\b(\d+)\s*-\s*(\d+)\b").expect("valid regex"))
}

fn re_percentage() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"\b(\d+(?:\.\d+)?)\s*%").expect("valid regex"))
}

/// Percentage range "10-20%": two integers around a dash with a single
/// trailing "%". Must be matched before the plain percentage pattern,
/// otherwise "20%" is consumed alone and the dash is left bare (#112).
fn re_percentage_range() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"\b(\d+)\s*-\s*(\d+)\s*%").expect("valid regex"))
}

/// Units `re_size` accepts, lowercased: the guard below reads the word right
/// after a number instead of scanning forward for a size match, which would
/// rescan the rest of the document for every number.
const SIZE_UNITS: [&str; 17] = [
    "kb", "mb", "gb", "tb", "ms", "sec", "min", "hr", "px", "em", "rem", "vh", "vw", "кб", "мб",
    "гб", "тб",
];

/// True when the number at `start..end` belongs to a percentage or a size,
/// which the gated unit phases own as a whole.
fn unit_owns(snapshot: &str, end: usize) -> bool {
    let after = snapshot[end..].trim_start_matches(' ');
    if after.starts_with('%') {
        return true;
    }

    // Left bound of a percentage range ("10-20%"): dash, digits, percent sign.
    if let Some(rest) = after.strip_prefix('-') {
        let digit_bytes = rest.len() - rest.trim_start_matches(|c: char| c.is_ascii_digit()).len();
        if digit_bytes > 0 && rest[digit_bytes..].trim_start_matches(' ').starts_with('%') {
            return true;
        }
    }

    // A size unit attached directly or after a single space ("100MB", "1.5 GB").
    let word: String = after.chars().take_while(|c| c.is_alphanumeric()).collect();
    let boundary = after
        .chars()
        .nth(word.chars().count())
        .is_none_or(|c| !c.is_alphanumeric());
    boundary && SIZE_UNITS.contains(&word.to_lowercase().as_str())
}

fn re_inline_code() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"`([^`\n]+)`").expect("valid regex"))
}

fn re_heading() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"(?m)^#{1,6}\s+").expect("valid regex"))
}

fn re_blockquote() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    // One or more nested `>` markers at the start of a line, with their
    // optional padding. An inline comparison (`a > b`) never matches.
    RE.get_or_init(|| Regex::new(r"(?m)^(?:[ \t]*>[ \t]?)+").expect("valid regex"))
}

fn re_md_link_full() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    // Match full [text](url).
    RE.get_or_init(|| Regex::new(r"\[([^\]]+)\]\([^)]+\)").expect("valid regex"))
}

fn re_md_list_number() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"(?m)^(\d+)\.\s+").expect("valid regex"))
}

fn re_camel_lower() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"\b([a-z]+(?:[A-Z][a-z]*)+)\b").expect("valid regex"))
}

fn re_pascal() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"\b([A-Z][a-z]+(?:[A-Z][a-z]+)+)\b").expect("valid regex"))
}

fn re_snake() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"\b([a-zA-Z_][a-zA-Z0-9]*(?:_[a-zA-Z0-9]+)+)\b").expect("valid regex")
    })
}

fn re_kebab() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"\b([a-zA-Z][a-zA-Z0-9]*(?:-[a-zA-Z0-9]+)+)\b").expect("valid regex")
    })
}

/// Byte ranges of the code-identifier patterns, sorted and non-overlapping.
///
/// The identifier phase and the English phase must agree on what an identifier
/// is: the first narrates or annotates it, the second must not re-read it when
/// the profile leaves identifiers to the engine while the user asked for
/// Russian prose.
fn find_identifier_ranges(text: &str) -> Vec<(usize, usize)> {
    let mut ranges: Vec<(usize, usize)> = [re_camel_lower(), re_pascal(), re_snake(), re_kebab()]
        .into_iter()
        .flat_map(|pattern| pattern.find_iter(text))
        .map(|m| (m.start(), m.end()))
        .collect();
    ranges.sort_unstable();
    // The kebab pattern claims a whole token while the camel or Pascal pattern
    // claims an identifier nested in it (`my-app-getUserData-flag`: both
    // `(0, 23)` and `(7, 18)`), and a binary search over overlapping ranges
    // can miss the outer one; the union covers every fragment either pattern
    // claimed.
    let mut merged: Vec<(usize, usize)> = Vec::with_capacity(ranges.len());
    for (start, end) in ranges {
        match merged.last_mut() {
            Some(last) if start < last.1 => last.1 = last.1.max(end),
            _ => merged.push((start, end)),
        }
    }
    merged
}

/// Whether `[start, end)` intersects a sorted, non-overlapping range list.
fn overlaps_ranges(ranges: &[(usize, usize)], start: usize, end: usize) -> bool {
    let first_after = ranges.partition_point(|(range_start, _)| *range_start < end);
    first_after > 0 && ranges[first_after - 1].1 > start
}

fn re_alnum_abbrev() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    // Alphanumeric tokens the model alphabet cannot read as one word:
    // `IPv6`, `HTTP/2`. Pure letter runs stay prose; this phase
    // exists so a Native prose route does not leave half-normalized fragments
    // (`HTTPслэш два`) behind.
    RE.get_or_init(|| {
        Regex::new(r"\b(?:[A-Z]{2,}|IPv)[0-9]+(?:\.[0-9]+)*\b|\b[A-Z]{2,}/[0-9]+\b")
            .expect("valid regex")
    })
}

/// Case-insensitive counterpart of [`re_alnum_abbrev`] for the editor's
/// built-in marker: the entry key is lowercased, while the pipeline regex
/// matches the original uppercase token.
fn re_alnum_abbrev_ci() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"(?i)\b(?:[A-Z]{2,}|IPv)[0-9]+(?:\.[0-9]+)*\b|\b[A-Z]{2,}/[0-9]+\b")
            .expect("valid regex")
    })
}

fn re_english_words() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    // Single letters included: a lone "x" or "I" must also become speakable
    // Cyrillic (read by letter name in process_english_tracked).
    RE.get_or_init(|| Regex::new(r"\b([A-Za-z]+)\b").expect("valid regex"))
}

fn re_mixed_alnum() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    // Letter+digit tokens such as `p95`, `v2`, `sha256` and `H264`. Mixed-case
    // identifiers (`getUserData2`) are deliberately excluded: the identifier
    // phase owns camelCase, and splitting one here would destroy its reading.
    RE.get_or_init(|| {
        Regex::new(
            r"\b(?:[a-z]+\d+[a-z0-9]*|[A-Z]+\d+[A-Z0-9]*|\d+[a-z]+[a-z0-9]*|\d+[A-Z]+[A-Z0-9]*)\b",
        )
        .expect("valid regex")
    })
}

fn re_user_dict_token() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    // Letters+digits with at least one letter: pure numbers stay with the
    // number phase, while alnum tokens like "IPv6"/"mp3" — captured by no
    // other prose phase — become user-dictionary candidates. At least one
    // letter is required, so a non-hit flows into later phases unchanged.
    RE.get_or_init(|| Regex::new(r"\b[A-Za-z0-9]*[A-Za-z][A-Za-z0-9]*\b").expect("valid regex"))
}

fn re_user_dict_token_cyrillic() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    // The Cyrillic mirror of `re_user_dict_token`: one Russian word (letters and
    // digits, at least one letter), so a stress entry like `замок = "зам+ок"`
    // can match. The two patterns are disjoint — a token belongs to one script —
    // and the pre-pass runs this one only when the dictionary has a Cyrillic
    // key, so a Latin-only dictionary pays nothing for Russian text.
    RE.get_or_init(|| Regex::new(r"\b[0-9]*[а-яёА-ЯЁ][0-9а-яёА-ЯЁ]*\b").expect("valid regex"))
}

fn re_user_dict_hyphenated() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    // One token with an internal hyphen (`UTF-8`, `e-mail`, `по-русски`). This
    // pattern feeds the whole-token pass that runs before the code-identifier
    // split, where the kebab pattern would not have claimed the token yet.
    // Both scripts are in the candidate class on purpose: keys are validated to
    // one script, so a mixed or digit-only candidate simply never hits.
    RE.get_or_init(|| {
        Regex::new(r"\b[0-9A-Za-zА-Яа-яЁё]+(?:-[0-9A-Za-zА-Яа-яЁё]+)+\b").expect("valid regex")
    })
}

fn re_number() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    // Match standalone integers: must be at a word boundary and not preceded/followed
    // by a dot (which would indicate a float/version, already handled earlier).
    // Since regex crate lacks lookbehind, we use \b which works for ASCII digit boundaries.
    RE.get_or_init(|| Regex::new(r"\b\d+\b").expect("valid regex"))
}

fn re_multi_spaces() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r" +").expect("valid regex"))
}

fn re_space_before_punct() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r" +([.,!?;:])").expect("valid regex"))
}

fn re_space_after_newline() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"\n +").expect("valid regex"))
}

fn re_space_before_newline() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r" +\n").expect("valid regex"))
}

fn re_collapse_newlines() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"\n{3,}").expect("valid regex"))
}

fn re_collapse_spaces() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"[ \t]+").expect("valid regex"))
}

fn re_tilde_approx() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    // Match ~<optional spaces><digit> — capture only the digit so that "~ 5"
    // collapses to a single space ("около 5", not "около  5").
    // Regex crate does not support lookahead, so we consume the digit and re-emit it.
    RE.get_or_init(|| Regex::new(r"~\s*(\d)").expect("valid regex"))
}

// ── Multi-char operators processed in tracked mode (longest first) ────────────

/// Operators handled by SymbolNormalizer, processed longest-first to avoid
/// partial matches (e.g. "===" must be checked before "==").
/// Single `=` is intentionally excluded — it would corrupt math formulas
/// like "α = β".
const TRACKED_OPERATOR_KEYS: &[&str] =
    &["===", "!==", "->", "=>", ">=", "<=", "!=", "==", "&&", "||"];

// ── TTSPipeline ────────────────────────────────────────────────────────────────

/// The pipeline's output: the text the engine will speak and the mapping back
/// to the original document.
///
/// The spans describe the fragments the adapter must treat specially (render
/// markup around them, or hand them to the engine as-is); they never appear in
/// `text` itself.
pub struct ProcessedText {
    /// Normalized text — the exact synthesis input.
    pub text: String,
    /// Codepoint-level bridge from `text` back to the original document.
    pub mapping: CharMapping,
    /// Fragments the adapter must render or wrap, sorted and non-overlapping.
    pub spans: Vec<Span>,
}

/// Main pipeline for TTS text preprocessing.
///
/// Owns all normalizer instances. Phase order matters: see `process_with_char_mapping`.
pub struct TTSPipeline {
    number_normalizer: NumberNormalizer,
    english_normalizer: EnglishNormalizer,
    abbrev_normalizer: AbbreviationNormalizer,
    symbol_normalizer: SymbolNormalizer,
    code_block_handler: CodeBlockHandler,
    code_normalizer: CodeIdentifierNormalizer,
    /// User-authored pronunciation overrides (change `user-dictionary`).
    /// Single home for the map; lookup sites receive it by reference.
    user_dictionary: UserDictionary,
    /// Whether an explicit dictionary stress mark reaches the text: derived
    /// from the profile at the start of every `process_for_engine` run, so an
    /// engine switch cannot inherit the previous run's decision.
    mark_stress: bool,
    /// How the user wants Latin prose read (the `english_reading` config
    /// value), resolved against the engine capability per run.
    english_reading: EnglishReading,
}

impl TTSPipeline {
    /// Build the pipeline with all normalizers in the default configuration.
    pub fn new() -> Self {
        Self {
            number_normalizer: NumberNormalizer::new(),
            english_normalizer: EnglishNormalizer::new(),
            abbrev_normalizer: AbbreviationNormalizer::new(),
            symbol_normalizer: SymbolNormalizer::new(),
            // Implicit default matches the `code_block_mode` config default
            // (brief); production code always sets the mode explicitly from
            // config, so this default only shows up in tests.
            code_block_handler: CodeBlockHandler::new(),
            code_normalizer: CodeIdentifierNormalizer::new(),
            user_dictionary: UserDictionary::default(),
            mark_stress: false,
            // Implicit default matches the `english_reading` config default
            // (auto = follow the engine); production code always sets the
            // preference from config.
            english_reading: EnglishReading::Auto,
        }
    }

    /// Current English reading preference (mirrors the `english_reading`
    /// config value).
    pub fn english_reading(&self) -> EnglishReading {
        self.english_reading
    }

    /// Set the English reading preference. Driven by config: at startup and
    /// whenever `update_config` changes `english_reading`.
    pub fn set_english_reading(&mut self, reading: EnglishReading) {
        self.english_reading = reading;
    }

    /// Current code block narration mode (mirrors the `code_block_mode`
    /// config value).
    pub fn code_block_mode(&self) -> CodeBlockMode {
        self.code_block_handler.mode()
    }

    /// Set the code block narration mode. Driven by config: at startup and
    /// whenever `update_config` changes `code_block_mode`.
    pub fn set_code_block_mode(&mut self, mode: CodeBlockMode) {
        self.code_block_handler.set_mode(mode);
    }

    /// Replace the user dictionary. Driven by the dictionary commands after
    /// every persisted change (save/import), same refresh pattern as
    /// `set_code_block_mode`.
    pub fn set_user_dictionary(&mut self, dict: UserDictionary) {
        self.user_dictionary = dict;
    }

    /// Read-only access for the commands layer (override-marker computation).
    pub fn user_dictionary(&self) -> &UserDictionary {
        &self.user_dictionary
    }

    /// Whether `word_lower` has a built-in reading at any dictionary lookup
    /// site (`IT_TERMS`, the abbreviation maps including their special
    /// cases, `CODE_WORDS`). Used by the commands layer to flag user entries
    /// that override built-ins under the strict Silero profile.
    pub fn builtin_contains(&self, word_lower: &str) -> bool {
        self.builtin_contains_for(&profiles::SILERO_PROFILE, word_lower)
    }

    /// Profile-aware built-in lookup for the editor's override marker: a table
    /// that the active profile does not run cannot be overridden, so it must
    /// not be marked as a built-in hit.
    pub fn builtin_contains_for(
        &self,
        profile: &tts_contract::TextProfile,
        word_lower: &str,
    ) -> bool {
        use crate::pipeline::normalizers::abbreviations;
        use crate::pipeline::normalizers::english::IT_TERMS;
        use tts_contract::Reading;

        let narrate_latin = self.english_reading.narrates(profile.reads_latin_natively);
        let narrate_identifiers = profile.route.identifier == Reading::Narrate;
        // Mixed letter/digit abbreviations are read as a whole on both routes
        // (phase 14.5), so their built-in reading exists even under Native.
        re_alnum_abbrev_ci().is_match(word_lower)
            || (narrate_latin
                && (IT_TERMS.contains_key(word_lower)
                    || abbreviations::special_cases().contains_key(word_lower)
                    || abbreviations::as_word().contains_key(word_lower)))
            || (narrate_identifiers && self.code_normalizer.contains_builtin(word_lower))
    }

    /// Process text for TTS with precise character-level mapping under the
    /// reference (Silero) profile.
    ///
    /// Compatibility wrapper for callers that normalize once for the strictest
    /// engine: the golden corpus, dictionary-entry validation and tests. Real
    /// synthesis goes through [`Self::process_for_engine`] with the active
    /// engine's profile.
    pub fn process_with_char_mapping(&mut self, input: &str) -> (String, CharMapping) {
        let processed = self.process_for_engine(input, &profiles::SILERO_PROFILE);
        (processed.text, processed.mapping)
    }

    /// Process text for TTS under `profile` — the text contract of the engine
    /// that will synthesize the result.
    ///
    /// The profile only selects which phase runs; the phase order itself is
    /// fixed (see the `text-pipeline` spec), so a profile can never make one
    /// phase consume another phase's fragment.
    pub fn process_for_engine(&mut self, input: &str, profile: &TextProfile) -> ProcessedText {
        if input.is_empty() {
            return ProcessedText {
                text: String::new(),
                mapping: CharMapping {
                    original: String::new(),
                    transformed: String::new(),
                    char_map: Vec::new(),
                },
                spans: Vec::new(),
            };
        }

        // Routing decisions, read once per call: a fragment is either narrated
        // in Russian by the pipeline or left for the engine, and the number and
        // unit phases run only when the engine cannot read them.
        let narrate_latin = self.english_reading.narrates(profile.reads_latin_natively);
        let narrate_identifiers = profile.route.identifier == Reading::Narrate;
        let expand_digits = profile.digits == Digits::Expanded;
        let expand_units = profile.units == Units::Expanded;

        // An explicit dictionary stress mark reaches the text only when the
        // active engine renders it; the dictionary carries the decision for the
        // lookup sites that never see the profile.
        self.mark_stress = profile.stress != Stress::None;

        // The language the engine reads a native fragment in — the same one a
        // `Native` span carries.
        let native_lang = profile.native_language();
        let mut recorded: Vec<RecordedSpan> = Vec::new();

        self.english_normalizer.clear_unknown_words();

        let mut tracked = TrackedText::new(input);

        // ── BOM removal ───────────────────────────────────────────────────────
        if tracked.text().starts_with('\u{feff}') {
            tracked.replace("\u{feff}", "");
        }

        // ── Phase 1: Code blocks (must run before space/dash normalization) ───
        // A block the profile routes Native leaves a placeholder here; the
        // code itself is restored only after every narrating phase has run.
        let user_dict = &self.user_dictionary;
        let native_code = self.code_block_handler.process(
            &mut tracked,
            user_dict,
            self.mark_stress,
            profile.route.code_block,
        );
        for block in &native_code {
            record_native(
                &mut tracked,
                SpanKind::CodeBlock,
                native_lang,
                |text| {
                    text.match_indices(&block.placeholder)
                        .map(|(start, matched)| (start, start + matched.len()))
                        .collect()
                },
                &mut recorded,
            );
        }

        // ── Phase 2: Quote normalization ─────────────────────────────────────
        tracked.replace("\u{ab}", "\""); // «
        tracked.replace("\u{bb}", "\""); // »
        tracked.replace("\u{201c}", "\""); // "
        tracked.replace("\u{201d}", "\""); // "
        tracked.replace("\u{2018}", "'"); // '
        tracked.replace("\u{2019}", "'"); // '

        // ── Phase 3: Dash normalization ───────────────────────────────────────
        tracked.replace("\u{2014}", "-"); // em-dash —
        tracked.replace("\u{2013}", "-"); // en-dash –

        // ── Phase 4: Whitespace normalization ─────────────────────────────────
        tracked.sub(re_collapse_newlines(), |_| "\n\n".to_string());
        tracked.sub(re_collapse_spaces(), |_| " ".to_string());

        if tracked.text().trim().is_empty() {
            return ProcessedText {
                text: String::new(),
                mapping: CharMapping {
                    original: input.to_string(),
                    transformed: String::new(),
                    char_map: Vec::new(),
                },
                spans: Vec::new(),
            };
        }

        // ── Phase 5: Inline code ─────────────────────────────────────────────
        self.process_inline_code_tracked(&mut tracked);

        // ── Phase 6: Markdown structure ───────────────────────────────────────
        self.process_markdown_tracked(&mut tracked, expand_digits);

        // ── Phase 7: URLs ─────────────────────────────────────────────────────
        {
            let num = &self.number_normalizer;
            let eng = &self.english_normalizer;
            let url_norm = URLPathNormalizer::new(eng, num)
                .with_user_dictionary(&self.user_dictionary, self.mark_stress);
            tracked.sub(re_url(), |caps| {
                let url = caps.get(0).unwrap().as_str();
                let (core, suffix) = split_trailing_punct(url);
                format!("{}{}", url_norm.normalize_url(core), suffix)
            });
            tracked.sub(re_email(), |caps| {
                url_norm.normalize_email(caps.get(0).unwrap().as_str())
            });
            // Scheme-less URLs ("www.example.com", "example.com/path") — after
            // schemed URLs and emails so their spans are already consumed.
            {
                let snapshot = tracked.text().to_string();
                let matches: Vec<(usize, usize, String)> = re_schemeless_url()
                    .find_iter(&snapshot)
                    .filter_map(|m| {
                        // Skip path-internal segments ("/home/site.dev/main.py"
                        // or "C:\app.dev\main.py" is a file path, not a domain).
                        if m.start() > 0 && snapshot[..m.start()].ends_with(['/', '\\']) {
                            return None;
                        }
                        let candidate = m.as_str();
                        let (core, suffix) = split_trailing_punct(candidate);
                        let host = core.split(['/', '?', '#']).next().unwrap_or(core);
                        let is_www = host
                            .split('.')
                            .next()
                            .is_some_and(|l| l.eq_ignore_ascii_case("www"));
                        let known_tld = host.rsplit('.').next().is_some_and(is_known_tld);
                        if is_www || known_tld {
                            Some((
                                m.start(),
                                m.end(),
                                format!("{}{}", url_norm.normalize_schemeless(core), suffix),
                            ))
                        } else {
                            None
                        }
                    })
                    .collect();
                tracked.replace_byte_ranges(matches);
            }
            tracked.sub(re_ip(), |caps| {
                url_norm.normalize_ip(caps.get(0).unwrap().as_str())
            });
            // A slash inside a token (`HTTP/2`) is not a filesystem path:
            // the path regex sees only the `/2` part, and normalizing it
            // would leave a mixed-script `HTTPслэш два`. Skip matches whose
            // preceding character is a word character; the alphanumeric
            // abbreviation phase reads the whole token instead.
            let path_snapshot = tracked.text().to_string();
            tracked.sub(re_path(), |caps| {
                let matched = caps.get(0).unwrap();
                let start = matched.start();
                if start > 0 && path_snapshot.as_bytes()[start - 1].is_ascii_alphanumeric() {
                    return matched.as_str().to_string();
                }
                let path = caps.get(1).unwrap().as_str();
                if path.contains('/') && (path.starts_with('/') || path.starts_with('~')) {
                    url_norm.normalize_filepath(path)
                } else {
                    path.to_string()
                }
            });
        }

        // ── Phase 8: Sizes (e.g. 100MB, 50ms) ────────────────────────────────
        // Unit expansion needs the number in words, so it runs only when the
        // profile expands units (which implies digits — see `TextProfile::validate`).
        if expand_units {
            let num = &self.number_normalizer;
            tracked.sub(re_size(), |caps| {
                num.normalize_size(caps.get(0).unwrap().as_str())
            });
        }

        // ── Phase 9: Dates and times ──────────────────────────────────────────
        // Must precede versions/ranges/numbers so "12.05.2024" and "14:30" are
        // read as calendar values instead of being torn apart by the version,
        // range, or bare-number phases. Runs after URLs/emails/IPs/paths
        // (Phases 7-8), whose regions are already consumed, so date-like
        // substrings inside them are not re-matched here.
        if expand_digits {
            let num = &self.number_normalizer;
            // normalize_date and normalize_time return their input unchanged on an
            // invalid calendar/clock value, so out-of-range matches become no-ops
            // and leave the region and its digits for the number phase.
            let normalize_date =
                |caps: &regex::Captures| num.normalize_date(caps.get(0).unwrap().as_str());
            tracked.sub(re_date_iso(), normalize_date);
            tracked.sub(re_date_dot(), normalize_date);
            tracked.sub(re_time(), |caps| {
                num.normalize_time(caps.get(0).unwrap().as_str())
            });
        }

        // ── Phase 9b: Percentage ranges (e.g. 10-20%) ────────────────────────
        // Must precede plain percentages: otherwise the trailing "20%" is
        // consumed alone, the range phase never sees "10-20", and the dash
        // is left bare (#112). The range reading already puts both bounds in
        // genitive, so the fixed genitive-plural "процентов" is always right
        // after "до <genitive>".
        if expand_units {
            let num = &self.number_normalizer;
            tracked.sub(re_percentage_range(), |caps| {
                let range = format!(
                    "{}-{}",
                    caps.get(1).unwrap().as_str(),
                    caps.get(2).unwrap().as_str()
                );
                let read = num.normalize_range(&range);
                if read == range {
                    // normalize_range refused the bounds (e.g. past i64):
                    // leave the whole match untouched, like the plain
                    // percentage phase does on unparsable input.
                    return caps.get(0).unwrap().as_str().to_string();
                }
                format!("{read} процентов")
            });
        }

        // ── Phase 10: Percentages ─────────────────────────────────────────────
        // Must precede versions: re_version matches a bare "12.5" inside "12.5%",
        // so running versions first would consume the number and leave a bare "%".
        if expand_units {
            let num = &self.number_normalizer;
            tracked.sub(re_percentage(), |caps| {
                num.normalize_percentage(caps.get(0).unwrap().as_str())
            });
        }

        // ── Phase 11: Ranges (e.g. 10-20) ────────────────────────────────────
        if expand_digits {
            let num = &self.number_normalizer;
            self.sub_number_phase(&mut tracked, re_range(), expand_units, |caps| {
                num.normalize_range(caps.get(0).unwrap().as_str())
            });
        }

        // ── Phase 12: Versions (e.g. v1.2.3) ─────────────────────────────────
        if expand_digits {
            let num = &self.number_normalizer;
            self.sub_number_phase(&mut tracked, re_version(), expand_units, |caps| {
                let v = caps.get(0).unwrap().as_str();
                if v.contains('.') {
                    num.normalize_version(v)
                } else {
                    v.to_string()
                }
            });
        }

        // ── Phase 12b: Leading-dot decimals (e.g. ".5" → "ноль точка пять") ───
        // Must run after versions (so "1.5" keeps its version reading) and
        // before operators/numbers (so the fraction digits are consumed
        // before the number phase — and its dot guard — runs).
        if expand_digits {
            let num = &self.number_normalizer;
            self.sub_number_phase(
                &mut tracked,
                re_leading_dot_decimal(),
                expand_units,
                |caps| {
                    let boundary = caps.get(1).unwrap().as_str();
                    let digits = caps.get(2).unwrap().as_str();
                    format!(
                        "{}{}",
                        boundary,
                        num.normalize_float(&format!("0.{digits}"))
                    )
                },
            );
        }

        // ── Phase 13: Operators ───────────────────────────────────────────────
        // Operators run before symbols so that multi-char operators like "=="
        // are matched before single "=".
        for op in TRACKED_OPERATOR_KEYS {
            let replacement = format!(" {} ", self.symbol_normalizer.normalize(op));
            tracked.replace(op, &replacement);
        }

        // ── Phase 14.5: Alphanumeric abbreviations ────────────────────────────
        // `IPv6` and `HTTP/2` are not prose words: the symbol and number phases
        // would leave the Latin prefix untouched and speak the rest in Russian
        // (`HTTPслэш два`). Read them as a whole, on both routes, so a Native
        // prose profile never emits a mixed-script fragment.
        {
            let abbrev = &self.abbrev_normalizer;
            let dictionary = &self.user_dictionary;
            let mark_stress = self.mark_stress;
            tracked.sub(re_alnum_abbrev(), |caps| {
                let token = caps.get(0).unwrap().as_str();
                dictionary
                    .resolve(&token.to_lowercase(), mark_stress)
                    .unwrap_or_else(|| abbrev.normalize(token))
            });
        }

        // ── Phase 14: Special symbols (Greek, math, arrows, tilde) ───────────
        for (symbol, replacement) in GREEK_LETTERS
            .iter()
            .chain(MATH_SYMBOLS.iter())
            .chain(ARROW_SYMBOLS.iter())
        {
            tracked.replace(symbol, &format!(" {} ", replacement));
        }
        // A bare `<`/`>` that survived the operator phase is markup to an
        // engine: TeraTTS reads the delimiters only as its own tag syntax and
        // rejects the segment, while the alphabet-gated engines drop the
        // character and lose the comparison or shell redirect it belonged to.
        // Multi-character operators are already consumed above, and native
        // code blocks sit behind placeholders restored after the narrating
        // phases, so only prose reaches this loop.
        // The reading comes from the shared symbol table, the same one the
        // inline-code and identifier normalizers use, so a wording change there
        // reaches prose too.
        for bracket in ["<", ">"] {
            let reading = self.symbol_normalizer.normalize(bracket);
            tracked.replace(bracket, &format!(" {reading} "));
        }

        // Tilde before a number means "approximately": ~46 → около 46.
        // We capture the digit(s) after the tilde and emit them after "около ".
        if expand_digits {
            tracked.sub(re_tilde_approx(), |caps| {
                format!("около {}", caps.get(1).unwrap().as_str())
            });
        }

        // ── Phase 14.9: Hyphenated dictionary keys ───────────────────────────
        // A hyphenated key is a whole-token reading, and the phase-15.5
        // pre-pass only ever sees its parts: resolve it here, ahead of the
        // identifier split, so it wins over its own parts and the built-in
        // tables on both the narrating and the native identifier route.
        self.process_hyphenated_dictionary_tracked(&mut tracked, &mut recorded);

        // ── Phase 15: Code identifiers ────────────────────────────────────────
        // A profile that routes identifiers `Native` keeps them verbatim for the
        // engine; the English phase below must not re-read them either, which is
        // why identifier routing implies prose routing (see `TextProfile`).
        if narrate_identifiers {
            let code = &self.code_normalizer;
            let user_dict = &self.user_dictionary;
            let mark_stress = self.mark_stress;
            tracked.sub(re_camel_lower(), |caps| {
                code.normalize_camel_case(caps.get(0).unwrap().as_str(), user_dict, mark_stress)
            });
            tracked.sub(re_pascal(), |caps| {
                code.normalize_camel_case(caps.get(0).unwrap().as_str(), user_dict, mark_stress)
            });
            tracked.sub(re_snake(), |caps| {
                code.normalize_snake_case(caps.get(0).unwrap().as_str(), user_dict, mark_stress)
            });
            tracked.sub(re_kebab(), |caps| {
                code.normalize_kebab_case(caps.get(0).unwrap().as_str(), user_dict, mark_stress)
            });
        } else {
            // The identifiers stay for the engine: annotate them. The prose scan
            // below would claim the same fragment, and `keep_specific_spans`
            // resolves that overlap in original coordinates.
            record_native(
                &mut tracked,
                SpanKind::CodeIdentifier,
                native_lang,
                find_identifier_ranges,
                &mut recorded,
            );
        }

        // ── Phase 15.5: User dictionary pre-pass ─────────────────────────────
        // After code-identifier splitting (single-token entries apply to
        // identifier parts, never whole identifiers; a hyphenated key was
        // resolved on the whole token in phase 14.9) and before English-word
        // resolution (user entries win over every built-in table). No-op for an
        // empty dictionary — output stays byte-identical.
        self.process_user_dictionary_tracked(&mut tracked, &mut recorded);

        // ── Phase 16: English words ───────────────────────────────────────────
        // Skipped entirely for a Latin-native profile: transliteration, the
        // built-in IT dictionary, abbreviation readings and single-letter names
        // are all the compensation a Russian-only engine needs.
        if narrate_latin {
            // A native identifier is the profile's decision and stays Latin even
            // when the user asked for Russian prose: the English block must not
            // re-read a fragment the identifier route left to the engine.
            self.process_english_tracked(&mut tracked, expand_digits, !narrate_identifiers);
        } else {
            record_native(
                &mut tracked,
                SpanKind::ProseLatin,
                native_lang,
                |text| {
                    re_english_words()
                        .find_iter(text)
                        .map(|m| (m.start(), m.end()))
                        .collect()
                },
                &mut recorded,
            );
            // A letter+digit token is identifier-like: the adapter sends it to
            // the engine's native language, not to the Russian number phase.
            record_native(
                &mut tracked,
                SpanKind::CodeIdentifier,
                native_lang,
                |text| {
                    re_mixed_alnum()
                        .find_iter(text)
                        .map(|m| (m.start(), m.end()))
                        .collect()
                },
                &mut recorded,
            );
        }

        // ── Phase 17: Numbers ─────────────────────────────────────────────────
        if expand_digits {
            self.process_numbers_tracked(&mut tracked, expand_units);
        }

        // ── Postprocess ───────────────────────────────────────────────────────
        tracked.sub(re_multi_spaces(), |_| " ".to_string());
        tracked.sub(re_space_before_punct(), |caps| {
            caps.get(1).unwrap().as_str().to_string()
        });
        tracked.sub(re_space_after_newline(), |_| "\n".to_string());
        tracked.sub(re_space_before_newline(), |_| "\n".to_string());

        let mut mapping = tracked.build_mapping();
        // The narrating phases are done, so the blocks the engine reads are
        // put back into the text; their mapping keeps the source block range.
        for block in native_code {
            let restored = mapping.substitute(&block.placeholder, &block.text);
            debug_assert!(
                restored,
                "native code block {:?} was lost before the mapping was built",
                block
            );
        }
        let result = mapping.transformed.trim().to_string();

        if result != mapping.transformed {
            // char_map is indexed by codepoints, so count whitespace in
            // codepoints too — `&str::len()` returns bytes and panics here
            // on multi-byte input (e.g. trailing space after Cyrillic).
            let leading_chars = mapping
                .transformed
                .chars()
                .take_while(|c| c.is_whitespace())
                .count();
            let trailing_chars = mapping
                .transformed
                .chars()
                .rev()
                .take_while(|c| c.is_whitespace())
                .count();
            let total = mapping.char_map.len();
            let end_idx = total - trailing_chars;
            let new_char_map = mapping.char_map[leading_chars..end_idx].to_vec();
            let final_mapping = CharMapping {
                original: mapping.original,
                transformed: result.clone(),
                char_map: new_char_map,
            };
            let spans = resolve_spans(&final_mapping.char_map, recorded);
            debug_assert_spans(&final_mapping, &spans);
            return ProcessedText {
                text: result,
                mapping: final_mapping,
                spans,
            };
        }

        let spans = resolve_spans(&mapping.char_map, recorded);
        debug_assert_spans(&mapping, &spans);
        ProcessedText {
            text: result,
            mapping,
            spans,
        }
    }

    // ── Private processing helpers ─────────────────────────────────────────────

    fn process_inline_code_tracked(&self, tracked: &mut TrackedText) {
        tracked.sub(re_inline_code(), |caps| {
            let code = caps.get(1).unwrap().as_str();

            // Pre-process Greek and special symbols
            let mut processed = code.to_string();
            let mut has_special = false;

            for (ch, repl) in GREEK_LETTERS.iter() {
                if processed.contains(*ch) {
                    processed = processed.replace(*ch, &format!(" {} ", repl));
                    has_special = true;
                }
            }
            for (ch, repl) in ARROW_SYMBOLS.iter() {
                if processed.contains(*ch) {
                    processed = processed.replace(*ch, &format!(" {} ", repl));
                    has_special = true;
                }
            }
            processed = processed.split_whitespace().collect::<Vec<_>>().join(" ");

            if has_special {
                return self.normalize_code_words(&processed);
            }

            let user_dict = &self.user_dictionary;
            if processed.contains('_') {
                self.code_normalizer
                    .normalize_snake_case(&processed, user_dict, self.mark_stress)
            } else if processed.contains('-') && !processed.starts_with('-') {
                self.code_normalizer
                    .normalize_kebab_case(&processed, user_dict, self.mark_stress)
            } else if processed.chars().skip(1).any(|c| c.is_uppercase())
                && processed.chars().any(|c| c.is_lowercase())
            {
                self.code_normalizer
                    .normalize_camel_case(&processed, user_dict, self.mark_stress)
            } else {
                self.normalize_code_words(&processed)
            }
        });
    }

    fn normalize_code_words(&self, code: &str) -> String {
        code.split_whitespace()
            .map(|word| {
                let lower = word.to_lowercase();
                // CodeIdentifierNormalizer.normalize_snake_case handles single words
                // (no underscores) correctly: it looks up the user dictionary,
                // then the CODE_WORDS dict, then transliterates.
                self.code_normalizer.normalize_snake_case(
                    &lower,
                    &self.user_dictionary,
                    self.mark_stress,
                )
            })
            .collect::<Vec<_>>()
            .join(" ")
    }

    fn process_markdown_tracked(&self, tracked: &mut TrackedText, expand_digits: bool) {
        tracked.sub(re_heading(), |_| String::new());
        // Blockquote markers are structure, not speech; leaving them in the
        // text also trips the TeraTTS tag validation on a leading `>`.
        tracked.sub(re_blockquote(), |_| String::new());

        // Markdown links: [text](url) → text (link text preserved for further normalization).
        //
        // Two-pass strip: first "[", then "](url)". Replacing the whole "[text](url)" at once
        // would assign every link-text character to the full "[text](url)" range, preventing
        // subsequent phases (CamelCase etc.) from processing those characters — TrackedText
        // skips regions already marked as replaced.
        {
            let snapshot = tracked.text().to_string();
            // Collect byte ranges of the "[" prefix and "](url)" suffix for each link,
            // in reverse document order so splicing later ranges first doesn't shift
            // the byte offsets of earlier ones.
            let mut link_ranges: Vec<(usize, usize, usize, usize)> = re_md_link_full()
                .captures_iter(&snapshot)
                .map(|caps| {
                    let full_m = caps.get(0).unwrap();
                    let text_m = caps.get(1).unwrap();
                    // "[" is a 1-byte ASCII char.
                    let bracket_start = full_m.start();
                    let bracket_end = bracket_start + 1; // just "["
                    // "](url)" starts right after the link text.
                    let suffix_start = text_m.end(); // byte after last char of link text
                    let suffix_end = full_m.end();
                    (bracket_start, bracket_end, suffix_start, suffix_end)
                })
                .collect();
            link_ranges.reverse();
            for (bracket_start, bracket_end, suffix_start, suffix_end) in link_ranges {
                // Remove "](url)" first (higher byte offset, so reverse order is correct).
                tracked.replace_byte_range(suffix_start, suffix_end, "");
                // A glued link brackets a word boundary (`CVE.[12]`); removing
                // the bracket without a separator would merge the link text
                // with the preceding token and hide the number from the
                // numbers phase. A space before the link text keeps the words
                // apart without disturbing links already preceded by one.
                let needs_separator = !snapshot[..bracket_start]
                    .chars()
                    .next_back()
                    .is_some_and(char::is_whitespace);
                tracked.replace_byte_range(
                    bracket_start,
                    bracket_end,
                    if needs_separator { " " } else { "" },
                );
            }
        }

        // Numbered lists: "1. " → "первое: ". Gated with the digit phases: the
        // marker is a number, and an engine that reads digits itself does not
        // need it spelled out.
        if !expand_digits {
            return;
        }
        let num = &self.number_normalizer;
        tracked.sub(re_md_list_number(), |caps| {
            let n: u32 = caps.get(1).unwrap().as_str().parse().unwrap_or(0);
            let ordinal = match n {
                1 => "первое".to_string(),
                2 => "второе".to_string(),
                3 => "третье".to_string(),
                4 => "четвёртое".to_string(),
                5 => "пятое".to_string(),
                6 => "шестое".to_string(),
                7 => "седьмое".to_string(),
                8 => "восьмое".to_string(),
                9 => "девятое".to_string(),
                10 => "десятое".to_string(),
                _ => num.normalize_number(&n.to_string()),
            };
            format!("{}:", ordinal)
        });
    }

    /// Whole-token user-dictionary pass for hyphenated keys (change
    /// `add-hyphenated-dictionary-keys`): replace a token that exactly matches
    /// a key containing a hyphen, case-insensitively, before the code-identifier
    /// phase splits kebab identifiers into parts.
    ///
    /// Only exact whole-token hits are replaced; everything else flows into the
    /// identifier phase unchanged, so part entries keep working and a
    /// hyphenated entry never rewrites a fragment of a longer identifier.
    fn process_hyphenated_dictionary_tracked(
        &mut self,
        tracked: &mut TrackedText,
        recorded: &mut Vec<RecordedSpan>,
    ) {
        if !self.user_dictionary.has_hyphenated_keys() {
            return;
        }
        let snapshot = tracked.text().to_string();
        let matches = self.dictionary_hits(&snapshot, re_user_dict_hyphenated());
        self.apply_dictionary_matches(tracked, recorded, matches);
    }

    /// User-dictionary pre-pass (change `user-dictionary`): replace tokens
    /// that exactly match an entry key, case-insensitively, with the entry's
    /// spoken form. Only exact hits are replaced — non-hits flow into the
    /// English-word phase unchanged, so built-in behavior and golden fixtures
    /// are unaffected.
    fn process_user_dictionary_tracked(
        &mut self,
        tracked: &mut TrackedText,
        recorded: &mut Vec<RecordedSpan>,
    ) {
        if self.user_dictionary.is_empty() {
            return;
        }
        let snapshot = tracked.text().to_string();
        // A script the dictionary cannot match costs no scan: the two candidate
        // patterns are disjoint, so a Latin-only dictionary never pays for the
        // Cyrillic pass and vice versa.
        let mut matches: Vec<(usize, usize, String)> = Vec::new();
        if self.user_dictionary.has_keys_in(KeyScript::Latin) {
            matches.extend(self.dictionary_hits(&snapshot, re_user_dict_token()));
        }
        if self.user_dictionary.has_keys_in(KeyScript::Cyrillic) {
            matches.extend(self.dictionary_hits(&snapshot, re_user_dict_token_cyrillic()));
        }
        // Two passes produce two sorted runs, not one: the replacement applies
        // in reverse order, so the merged list has to be sorted by start.
        matches.sort_by_key(|(start, _, _)| *start);
        self.apply_dictionary_matches(tracked, recorded, matches);
    }

    /// Record dictionary hits as override spans and replace them in the text.
    /// Shared by both dictionary passes, so "a hit is an override span plus a
    /// replacement" has one home.
    fn apply_dictionary_matches(
        &self,
        tracked: &mut TrackedText,
        recorded: &mut Vec<RecordedSpan>,
        matches: Vec<(usize, usize, String)>,
    ) {
        if matches.is_empty() {
            return;
        }
        let offsets = byte_to_char_offsets(tracked.text());
        let ranges: Vec<(usize, usize)> = matches.iter().map(|(s, e, _)| (*s, *e)).collect();
        record_byte_ranges(
            tracked,
            &offsets,
            SpanKind::DictionaryOverride,
            None,
            &ranges,
            recorded,
        );
        tracked.replace_byte_ranges(matches);
    }

    /// Exact case-insensitive dictionary hits of one candidate pattern, with the
    /// text to insert already resolved for the active stress mode.
    fn dictionary_hits(&self, text: &str, pattern: &Regex) -> Vec<(usize, usize, String)> {
        pattern
            .captures_iter(text)
            .filter_map(|caps| {
                let m = caps.get(0)?;
                let word_lower = m.as_str().to_lowercase();
                self.user_dictionary
                    .resolve(&word_lower, self.mark_stress)
                    .map(|to| (m.start(), m.end(), to))
            })
            .collect()
    }

    /// Reading of one pure-letter English run, shared by the pure-word scan
    /// and the letter/digit split so both paths resolve a run identically.
    fn english_word_reading(&mut self, word: &str) -> String {
        let word_lower = word.to_lowercase();

        // Priority: single letter by name → IT_TERMS →
        // composed inflection → abbreviations → transliterate
        use crate::pipeline::normalizers::english::IT_TERMS;
        if word.len() == 1 {
            // Lone letters are read by English letter name via the same
            // table as code identifiers ("x" → "икс"), not
            // transliterated, and are not tracked as unknown words.
            CodeIdentifierNormalizer::spell_abbreviation(word)
        } else if let Some(v) = IT_TERMS.get(word_lower.as_str()) {
            v.to_string()
        } else if let Some(composed) = self.english_normalizer.compose_inflection(word) {
            // Bounded inflection composition is attempted before the
            // all-uppercase abbreviation rule, so an all-caps inflected
            // form ("SUPPORTING") composes the curated stem reading
            // instead of being spelled letter by letter. The helper
            // fires only for a recognized suffix on a curated stem and
            // never records unknown words, so pure abbreviations
            // ("API", "HTML", "SQL", "JSON") keep their readings.
            composed
        } else if word.chars().all(|c| c.is_ascii_uppercase()) && word.len() >= 2 {
            if self.abbrev_normalizer.reads_as_word(word) {
                // Long vowel-rich all-caps tokens ("DEVELOPER",
                // "JAVASCRIPT") are words, not initialisms: spelling
                // them letter by letter is unreadable. They go through
                // the digraph transliteration and are tracked as
                // unknown, like any other transliterated word.
                self.english_normalizer.normalize(word, true)
            } else {
                self.abbrev_normalizer.normalize(word)
            }
        } else if crate::pipeline::normalizers::abbreviations::as_word()
            .contains_key(word_lower.as_str())
        {
            crate::pipeline::normalizers::abbreviations::as_word()[word_lower.as_str()].to_string()
        } else {
            self.english_normalizer.normalize(word, true)
        }
    }

    /// Reading of a token that mixes letters and digits.
    ///
    /// A curated whole-token reading wins (`mp3` → "эм пэ три"); otherwise the
    /// token is split into letter and digit runs and each run is resolved by
    /// the normalizer that owns it. The split happens here, after the
    /// identifier phase, so camelCase/snake_case identifiers are never torn
    /// apart by it.
    fn mixed_alnum_reading(&mut self, token: &str, expand_digits: bool) -> String {
        use crate::pipeline::normalizers::english::IT_TERMS;
        let lower = token.to_lowercase();
        if let Some(v) = IT_TERMS.get(lower.as_str()) {
            return v.to_string();
        }

        let chars: Vec<char> = token.chars().collect();
        let mut parts = Vec::new();
        let mut index = 0;
        while index < chars.len() {
            let digits = chars[index].is_ascii_digit();
            let mut end = index + 1;
            while end < chars.len() && chars[end].is_ascii_digit() == digits {
                end += 1;
            }
            let run: String = chars[index..end].iter().collect();
            if digits {
                if expand_digits {
                    parts.push(self.number_normalizer.normalize_number(&run));
                } else {
                    parts.push(run);
                }
            } else if run.len() == 1 {
                parts.push(CodeIdentifierNormalizer::spell_abbreviation(&run));
            } else if let Some(reading) = self.code_normalizer.builtin_reading(&run.to_lowercase())
            {
                parts.push(reading.to_string());
            } else {
                parts.push(self.english_normalizer.normalize(&run, true));
            }
            index = end;
        }
        parts.join(" ")
    }

    /// Transliterates English words, abbreviations and letter-digit tokens.
    ///
    /// `protect_native_identifiers` keeps the identifiers the profile left to
    /// the engine out of the match set when the user asked for Russian prose —
    /// the mixed mode. The ranges are taken from the same snapshot the matches
    /// are taken from, so the special-term rewrites above cannot shift them,
    /// and an identifier the user dictionary already partly rewrote leaves its
    /// remaining Latin parts to transliteration.
    fn process_english_tracked(
        &mut self,
        tracked: &mut TrackedText,
        expand_digits: bool,
        protect_native_identifiers: bool,
    ) {
        // Special programming language terms (C++, C#, F#) before general English processing.
        let special_terms: &[(&str, &str)] = &[
            ("C++", "си плюс плюс"),
            ("c++", "си плюс плюс"),
            ("C#", "си шарп"),
            ("c#", "си шарп"),
            ("F#", "эф шарп"),
            ("f#", "эф шарп"),
        ];
        for (term, replacement) in special_terms {
            tracked.replace(term, replacement);
        }

        // General English word handling via IT_TERMS, abbreviations, and transliteration.
        // Collect matches first, then process — avoids borrow issues with &mut self.
        let snapshot = tracked.text().to_string();
        let protected = if protect_native_identifiers {
            find_identifier_ranges(&snapshot)
        } else {
            Vec::new()
        };
        let mut matches: Vec<(usize, usize, String)> = re_english_words()
            .captures_iter(&snapshot)
            .map(|caps| {
                let m = caps.get(0).unwrap();
                (m.start(), m.end(), self.english_word_reading(m.as_str()))
            })
            .collect();
        matches.extend(re_mixed_alnum().captures_iter(&snapshot).map(|caps| {
            let m = caps.get(0).unwrap();
            (
                m.start(),
                m.end(),
                self.mixed_alnum_reading(m.as_str(), expand_digits),
            )
        }));
        matches.retain(|(start, end, _)| !overlaps_ranges(&protected, *start, *end));
        matches.sort_unstable_by_key(|(start, _, _)| *start);

        tracked.replace_byte_ranges(matches);
    }

    /// Runs a digit phase, skipping matches that belong to a unit construct
    /// when the unit phases are gated off (`units: Native`).
    ///
    /// Without this the skipped unit phases would leave their number to the
    /// digit phases below, which read "50 %" or "1.5 GB" as a bare number and
    /// strand the unit.
    fn sub_number_phase(
        &self,
        tracked: &mut TrackedText,
        pattern: &Regex,
        expand_units: bool,
        mut replacer: impl FnMut(&regex::Captures) -> String,
    ) {
        if expand_units {
            tracked.sub(pattern, |caps| replacer(caps));
            return;
        }
        let snapshot = tracked.text().to_string();
        tracked.sub(pattern, |caps| {
            let m = caps.get(0).unwrap();
            if unit_owns(&snapshot, m.end()) {
                m.as_str().to_string()
            } else {
                replacer(caps)
            }
        });
    }

    fn process_numbers_tracked(&self, tracked: &mut TrackedText, expand_units: bool) {
        // Effective pattern: `(?<![.])(\d+)(?![.]\d)` on top of the Unicode-aware
        // `\b\d+\b` from re_number(). The regex crate lacks lookbehind/lookahead,
        // and `\b` already rejects digits, letters, and underscore next to the
        // match, so the manual guard below only needs to exclude the digit
        // separator '.': a dot directly followed by another digit (float and
        // version separators, owned by earlier pipeline phases; bare
        // integer-less decimals like ".5" are consumed by the leading-dot
        // decimal phase, so a surviving "dot + digit" fragment belongs to a
        // dotted label like "example.5"). A dot followed
        // by whitespace, end of text, or a non-digit is terminal punctuation,
        // not a separator — the number before it is read normally.
        let snapshot = tracked.text().to_string();

        let matches: Vec<(usize, usize, String)> = re_number()
            .find_iter(&snapshot)
            .filter_map(|m| {
                let start = m.start();
                let end = m.end();

                let preceded_ok = start == 0 || !snapshot[..start].ends_with('.');
                // A percent sign is not a word character, so `\b` does not
                // protect it, and a size unit may follow a space: with units
                // left to the engine, the number phase would read the "50" of
                // "50 %" or the "1.5" of "1.5 GB" and strand the unit. The
                // skipped unit phases own that fragment, so the number stays
                // with it.
                let unit_follows = unit_owns(&snapshot, end);
                let followed_ok = (expand_units || !unit_follows)
                    && (end >= snapshot.len()
                        || !snapshot[end..].starts_with('.')
                        || !snapshot[end + 1..].starts_with(|c: char| c.is_ascii_digit()));

                if preceded_ok && followed_ok {
                    let replacement = self.number_normalizer.normalize_number(m.as_str());
                    Some((start, end, replacement))
                } else {
                    None
                }
            })
            .collect();

        tracked.replace_byte_ranges(matches);
    }
}

impl Default for TTSPipeline {
    fn default() -> Self {
        Self::new()
    }
}

impl TTSPipeline {
    /// Normalized text under the reference profile, mapping discarded.
    ///
    /// Test-only: production callers need the mapping for word highlighting, so
    /// no bare `process()` exists on the public surface.
    #[cfg(test)]
    fn process_text(&mut self, input: &str) -> String {
        self.process_for_engine(input, &profiles::SILERO_PROFILE)
            .text
    }
}

// ── Tests ──────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests;
