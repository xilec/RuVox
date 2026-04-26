use std::collections::HashMap;

use once_cell::sync::Lazy;
use regex::Regex;

use crate::pipeline::constants::{ARROW_SYMBOLS, GREEK_LETTERS, MATH_SYMBOLS};
use crate::pipeline::normalizers::code::CodeIdentifierNormalizer;
use crate::pipeline::tracked_text::TrackedText;

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

/// Inline mode-switch directives embedded in the document.
static RE_MODE_SWITCH: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"<!--\s*ruvox-code:\s*(full|brief)\s*-->").expect("valid regex"));

// ── CodeBlockMode ──────────────────────────────────────────────────────────

/// Controls how fenced code blocks are rendered into speakable text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CodeBlockMode {
    /// Replace the block with a short description (language + line count).
    Brief,
    /// Read the code line-by-line, normalising identifiers and symbols.
    Full,
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

    /// Switch mode at runtime (used by mode-switch directives).
    pub fn set_mode(&mut self, mode: CodeBlockMode) {
        self.mode = mode;
    }

    // ── Public processing entry points ─────────────────────────────────

    /// Process a single code block (code content + optional language tag).
    ///
    /// This is the low-level entry used in tests and by `process()`.
    pub fn process_block(&self, code: &str, language: Option<&str>) -> String {
        match self.mode {
            CodeBlockMode::Brief => self.brief_description(language),
            CodeBlockMode::Full => self.full_normalize(code, language),
        }
    }

    /// In-place replacement of all fenced code blocks in `tracked`.
    ///
    /// Also handles `<!-- ruvox-code: full|brief -->` directives that appear
    /// *before* a block, allowing per-section mode overrides.
    pub fn process(&self, tracked: &mut TrackedText) {
        // Collect positions of mode-switch directives so we can determine
        // the effective mode for each code block.
        let directives = self.collect_directives(tracked.text());

        // Build a snapshot of block match positions with their effective modes.
        let snapshot = tracked.text().to_string();
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
                let block_start = m.start();

                // Effective mode: last directive whose byte position is before this block
                let effective_mode = directives
                    .iter()
                    .rev()
                    .find(|(pos, _)| *pos < block_start)
                    .map(|(_, mode)| *mode)
                    .unwrap_or(self.mode);

                // Mermaid diagrams are never read aloud.
                let replacement = if language.is_some_and(|l| l.eq_ignore_ascii_case("mermaid")) {
                    "Тут мермэйд диаграмма".to_string()
                } else {
                    let handler = CodeBlockHandler {
                        mode: effective_mode,
                        code_normalizer: CodeIdentifierNormalizer::new(),
                    };
                    handler.process_block(code, language)
                };

                (m.start(), m.end(), replacement)
            })
            .collect();

        // Apply replacements via TrackedText in reverse order (right-to-left)
        // so that byte offsets remain valid after each substitution.
        for (start, end, replacement) in blocks.into_iter().rev() {
            // Extract the slice safely; it may already have been consumed if
            // an earlier (larger) block covered this range.
            if end > snapshot.len() || start > end {
                continue;
            }
            let original_slice = &snapshot[start..end];
            tracked.replace(original_slice, &replacement);
        }

        // Remove mode-switch directives from the output (they are control markers,
        // not content that should be spoken).
        let directive_pattern = Regex::new(r"<!--\s*ruvox-code:\s*(?:full|brief)\s*-->")
            .expect("valid regex");
        tracked.sub(&directive_pattern, |_| String::new());
    }

    // ── Private helpers ────────────────────────────────────────────────

    fn collect_directives(&self, text: &str) -> Vec<(usize, CodeBlockMode)> {
        RE_MODE_SWITCH
            .captures_iter(text)
            .map(|caps| {
                let m = caps.get(0).unwrap();
                let mode_str = caps.get(1).unwrap().as_str();
                let mode = if mode_str == "full" {
                    CodeBlockMode::Full
                } else {
                    CodeBlockMode::Brief
                };
                (m.start(), mode)
            })
            .collect()
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

    fn full_normalize(&self, code: &str, _language: Option<&str>) -> String {
        let tokens = self.tokenize(code);
        let normalized: Vec<String> = tokens
            .into_iter()
            .filter_map(|t| {
                let n = self.normalize_token(t);
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

        RE_TOKENS
            .find_iter(code)
            .map(|m| m.as_str())
            .collect()
    }

    fn normalize_token(&self, token: &str) -> String {
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
            // Try CODE_WORDS first; fall back to basic transliterate
            return self.normalize_simple_word(&lower);
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
                return self.code_normalizer.normalize_snake_case(token);
            }
            // CamelCase/PascalCase heuristic: has uppercase after first char
            if token.chars().skip(1).any(|c| c.is_ascii_uppercase()) {
                return self.code_normalizer.normalize_camel_case(token);
            }
            // Plain lowercase (or all-caps)
            let lower = token.to_lowercase();
            return self.normalize_simple_word(&lower);
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

    /// Look up a lower-cased word in CODE_WORDS and fall back to basic transliterate.
    fn normalize_simple_word(&self, lower: &str) -> String {
        // Delegate through the public method surface that is available.
        // CodeIdentifierNormalizer exposes normalize_snake_case which handles
        // single-segment inputs correctly (no underscores → single part).
        self.code_normalizer.normalize_snake_case(lower)
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

    fn brief_handler() -> CodeBlockHandler {
        CodeBlockHandler::with_mode(CodeBlockMode::Brief)
    }

    fn full_handler() -> CodeBlockHandler {
        CodeBlockHandler::with_mode(CodeBlockMode::Full)
    }

    // ── TestCodeBlockBriefMode (14 cases) ──────────────────────────────

    #[test]
    fn brief_python() {
        assert_eq!(
            brief_handler().process_block("print('hello')", Some("python")),
            "далее следует пример кода на пайтон"
        );
    }

    #[test]
    fn brief_javascript() {
        assert_eq!(
            brief_handler().process_block("const x = 1;", Some("javascript")),
            "далее следует пример кода на джаваскрипт"
        );
    }

    #[test]
    fn brief_typescript() {
        assert_eq!(
            brief_handler().process_block("let y: string;", Some("typescript")),
            "далее следует пример кода на тайпскрипт"
        );
    }

    #[test]
    fn brief_bash() {
        assert_eq!(
            brief_handler().process_block("echo hi", Some("bash")),
            "далее следует пример кода на баш"
        );
    }

    #[test]
    fn brief_sql() {
        assert_eq!(
            brief_handler().process_block("SELECT 1", Some("sql")),
            "далее следует пример кода на эс кью эл"
        );
    }

    #[test]
    fn brief_json() {
        assert_eq!(
            brief_handler().process_block("{}", Some("json")),
            "далее следует пример кода на джейсон"
        );
    }

    #[test]
    fn brief_yaml() {
        assert_eq!(
            brief_handler().process_block("key: val", Some("yaml")),
            "далее следует пример кода на ямл"
        );
    }

    #[test]
    fn brief_html() {
        assert_eq!(
            brief_handler().process_block("<p>hi</p>", Some("html")),
            "далее следует пример кода на эйч ти эм эл"
        );
    }

    #[test]
    fn brief_css() {
        assert_eq!(
            brief_handler().process_block("body{}", Some("css")),
            "далее следует пример кода на си эс эс"
        );
    }

    #[test]
    fn brief_go() {
        assert_eq!(
            brief_handler().process_block("func main(){}", Some("go")),
            "далее следует пример кода на го"
        );
    }

    #[test]
    fn brief_rust() {
        assert_eq!(
            brief_handler().process_block("fn main(){}", Some("rust")),
            "далее следует пример кода на раст"
        );
    }

    #[test]
    fn brief_java() {
        assert_eq!(
            brief_handler().process_block("class A{}", Some("java")),
            "далее следует пример кода на джава"
        );
    }

    #[test]
    fn brief_no_language() {
        assert_eq!(
            brief_handler().process_block("x = 1", None),
            "далее следует блок кода"
        );
    }

    #[test]
    fn brief_empty_language() {
        assert_eq!(
            brief_handler().process_block("x = 1", Some("")),
            "далее следует блок кода"
        );
    }

    // ── TestCodeBlockFullMode (3 cases) ────────────────────────────────

    #[test]
    fn full_python_def_hello() {
        let result = full_handler().process_block("def hello():\n    print('world')", Some("python"));
        let lower = result.to_lowercase();
        for expected in &["деф", "хелло", "принт", "ворлд"] {
            assert!(
                lower.contains(expected),
                "expected {:?} in {:?}",
                expected,
                result
            );
        }
    }

    #[test]
    fn full_js_const_x() {
        let result = full_handler().process_block("const x = 42;", Some("javascript"));
        let lower = result.to_lowercase();
        for expected in &["конст", "икс", "сорок два"] {
            assert!(
                lower.contains(expected),
                "expected {:?} in {:?}",
                expected,
                result
            );
        }
    }

    #[test]
    fn full_function_call() {
        let result = full_handler().process_block("getUserData(userId)", None);
        let lower = result.to_lowercase();
        for expected in &["гет юзер дата", "юзер ай ди"] {
            assert!(
                lower.contains(expected),
                "expected {:?} in {:?}",
                expected,
                result
            );
        }
    }

    // ── TestModeSwitch (4 cases) ────────────────────────────────────────

    #[test]
    fn mode_switch_to_brief() {
        let mut h = CodeBlockHandler::with_mode(CodeBlockMode::Full);
        h.set_mode(CodeBlockMode::Brief);
        assert_eq!(h.mode(), CodeBlockMode::Brief);
    }

    #[test]
    fn mode_switch_to_full() {
        let mut h = CodeBlockHandler::with_mode(CodeBlockMode::Brief);
        h.set_mode(CodeBlockMode::Full);
        assert_eq!(h.mode(), CodeBlockMode::Full);
    }

    #[test]
    fn mode_default_is_brief() {
        let h = CodeBlockHandler::new();
        assert_eq!(h.mode(), CodeBlockMode::Brief);
    }

    #[test]
    fn mode_switch_via_process() {
        // A directive switches the effective mode for blocks that follow.
        let text = "<!-- ruvox-code: full -->\n```python\nprint('hi')\n```";
        let mut tracked = TrackedText::new(text);
        // Handler starts in Brief; directive should upgrade to Full for this block.
        let h = CodeBlockHandler::with_mode(CodeBlockMode::Brief);
        h.process(&mut tracked);
        let result = tracked.text();
        // Brief would give "далее следует пример кода на пайтон"; Full should contain "принт".
        assert!(
            result.contains("принт"),
            "expected full-mode output, got: {:?}",
            result
        );
    }

    // ── TestLanguageNames (19 cases) ────────────────────────────────────

    #[test]
    fn lang_py() {
        assert!(brief_handler()
            .process_block("", Some("py"))
            .contains("пайтон"));
    }

    #[test]
    fn lang_python() {
        assert!(brief_handler()
            .process_block("", Some("python"))
            .contains("пайтон"));
    }

    #[test]
    fn lang_js() {
        assert!(brief_handler()
            .process_block("", Some("js"))
            .contains("джаваскрипт"));
    }

    #[test]
    fn lang_javascript() {
        assert!(brief_handler()
            .process_block("", Some("javascript"))
            .contains("джаваскрипт"));
    }

    #[test]
    fn lang_ts() {
        assert!(brief_handler()
            .process_block("", Some("ts"))
            .contains("тайпскрипт"));
    }

    #[test]
    fn lang_typescript() {
        assert!(brief_handler()
            .process_block("", Some("typescript"))
            .contains("тайпскрипт"));
    }

    #[test]
    fn lang_sh() {
        assert!(brief_handler()
            .process_block("", Some("sh"))
            .contains("шелл"));
    }

    #[test]
    fn lang_bash() {
        assert!(brief_handler()
            .process_block("", Some("bash"))
            .contains("баш"));
    }

    #[test]
    fn lang_shell() {
        assert!(brief_handler()
            .process_block("", Some("shell"))
            .contains("шелл"));
    }

    #[test]
    fn lang_yml() {
        assert!(brief_handler()
            .process_block("", Some("yml"))
            .contains("ямл"));
    }

    #[test]
    fn lang_yaml() {
        assert!(brief_handler()
            .process_block("", Some("yaml"))
            .contains("ямл"));
    }

    #[test]
    fn lang_md() {
        assert!(brief_handler()
            .process_block("", Some("md"))
            .contains("маркдаун"));
    }

    #[test]
    fn lang_markdown() {
        assert!(brief_handler()
            .process_block("", Some("markdown"))
            .contains("маркдаун"));
    }

    #[test]
    fn lang_cpp() {
        assert!(brief_handler()
            .process_block("", Some("cpp"))
            .contains("си плюс плюс"));
    }

    #[test]
    fn lang_cpp_symbol() {
        // "c++" contains '+' which is not a \w char, map it directly
        assert!(brief_handler()
            .process_block("", Some("c++"))
            .contains("си плюс плюс"));
    }

    #[test]
    fn lang_cs() {
        assert!(brief_handler()
            .process_block("", Some("cs"))
            .contains("си шарп"));
    }

    #[test]
    fn lang_csharp() {
        assert!(brief_handler()
            .process_block("", Some("csharp"))
            .contains("си шарп"));
    }

    #[test]
    fn lang_dockerfile() {
        assert!(brief_handler()
            .process_block("", Some("dockerfile"))
            .contains("докерфайл"));
    }

    #[test]
    fn lang_makefile() {
        assert!(brief_handler()
            .process_block("", Some("makefile"))
            .contains("мейкфайл"));
    }

    // ── process() with TrackedText ──────────────────────────────────────

    #[test]
    fn process_brief_replaces_fenced_block() {
        let text = "Смотри:\n```python\nprint('hi')\n```\nКонец.";
        let mut tracked = TrackedText::new(text);
        let h = CodeBlockHandler::with_mode(CodeBlockMode::Brief);
        h.process(&mut tracked);
        let result = tracked.text();
        assert!(result.contains("далее следует пример кода на пайтон"), "got: {:?}", result);
        assert!(!result.contains("```"), "backticks should be gone, got: {:?}", result);
    }

    #[test]
    fn process_mermaid_replaced_with_marker() {
        let text = "Диаграмма:\n```mermaid\ngraph TD\nA-->B\n```\nКонец.";
        let mut tracked = TrackedText::new(text);
        let h = CodeBlockHandler::with_mode(CodeBlockMode::Brief);
        h.process(&mut tracked);
        let result = tracked.text();
        assert!(result.contains("Тут мермэйд диаграмма"), "got: {:?}", result);
    }

    #[test]
    fn process_mode_directive_switches_to_full() {
        let text =
            "<!-- ruvox-code: full -->\n```python\nprint('world')\n```";
        let mut tracked = TrackedText::new(text);
        let h = CodeBlockHandler::with_mode(CodeBlockMode::Brief);
        h.process(&mut tracked);
        let result = tracked.text();
        // Full mode should contain "принт" and "ворлд"
        assert!(result.contains("принт"), "expected full mode, got: {:?}", result);
        assert!(result.contains("ворлд"), "expected full mode, got: {:?}", result);
    }

    #[test]
    fn process_mode_directive_brief_after_full() {
        let text = concat!(
            "<!-- ruvox-code: full -->\n```python\nprint('world')\n```\n",
            "<!-- ruvox-code: brief -->\n```python\nprint('world')\n```"
        );
        let mut tracked = TrackedText::new(text);
        let h = CodeBlockHandler::with_mode(CodeBlockMode::Brief);
        h.process(&mut tracked);
        let result = tracked.text();
        // Second block should be brief
        assert!(result.contains("далее следует пример кода на пайтон"), "got: {:?}", result);
    }
}
