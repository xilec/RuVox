pub mod constants;
pub mod html_extractor;
pub mod normalizers;
pub mod tracked_text;

use std::sync::OnceLock;

use regex::Regex;

use crate::pipeline::constants::{ARROW_SYMBOLS, GREEK_LETTERS, MATH_SYMBOLS};
use crate::pipeline::normalizers::abbreviations::AbbreviationNormalizer;
use crate::pipeline::normalizers::code::CodeIdentifierNormalizer;
use crate::pipeline::normalizers::code_blocks::CodeBlockHandler;
use crate::pipeline::normalizers::english::EnglishNormalizer;
use crate::pipeline::normalizers::numbers::NumberNormalizer;
use crate::pipeline::normalizers::symbols::SymbolNormalizer;
use crate::pipeline::normalizers::urls::URLPathNormalizer;
use crate::pipeline::tracked_text::{CharMapping, TrackedText};

// ── Static compiled regexes ───────────────────────────────────────────────────

fn re_url() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r#"https?://[^\s<>"'\)]+|ftp://[^\s<>"'\)]+|ssh://[^\s<>"'\)]+|git://[^\s<>"'\)]+"#)
            .expect("valid regex")
    })
}

fn re_email() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"[a-zA-Z0-9._%+\-]+@[a-zA-Z0-9.\-]+\.[a-zA-Z]{2,}").expect("valid regex")
    })
}

fn re_ip() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"\b(\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3})\b").expect("valid regex")
    })
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

fn re_version() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(
            r"(?i)\bv?(\d+\.\d+(?:\.\d+)?(?:-(?:alpha|beta|rc|dev|stable|release)\d*)?)\b",
        )
        .expect("valid regex")
    })
}

fn re_range() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"\b(\d+)\s*-\s*(\d+)\b").expect("valid regex"))
}

fn re_percentage() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"\b(\d+(?:\.\d+)?)\s*%").expect("valid regex"))
}

fn re_inline_code() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"`([^`\n]+)`").expect("valid regex"))
}

fn re_heading() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"(?m)^#{1,6}\s+").expect("valid regex")
    })
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
    RE.get_or_init(|| Regex::new(r"\b([A-Za-z][A-Za-z]+)\b").expect("valid regex"))
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
    // Match ~<optional spaces><digit> — capture the digit to preserve it in replacement.
    // Regex crate does not support lookahead, so we consume the digit and re-emit it.
    RE.get_or_init(|| Regex::new(r"~(\s*\d)").expect("valid regex"))
}

// ── Multi-char operators processed in tracked mode (longest first) ────────────

/// Operators handled by SymbolNormalizer, processed longest-first to avoid
/// partial matches (e.g. "===" must be checked before "==").
/// Mirrors legacy `_TRACKED_OPERATOR_KEYS` — single `=` is intentionally excluded
/// (it would corrupt math formulas like "α = β").
const TRACKED_OPERATOR_KEYS: &[&str] = &[
    "===", "!==", "->", "=>", ">=", "<=", "!=", "==", "&&", "||",
];

// ── TTSPipeline ────────────────────────────────────────────────────────────────

/// Main pipeline for TTS text preprocessing.
///
/// Owns all normalizer instances. Processing phases follow the exact order
/// defined in legacy `pipeline.py::process_with_char_mapping`.
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
            // Legacy default is "full" (read code content, not brief description).
            code_block_handler: CodeBlockHandler::with_mode(
                crate::pipeline::normalizers::code_blocks::CodeBlockMode::Full,
            ),
            code_normalizer: CodeIdentifierNormalizer::new(),
        }
    }

    /// Process text for TTS. Returns normalized text without position mapping.
    pub fn process(&mut self, input: &str) -> String {
        let (result, _) = self.process_with_char_mapping(input);
        result
    }

    /// Process text for TTS with precise character-level mapping.
    ///
    /// Phase order mirrors legacy pipeline.py::process_with_char_mapping.
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
            tracked.sub(re_url(), |caps| url_norm.normalize_url(caps.get(0).unwrap().as_str()));
            tracked.sub(re_email(), |caps| {
                url_norm.normalize_email(caps.get(0).unwrap().as_str())
            });
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

        // ── Phase 9: Versions (e.g. v1.2.3) ──────────────────────────────────
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

        // ── Phase 10: Ranges (e.g. 10-20) ────────────────────────────────────
        {
            let num = &self.number_normalizer;
            tracked.sub(re_range(), |caps| {
                num.normalize_range(caps.get(0).unwrap().as_str())
            });
        }

        // ── Phase 11: Percentages ─────────────────────────────────────────────
        {
            let num = &self.number_normalizer;
            tracked.sub(re_percentage(), |caps| {
                num.normalize_percentage(caps.get(0).unwrap().as_str())
            });
        }

        // ── Phase 12: Operators ───────────────────────────────────────────────
        // Phase order mirrors legacy pipeline.py: operators before symbols so that
        // multi-char operators like "==" are matched before single "=".
        for op in TRACKED_OPERATOR_KEYS {
            let replacement = format!(" {} ", self.symbol_normalizer.normalize(op));
            tracked.replace(op, &replacement);
        }

        // ── Phase 13: Special symbols (Greek, math, arrows, tilde) ───────────
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

        // ── Phase 14: Code identifiers ────────────────────────────────────────
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

        // ── Phase 15: English words ───────────────────────────────────────────
        self.process_english_tracked(&mut tracked);

        // ── Phase 16: Numbers ─────────────────────────────────────────────────
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
            let leading = mapping.transformed.len() - mapping.transformed.trim_start().len();
            let trailing = mapping.transformed.len() - mapping.transformed.trim_end().len();
            let chars: Vec<char> = mapping.transformed.chars().collect();
            let leading_chars = chars[..leading].iter().collect::<String>().chars().count();
            let trailing_chars = if trailing > 0 {
                chars[mapping.transformed.len() - trailing..]
                    .iter()
                    .collect::<String>()
                    .chars()
                    .count()
            } else {
                0
            };
            let total = mapping.char_map.len();
            let end_idx = if trailing_chars > 0 {
                total - trailing_chars
            } else {
                total
            };
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
        // Mirrors legacy Python which strips "[" and "](url)" in two separate passes so that
        // each link-text character retains its exact original-text position. Replacing the
        // whole "[text](url)" at once would assign every replacement character to the full
        // "[text](url)" range, preventing subsequent phases (CamelCase etc.) from processing
        // those characters (TrackedText skips regions that are already marked as replaced).
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

                // Priority: IT_TERMS → custom terms → abbreviations → transliterate
                use crate::pipeline::normalizers::english::IT_TERMS;
                let replacement = if let Some(v) = IT_TERMS.get(word_lower.as_str()) {
                    v.to_string()
                } else if word.chars().all(|c| c.is_ascii_uppercase()) && word.len() >= 2 {
                    self.abbrev_normalizer.normalize(word)
                } else if crate::pipeline::normalizers::abbreviations::as_word()
                    .contains_key(word_lower.as_str())
                {
                    crate::pipeline::normalizers::abbreviations::as_word()
                        [word_lower.as_str()]
                    .to_string()
                } else {
                    self.english_normalizer.normalize(word, true)
                };
                (m.start(), m.end(), replacement)
            })
            .collect();

        // Apply replacements via TrackedText in reverse order.
        for (start, end, replacement) in matches.into_iter().rev() {
            let original = &snapshot[start..end];
            if replacement != original {
                tracked.replace(original, &replacement);
            }
        }
    }

    fn process_numbers_tracked(&self, tracked: &mut TrackedText) {
        // Collect matches with context to replicate legacy `(?<![.\d])(\d+)(?![.\d]|[a-zA-Zа-яА-Я])`.
        // The regex crate lacks lookbehind/lookahead; we apply context check in the closure.
        let snapshot = tracked.text().to_string();
        let bytes = snapshot.as_bytes();

        let matches: Vec<(usize, usize, String)> = re_number()
            .find_iter(&snapshot)
            .filter_map(|m| {
                let start = m.start();
                let end = m.end();

                // Check char before: must not be '.' or digit
                let preceded_ok = start == 0 || {
                    let prev = snapshot[..start].chars().next_back().unwrap();
                    prev != '.' && !prev.is_ascii_digit()
                };

                // Check char after: must not be '.', digit, or Latin/Cyrillic letter
                let followed_ok = end >= snapshot.len() || {
                    let next_byte = bytes[end];
                    let next_str = &snapshot[end..];
                    let next_ch = next_str.chars().next().unwrap();
                    next_byte != b'.' && !next_ch.is_ascii_digit()
                        && !next_ch.is_ascii_alphabetic()
                        && !next_ch.is_alphabetic()
                };

                if preceded_ok && followed_ok {
                    let replacement = self.number_normalizer.normalize_number(m.as_str());
                    Some((start, end, replacement))
                } else {
                    None
                }
            })
            .collect();

        // Apply in reverse order so byte offsets stay valid.
        for (start, end, replacement) in matches.into_iter().rev() {
            let original = &snapshot[start..end];
            if replacement != original {
                tracked.replace(original, &replacement);
            }
        }
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
    use std::fs;

    fn fixture_path(name: &str, ext: &str) -> std::path::PathBuf {
        // Tests run from src-tauri/ or from workspace root; try both.
        let p1 = std::path::PathBuf::from(format!("tests/fixtures/pipeline/{}.{}", name, ext));
        let p2 = std::path::PathBuf::from(format!(
            "../src-tauri/tests/fixtures/pipeline/{}.{}",
            name, ext
        ));
        if p1.exists() {
            p1
        } else {
            p2
        }
    }

    fn read_fixture(name: &str, ext: &str) -> String {
        let path = fixture_path(name, ext);
        fs::read_to_string(&path)
            .unwrap_or_else(|_| panic!("fixture not found: {}", path.display()))
            .trim_end_matches('\n')
            .to_string()
    }

    fn run_fixture(name: &str) -> (String, String) {
        let input = read_fixture(name, "input.txt");
        let expected = read_fixture(name, "expected.txt");
        let mut pipeline = TTSPipeline::new();
        let got = pipeline.process(&input);
        (got, expected)
    }

    // ── Golden fixture integration tests ──────────────────────────────────────

    #[test]
    fn golden_number_plain() {
        let (got, expected) = run_fixture("number_plain");
        assert_eq!(got, expected, "number_plain fixture mismatch");
    }

    #[test]
    fn golden_size_mb() {
        let (got, expected) = run_fixture("size_mb");
        assert_eq!(got, expected, "size_mb fixture mismatch");
    }

    #[test]
    fn golden_size_kb() {
        let (got, expected) = run_fixture("size_kb");
        assert_eq!(got, expected, "size_kb fixture mismatch");
    }

    #[test]
    fn golden_size_gb() {
        let (got, expected) = run_fixture("size_gb");
        assert_eq!(got, expected, "size_gb fixture mismatch");
    }

    #[test]
    fn golden_camelcase() {
        let (got, expected) = run_fixture("camelcase");
        assert_eq!(got, expected, "camelcase fixture mismatch");
    }

    #[test]
    fn golden_snake_case() {
        let (got, expected) = run_fixture("snake_case");
        assert_eq!(got, expected, "snake_case fixture mismatch");
    }

    #[test]
    fn golden_kebab_case() {
        let (got, expected) = run_fixture("kebab_case");
        assert_eq!(got, expected, "kebab_case fixture mismatch");
    }

    #[test]
    fn golden_markdown_header() {
        let (got, expected) = run_fixture("markdown_header");
        assert_eq!(got, expected, "markdown_header fixture mismatch");
    }

    #[test]
    fn golden_markdown_mermaid() {
        let (got, expected) = run_fixture("markdown_mermaid");
        assert_eq!(got, expected, "markdown_mermaid fixture mismatch");
    }

    #[test]
    fn golden_operators() {
        let (got, expected) = run_fixture("operators");
        assert_eq!(got, expected, "operators fixture mismatch");
    }

    #[test]
    fn golden_percentage_int() {
        let (got, expected) = run_fixture("percentage_int");
        assert_eq!(got, expected, "percentage_int fixture mismatch");
    }

    #[test]
    fn golden_range_simple() {
        let (got, expected) = run_fixture("range_simple");
        assert_eq!(got, expected, "range_simple fixture mismatch");
    }

    #[test]
    fn golden_url_https() {
        let (got, expected) = run_fixture("url_https");
        assert_eq!(got, expected, "url_https fixture mismatch");
    }

    #[test]
    fn golden_email() {
        let (got, expected) = run_fixture("email");
        assert_eq!(got, expected, "email fixture mismatch");
    }

    #[test]
    fn golden_ip_address() {
        let (got, expected) = run_fixture("ip_address");
        assert_eq!(got, expected, "ip_address fixture mismatch");
    }

    #[test]
    fn golden_markdown_code_block() {
        let (got, expected) = run_fixture("markdown_code_block");
        assert_eq!(got, expected, "markdown_code_block fixture mismatch");
    }

    #[test]
    fn golden_markdown_inline_code() {
        let (got, expected) = run_fixture("markdown_inline_code");
        assert_eq!(got, expected, "markdown_inline_code fixture mismatch");
    }

    #[test]
    fn golden_greek_letters() {
        let (got, expected) = run_fixture("greek_letters");
        assert_eq!(got, expected, "greek_letters fixture mismatch");
    }

    #[test]
    fn golden_arrow_symbols() {
        let (got, expected) = run_fixture("arrow_symbols");
        assert_eq!(got, expected, "arrow_symbols fixture mismatch");
    }

    #[test]
    fn golden_math_symbols() {
        let (got, expected) = run_fixture("math_symbols");
        assert_eq!(got, expected, "math_symbols fixture mismatch");
    }

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

    #[test]
    fn golden_mixed_paragraph() {
        let (got, expected) = run_fixture("mixed_paragraph");
        assert_eq!(got, expected, "mixed_paragraph fixture mismatch");
    }

    #[test]
    fn golden_abbreviation_http() {
        let (got, expected) = run_fixture("abbreviation_http");
        assert_eq!(got, expected, "abbreviation_http fixture mismatch");
    }

    #[test]
    fn golden_abbreviation_upper() {
        let (got, expected) = run_fixture("abbreviation_upper");
        assert_eq!(got, expected, "abbreviation_upper fixture mismatch");
    }

    #[test]
    fn golden_number_decimal() {
        let (got, expected) = run_fixture("number_decimal");
        assert_eq!(got, expected, "number_decimal fixture mismatch");
    }

    #[test]
    fn golden_number_large() {
        let (got, expected) = run_fixture("number_large");
        assert_eq!(got, expected, "number_large fixture mismatch");
    }

    #[test]
    fn golden_pascalcase() {
        let (got, expected) = run_fixture("pascalcase");
        assert_eq!(got, expected, "pascalcase fixture mismatch");
    }

    #[test]
    fn golden_english_word_common() {
        let (got, expected) = run_fixture("english_word_common");
        assert_eq!(got, expected, "english_word_common fixture mismatch");
    }

    #[test]
    fn golden_english_word_it() {
        let (got, expected) = run_fixture("english_word_it");
        assert_eq!(got, expected, "english_word_it fixture mismatch");
    }

    #[test]
    fn golden_filepath_absolute() {
        let (got, expected) = run_fixture("filepath_absolute");
        assert_eq!(got, expected, "filepath_absolute fixture mismatch");
    }

    #[test]
    fn golden_markdown_link() {
        let (got, expected) = run_fixture("markdown_link");
        assert_eq!(got, expected, "markdown_link fixture mismatch");
    }

    #[test]
    fn golden_markdown_list_number() {
        let (got, expected) = run_fixture("markdown_list_number");
        assert_eq!(got, expected, "markdown_list_number fixture mismatch");
    }

    #[test]
    fn golden_range_years() {
        let (got, expected) = run_fixture("range_years");
        assert_eq!(got, expected, "range_years fixture mismatch");
    }

    #[test]
    fn golden_duration_ms() {
        let (got, expected) = run_fixture("duration_ms");
        assert_eq!(got, expected, "duration_ms fixture mismatch");
    }

    #[test]
    fn golden_percentage_decimal() {
        let (got, expected) = run_fixture("percentage_decimal");
        assert_eq!(got, expected, "percentage_decimal fixture mismatch");
    }

    #[test]
    fn golden_version_patch() {
        let (got, expected) = run_fixture("version_patch");
        assert_eq!(got, expected, "version_patch fixture mismatch");
    }

    #[test]
    fn golden_version_prerelease() {
        let (got, expected) = run_fixture("version_prerelease");
        assert_eq!(got, expected, "version_prerelease fixture mismatch");
    }

    #[test]
    fn golden_version_semver() {
        let (got, expected) = run_fixture("version_semver");
        assert_eq!(got, expected, "version_semver fixture mismatch");
    }
}
