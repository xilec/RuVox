use std::collections::HashMap;

use once_cell::sync::Lazy;
use regex::Regex;

use crate::dictionary::UserDictionary;
use crate::pipeline::constants::{ARROW_SYMBOLS, GREEK_LETTERS, MATH_SYMBOLS};
use crate::pipeline::normalizers::code::CodeIdentifierNormalizer;
use crate::pipeline::tracked_text::TrackedText;
use tts_contract::Reading;

// ── Language names ─────────────────────────────────────────────────────────

/// Maps lowercase language identifiers to Russian pronunciation.
///
/// Covers 19 commonly-encountered languages plus a handful of frequent extras.
pub static LANGUAGE_NAMES: Lazy<HashMap<&'static str, &'static str>> = Lazy::new(|| {
    let mut m = HashMap::new();
    m.insert("python", "пайтон");
    m.insert("py", "пайтон");
    m.insert("javascript", "джаваскрипт");
    m.insert("js", "джаваскрипт");
    m.insert("typescript", "тайпскрипт");
    m.insert("ts", "тайпскрипт");
    m.insert("bash", "баш");
    m.insert("sh", "шелл");
    m.insert("shell", "шелл");
    m.insert("zsh", "зи шелл");
    m.insert("sql", "эс кью эл");
    m.insert("json", "джейсон");
    m.insert("yaml", "ямл");
    m.insert("yml", "ямл");
    m.insert("html", "эйч ти эм эл");
    m.insert("css", "си эс эс");
    m.insert("go", "го");
    m.insert("golang", "голанг");
    m.insert("rust", "раст");
    m.insert("java", "джава");
    m.insert("kotlin", "котлин");
    m.insert("swift", "свифт");
    m.insert("ruby", "руби");
    m.insert("php", "пи эйч пи");
    m.insert("c", "си");
    m.insert("cpp", "си плюс плюс");
    m.insert("c++", "си плюс плюс");
    m.insert("cs", "си шарп");
    m.insert("csharp", "си шарп");
    m.insert("c#", "си шарп");
    m.insert("markdown", "маркдаун");
    m.insert("md", "маркдаун");
    m.insert("xml", "икс эм эл");
    m.insert("toml", "томл");
    m.insert("dockerfile", "докерфайл");
    m.insert("makefile", "мейкфайл");
    m.insert("graphql", "граф кью эл");
    m.insert("scss", "эс си эс эс");
    m.insert("sass", "сасс");
    m.insert("less", "лесс");
    m.insert("vue", "вью");
    m.insert("jsx", "джей эс икс");
    m.insert("tsx", "ти эс икс");
    m.insert("r", "ар");
    m.insert("perl", "перл");
    m.insert("lua", "луа");
    m.insert("elixir", "эликсир");
    m.insert("erlang", "эрланг");
    m.insert("haskell", "хаскелл");
    m.insert("scala", "скала");
    m.insert("clojure", "кложур");
    m.insert("dart", "дарт");
    m.insert("nginx", "энджинкс");
    m.insert("apache", "апачи");
    m.insert("terraform", "терраформ");
    m.insert("powershell", "пауэршелл");
    m.insert("mermaid", "мёрмэйд");
    m
});

// ── Special symbols for code context ──────────────────────────────────────

/// Arrows + subset of math symbols that appear in code and need spoken form.
pub static SPECIAL_SYMBOLS: Lazy<HashMap<&'static str, &'static str>> = Lazy::new(|| {
    let mut m = HashMap::new();
    // All arrow symbols
    for (k, v) in ARROW_SYMBOLS.iter() {
        m.insert(*k, *v);
    }
    // Subset of math symbols relevant to code
    for key in &["∞", "∈", "∉", "∀", "∃", "≠", "≤", "≥"] {
        if let Some(v) = MATH_SYMBOLS.get(*key) {
            m.insert(*key, *v);
        }
    }
    m
});

// ── Regexes ────────────────────────────────────────────────────────────────

/// Fenced code block: ```lang\n...\n```  (DOTALL)
static RE_FENCED_BLOCK: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?s)```(\w*)\n(.*?)```").expect("valid regex"));

// ── NativeBlock ────────────────────────────────────────────────────────────

/// A fenced block the engine reads itself.
///
/// The phase cannot leave the code in the text — the narrating phases that run
/// after it would rewrite the block — and it cannot restore the code through
/// `TrackedText` either, because a replacement nested inside an existing
/// replacement is rejected. The code therefore waits here behind a placeholder
/// until the finished `CharMapping` splices it in.
#[derive(Debug)]
pub struct NativeBlock {
    /// Private-use stand-in that survives every narrating phase.
    pub placeholder: String,
    /// The code text to substitute, with its whitespace collapsed.
    pub text: String,
}

/// A private-use placeholder for the block at `index`.
///
/// No normalizer regex matches private-use codepoints and they cannot occur in
/// text the pipeline emits, so every narrating phase steps over the stand-in.
/// Two codepoints keep the value unique until a document has millions of
/// blocks, far past anything a narration is built from.
fn placeholder_for(index: usize) -> String {
    /// First codepoint of the Unicode private-use area (U+E000..=U+F8FF).
    const FIRST: u32 = 0xE000;
    const COUNT: usize = 0x1900;

    fn private_use(offset: usize) -> char {
        char::from_u32(FIRST + offset as u32).expect("the private-use range is valid")
    }

    let high = (index / COUNT) % COUNT;
    let low = index % COUNT;
    format!("{}{}", private_use(high), private_use(low))
}

/// The code text as the engine should read it.
///
/// Line layout is not readable content, and TeraTTS drops `\n` from its model
/// alphabet, which would glue the last word of one line to the first of the
/// next; a single space keeps the words apart for both engines.
fn collapse_code_whitespace(code: &str) -> String {
    code.split_whitespace().collect::<Vec<_>>().join(" ")
}

// ── CodeBlockMode ──────────────────────────────────────────────────────────

/// Controls how fenced code blocks are rendered into speakable text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CodeBlockMode {
    /// Replace the block with a short description ("далее следует пример
    /// кода на <язык>").
    Brief,
    /// Read the code line-by-line, normalising identifiers and symbols.
    Full,
}

impl CodeBlockMode {
    /// Parse a `code_block_mode` config value. `"skip"` is the legacy alias
    /// for Brief from before the setting was wired; unknown values fall
    /// back to the product default.
    pub fn from_config(value: &str) -> Self {
        match value {
            "read" => CodeBlockMode::Full,
            "brief" | "skip" => CodeBlockMode::Brief,
            _ => CodeBlockMode::Brief,
        }
    }

    /// Canonical config string for this mode (never the legacy `"skip"`).
    pub fn as_config_str(self) -> &'static str {
        match self {
            CodeBlockMode::Brief => "brief",
            CodeBlockMode::Full => "read",
        }
    }
}

// ── CodeBlockHandler ───────────────────────────────────────────────────────

/// Processes fenced Markdown code blocks for TTS output.
///
/// Integrates with `CodeIdentifierNormalizer` for Full mode.
pub struct CodeBlockHandler {
    mode: CodeBlockMode,
    code_normalizer: CodeIdentifierNormalizer,
}

impl CodeBlockHandler {
    /// Create with the default `Brief` mode.
    pub fn new() -> Self {
        Self {
            mode: CodeBlockMode::Brief,
            code_normalizer: CodeIdentifierNormalizer::new(),
        }
    }

    /// Create with an explicit mode.
    pub fn with_mode(mode: CodeBlockMode) -> Self {
        Self {
            mode,
            code_normalizer: CodeIdentifierNormalizer::new(),
        }
    }

    /// Current operating mode.
    pub fn mode(&self) -> CodeBlockMode {
        self.mode
    }

    /// Switch mode at runtime (driven by the `code_block_mode` config).
    pub fn set_mode(&mut self, mode: CodeBlockMode) {
        self.mode = mode;
    }

    // ── Public processing entry points ─────────────────────────────────

    /// Process a single code block (code content + optional language tag)
    /// under the handler's mode.
    ///
    /// This is the low-level entry used in tests; `process()` applies the
    /// same mode to every block in the document.
    pub fn process_block(
        &self,
        code: &str,
        language: Option<&str>,
        user_dict: &UserDictionary,
        mark_stress: bool,
    ) -> String {
        self.process_block_with_mode(self.mode, code, language, user_dict, mark_stress)
    }

    /// In-place replacement of all fenced code blocks in `tracked`, each
    /// under the handler's mode (owned by the `code_block_mode` config) and
    /// the profile's code-block route.
    ///
    /// Returns the blocks the engine reads itself. In the read mode a
    /// [`Reading::Native`] block is not rendered: its content is replaced by a
    /// private-use placeholder and returned for the pipeline to splice into
    /// the final mapping after every narrating phase has run.
    pub fn process(
        &self,
        tracked: &mut TrackedText,
        user_dict: &UserDictionary,
        mark_stress: bool,
        route: Reading,
    ) -> Vec<NativeBlock> {
        let snapshot = tracked.text().to_string();
        let mut native_blocks: Vec<NativeBlock> = Vec::new();
        let mut next_placeholder = 0usize;
        let blocks: Vec<(usize, usize, String)> = RE_FENCED_BLOCK
            .captures_iter(&snapshot)
            .map(|caps| {
                let m = caps.get(0).unwrap();
                let language_raw = caps.get(1).map(|l| l.as_str()).unwrap_or("");
                let language: Option<&str> = if language_raw.is_empty() {
                    None
                } else {
                    Some(language_raw)
                };
                let code = caps.get(2).map(|c| c.as_str()).unwrap_or("").trim();

                // Mermaid diagrams are never read aloud, whatever the mode and
                // route say.
                let replacement = if language.is_some_and(|l| l.eq_ignore_ascii_case("mermaid")) {
                    "Тут мермэйд диаграмма".to_string()
                } else if self.mode == CodeBlockMode::Full && route == Reading::Native {
                    let text = collapse_code_whitespace(code);
                    if text.is_empty() {
                        String::new()
                    } else {
                        // A block may be replaced by the code and the speaker
                        // may have pasted private-use glyphs; skip any stand-in
                        // the input already contains so the search in the
                        // mapping stays unambiguous.
                        let placeholder = loop {
                            let candidate = placeholder_for(next_placeholder);
                            next_placeholder += 1;
                            if !snapshot.contains(&candidate) {
                                break candidate;
                            }
                        };
                        native_blocks.push(NativeBlock {
                            placeholder: placeholder.clone(),
                            text,
                        });
                        placeholder
                    }
                } else {
                    self.process_block_with_mode(self.mode, code, language, user_dict, mark_stress)
                };

                (m.start(), m.end(), replacement)
            })
            .collect();

        tracked.replace_byte_ranges(blocks);
        native_blocks
    }

    // ── Private helpers ────────────────────────────────────────────────

    /// Process a block under an explicit mode.
    fn process_block_with_mode(
        &self,
        mode: CodeBlockMode,
        code: &str,
        language: Option<&str>,
        user_dict: &UserDictionary,
        mark_stress: bool,
    ) -> String {
        match mode {
            CodeBlockMode::Brief => self.brief_description(language),
            CodeBlockMode::Full => self.full_normalize(code, language, user_dict, mark_stress),
        }
    }

    fn brief_description(&self, language: Option<&str>) -> String {
        match language {
            Some(lang) if !lang.is_empty() => {
                let lang_lower = lang.to_lowercase();
                let lang_name = LANGUAGE_NAMES
                    .get(lang_lower.as_str())
                    .copied()
                    .unwrap_or(lang);
                format!("далее следует пример кода на {}", lang_name)
            }
            _ => "далее следует блок кода".to_string(),
        }
    }

    fn full_normalize(
        &self,
        code: &str,
        _language: Option<&str>,
        user_dict: &UserDictionary,
        mark_stress: bool,
    ) -> String {
        let tokens = self.tokenize(code);
        let normalized: Vec<String> = tokens
            .into_iter()
            .filter_map(|t| {
                let n = self.normalize_token(t, user_dict, mark_stress);
                if n.is_empty() { None } else { Some(n) }
            })
            .collect();
        normalized.join(" ")
    }

    fn tokenize<'a>(&self, code: &'a str) -> Vec<&'a str> {
        // Build the combined tokenisation regex on first use.
        // We match (in priority order):
        //   1. Greek letters (multi-char Unicode — must come before single-char fallbacks)
        //   2. Special symbols (arrows + math subset)
        //   3. String literals  '...' or "..."
        //   4. Identifiers
        //   5. Integer literals
        //   6. Brackets
        //   7. Multi-char operators
        //   8. Punctuation
        static RE_TOKENS: Lazy<Regex> = Lazy::new(|| {
            // Collect all special-char patterns (longest first to avoid partial matches)
            let mut special: Vec<String> = GREEK_LETTERS
                .keys()
                .chain(SPECIAL_SYMBOLS.keys())
                .map(|k| regex::escape(k))
                .collect();
            // Longest first so e.g. "⇔" beats "⇒"
            special.sort_by_key(|s| std::cmp::Reverse(s.len()));
            special.dedup();

            let special_pat = special.join("|");

            let pattern = format!(
                r#"(?:{special})|[a-zA-Z_][a-zA-Z0-9_]*|\d+|'[^']*'|"[^"]*"|[()\[\]{{}}]|[+\-*/=<>!&|]{{1,3}}|[.,;:]"#,
                special = special_pat,
            );

            Regex::new(&pattern).expect("valid tokeniser regex")
        });

        RE_TOKENS.find_iter(code).map(|m| m.as_str()).collect()
    }

    fn normalize_token(
        &self,
        token: &str,
        user_dict: &UserDictionary,
        mark_stress: bool,
    ) -> String {
        if token.is_empty() {
            return String::new();
        }

        // Greek letters
        if let Some(name) = GREEK_LETTERS.get(token) {
            return name.to_string();
        }

        // Special symbols (arrows + math subset)
        if let Some(name) = SPECIAL_SYMBOLS.get(token) {
            return name.to_string();
        }

        // String literals — extract content, normalise via code_normalizer
        if (token.starts_with('\'') && token.ends_with('\''))
            || (token.starts_with('"') && token.ends_with('"'))
        {
            let content = &token[1..token.len() - 1];
            if content.is_empty() {
                return String::new();
            }
            let lower = content.to_lowercase();
            // Try the user dictionary, then CODE_WORDS; fall back to basic transliterate
            return self.normalize_simple_word(&lower, user_dict, mark_stress);
        }

        // Integer literals
        if token.chars().all(|c| c.is_ascii_digit()) {
            if let Ok(n) = token.parse::<u64>() {
                return number_to_russian(n);
            }
        }

        // Identifiers
        if token
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
            && token.chars().all(|c| c.is_alphanumeric() || c == '_')
        {
            if token.contains('_') {
                return self
                    .code_normalizer
                    .normalize_snake_case(token, user_dict, mark_stress);
            }
            // CamelCase/PascalCase heuristic: has uppercase after first char
            if token.chars().skip(1).any(|c| c.is_ascii_uppercase()) {
                return self
                    .code_normalizer
                    .normalize_camel_case(token, user_dict, mark_stress);
            }
            // Plain lowercase (or all-caps)
            let lower = token.to_lowercase();
            return self.normalize_simple_word(&lower, user_dict, mark_stress);
        }

        // Operators, brackets, and punctuation — look up in the shared SYMBOLS dictionary
        // via SymbolNormalizer. Unknown tokens return empty string.
        use crate::pipeline::normalizers::symbols::SymbolNormalizer;
        let sym = SymbolNormalizer::new();
        let spoken = sym.normalize(token);
        if spoken != token {
            spoken.to_string()
        } else {
            String::new()
        }
    }

    /// Look up a lower-cased word in the user dictionary, then CODE_WORDS,
    /// and fall back to basic transliterate.
    fn normalize_simple_word(
        &self,
        lower: &str,
        user_dict: &UserDictionary,
        mark_stress: bool,
    ) -> String {
        // Delegate through the public method surface that is available.
        // CodeIdentifierNormalizer exposes normalize_snake_case which handles
        // single-segment inputs correctly (no underscores → single part).
        self.code_normalizer
            .normalize_snake_case(lower, user_dict, mark_stress)
    }
}

impl Default for CodeBlockHandler {
    fn default() -> Self {
        Self::new()
    }
}

// ── Minimal number-to-Russian used by Full mode ────────────────────────────

/// Small integer to Russian words. Mirrors the function in code.rs.
fn number_to_russian(n: u64) -> String {
    match n {
        0 => "ноль".to_string(),
        1 => "один".to_string(),
        2 => "два".to_string(),
        3 => "три".to_string(),
        4 => "четыре".to_string(),
        5 => "пять".to_string(),
        6 => "шесть".to_string(),
        7 => "семь".to_string(),
        8 => "восемь".to_string(),
        9 => "девять".to_string(),
        10 => "десять".to_string(),
        11 => "одиннадцать".to_string(),
        12 => "двенадцать".to_string(),
        13 => "тринадцать".to_string(),
        14 => "четырнадцать".to_string(),
        15 => "пятнадцать".to_string(),
        16 => "шестнадцать".to_string(),
        17 => "семнадцать".to_string(),
        18 => "восемнадцать".to_string(),
        19 => "девятнадцать".to_string(),
        20 => "двадцать".to_string(),
        21 => "двадцать один".to_string(),
        22 => "двадцать два".to_string(),
        23 => "двадцать три".to_string(),
        24 => "двадцать четыре".to_string(),
        25 => "двадцать пять".to_string(),
        26 => "двадцать шесть".to_string(),
        27 => "двадцать семь".to_string(),
        28 => "двадцать восемь".to_string(),
        29 => "двадцать девять".to_string(),
        30 => "тридцать".to_string(),
        32 => "тридцать два".to_string(),
        40 => "сорок".to_string(),
        42 => "сорок два".to_string(),
        50 => "пятьдесят".to_string(),
        60 => "шестьдесят".to_string(),
        64 => "шестьдесят четыре".to_string(),
        70 => "семьдесят".to_string(),
        80 => "восемьдесят".to_string(),
        90 => "девяносто".to_string(),
        100 => "сто".to_string(),
        _ => n
            .to_string()
            .chars()
            .map(|c| match c {
                '0' => "ноль",
                '1' => "один",
                '2' => "два",
                '3' => "три",
                '4' => "четыре",
                '5' => "пять",
                '6' => "шесть",
                '7' => "семь",
                '8' => "восемь",
                '9' => "девять",
                _ => "",
            })
            .collect::<Vec<_>>()
            .join(" "),
    }
}

// ── Unit tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dictionary::UserDictionary;
    use test_case::test_case;

    fn no_dict() -> UserDictionary {
        UserDictionary::default()
    }

    fn brief_handler() -> CodeBlockHandler {
        CodeBlockHandler::with_mode(CodeBlockMode::Brief)
    }

    fn full_handler() -> CodeBlockHandler {
        CodeBlockHandler::with_mode(CodeBlockMode::Full)
    }

    // ── Brief mode language descriptions (merges former TestCodeBlockBriefMode
    // and TestLanguageNames — brief_description() ignores `code` entirely, so
    // both groups exercised the same function keyed only on `language`; the
    // 5 overlapping codes [python, javascript, typescript, bash, yaml] were
    // genuine duplicates, now asserted once each with the (stronger) exact
    // sentence instead of the former `.contains()` check) ──────────────────

    #[test_case(Some("python") => "далее следует пример кода на пайтон"; "python")]
    #[test_case(Some("py") => "далее следует пример кода на пайтон"; "py")]
    #[test_case(Some("javascript") => "далее следует пример кода на джаваскрипт"; "javascript")]
    #[test_case(Some("js") => "далее следует пример кода на джаваскрипт"; "js")]
    #[test_case(Some("typescript") => "далее следует пример кода на тайпскрипт"; "typescript")]
    #[test_case(Some("ts") => "далее следует пример кода на тайпскрипт"; "ts")]
    #[test_case(Some("bash") => "далее следует пример кода на баш"; "bash")]
    #[test_case(Some("sh") => "далее следует пример кода на шелл"; "sh")]
    #[test_case(Some("shell") => "далее следует пример кода на шелл"; "shell")]
    #[test_case(Some("sql") => "далее следует пример кода на эс кью эл"; "sql")]
    #[test_case(Some("json") => "далее следует пример кода на джейсон"; "json")]
    #[test_case(Some("yaml") => "далее следует пример кода на ямл"; "yaml")]
    #[test_case(Some("yml") => "далее следует пример кода на ямл"; "yml")]
    #[test_case(Some("html") => "далее следует пример кода на эйч ти эм эл"; "html")]
    #[test_case(Some("css") => "далее следует пример кода на си эс эс"; "css")]
    #[test_case(Some("go") => "далее следует пример кода на го"; "go")]
    #[test_case(Some("rust") => "далее следует пример кода на раст"; "rust")]
    #[test_case(Some("java") => "далее следует пример кода на джава"; "java")]
    #[test_case(Some("md") => "далее следует пример кода на маркдаун"; "md")]
    #[test_case(Some("markdown") => "далее следует пример кода на маркдаун"; "markdown")]
    #[test_case(Some("cpp") => "далее следует пример кода на си плюс плюс"; "cpp")]
    #[test_case(Some("c++") => "далее следует пример кода на си плюс плюс"; "cpp_symbol")]
    #[test_case(Some("cs") => "далее следует пример кода на си шарп"; "cs")]
    #[test_case(Some("csharp") => "далее следует пример кода на си шарп"; "csharp")]
    #[test_case(Some("dockerfile") => "далее следует пример кода на докерфайл"; "dockerfile")]
    #[test_case(Some("makefile") => "далее следует пример кода на мейкфайл"; "makefile")]
    #[test_case(None => "далее следует блок кода"; "no_language")]
    #[test_case(Some("") => "далее следует блок кода"; "empty_language")]
    fn brief_language(language: Option<&str>) -> String {
        brief_handler().process_block("", language, &no_dict(), false)
    }

    // ── TestCodeBlockFullMode (3 cases) ────────────────────────────────

    #[test_case("def hello():\n    print('world')", Some("python") => "деф хеллоу открывающая скобка закрывающая скобка двоеточие принт открывающая скобка ворлд закрывающая скобка"; "python_def_hello")]
    #[test_case("const x = 42;", Some("javascript") => "конст икс равно сорок два точка с запятой"; "js_const_x")]
    #[test_case("getUserData(userId)", None => "гет юзер дата открывающая скобка юзер ай ди закрывающая скобка"; "function_call")]
    fn full_mode(code: &str, language: Option<&str>) -> String {
        full_handler().process_block(code, language, &no_dict(), false)
    }

    // ── TestModeSwitch (4 cases) ────────────────────────────────────────

    #[test_case(CodeBlockMode::Full, CodeBlockMode::Brief; "to_brief")]
    #[test_case(CodeBlockMode::Brief, CodeBlockMode::Full; "to_full")]
    fn mode_switch(initial: CodeBlockMode, set_to: CodeBlockMode) {
        let mut h = CodeBlockHandler::with_mode(initial);
        h.set_mode(set_to);
        assert_eq!(h.mode(), set_to);
    }

    #[test]
    fn mode_default_is_brief() {
        let h = CodeBlockHandler::new();
        assert_eq!(h.mode(), CodeBlockMode::Brief);
    }

    #[test]
    fn legacy_directive_does_not_switch_mode() {
        // A legacy `ruvox-code` directive no longer changes the effective
        // mode: the comment is ordinary text and the handler's mode wins.
        let text = "<!-- ruvox-code: brief -->\n```python\nprint('hi')\n```";
        let mut tracked = TrackedText::new(text);
        let h = CodeBlockHandler::with_mode(CodeBlockMode::Full);
        h.process(&mut tracked, &no_dict(), false, Reading::Narrate);
        let result = tracked.text();
        // Full mode keeps reading the code; the comment survives as text for
        // the symbol phases (no directive interpretation, no removal).
        assert!(result.contains("принт открывающая скобка хи закрывающая скобка"));
        assert!(result.contains("ruvox-code"));
    }

    // ── Config value conversions ───────────────────────────────────────

    #[test_case("brief" => CodeBlockMode::Brief; "brief")]
    #[test_case("skip" => CodeBlockMode::Brief; "legacy_skip_alias")]
    #[test_case("read" => CodeBlockMode::Full; "read")]
    #[test_case("loud" => CodeBlockMode::Brief; "unknown_falls_back")]
    #[test_case("" => CodeBlockMode::Brief; "empty_falls_back")]
    fn from_config_maps_values(value: &str) -> CodeBlockMode {
        CodeBlockMode::from_config(value)
    }

    #[test]
    fn as_config_str_is_canonical() {
        assert_eq!(CodeBlockMode::Brief.as_config_str(), "brief");
        assert_eq!(CodeBlockMode::Full.as_config_str(), "read");
        // Round-trip never emits the legacy alias.
        assert_eq!(CodeBlockMode::from_config("skip").as_config_str(), "brief");
    }

    // ── process() with TrackedText ──────────────────────────────────────

    #[test]
    fn process_brief_replaces_fenced_block() {
        let text = "Смотри:\n```python\nprint('hi')\n```\nКонец.";
        let mut tracked = TrackedText::new(text);
        let h = CodeBlockHandler::with_mode(CodeBlockMode::Brief);
        h.process(&mut tracked, &no_dict(), false, Reading::Narrate);
        let result = tracked.text();
        // Exact form also proves the backticks are gone (no "```" substring).
        assert_eq!(
            result,
            "Смотри:\nдалее следует пример кода на пайтон\nКонец."
        );
    }

    #[test]
    fn process_mermaid_replaced_with_marker() {
        let text = "Диаграмма:\n```mermaid\ngraph TD\nA-->B\n```\nКонец.";
        let mut tracked = TrackedText::new(text);
        let h = CodeBlockHandler::with_mode(CodeBlockMode::Brief);
        h.process(&mut tracked, &no_dict(), false, Reading::Narrate);
        let result = tracked.text();
        assert_eq!(result, "Диаграмма:\nТут мермэйд диаграмма\nКонец.");
    }

    #[test]
    fn process_identical_blocks_each_replaced() {
        // Regression (#84): two byte-identical blocks must each get their own
        // replacement, even when the replacement strings are identical.
        let text = "```python\nprint('world')\n```\n```python\nprint('world')\n```";
        let mut tracked = TrackedText::new(text);
        let h = CodeBlockHandler::with_mode(CodeBlockMode::Full);
        h.process(&mut tracked, &no_dict(), false, Reading::Narrate);
        let reading = "принт открывающая скобка ворлд закрывающая скобка";
        assert_eq!(tracked.text(), format!("{reading}\n{reading}"));
    }

    // ── Native route ────────────────────────────────────────────────────

    #[test]
    fn process_native_route_returns_code_and_leaves_a_placeholder() {
        let text = "Код:\n```rust\nlet x = 42;\nif x == 0 { return; }\n```\nКонец.";
        let mut tracked = TrackedText::new(text);
        let native = full_handler().process(&mut tracked, &no_dict(), false, Reading::Native);

        assert_eq!(native.len(), 1);
        assert_eq!(native[0].text, "let x = 42; if x == 0 { return; }");
        let result = tracked.text();
        assert!(
            result.contains(&native[0].placeholder),
            "the code must leave the phase behind its placeholder"
        );
        assert!(!result.contains("```"));
        assert!(!result.contains("равно"));
    }

    #[test]
    fn process_native_route_keeps_brief_mode_authoritative() {
        let mut tracked = TrackedText::new("```python\nprint('hi')\n```");
        let native = brief_handler().process(&mut tracked, &no_dict(), false, Reading::Native);

        assert!(native.is_empty());
        assert_eq!(tracked.text(), "далее следует пример кода на пайтон");
    }

    #[test]
    fn process_native_route_keeps_the_mermaid_marker() {
        let mut tracked = TrackedText::new("```mermaid\ngraph TD\nA-->B\n```");
        let native = full_handler().process(&mut tracked, &no_dict(), false, Reading::Native);

        assert!(native.is_empty());
        assert_eq!(tracked.text(), "Тут мермэйд диаграмма");
    }

    #[test]
    fn process_native_route_gives_each_block_its_own_placeholder() {
        let text = "```rust\nlet a = 1;\n```\n```rust\nlet b = 2;\n```";
        let mut tracked = TrackedText::new(text);
        let native = full_handler().process(&mut tracked, &no_dict(), false, Reading::Native);

        assert_eq!(native.len(), 2);
        assert_ne!(native[0].placeholder, native[1].placeholder);
        assert_eq!(native[0].text, "let a = 1;");
        assert_eq!(native[1].text, "let b = 2;");
    }

    #[test]
    fn process_native_route_drops_an_empty_block() {
        let mut tracked = TrackedText::new("До\n```python\n\n```\nПосле");
        let native = full_handler().process(&mut tracked, &no_dict(), false, Reading::Native);

        assert!(native.is_empty());
        assert_eq!(tracked.text(), "До\n\nПосле");
    }

    #[test]
    fn process_native_route_skips_a_placeholder_already_in_the_text() {
        // A pasted private-use pair must not be mistaken for the stand-in: the
        // mapping search treats every occurrence as the placeholder.
        let pasted = "\u{e000}\u{e000}";
        let text = format!("{pasted}\n```rust\nlet a = 1;\n```");
        let mut tracked = TrackedText::new(&text);
        let native = full_handler().process(&mut tracked, &no_dict(), false, Reading::Native);

        assert_eq!(native.len(), 1);
        assert_ne!(native[0].placeholder, pasted);
        assert!(
            tracked.text().starts_with(&format!("{pasted}\n")),
            "the pasted glyphs stay untouched: {:?}",
            tracked.text()
        );
        assert_eq!(tracked.text().matches(&native[0].placeholder).count(), 1);
    }
}
