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
    phrases.sort_by_key(|p| std::cmp::Reverse(p.0.len()));
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
    use test_case::test_case;

    fn normalizer() -> EnglishNormalizer {
        EnglishNormalizer::new()
    }

    // ── IT_TERMS dictionary ───────────────────────────────────────────────────

    #[test_case("feature" => "фича"; "feature")]
    #[test_case("branch" => "бранч"; "branch")]
    #[test_case("merge" => "мёрдж"; "merge")]
    #[test_case("commit" => "коммит"; "commit")]
    #[test_case("pull" => "пулл"; "pull")]
    #[test_case("checkout" => "чекаут"; "checkout")]
    #[test_case("rebase" => "рибейз"; "rebase")]
    #[test_case("stash" => "стэш"; "stash")]
    fn it_terms_git(word: &str) -> String {
        normalizer().normalize(word, false)
    }

    #[test_case("review" => "ревью"; "review")]
    #[test_case("deploy" => "деплой"; "deploy")]
    #[test_case("release" => "релиз"; "release")]
    #[test_case("debug" => "дебаг"; "debug")]
    #[test_case("bug" => "баг"; "bug")]
    #[test_case("refactor" => "рефакторинг"; "refactor")]
    #[test_case("scrum" => "скрам"; "scrum")]
    #[test_case("agile" => "эджайл"; "agile")]
    fn it_terms_dev_process(word: &str) -> String {
        normalizer().normalize(word, false)
    }

    #[test_case("framework" => "фреймворк"; "framework")]
    #[test_case("library" => "лайбрари"; "library")]
    #[test_case("package" => "пакет"; "package")]
    #[test_case("module" => "модуль"; "module")]
    #[test_case("function" => "функция"; "function")]
    #[test_case("method" => "метод"; "method")]
    #[test_case("class" => "класс"; "class")]
    #[test_case("object" => "объект"; "object")]
    #[test_case("interface" => "интерфейс"; "interface")]
    #[test_case("callback" => "коллбэк"; "callback")]
    #[test_case("promise" => "промис"; "promise")]
    #[test_case("handler" => "хендлер"; "handler")]
    #[test_case("listener" => "листенер"; "listener")]
    #[test_case("middleware" => "мидлвэр"; "middleware")]
    #[test_case("endpoint" => "эндпоинт"; "endpoint")]
    #[test_case("router" => "роутер"; "router")]
    #[test_case("controller" => "контроллер"; "controller")]
    #[test_case("service" => "сервис"; "service")]
    #[test_case("repository" => "репозиторий"; "repository")]
    fn it_terms_architecture(word: &str) -> String {
        normalizer().normalize(word, false)
    }

    #[test_case("cache" => "кэш"; "cache")]
    #[test_case("queue" => "кью"; "queue")]
    #[test_case("array" => "массив"; "array")]
    #[test_case("string" => "строка"; "string")]
    #[test_case("boolean" => "булеан"; "boolean")]
    #[test_case("null" => "налл"; "null")]
    #[test_case("undefined" => "андефайнд"; "undefined")]
    #[test_case("default" => "дефолт"; "default")]
    #[test_case("index" => "индекс"; "index")]
    #[test_case("query" => "квери"; "query")]
    fn it_terms_data(word: &str) -> String {
        normalizer().normalize(word, false)
    }

    #[test_case("docker" => "докер"; "docker")]
    #[test_case("container" => "контейнер"; "container")]
    #[test_case("kubernetes" => "кубернетис"; "kubernetes")]
    #[test_case("cluster" => "кластер"; "cluster")]
    #[test_case("node" => "нода"; "node")]
    #[test_case("pod" => "под"; "pod")]
    #[test_case("nginx" => "энджинкс"; "nginx")]
    #[test_case("backup" => "бэкап"; "backup")]
    #[test_case("client" => "клиент"; "client")]
    fn it_terms_infrastructure(word: &str) -> String {
        normalizer().normalize(word, false)
    }

    #[test_case("test" => "тест"; "test")]
    #[test_case("mock" => "мок"; "mock")]
    #[test_case("stub" => "стаб"; "stub")]
    #[test_case("spec" => "спек"; "spec")]
    #[test_case("build" => "билд"; "build")]
    #[test_case("bundle" => "бандл"; "bundle")]
    #[test_case("compile" => "компайл"; "compile")]
    #[test_case("webpack" => "вебпак"; "webpack")]
    fn it_terms_testing_and_build(word: &str) -> String {
        normalizer().normalize(word, false)
    }

    #[test_case("python" => "пайтон"; "python")]
    #[test_case("typescript" => "тайпскрипт"; "typescript")]
    #[test_case("rust" => "раст"; "rust")]
    #[test_case("golang" => "голанг"; "golang")]
    #[test_case("kotlin" => "котлин"; "kotlin")]
    fn it_terms_languages(word: &str) -> String {
        normalizer().normalize(word, false)
    }

    #[test_case("react" => "риакт"; "react")]
    #[test_case("angular" => "ангуляр"; "angular")]
    #[test_case("vue" => "вью"; "vue")]
    #[test_case("svelte" => "свелт"; "svelte")]
    #[test_case("next" => "некст"; "next")]
    #[test_case("express" => "экспресс"; "express")]
    #[test_case("django" => "джанго"; "django")]
    #[test_case("flask" => "фласк"; "flask")]
    #[test_case("fastapi" => "фаст эй пи ай"; "fastapi")]
    #[test_case("laravel" => "ларавел"; "laravel")]
    #[test_case("redis" => "редис"; "redis")]
    #[test_case("mongo" => "монго"; "mongo")]
    #[test_case("postgres" => "постгрес"; "postgres")]
    #[test_case("github" => "гитхаб"; "github")]
    #[test_case("jira" => "джира"; "jira")]
    #[test_case("slack" => "слэк"; "slack")]
    #[test_case("postman" => "постман"; "postman")]
    fn it_terms_frameworks(word: &str) -> String {
        normalizer().normalize(word, false)
    }

    #[test_case("c++" => "си плюс плюс"; "cpp")]
    #[test_case("c#" => "си шарп"; "csharp")]
    #[test_case("f#" => "эф шарп"; "fsharp")]
    fn it_terms_special_syntax(word: &str) -> String {
        normalizer().normalize(word, false)
    }

    // ── Case insensitivity ────────────────────────────────────────────────────

    #[test_case("Feature" => "фича"; "feature")]
    #[test_case("BRANCH" => "бранч"; "branch")]
    #[test_case("Merge" => "мёрдж"; "merge")]
    #[test_case("COMMIT" => "коммит"; "commit")]
    #[test_case("Pull Request" => "пулл реквест"; "pull_request_phrase")]
    #[test_case("CODE REVIEW" => "код ревью"; "code_review_phrase")]
    fn case_insensitive(word: &str) -> String {
        normalizer().normalize(word, false)
    }

    // ── Multi-word phrases ────────────────────────────────────────────────────

    #[test_case("pull request" => "пулл реквест"; "pull_request")]
    #[test_case("merge request" => "мёрдж реквест"; "merge_request")]
    #[test_case("code review" => "код ревью"; "code_review")]
    #[test_case("feature branch" => "фича бранч"; "feature_branch")]
    #[test_case("stack trace" => "стэк трейс"; "stack_trace")]
    #[test_case("daily standup" => "дейли стендап"; "daily_standup")]
    #[test_case("hot fix" => "хот фикс"; "hot_fix")]
    #[test_case("hot reload" => "хот релоуд"; "hot_reload")]
    #[test_case("live reload" => "лайв релоуд"; "live_reload")]
    #[test_case("dry run" => "драй ран"; "dry_run")]
    #[test_case("tech debt" => "тек дет"; "tech_debt")]
    #[test_case("code smell" => "код смелл"; "code_smell")]
    #[test_case("best practice" => "бест практис"; "best_practice")]
    #[test_case("use case" => "юз кейс"; "use_case")]
    #[test_case("edge case" => "эдж кейс"; "edge_case")]
    fn multiword_phrases(phrase: &str) -> String {
        normalizer().normalize(phrase, false)
    }

    // ── Transliteration (simple fallback), no duplicate elsewhere ────────────

    #[test_case("ship" => "шип"; "ship")]
    #[test_case("chip" => "чип"; "chip")]
    #[test_case("sing" => "синг"; "sing")]
    #[test_case("back" => "бак"; "back")]
    #[test_case("phone" => "фоне"; "phone")]
    #[test_case("see" => "си"; "see")]
    #[test_case("moon" => "мун"; "moon")]
    #[test_case("rain" => "рэйн"; "rain")]
    #[test_case("boy" => "бой"; "boy")]
    #[test_case("mp3" => "мп3"; "preserves_digits_mp3")]
    #[test_case("v8" => "в8"; "preserves_digits_v8")]
    #[test_case("" => ""; "empty")]
    fn translit_only(word: &str) -> String {
        transliterate_simple(word)
    }

    // ── Transliteration fallback: dedup of two call paths ────────────────────
    //
    // The same (word, expected) pairs used to be asserted twice: once directly
    // via `transliterate_simple`, once indirectly via `EnglishNormalizer::normalize`
    // (these words are not in IT_TERMS, so normalize() falls back to
    // transliterate_simple() internally). Both invariants are preserved here in
    // a single table instead of two separate test groups.

    #[test_case("push", "пуш"; "push")]
    #[test_case("fix", "фикс"; "fix")]
    #[test_case("sprint", "спринт"; "sprint")]
    #[test_case("lint", "линт"; "lint")]
    #[test_case("javascript", "джаваскрипт"; "javascript")]
    #[test_case("swift", "свифт"; "swift")]
    #[test_case("java", "джава"; "java")]
    #[test_case("ruby", "руби"; "ruby")]
    #[test_case("scala", "скала"; "scala")]
    #[test_case("spring", "спринг"; "spring")]
    #[test_case("gitlab", "гитлаб"; "gitlab")]
    #[test_case("figma", "фигма"; "figma")]
    #[test_case("kafka", "кафка"; "kafka")]
    #[test_case("server", "сервер"; "server")]
    fn translit_fallback_matches_normalize(word: &str, expected: &str) {
        assert_eq!(transliterate_simple(word), expected);
        assert_eq!(normalizer().normalize(word, false), expected);
    }

    // ── Custom terms ──────────────────────────────────────────────────────────

    #[test_case("api", "апи", &["api", "API"]; "override_it_terms")]
    #[test_case("foobar", "фубар", &["foobar"]; "new_word")]
    #[test_case("MyTerm", "майтёрм", &["myterm", "MYTERM"]; "case_insensitive_key")]
    fn custom_terms(key: &str, value: &str, queries: &[&str]) {
        let mut en = normalizer();
        let mut custom = HashMap::new();
        custom.insert(key.to_string(), value.to_string());
        en.add_custom_terms(&custom);
        for q in queries {
            assert_eq!(en.normalize(q, false), value);
        }
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
}
