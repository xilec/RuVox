use aho_corasick::{AhoCorasick, AhoCorasickBuilder, MatchKind};
use once_cell::sync::Lazy;
use std::collections::HashMap;

/// IT terms with established Russian pronunciation.
///
/// Keys are lowercase English. Values are Russian phonetic spelling.
/// Words matching simple transliteration output are handled by `transliterate_simple`.
pub static IT_TERMS: Lazy<HashMap<&'static str, &'static str>> = Lazy::new(|| {
    let mut m = HashMap::new();
    // Programming languages (special syntax or differs from transliteration)
    m.insert("c++", "си плюс плюс");
    m.insert("c#", "си шарп");
    m.insert("f#", "эф шарп");
    m.insert("haskell", "хаскелл");
    m.insert("ocaml", "окамл");
    m.insert("erlang", "эрланг");
    m.insert("elixir", "эликсир");
    m.insert("clojure", "кложур");
    m.insert("prolog", "пролог");
    m.insert("fortran", "фортран");
    m.insert("cobol", "кобол");
    m.insert("pascal", "паскаль");
    m.insert("delphi", "делфи");
    m.insert("php", "пи эйч пи");
    m.insert("sql", "эс кью эль");
    m.insert("html", "эйч ти эм эль");
    m.insert("css", "си эс эс");
    m.insert("xml", "икс эм эль");
    m.insert("json", "джейсон");
    m.insert("yaml", "ямл");
    m.insert("toml", "томл");
    m.insert("js", "джи эс");
    m.insert("ts", "ти эс");
    // English numerals (where transliteration differs from expected)
    m.insert("zero", "зиро");
    m.insert("seven", "сэвен");
    m.insert("ten", "тен");
    m.insert("eleven", "илэвен");
    m.insert("twelve", "твелв");
    m.insert("thirteen", "сёртин");
    m.insert("seventeen", "сэвентин");
    m.insert("twenty", "твенти");
    // Common code terms
    m.insert("eval", "эвал");
    m.insert("plus", "плас");
    m.insert("succ", "сакс");
    m.insert("synthesize", "синтесайз");
    m.insert("addition", "эдишн");
    // Common type/term names
    m.insert("nat", "нат");
    m.insert("uint", "юинт");
    m.insert("float", "флоат");
    m.insert("double", "дабл");
    m.insert("trait", "трейт");
    m.insert("traits", "трейтс");
    m.insert("impl", "импл");
    m.insert("async", "асинк");
    m.insert("await", "эвейт");
    m.insert("const", "конст");
    m.insert("static", "статик");
    m.insert("override", "оверрайд");
    m.insert("virtual", "виртуал");
    m.insert("abstract", "абстракт");
    m.insert("private", "прайвит");
    m.insert("protected", "протектед");
    m.insert("generic", "дженерик");
    m.insert("template", "темплейт");
    // Git/VCS terms
    m.insert("feature", "фича");
    m.insert("branch", "бранч");
    m.insert("merge", "мёрдж");
    m.insert("commit", "коммит");
    m.insert("pull", "пулл");
    m.insert("checkout", "чекаут");
    m.insert("rebase", "рибейз");
    m.insert("stash", "стэш");
    // Development process
    m.insert("review", "ревью");
    m.insert("deploy", "деплой");
    m.insert("release", "релиз");
    m.insert("debug", "дебаг");
    m.insert("bug", "баг");
    m.insert("refactor", "рефакторинг");
    m.insert("agile", "эджайл");
    m.insert("scrum", "скрам");
    // Architecture/Code
    m.insert("framework", "фреймворк");
    m.insert("library", "лайбрари");
    m.insert("package", "пакет");
    m.insert("module", "модуль");
    m.insert("function", "функция");
    m.insert("method", "метод");
    m.insert("class", "класс");
    m.insert("object", "объект");
    m.insert("interface", "интерфейс");
    m.insert("callback", "коллбэк");
    m.insert("promise", "промис");
    m.insert("handler", "хендлер");
    m.insert("listener", "листенер");
    m.insert("middleware", "мидлвэр");
    m.insert("endpoint", "эндпоинт");
    m.insert("router", "роутер");
    m.insert("controller", "контроллер");
    m.insert("service", "сервис");
    m.insert("repository", "репозиторий");
    // Data
    m.insert("cache", "кэш");
    m.insert("queue", "кью");
    m.insert("array", "массив");
    m.insert("string", "строка");
    m.insert("boolean", "булеан");
    m.insert("null", "налл");
    m.insert("undefined", "андефайнд");
    m.insert("default", "дефолт");
    m.insert("index", "индекс");
    m.insert("query", "квери");
    // Infrastructure
    m.insert("docker", "докер");
    m.insert("container", "контейнер");
    m.insert("kubernetes", "кубернетис");
    m.insert("cluster", "кластер");
    m.insert("node", "нода");
    m.insert("pod", "под");
    m.insert("nginx", "энджинкс");
    m.insert("backup", "бэкап");
    m.insert("client", "клиент");
    // Testing
    m.insert("test", "тест");
    m.insert("mock", "мок");
    m.insert("stub", "стаб");
    m.insert("spec", "спек");
    // Build
    m.insert("build", "билд");
    m.insert("bundle", "бандл");
    m.insert("compile", "компайл");
    m.insert("webpack", "вебпак");
    // Programming languages
    m.insert("python", "пайтон");
    m.insert("typescript", "тайпскрипт");
    m.insert("rust", "раст");
    m.insert("golang", "голанг");
    m.insert("kotlin", "котлин");
    // Frameworks and tools
    m.insert("react", "риакт");
    m.insert("angular", "ангуляр");
    m.insert("vue", "вью");
    m.insert("svelte", "свелт");
    m.insert("next", "некст");
    m.insert("express", "экспресс");
    m.insert("django", "джанго");
    m.insert("flask", "фласк");
    m.insert("fastapi", "фаст эй пи ай");
    m.insert("laravel", "ларавел");
    m.insert("redis", "редис");
    m.insert("mongo", "монго");
    m.insert("postgres", "постгрес");
    m.insert("github", "гитхаб");
    m.insert("jira", "джира");
    m.insert("slack", "слэк");
    m.insert("postman", "постман");
    // Additional common terms
    m.insert("request", "реквест");
    m.insert("trace", "трейс");
    m.insert("daily", "дейли");
    m.insert("standup", "стендап");
    m.insert("hot", "хот");
    m.insert("reload", "релоуд");
    m.insert("tech", "тек");
    m.insert("debt", "дет");
    m.insert("code", "код");
    m.insert("smell", "смелл");
    m.insert("best", "бест");
    m.insert("practice", "практис");
    m.insert("use", "юз");
    m.insert("case", "кейс");
    // Common words in paths/URLs
    m.insert("home", "хоум");
    m.insert("docs", "докс");
    m.insert("user", "юзер");
    m.insert("users", "юзерс");
    m.insert("admin", "админ");
    m.insert("support", "саппорт");
    m.insert("config", "конфиг");
    m.insert("data", "дата");
    m.insert("files", "файлс");
    m.insert("download", "даунлоад");
    m.insert("upload", "аплоад");
    m.insert("report", "репорт");
    m.insert("documents", "документс");
    m.insert("localhost", "локалхост");
    m.insert("api", "эй пи ай");
    m.insert("app", "апп");
    m.insert("web", "веб");
    m.insert("src", "сорс");
    m.insert("tmp", "темп");
    m.insert("etc", "етс");
    m.insert("opt", "опт");
    // File extensions
    m.insert("pdf", "пдф");
    m.insert("doc", "док");
    m.insert("txt", "тэкст");
    m.insert("csv", "си эс ви");
    m.insert("png", "пнг");
    m.insert("jpg", "джэйпег");
    m.insert("svg", "эс ви джи");
    m.insert("mp3", "эм пэ три");
    m.insert("mp4", "эм пэ четыре");
    // Common words
    m.insert("hello", "хеллоу");
    m.insert("world", "ворлд");
    m.insert("example", "экзампл");
    m.insert("tutorial", "тьюториал");
    m.insert("company", "компани");
    m.insert("repo", "репо");
    m
});

/// Multi-word IT phrases checked before single words.
static MULTI_WORD_PHRASES: Lazy<Vec<(&'static str, &'static str)>> = Lazy::new(|| {
    let mut phrases = vec![
        ("pull request", "пулл реквест"),
        ("merge request", "мёрдж реквест"),
        ("code review", "код ревью"),
        ("feature branch", "фича бранч"),
        ("stack trace", "стэк трейс"),
        ("daily standup", "дейли стендап"),
        ("hot fix", "хот фикс"),
        ("hot reload", "хот релоуд"),
        ("live reload", "лайв релоуд"),
        ("dry run", "драй ран"),
        ("tech debt", "тек дет"),
        ("code smell", "код смелл"),
        ("best practice", "бест практис"),
        ("use case", "юз кейс"),
        ("edge case", "эдж кейс"),
    ];
    // Sort by descending length for longest-match behaviour
    phrases.sort_by(|a, b| b.0.len().cmp(&a.0.len()));
    phrases
});

/// Digraph-first transliteration patterns (longest-match via aho-corasick).
///
/// Digraphs must appear before single letters in the patterns list; aho-corasick
/// is built with `MatchKind::LeftmostLongest` which picks the longest match at
/// each position.
static TRANSLIT_AC: Lazy<(AhoCorasick, Vec<&'static str>)> = Lazy::new(|| {
    // Patterns ordered longest-first so LeftmostLongest works correctly for
    // patterns of equal start position.
    let patterns: &[(&str, &str)] = &[
        // 4-char sequences
        ("tion", "шн"),
        ("sion", "жн"),
        // Digraphs
        ("sh", "ш"),
        ("ch", "ч"),
        ("th", "с"),
        ("ph", "ф"),
        ("wh", "в"),
        ("ck", "к"),
        ("gh", "г"),
        ("ng", "нг"),
        ("qu", "кв"),
        ("ee", "и"),
        ("oo", "у"),
        ("ea", "и"),
        ("ou", "ау"),
        ("ow", "оу"),
        ("ai", "эй"),
        ("ay", "эй"),
        ("ey", "эй"),
        ("ei", "эй"),
        ("ie", "и"),
        ("oa", "оу"),
        ("oi", "ой"),
        ("oy", "ой"),
        ("au", "о"),
        ("aw", "о"),
        ("ew", "ью"),
        // Single letters
        ("a", "а"),
        ("b", "б"),
        ("c", "к"),
        ("d", "д"),
        ("e", "е"),
        ("f", "ф"),
        ("g", "г"),
        ("h", "х"),
        ("i", "и"),
        ("j", "дж"),
        ("k", "к"),
        ("l", "л"),
        ("m", "м"),
        ("n", "н"),
        ("o", "о"),
        ("p", "п"),
        ("q", "к"),
        ("r", "р"),
        ("s", "с"),
        ("t", "т"),
        ("u", "у"),
        ("v", "в"),
        ("w", "в"),
        ("x", "кс"),
        ("y", "и"),
        ("z", "з"),
    ];

    let (pats, repls): (Vec<&str>, Vec<&str>) = patterns.iter().cloned().unzip();

    let ac = AhoCorasickBuilder::new()
        .match_kind(MatchKind::LeftmostLongest)
        .ascii_case_insensitive(true)
        .build(pats.clone())
        .expect("valid aho-corasick patterns");

    (ac, repls)
});

/// Normalizes an English word or phrase to Russian phonetic spelling.
///
/// Lookup order:
/// 1. Multi-word phrases (exact, case-insensitive, longest match).
/// 2. Custom user-supplied terms.
/// 3. Built-in `IT_TERMS` dictionary.
/// 4. Simple character-by-character transliteration via `TRANSLIT_AC`.
pub struct EnglishNormalizer {
    custom_terms: HashMap<String, String>,
    unknown_words: HashMap<String, String>,
}

impl Default for EnglishNormalizer {
    fn default() -> Self {
        Self::new()
    }
}

impl EnglishNormalizer {
    pub fn new() -> Self {
        Self {
            custom_terms: HashMap::new(),
            unknown_words: HashMap::new(),
        }
    }

    /// Add user-supplied term overrides (merged with existing).
    pub fn add_custom_terms(&mut self, terms: &HashMap<String, String>) {
        for (k, v) in terms {
            self.custom_terms.insert(k.to_lowercase(), v.clone());
        }
    }

    /// Normalize an English word or multi-word phrase.
    ///
    /// `track_unknown` — if `true`, words resolved via transliteration are
    /// stored in `unknown_words` for later inspection.
    pub fn normalize(&mut self, text: &str, track_unknown: bool) -> String {
        if text.is_empty() {
            return text.to_string();
        }

        let text_lower = text.to_lowercase();

        // 1. Multi-word phrases (exact match, already sorted longest-first)
        for (phrase, translation) in MULTI_WORD_PHRASES.iter() {
            if text_lower == *phrase {
                return translation.to_string();
            }
        }

        // 2. Custom terms
        if let Some(v) = self.custom_terms.get(text_lower.as_str()) {
            return v.clone();
        }

        // 3. Built-in IT_TERMS
        if let Some(v) = IT_TERMS.get(text_lower.as_str()) {
            return v.to_string();
        }

        // 4. Transliteration fallback
        let result = transliterate_simple(&text_lower);

        if track_unknown && !self.unknown_words.contains_key(text_lower.as_str()) {
            self.unknown_words
                .insert(text_lower.clone(), result.clone());
        }

        result
    }

    /// Returns a snapshot of words resolved via transliteration fallback.
    pub fn get_unknown_words(&self) -> &HashMap<String, String> {
        &self.unknown_words
    }

    /// Clear the unknown-words tracking map.
    pub fn clear_unknown_words(&mut self) {
        self.unknown_words.clear();
    }
}

/// Transliterate an ASCII lowercase string to Russian using digraph-first rules.
///
/// Non-ASCII / non-alpha characters (digits, punctuation) are passed through as-is.
pub fn transliterate_simple(input: &str) -> String {
    let (ac, repls) = &*TRANSLIT_AC;

    // Process only the ASCII-alphabetic prefix; preserve other chars verbatim.
    // We iterate byte-by-byte and let aho-corasick handle the alpha runs.
    let input_lower = input.to_lowercase();
    let bytes = input_lower.as_bytes();
    let mut result = String::with_capacity(input_lower.len() * 2);
    let mut i = 0;

    while i < bytes.len() {
        if bytes[i].is_ascii_alphabetic() {
            // Find the end of this alpha run
            let start = i;
            while i < bytes.len() && bytes[i].is_ascii_alphabetic() {
                i += 1;
            }
            let segment = &input_lower[start..i];

            // Replace using aho-corasick (leftmost-longest)
            let mut last = 0usize;
            for m in ac.find_iter(segment) {
                // Pass through any gap before this match (should not happen for
                // pure alpha input, but be defensive)
                result.push_str(&segment[last..m.start()]);
                result.push_str(repls[m.pattern().as_usize()]);
                last = m.end();
            }
            result.push_str(&segment[last..]);
        } else {
            // Non-alpha character: pass through verbatim
            result.push(bytes[i] as char);
            i += 1;
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn normalizer() -> EnglishNormalizer {
        EnglishNormalizer::new()
    }

    // ── IT_TERMS dictionary ───────────────────────────────────────────────────

    #[test]
    fn test_it_terms_git() {
        let mut en = normalizer();
        assert_eq!(en.normalize("feature", false), "фича");
        assert_eq!(en.normalize("branch", false), "бранч");
        assert_eq!(en.normalize("merge", false), "мёрдж");
        assert_eq!(en.normalize("commit", false), "коммит");
        assert_eq!(en.normalize("pull", false), "пулл");
        assert_eq!(en.normalize("checkout", false), "чекаут");
        assert_eq!(en.normalize("rebase", false), "рибейз");
        assert_eq!(en.normalize("stash", false), "стэш");
    }

    #[test]
    fn test_it_terms_dev_process() {
        let mut en = normalizer();
        assert_eq!(en.normalize("review", false), "ревью");
        assert_eq!(en.normalize("deploy", false), "деплой");
        assert_eq!(en.normalize("release", false), "релиз");
        assert_eq!(en.normalize("debug", false), "дебаг");
        assert_eq!(en.normalize("bug", false), "баг");
        assert_eq!(en.normalize("refactor", false), "рефакторинг");
        assert_eq!(en.normalize("scrum", false), "скрам");
        assert_eq!(en.normalize("agile", false), "эджайл");
    }

    #[test]
    fn test_it_terms_architecture() {
        let mut en = normalizer();
        assert_eq!(en.normalize("framework", false), "фреймворк");
        assert_eq!(en.normalize("library", false), "лайбрари");
        assert_eq!(en.normalize("package", false), "пакет");
        assert_eq!(en.normalize("module", false), "модуль");
        assert_eq!(en.normalize("function", false), "функция");
        assert_eq!(en.normalize("method", false), "метод");
        assert_eq!(en.normalize("class", false), "класс");
        assert_eq!(en.normalize("object", false), "объект");
        assert_eq!(en.normalize("interface", false), "интерфейс");
        assert_eq!(en.normalize("callback", false), "коллбэк");
        assert_eq!(en.normalize("promise", false), "промис");
        assert_eq!(en.normalize("handler", false), "хендлер");
        assert_eq!(en.normalize("listener", false), "листенер");
        assert_eq!(en.normalize("middleware", false), "мидлвэр");
        assert_eq!(en.normalize("endpoint", false), "эндпоинт");
        assert_eq!(en.normalize("router", false), "роутер");
        assert_eq!(en.normalize("controller", false), "контроллер");
        assert_eq!(en.normalize("service", false), "сервис");
        assert_eq!(en.normalize("repository", false), "репозиторий");
    }

    #[test]
    fn test_it_terms_data() {
        let mut en = normalizer();
        assert_eq!(en.normalize("cache", false), "кэш");
        assert_eq!(en.normalize("queue", false), "кью");
        assert_eq!(en.normalize("array", false), "массив");
        assert_eq!(en.normalize("string", false), "строка");
        assert_eq!(en.normalize("boolean", false), "булеан");
        assert_eq!(en.normalize("null", false), "налл");
        assert_eq!(en.normalize("undefined", false), "андефайнд");
        assert_eq!(en.normalize("default", false), "дефолт");
        assert_eq!(en.normalize("index", false), "индекс");
        assert_eq!(en.normalize("query", false), "квери");
    }

    #[test]
    fn test_it_terms_infrastructure() {
        let mut en = normalizer();
        assert_eq!(en.normalize("docker", false), "докер");
        assert_eq!(en.normalize("container", false), "контейнер");
        assert_eq!(en.normalize("kubernetes", false), "кубернетис");
        assert_eq!(en.normalize("cluster", false), "кластер");
        assert_eq!(en.normalize("node", false), "нода");
        assert_eq!(en.normalize("pod", false), "под");
        assert_eq!(en.normalize("nginx", false), "энджинкс");
        assert_eq!(en.normalize("backup", false), "бэкап");
        assert_eq!(en.normalize("client", false), "клиент");
    }

    #[test]
    fn test_it_terms_testing_and_build() {
        let mut en = normalizer();
        assert_eq!(en.normalize("test", false), "тест");
        assert_eq!(en.normalize("mock", false), "мок");
        assert_eq!(en.normalize("stub", false), "стаб");
        assert_eq!(en.normalize("spec", false), "спек");
        assert_eq!(en.normalize("build", false), "билд");
        assert_eq!(en.normalize("bundle", false), "бандл");
        assert_eq!(en.normalize("compile", false), "компайл");
        assert_eq!(en.normalize("webpack", false), "вебпак");
    }

    #[test]
    fn test_it_terms_languages() {
        let mut en = normalizer();
        assert_eq!(en.normalize("python", false), "пайтон");
        assert_eq!(en.normalize("typescript", false), "тайпскрипт");
        assert_eq!(en.normalize("rust", false), "раст");
        assert_eq!(en.normalize("golang", false), "голанг");
        assert_eq!(en.normalize("kotlin", false), "котлин");
    }

    #[test]
    fn test_it_terms_frameworks() {
        let mut en = normalizer();
        assert_eq!(en.normalize("react", false), "риакт");
        assert_eq!(en.normalize("angular", false), "ангуляр");
        assert_eq!(en.normalize("vue", false), "вью");
        assert_eq!(en.normalize("svelte", false), "свелт");
        assert_eq!(en.normalize("next", false), "некст");
        assert_eq!(en.normalize("express", false), "экспресс");
        assert_eq!(en.normalize("django", false), "джанго");
        assert_eq!(en.normalize("flask", false), "фласк");
        assert_eq!(en.normalize("fastapi", false), "фаст эй пи ай");
        assert_eq!(en.normalize("laravel", false), "ларавел");
        assert_eq!(en.normalize("redis", false), "редис");
        assert_eq!(en.normalize("mongo", false), "монго");
        assert_eq!(en.normalize("postgres", false), "постгрес");
        assert_eq!(en.normalize("github", false), "гитхаб");
        assert_eq!(en.normalize("jira", false), "джира");
        assert_eq!(en.normalize("slack", false), "слэк");
        assert_eq!(en.normalize("postman", false), "постман");
    }

    // ── Case insensitivity ────────────────────────────────────────────────────

    #[test]
    fn test_case_insensitivity() {
        let mut en = normalizer();
        assert_eq!(en.normalize("Feature", false), "фича");
        assert_eq!(en.normalize("BRANCH", false), "бранч");
        assert_eq!(en.normalize("Merge", false), "мёрдж");
        assert_eq!(en.normalize("COMMIT", false), "коммит");
    }

    // ── Multi-word phrases ────────────────────────────────────────────────────

    #[test]
    fn test_multiword_phrases() {
        let mut en = normalizer();
        assert_eq!(en.normalize("pull request", false), "пулл реквест");
        assert_eq!(en.normalize("merge request", false), "мёрдж реквест");
        assert_eq!(en.normalize("code review", false), "код ревью");
        assert_eq!(en.normalize("feature branch", false), "фича бранч");
        assert_eq!(en.normalize("stack trace", false), "стэк трейс");
        assert_eq!(en.normalize("daily standup", false), "дейли стендап");
        assert_eq!(en.normalize("hot fix", false), "хот фикс");
        assert_eq!(en.normalize("hot reload", false), "хот релоуд");
        assert_eq!(en.normalize("live reload", false), "лайв релоуд");
        assert_eq!(en.normalize("dry run", false), "драй ран");
        assert_eq!(en.normalize("tech debt", false), "тек дет");
        assert_eq!(en.normalize("code smell", false), "код смелл");
        assert_eq!(en.normalize("best practice", false), "бест практис");
        assert_eq!(en.normalize("use case", false), "юз кейс");
        assert_eq!(en.normalize("edge case", false), "эдж кейс");
    }

    #[test]
    fn test_multiword_phrases_case_insensitive() {
        let mut en = normalizer();
        assert_eq!(en.normalize("Pull Request", false), "пулл реквест");
        assert_eq!(en.normalize("CODE REVIEW", false), "код ревью");
    }

    // ── Transliteration (simple fallback) ────────────────────────────────────

    #[test]
    fn test_translit_push() {
        // p→п, u→у, sh→ш
        assert_eq!(transliterate_simple("push"), "пуш");
    }

    #[test]
    fn test_translit_fix() {
        // f→ф, i→и, x→кс
        assert_eq!(transliterate_simple("fix"), "фикс");
    }

    #[test]
    fn test_translit_sprint() {
        // s→с, p→п, r→р, i→и, n→н, t→т
        assert_eq!(transliterate_simple("sprint"), "спринт");
    }

    #[test]
    fn test_translit_lint() {
        // l→л, i→и, n→н, t→т
        assert_eq!(transliterate_simple("lint"), "линт");
    }

    #[test]
    fn test_translit_javascript() {
        // j→дж, a→а, v→в, a→а, s→с, c→к, r→р, i→и, p→п, t→т
        assert_eq!(transliterate_simple("javascript"), "джаваскрипт");
    }

    #[test]
    fn test_translit_swift() {
        // s→с, w→в, i→и, f→ф, t→т
        assert_eq!(transliterate_simple("swift"), "свифт");
    }

    #[test]
    fn test_translit_java() {
        // j→дж, a→а, v→в, a→а
        assert_eq!(transliterate_simple("java"), "джава");
    }

    #[test]
    fn test_translit_ruby() {
        // r→р, u→у, b→б, y→и
        assert_eq!(transliterate_simple("ruby"), "руби");
    }

    #[test]
    fn test_translit_scala() {
        // s→с, c→к, a→а, l→л, a→а
        assert_eq!(transliterate_simple("scala"), "скала");
    }

    #[test]
    fn test_translit_spring() {
        // s→с, p→п, r→р, i→и, ng→нг
        assert_eq!(transliterate_simple("spring"), "спринг");
    }

    #[test]
    fn test_translit_gitlab() {
        // g→г, i→и, t→т, l→л, a→а, b→б
        assert_eq!(transliterate_simple("gitlab"), "гитлаб");
    }

    #[test]
    fn test_translit_figma() {
        // f→ф, i→и, g→г, m→м, a→а
        assert_eq!(transliterate_simple("figma"), "фигма");
    }

    #[test]
    fn test_translit_kafka() {
        // k→к, a→а, f→ф, k→к, a→а
        assert_eq!(transliterate_simple("kafka"), "кафка");
    }

    #[test]
    fn test_translit_server() {
        // s→с, er→ер, v→в, er→ер
        assert_eq!(transliterate_simple("server"), "сервер");
    }

    #[test]
    fn test_translit_digraphs() {
        // sh, ch, th, ph, ng, ck
        assert_eq!(transliterate_simple("ship"), "шип");
        assert_eq!(transliterate_simple("chip"), "чип");
        assert_eq!(transliterate_simple("sing"), "синг");
        assert_eq!(transliterate_simple("back"), "бак");
        assert_eq!(transliterate_simple("phone"), "фоне");
    }

    #[test]
    fn test_translit_vowel_digraphs() {
        assert_eq!(transliterate_simple("see"), "си");
        assert_eq!(transliterate_simple("moon"), "мун");
        assert_eq!(transliterate_simple("rain"), "рэйн");
        assert_eq!(transliterate_simple("boy"), "бой");
    }

    #[test]
    fn test_translit_preserves_digits() {
        // Non-alpha characters pass through verbatim; alpha chars transliterate
        assert_eq!(transliterate_simple("mp3"), "мп3");
        assert_eq!(transliterate_simple("v8"), "в8");
    }

    #[test]
    fn test_translit_empty() {
        assert_eq!(transliterate_simple(""), "");
    }

    // ── Custom terms ──────────────────────────────────────────────────────────

    #[test]
    fn test_custom_terms_override_it_terms() {
        let mut en = normalizer();
        let mut custom = HashMap::new();
        custom.insert("api".to_string(), "апи".to_string());
        en.add_custom_terms(&custom);
        assert_eq!(en.normalize("api", false), "апи");
        assert_eq!(en.normalize("API", false), "апи");
    }

    #[test]
    fn test_custom_terms_new_word() {
        let mut en = normalizer();
        let mut custom = HashMap::new();
        custom.insert("foobar".to_string(), "фубар".to_string());
        en.add_custom_terms(&custom);
        assert_eq!(en.normalize("foobar", false), "фубар");
    }

    #[test]
    fn test_custom_terms_case_insensitive_key() {
        let mut en = normalizer();
        let mut custom = HashMap::new();
        custom.insert("MyTerm".to_string(), "майтёрм".to_string());
        en.add_custom_terms(&custom);
        assert_eq!(en.normalize("myterm", false), "майтёрм");
        assert_eq!(en.normalize("MYTERM", false), "майтёрм");
    }

    // ── Unknown word tracking ─────────────────────────────────────────────────

    #[test]
    fn test_unknown_word_tracking_enabled() {
        let mut en = normalizer();
        // "xyzzy" is not in IT_TERMS → tracked
        let result = en.normalize("xyzzy", true);
        assert!(!result.is_empty());
        assert!(en.get_unknown_words().contains_key("xyzzy"));
        assert_eq!(en.get_unknown_words()["xyzzy"], result);
    }

    #[test]
    fn test_unknown_word_tracking_disabled() {
        let mut en = normalizer();
        en.normalize("xyzzy", false);
        assert!(!en.get_unknown_words().contains_key("xyzzy"));
    }

    #[test]
    fn test_known_word_not_tracked() {
        let mut en = normalizer();
        en.normalize("api", true);
        assert!(!en.get_unknown_words().contains_key("api"));
    }

    #[test]
    fn test_clear_unknown_words() {
        let mut en = normalizer();
        en.normalize("xyzzy", true);
        en.normalize("quux", true);
        assert_eq!(en.get_unknown_words().len(), 2);
        en.clear_unknown_words();
        assert!(en.get_unknown_words().is_empty());
    }

    #[test]
    fn test_unknown_word_deduplicated() {
        let mut en = normalizer();
        let r1 = en.normalize("xyzzy", true);
        let r2 = en.normalize("xyzzy", true);
        assert_eq!(r1, r2);
        assert_eq!(en.get_unknown_words().len(), 1);
    }

    // ── Empty / edge cases ────────────────────────────────────────────────────

    #[test]
    fn test_empty_string() {
        let mut en = normalizer();
        assert_eq!(en.normalize("", false), "");
    }

    #[test]
    fn test_it_terms_with_plus_signs() {
        let mut en = normalizer();
        assert_eq!(en.normalize("c++", false), "си плюс плюс");
        assert_eq!(en.normalize("c#", false), "си шарп");
        assert_eq!(en.normalize("f#", false), "эф шарп");
    }

    // ── Programming languages resolved via transliteration ───────────────────

    #[test]
    fn test_normalize_javascript_via_translit() {
        let mut en = normalizer();
        assert_eq!(en.normalize("javascript", false), "джаваскрипт");
    }

    #[test]
    fn test_normalize_swift_via_translit() {
        let mut en = normalizer();
        assert_eq!(en.normalize("swift", false), "свифт");
    }

    #[test]
    fn test_normalize_java_via_translit() {
        let mut en = normalizer();
        assert_eq!(en.normalize("java", false), "джава");
    }

    #[test]
    fn test_normalize_ruby_via_translit() {
        let mut en = normalizer();
        assert_eq!(en.normalize("ruby", false), "руби");
    }

    #[test]
    fn test_normalize_scala_via_translit() {
        let mut en = normalizer();
        assert_eq!(en.normalize("scala", false), "скала");
    }

    #[test]
    fn test_normalize_spring_via_translit() {
        let mut en = normalizer();
        assert_eq!(en.normalize("spring", false), "спринг");
    }

    #[test]
    fn test_normalize_gitlab_via_translit() {
        let mut en = normalizer();
        assert_eq!(en.normalize("gitlab", false), "гитлаб");
    }

    #[test]
    fn test_normalize_figma_via_translit() {
        let mut en = normalizer();
        assert_eq!(en.normalize("figma", false), "фигма");
    }

    #[test]
    fn test_normalize_kafka_via_translit() {
        let mut en = normalizer();
        assert_eq!(en.normalize("kafka", false), "кафка");
    }

    #[test]
    fn test_normalize_server_via_translit() {
        let mut en = normalizer();
        assert_eq!(en.normalize("server", false), "сервер");
    }

    #[test]
    fn test_normalize_push_via_translit() {
        let mut en = normalizer();
        assert_eq!(en.normalize("push", false), "пуш");
    }

    #[test]
    fn test_normalize_fix_via_translit() {
        let mut en = normalizer();
        assert_eq!(en.normalize("fix", false), "фикс");
    }

    #[test]
    fn test_normalize_sprint_via_translit() {
        let mut en = normalizer();
        assert_eq!(en.normalize("sprint", false), "спринт");
    }

    #[test]
    fn test_normalize_lint_via_translit() {
        let mut en = normalizer();
        assert_eq!(en.normalize("lint", false), "линт");
    }
}
