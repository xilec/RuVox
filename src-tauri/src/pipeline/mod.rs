pub mod constants;
pub mod normalizers;
pub mod tracked_text;

use std::sync::OnceLock;

use regex::Regex;

use crate::pipeline::constants::{ARROW_SYMBOLS, GREEK_LETTERS, MATH_SYMBOLS};
use crate::pipeline::normalizers::abbreviations::AbbreviationNormalizer;
use crate::pipeline::normalizers::code::CodeIdentifierNormalizer;
use crate::pipeline::normalizers::code_blocks::{CodeBlockHandler, CodeBlockMode};
use crate::pipeline::normalizers::english::EnglishNormalizer;
use crate::pipeline::normalizers::numbers::NumberNormalizer;
use crate::pipeline::normalizers::symbols::SymbolNormalizer;
use crate::pipeline::normalizers::urls::{URLPathNormalizer, is_known_tld};
use crate::pipeline::tracked_text::{CharMapping, TrackedText};

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

fn re_inline_code() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"`([^`\n]+)`").expect("valid regex"))
}

fn re_heading() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"(?m)^#{1,6}\s+").expect("valid regex"))
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

fn re_english_words() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    // Single letters included: a lone "x" or "I" must also become speakable
    // Cyrillic (read by letter name in process_english_tracked).
    RE.get_or_init(|| Regex::new(r"\b([A-Za-z]+)\b").expect("valid regex"))
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
        }
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

    /// Process text for TTS. Returns normalized text without position mapping.
    pub fn process(&mut self, input: &str) -> String {
        let (result, _) = self.process_with_char_mapping(input);
        result
    }

    /// Process text for TTS with precise character-level mapping.
    pub fn process_with_char_mapping(&mut self, input: &str) -> (String, CharMapping) {
        if input.is_empty() {
            return (
                String::new(),
                CharMapping {
                    original: String::new(),
                    transformed: String::new(),
                    char_map: Vec::new(),
                },
            );
        }

        self.english_normalizer.clear_unknown_words();

        let mut tracked = TrackedText::new(input);

        // ── BOM removal ───────────────────────────────────────────────────────
        if tracked.text().starts_with('\u{feff}') {
            tracked.replace("\u{feff}", "");
        }

        // ── Phase 1: Code blocks (must run before space/dash normalization) ───
        self.code_block_handler.process(&mut tracked);

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
            return (
                String::new(),
                CharMapping {
                    original: input.to_string(),
                    transformed: String::new(),
                    char_map: Vec::new(),
                },
            );
        }

        // ── Phase 5: Inline code ─────────────────────────────────────────────
        self.process_inline_code_tracked(&mut tracked);

        // ── Phase 6: Markdown structure ───────────────────────────────────────
        self.process_markdown_tracked(&mut tracked);

        // ── Phase 7: URLs ─────────────────────────────────────────────────────
        {
            let num = &self.number_normalizer;
            let eng = &self.english_normalizer;
            let url_norm = URLPathNormalizer::new(eng, num);
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
            tracked.sub(re_path(), |caps| {
                let path = caps.get(1).unwrap().as_str();
                if path.contains('/') && (path.starts_with('/') || path.starts_with('~')) {
                    url_norm.normalize_filepath(path)
                } else {
                    path.to_string()
                }
            });
        }

        // ── Phase 8: Sizes (e.g. 100MB, 50ms) ────────────────────────────────
        {
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
        {
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
        {
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
        {
            let num = &self.number_normalizer;
            tracked.sub(re_percentage(), |caps| {
                num.normalize_percentage(caps.get(0).unwrap().as_str())
            });
        }

        // ── Phase 11: Ranges (e.g. 10-20) ────────────────────────────────────
        {
            let num = &self.number_normalizer;
            tracked.sub(re_range(), |caps| {
                num.normalize_range(caps.get(0).unwrap().as_str())
            });
        }

        // ── Phase 12: Versions (e.g. v1.2.3) ─────────────────────────────────
        {
            let num = &self.number_normalizer;
            tracked.sub(re_version(), |caps| {
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
        {
            let num = &self.number_normalizer;
            tracked.sub(re_leading_dot_decimal(), |caps| {
                let boundary = caps.get(1).unwrap().as_str();
                let digits = caps.get(2).unwrap().as_str();
                format!(
                    "{}{}",
                    boundary,
                    num.normalize_float(&format!("0.{digits}"))
                )
            });
        }

        // ── Phase 13: Operators ───────────────────────────────────────────────
        // Operators run before symbols so that multi-char operators like "=="
        // are matched before single "=".
        for op in TRACKED_OPERATOR_KEYS {
            let replacement = format!(" {} ", self.symbol_normalizer.normalize(op));
            tracked.replace(op, &replacement);
        }

        // ── Phase 14: Special symbols (Greek, math, arrows, tilde) ───────────
        for (symbol, replacement) in GREEK_LETTERS
            .iter()
            .chain(MATH_SYMBOLS.iter())
            .chain(ARROW_SYMBOLS.iter())
        {
            tracked.replace(symbol, &format!(" {} ", replacement));
        }
        // Tilde before a number means "approximately": ~46 → около 46.
        // We capture the digit(s) after the tilde and emit them after "около ".
        tracked.sub(re_tilde_approx(), |caps| {
            format!("около {}", caps.get(1).unwrap().as_str())
        });

        // ── Phase 15: Code identifiers ────────────────────────────────────────
        {
            let code = &self.code_normalizer;
            tracked.sub(re_camel_lower(), |caps| {
                code.normalize_camel_case(caps.get(0).unwrap().as_str())
            });
            tracked.sub(re_pascal(), |caps| {
                code.normalize_camel_case(caps.get(0).unwrap().as_str())
            });
            tracked.sub(re_snake(), |caps| {
                code.normalize_snake_case(caps.get(0).unwrap().as_str())
            });
            tracked.sub(re_kebab(), |caps| {
                code.normalize_kebab_case(caps.get(0).unwrap().as_str())
            });
        }

        // ── Phase 16: English words ───────────────────────────────────────────
        self.process_english_tracked(&mut tracked);

        // ── Phase 17: Numbers ─────────────────────────────────────────────────
        self.process_numbers_tracked(&mut tracked);

        // ── Postprocess ───────────────────────────────────────────────────────
        tracked.sub(re_multi_spaces(), |_| " ".to_string());
        tracked.sub(re_space_before_punct(), |caps| {
            caps.get(1).unwrap().as_str().to_string()
        });
        tracked.sub(re_space_after_newline(), |_| "\n".to_string());
        tracked.sub(re_space_before_newline(), |_| "\n".to_string());

        let mapping = tracked.build_mapping();
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
            return (result, final_mapping);
        }

        (result, mapping)
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

            if processed.contains('_') {
                self.code_normalizer.normalize_snake_case(&processed)
            } else if processed.contains('-') && !processed.starts_with('-') {
                self.code_normalizer.normalize_kebab_case(&processed)
            } else if processed.chars().skip(1).any(|c| c.is_uppercase())
                && processed.chars().any(|c| c.is_lowercase())
            {
                self.code_normalizer.normalize_camel_case(&processed)
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
                // (no underscores) correctly: it looks up CODE_WORDS dict then transliterates.
                self.code_normalizer.normalize_snake_case(&lower)
            })
            .collect::<Vec<_>>()
            .join(" ")
    }

    fn process_markdown_tracked(&self, tracked: &mut TrackedText) {
        tracked.sub(re_heading(), |_| String::new());

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
                // Remove the leading "[" (lower byte offset, now safely independent).
                tracked.replace_byte_range(bracket_start, bracket_end, "");
            }
        }

        // Numbered lists: "1. " → "первое: "
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

    fn process_english_tracked(&mut self, tracked: &mut TrackedText) {
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
        let matches: Vec<(usize, usize, String)> = re_english_words()
            .captures_iter(&snapshot)
            .map(|caps| {
                let m = caps.get(0).unwrap();
                let word = m.as_str();
                let word_lower = word.to_lowercase();

                // Priority: single letter by name → IT_TERMS → custom terms →
                // abbreviations → transliterate
                use crate::pipeline::normalizers::english::IT_TERMS;
                let replacement = if word.len() == 1 {
                    // Lone letters are read by English letter name via the same
                    // table as code identifiers ("x" → "икс"), not
                    // transliterated, and are not tracked as unknown words.
                    CodeIdentifierNormalizer::spell_abbreviation(word)
                } else if let Some(v) = IT_TERMS.get(word_lower.as_str()) {
                    v.to_string()
                } else if word.chars().all(|c| c.is_ascii_uppercase()) && word.len() >= 2 {
                    self.abbrev_normalizer.normalize(word)
                } else if crate::pipeline::normalizers::abbreviations::as_word()
                    .contains_key(word_lower.as_str())
                {
                    crate::pipeline::normalizers::abbreviations::as_word()[word_lower.as_str()]
                        .to_string()
                } else {
                    self.english_normalizer.normalize(word, true)
                };
                (m.start(), m.end(), replacement)
            })
            .collect();

        tracked.replace_byte_ranges(matches);
    }

    fn process_numbers_tracked(&self, tracked: &mut TrackedText) {
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
                let followed_ok = end >= snapshot.len()
                    || !snapshot[end..].starts_with('.')
                    || !snapshot[end + 1..].starts_with(|c: char| c.is_ascii_digit());

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

// ── Tests ──────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Unit-level sanity tests ────────────────────────────────────────────────

    #[test]
    fn pipeline_empty_input() {
        let mut p = TTSPipeline::new();
        assert_eq!(p.process(""), "");
    }

    #[test]
    fn pipeline_plain_russian() {
        let mut p = TTSPipeline::new();
        let input = "Привет мир";
        assert_eq!(p.process(input), "Привет мир");
    }

    #[test]
    fn pipeline_invalid_time_falls_through_to_numbers() {
        // Invariant: normalize_time returns an out-of-range clock value unchanged,
        // so the time phase is a no-op and does not consume the region. TrackedText
        // skips no-op replacements, leaving the digits for the number phase to read.
        let mut p = TTSPipeline::new();
        assert_eq!(p.process("В 25:00 встреча"), "В двадцать пять:ноль встреча");
    }

    #[test]
    fn pipeline_number_inline() {
        let mut p = TTSPipeline::new();
        assert_eq!(p.process("Версия 3"), "Версия три");
    }

    #[test]
    fn pipeline_mermaid_marker() {
        let mut p = TTSPipeline::new();
        let input = "```mermaid\ngraph TD\nA-->B\n```";
        assert_eq!(p.process(input), "Тут мермэйд диаграмма");
    }

    #[test]
    fn pipeline_code_block_mode_default_is_brief() {
        let p = TTSPipeline::new();
        assert_eq!(p.code_block_mode(), CodeBlockMode::Brief);
    }

    #[test]
    fn pipeline_code_block_mode_switch_changes_narration() {
        let mut p = TTSPipeline::new();
        let input = "```python\nprint('hi')\n```";
        assert_eq!(p.process(input), "далее следует пример кода на пайтон");
        p.set_code_block_mode(CodeBlockMode::Full);
        assert_eq!(
            p.process(input),
            "принт открывающая скобка хи закрывающая скобка"
        );
    }

    #[test]
    fn pipeline_char_mapping_nonempty() {
        let mut p = TTSPipeline::new();
        let input = "getUserData";
        let (result, mapping) = p.process_with_char_mapping(input);
        assert!(!result.is_empty());
        assert_eq!(mapping.original, input);
        assert!(!mapping.char_map.is_empty());
    }

    #[test]
    fn pipeline_process_vs_char_mapping_consistent() {
        let mut p = TTSPipeline::new();
        let input = "Файл весит 100MB и версия v2.3.1.";
        let direct = p.process(input);
        let (mapped, _) = p.process_with_char_mapping(input);
        assert_eq!(direct, mapped);
    }

    // Pre-normalization (quotes, dashes, whitespace collapse, tilde) is covered
    // solely by golden fixtures: quotes_dashes, whitespace_newline_collapse,
    // tilde_approx (src-tauri/tests/fixtures/pipeline/).

    // ── Numbers adjacent to letters ──────────────────────────────────────────

    #[test]
    fn pipeline_number_before_terminal_dot_is_read() {
        // Regression (#111): the number guard treated every trailing dot as a
        // decimal/version separator, so a number before a sentence-ending
        // period was left as unreadable digits.
        let mut p = TTSPipeline::new();
        assert_eq!(p.process("Встреча в 5."), "Встреча в пять.");
        assert_eq!(
            p.process("Сначала пункт 3. Потом пункт 4."),
            "Сначала пункт три. Потом пункт четыре."
        );
    }

    #[test]
    fn pipeline_dot_between_digits_stays_separator() {
        // A float-like fragment keeps its version-path reading ("точка …"),
        // not two independently expanded numbers — the tightened guard must
        // not change this path.
        let mut p = TTSPipeline::new();
        assert_eq!(
            p.process("Остаток 3.14 в конце"),
            "Остаток три точка четырнадцать в конце"
        );
    }

    #[test]
    fn pipeline_leading_dot_decimal_is_read_as_zero_point() {
        // Regression (#147): the number guard skipped any digit directly
        // preceded by a dot, so a bare decimal fraction survived as
        // unreadable digits. Read it as a proper decimal, identical to the
        // explicit-zero form.
        let mut p = TTSPipeline::new();
        assert_eq!(p.process("Вес .5 кг"), "Вес ноль точка пять кг");
        assert_eq!(
            p.process(".75 вероятность"),
            "ноль точка семь пять вероятность"
        );
    }

    #[test]
    fn pipeline_leading_dot_decimal_after_letter_is_untouched() {
        // A dot preceded by a letter belongs to a dotted label ("example.5"),
        // not a decimal — the leading-dot phase must skip it, and the number
        // phase guard keeps the digit unexpanded as before.
        let mut p = TTSPipeline::new();
        assert_eq!(p.process("файл example.5"), "файл экзампл.5");
    }

    #[test]
    fn pipeline_float_keeps_version_reading() {
        // "1.5" is consumed by the version phase before the leading-dot
        // phase runs — no double-consume, no "ноль" prefix.
        let mut p = TTSPipeline::new();
        assert_eq!(p.process("Точность 1.5"), "Точность один точка пять");
    }

    #[test]
    fn pipeline_percentage_range_is_read_as_a_unit() {
        // Regression (#112): after the percentages-before-ranges phase swap
        // the trailing "20%" was consumed alone, leaving "10-" with a bare
        // dash. The percentage-range phase must claim "N-M%" as one region.
        let mut p = TTSPipeline::new();
        assert_eq!(
            p.process("Рост на 10-20% за квартал"),
            "Рост на от десяти до двадцати процентов за квартал"
        );
        // Whitespace around the dash and before "%" is tolerated.
        assert_eq!(p.process("10 - 20 %"), "от десяти до двадцати процентов");
    }

    #[test]
    fn pipeline_percentage_range_glued_to_letter_not_expanded() {
        // A digit glued to a letter is not a range bound: "\b" rejects the
        // percentage-range match, so no "от … до …" reading. The plain
        // percentage phase still claims "20%" on its own (the "-" provides
        // its boundary) — pre-existing semantics, pinned here so a future
        // regex change cannot quietly turn this into a range.
        let mut p = TTSPipeline::new();
        assert_eq!(
            p.process("метрика x10-20%"),
            "метрика x10-двадцать процентов"
        );
    }

    #[test]
    fn pipeline_plain_range_and_percentage_unchanged() {
        // The new phase must not steal plain ranges or plain percentages.
        let mut p = TTSPipeline::new();
        assert_eq!(
            p.process("диапазон 10-20"),
            "диапазон от десяти до двадцати"
        );
        assert_eq!(p.process("загрузка 20%"), "загрузка двадцать процентов");
    }

    #[test]
    fn pipeline_number_adjacent_to_letter_not_expanded() {
        // Digits glued to a Latin letter belong to the code-identifier phase,
        // so the number phase must leave leftovers like v1 / x2 / app2 alone.
        let mut p = TTSPipeline::new();
        assert_eq!(p.process("Запусти v1 сейчас"), "Запусти v1 сейчас");
        assert_eq!(p.process("Координата x2 равна"), "Координата x2 равна");
        assert_eq!(p.process("Файл app2 лежит тут"), "Файл app2 лежит тут");
    }

    #[test]
    fn trim_fixup_handles_multibyte_input() {
        // Regression: char_map is indexed by codepoints, but the trim
        // fixup used &str::len() (bytes), so the post-trim slice
        // panicked whenever the surrounding text contained multi-byte
        // characters (any Cyrillic / non-ASCII).
        let mut pipeline = TTSPipeline::new();
        let (result, mapping) = pipeline.process_with_char_mapping("  привет мир  ");
        assert!(!result.starts_with(char::is_whitespace));
        assert!(!result.ends_with(char::is_whitespace));
        assert_eq!(mapping.char_map.len(), result.chars().count());
    }

    #[test]
    fn pipeline_english_word_embedded_in_earlier_word() {
        // Regression: replacements were applied via literal `TrackedText::replace`,
        // which hit the shorter word embedded in an earlier longer token
        // ("use" inside "user"), leaving Latin debris for Silero.
        let mut p = TTSPipeline::new();
        assert_eq!(p.process("user use"), "юзер юз");
        assert_eq!(p.process("use user"), "юз юзер");
        assert_eq!(p.process("tests test"), "тестс тест");
        assert_eq!(p.process("test tests"), "тест тестс");
    }

    #[test]
    fn pipeline_number_embedded_in_earlier_number() {
        // Regression (#75): same literal-replace bug in the numbers phase —
        // "1" replaced inside "10", "42" inside "142".
        let mut p = TTSPipeline::new();
        assert_eq!(p.process("10:1"), "десять:один");
        assert_eq!(
            p.process("Счёт 10:1 в нашу пользу."),
            "Счёт десять:один в нашу пользу."
        );
        assert_eq!(p.process("142 42"), "сто сорок два сорок два");
        assert_eq!(p.process("42 142"), "сорок два сто сорок два");
    }

    #[test]
    fn pipeline_single_latin_letters_read_by_name() {
        // Lone letters used to stay Latin (re_english_words required 2+
        // letters) and were silently dropped by Silero.
        let mut p = TTSPipeline::new();
        assert_eq!(
            p.process("Переменная x равна 5"),
            "Переменная икс равна пять"
        );
        assert_eq!(p.process("пункты a и I"), "пункты эй и ай");
    }

    #[test]
    fn pipeline_single_letters_not_tracked_as_unknown() {
        // Letter-name spelling is a dictionary lookup, not a transliteration
        // fallback — it must not enter the unknown-words map.
        let mut p = TTSPipeline::new();
        p.process("Переменная x равна 5");
        assert!(p.english_normalizer.get_unknown_words().is_empty());
    }

    // ── Scheme-less URLs ─────────────────────────────────────────────────────

    #[test]
    fn pipeline_schemeless_www_domain() {
        let mut p = TTSPipeline::new();
        assert_eq!(
            p.process("Сайт www.example.com недоступен"),
            "Сайт ввв точка экзампл точка ком недоступен"
        );
    }

    #[test]
    fn pipeline_schemeless_bare_domain_with_path() {
        let mut p = TTSPipeline::new();
        assert_eq!(
            p.process("документация на docs.python.org/3/tutorial"),
            "документация на докс точка пайтон точка орг слэш три слэш тьюториал"
        );
    }

    #[test]
    fn pipeline_schemeless_domain_keeps_sentence_punctuation() {
        // The sentence-ending dot is not part of the domain and must not be
        // read as "точка".
        let mut p = TTSPipeline::new();
        assert_eq!(
            p.process("Смотри example.com, там документация."),
            "Смотри экзампл точка ком, там документация."
        );
    }

    #[test]
    fn pipeline_schemeless_skips_email_domain() {
        // The email pass consumes the whole address first; the domain inside
        // must not be re-matched as a bare domain.
        let mut p = TTSPipeline::new();
        assert_eq!(
            p.process("Пишите на user@example.com"),
            "Пишите на юзер собака экзампл точка ком"
        );
    }

    #[test]
    fn pipeline_schemeless_false_positive_guards() {
        // Filenames (suffix not in TLD_MAP) and versions (numeric last label)
        // must not be detected as bare domains.
        let mut p = TTSPipeline::new();
        assert_eq!(
            p.process("открой file.txt и config.yaml"),
            "открой файл.тэкст и конфиг.ямл"
        );
        assert_eq!(p.process("версия 1.2.3"), "версия один точка два точка три");
    }

    #[test]
    fn pipeline_schemeless_skips_path_internal_segment() {
        // "site.dev" inside a file path is a directory, not a domain.
        let mut p = TTSPipeline::new();
        assert_eq!(
            p.process("Файл /home/site.dev/main.py лежит тут"),
            "Файл слэш хоум слэш сайт точка дев слэш мэйн точка пи лежит тут"
        );
        // Same guard for Windows-style separators: "app.dev" after a
        // backslash must not be read as a domain. (Backslash paths are not
        // matched by re_path today at all — pre-existing gap, out of scope.)
        assert_eq!(
            p.process(r"путь C:\projects\app.dev\main.py тут"),
            "путь си:\\проджектс\\апп.дев\\мэйн.пи тут"
        );
    }
}
