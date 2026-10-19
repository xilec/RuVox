use aho_corasick::{AhoCorasick, AhoCorasickBuilder, MatchKind};
use once_cell::sync::Lazy;
use std::collections::HashMap;

/// English letter names — the single home for letter-name readings
/// (issue #120). Used by abbreviation spelling, code-identifier spelling,
/// and lone-letter reading, so "x" sounds "икс" in every context.
/// Keys are lowercase.
pub static LETTER_NAMES: [(char, &str); 26] = [
    ('a', "эй"),
    ('b', "би"),
    ('c', "си"),
    ('d', "ди"),
    ('e', "и"),
    ('f', "эф"),
    ('g', "джи"),
    ('h', "эйч"),
    ('i', "ай"),
    ('j', "джей"),
    ('k', "кей"),
    ('l', "эл"),
    ('m', "эм"),
    ('n', "эн"),
    ('o', "о"),
    ('p', "пи"),
    ('q', "кью"),
    ('r', "ар"),
    ('s', "эс"),
    ('t', "ти"),
    ('u', "ю"),
    ('v', "ви"),
    ('w', "дабл ю"),
    ('x', "икс"),
    ('y', "вай"),
    ('z', "зет"),
];

/// Look up an English letter name case-insensitively.
pub fn letter_name(c: char) -> Option<&'static str> {
    let lower = c.to_ascii_lowercase();
    LETTER_NAMES
        .iter()
        .find(|(k, _)| *k == lower)
        .map(|(_, v)| *v)
}

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
    m.insert("sync", "синк");
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
    m.insert("pdf", "пэ дэ эф");
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
    // Common words that digraph transliteration misreads (silent "e",
    // vowel digraphs it does not know) — issue #139
    m.insert("byte", "байт");
    m.insert("change", "чейндж");
    m.insert("device", "девайс");
    m.insert("drive", "драйв");
    m.insert("false", "фолс");
    m.insert("file", "файл");
    m.insert("guide", "гайд");
    m.insert("image", "имидж");
    m.insert("issue", "ишью");
    m.insert("line", "лайн");
    m.insert("mime", "майм");
    m.insert("mode", "мод");
    m.insert("page", "пейдж");
    m.insert("pipe", "пайп");
    m.insert("site", "сайт");
    m.insert("size", "сайз");
    m.insert("source", "сорс");
    m.insert("stage", "стейдж");
    // Extra IT terms formerly backed only by exception fallback
    m.insert("ajax", "эйджакс");
    m.insert("cd", "си ди");
    m.insert("crud", "крад");
    m.insert("graphql", "граф кью эль");
    m.insert("imap", "ай мап");
    m.insert("ios", "ай оу эс");
    m.insert("iot", "ай о ти");
    m.insert("jpeg", "джейпег");
    m.insert("macos", "мак оу эс");
    m.insert("oauth", "о ауз");
    m.insert("true", "тру");
    m.insert("type", "тайп");
    m
});

/// Recognized English inflection suffixes for bounded morphology on known IT
/// stems. A suffix that merely ends in a shorter one is listed first (`ments`
/// and `ers` before `s`, `ies` before `es`) so the full suffix wins and the
/// stem is not truncated.
const INFLECTION_SUFFIXES: [&str; 12] = [
    "ation", "able", "ally", "ment", "ments", "ing", "ers", "er", "ies", "es", "ed", "s",
];

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
/// 2. Built-in `IT_TERMS` dictionary.
/// 3. Bounded inflection composition for known `IT_TERMS` stems.
/// 4. Simple character-by-character transliteration via `TRANSLIT_AC`.
///
/// User-supplied overrides do not live here: the user dictionary is applied
/// by the pipeline's dedicated pre-pass phase before English-word resolution
/// (change `user-dictionary`), which supersedes the former `custom_terms`
/// hook.
pub struct EnglishNormalizer {
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
            unknown_words: HashMap::new(),
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

        // 2. Built-in IT_TERMS
        if let Some(v) = IT_TERMS.get(text_lower.as_str()) {
            return v.to_string();
        }

        // 3. Bounded morphology for known IT stems (base reading + suffix reading)
        if let Some(composed) = self.compose_inflection(text) {
            return composed;
        }

        // 4. Transliteration fallback
        let result = transliterate_simple(&text_lower);

        if track_unknown && !self.unknown_words.contains_key(text_lower.as_str()) {
            self.unknown_words
                .insert(text_lower.clone(), result.clone());
        }

        result
    }

    /// Compose a curated `IT_TERMS` stem reading with the transliterated
    /// inflection suffix, e.g. "supporting" → "саппортинг".
    ///
    /// Fires only when the (case-insensitive) word ends in a recognized English
    /// inflection suffix (`INFLECTION_SUFFIXES`) and its base is a curated
    /// `IT_TERMS` entry. The base is the stripped stem, the stripped stem with a
    /// dropped silent "e" restored ("uses" → `use` + "es"), or — for "-ies" —
    /// `stem + "y"` ("repositories" → `repository`). Returns `None` for every
    /// other word — in particular for pure abbreviations such as "html" — so
    /// callers never get an arbitrary word transliterated through this path.
    pub fn compose_inflection(&self, word: &str) -> Option<String> {
        let word_lower = word.to_lowercase();
        for suffix in INFLECTION_SUFFIXES {
            if word_lower.len() <= suffix.len() {
                continue;
            }
            let Some(stem) = word_lower.strip_suffix(suffix) else {
                continue;
            };
            // A one-letter stem cannot hold a curated reading; two letters are
            // reachable only through the silent-e restoration below ("us" →
            // "use").
            if stem.len() < 2 {
                continue;
            }

            if suffix == "ies" {
                // "-ies" is the plural of "-y", so only a real "-y" base
                // composes ("repositories" → `repository`); a stem that happens
                // to end in "ies" otherwise ("species") must not compose. The
                // Russian reading drops the base's trailing "й", because the
                // plural adds a single "с".
                if let Some(base) = IT_TERMS.get(format!("{stem}y").as_str()) {
                    let trimmed = base.strip_suffix('й').unwrap_or(base);
                    return Some(format!("{trimmed}с"));
                }
                continue;
            }

            // Exact stem hit keeps its precedence and its minimum length, so
            // already-working forms are untouched.
            if stem.len() >= 3 {
                if let Some(base) = IT_TERMS.get(stem) {
                    return Some(format!("{}{}", base, transliterate_simple(suffix)));
                }
            }

            // The stem may have dropped a silent "e" before this suffix
            // ("us" → "use", "cach" → "cache"). "-ally" is excluded: it
            // attaches to an "-al" base ("practical" → "practically"), so a
            // restored silent "e" would compose the wrong base ("practice" →
            // "практисалли").
            if suffix == "ally" {
                continue;
            }
            let restored = format!("{stem}e");
            let Some(base) = IT_TERMS.get(restored.as_str()) else {
                continue;
            };
            // English spells a syllabic "es" only after a sibilant stem
            // ("uses" → "юзес", "caches" → "кэшес"); a non-sibilant stem takes
            // the plain "s" plural ("features" → "фичас"), which a shorter
            // suffix composes.
            if suffix == "es" && !es_reading_is_syllabic(stem) {
                continue;
            }
            return Some(format!("{}{}", base, transliterate_simple(suffix)));
        }
        None
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

/// Whether the English "-es" plural of a curated base that dropped a silent "e"
/// (`stem`, without that "e") is spelled with a syllabic "ес".
///
/// The plural is syllabic only after a sibilant ("uses" → "юзес", "caches" →
/// "кэшес"); any other stem takes a plain "s" ("features" → "фичас").
fn es_reading_is_syllabic(stem: &str) -> bool {
    ["ch", "sh", "s", "z", "x", "c", "g"]
        .iter()
        .any(|ending| stem.ends_with(ending))
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
            let mut segment = input_lower[start..i].to_string();
            for (old, new) in [("ce", "се"), ("ci", "си"), ("cy", "си")] {
                segment = segment.replace(old, new);
            }
            // Replace using aho-corasick (leftmost-longest)
            let mut last = 0usize;
            for m in ac.find_iter(&segment) {
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

    // Words that digraph transliteration misreads (silent "e" etc.) — #139.
    // "file" is the reported case: it was read "филе" while "files" already
    // had the correct "файлс" entry.
    #[test_case("file" => "файл"; "file")]
    #[test_case("byte" => "байт"; "byte")]
    #[test_case("page" => "пейдж"; "page")]
    #[test_case("site" => "сайт"; "site")]
    #[test_case("type" => "тайп"; "type_")]
    #[test_case("size" => "сайз"; "size")]
    #[test_case("line" => "лайн"; "line")]
    #[test_case("guide" => "гайд"; "guide")]
    #[test_case("image" => "имидж"; "image")]
    #[test_case("stage" => "стейдж"; "stage")]
    #[test_case("change" => "чейндж"; "change")]
    #[test_case("issue" => "ишью"; "issue")]
    #[test_case("device" => "девайс"; "device")]
    #[test_case("drive" => "драйв"; "drive")]
    #[test_case("mime" => "майм"; "mime")]
    #[test_case("pipe" => "пайп"; "pipe")]
    #[test_case("mode" => "мод"; "mode")]
    #[test_case("source" => "сорс"; "source")]
    #[test_case("true" => "тру"; "true_")]
    #[test_case("false" => "фолс"; "false_")]
    fn it_terms_misread_words(word: &str) -> String {
        normalizer().normalize(word, false)
    }

    // Extensions read letter-by-letter, like "html" — issue #140.
    #[test_case("pdf" => "пэ дэ эф"; "pdf")]
    #[test_case("csv" => "си эс ви"; "csv")]
    #[test_case("svg" => "эс ви джи"; "svg")]
    fn it_terms_file_extensions_spelled(word: &str) -> String {
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
    fn test_composed_form_not_tracked_but_raw_fallback_is() {
        // A composed inflected form (here with a restored silent "e") resolves
        // from a curated reading, so it is not an unknown word; a stem that no
        // longer composes through "-ies" falls back to transliteration and is
        // tracked.
        let mut en = normalizer();
        en.normalize("uses", true);
        assert!(!en.get_unknown_words().contains_key("uses"));
        en.normalize("species", true);
        assert!(en.get_unknown_words().contains_key("species"));
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
        let mut en = EnglishNormalizer::new();
        assert_eq!(en.normalize("", false), "");
    }

    // ── English morphological variants ───────────────────────────────────────

    #[test_case("support" => "саппорт"; "support")]
    #[test_case("supports" => "саппортс"; "supports")]
    #[test_case("supported" => "саппортед"; "supported")]
    #[test_case("supporting" => "саппортинг"; "supporting")]
    #[test_case("supporter" => "саппортер"; "supporter")]
    #[test_case("feature" => "фича"; "feature")]
    #[test_case("features" => "фичас"; "features")]
    #[test_case("using" => "юзинг"; "using")]
    #[test_case("uses" => "юзес"; "uses")]
    #[test_case("used" => "юзед"; "used")]
    #[test_case("caches" => "кэшес"; "caches")]
    #[test_case("cached" => "кэшед"; "cached")]
    #[test_case("repositories" => "репозиторис"; "repositories")]
    #[test_case("compilation" => "компайлашн"; "compilation")]
    #[test_case("practically" => "практикалли"; "practically")]
    #[test_case("null" => "налл"; "null")]
    #[test_case("nullable" => "наллабле"; "nullable")]
    #[test_case("build" => "билд"; "build")]
    #[test_case("building" => "билдинг"; "building")]
    #[test_case("builds" => "билдс"; "builds")]
    fn english_morphological_forms(word: &str) -> String {
        EnglishNormalizer::new().normalize(word, false)
    }

    // ── Inflection composition helper (#288) ─────────────────────────────────

    #[test]
    fn compose_inflection_composes_known_stems() {
        // The helper is the single source of truth for inflection phonetics,
        // shared by `normalize` and the pipeline's all-caps path.
        let en = EnglishNormalizer::new();
        assert_eq!(
            en.compose_inflection("supporting").as_deref(),
            Some("саппортинг")
        );
        assert_eq!(
            en.compose_inflection("building").as_deref(),
            Some("билдинг")
        );
        assert_eq!(en.compose_inflection("features").as_deref(), Some("фичас"));
    }

    #[test]
    fn compose_inflection_is_case_insensitive() {
        // Composition does not depend on the word's letter case, so an
        // all-uppercase inflected form composes the curated stem reading too.
        let en = EnglishNormalizer::new();
        assert_eq!(
            en.compose_inflection("SUPPORTING").as_deref(),
            Some("саппортинг")
        );
        assert_eq!(en.compose_inflection("USES").as_deref(), Some("юзес"));
        assert_eq!(
            en.compose_inflection("REPOSITORIES").as_deref(),
            Some("репозиторис")
        );
    }

    #[test]
    fn compose_inflection_restores_dropped_silent_e() {
        // The stripped stem dropped a silent "e" ("us" → "use", "cach" →
        // "cache"); the restored base composes instead of degrading.
        let en = EnglishNormalizer::new();
        assert_eq!(en.compose_inflection("using").as_deref(), Some("юзинг"));
        assert_eq!(en.compose_inflection("uses").as_deref(), Some("юзес"));
        assert_eq!(en.compose_inflection("used").as_deref(), Some("юзед"));
        assert_eq!(en.compose_inflection("caches").as_deref(), Some("кэшес"));
        assert_eq!(en.compose_inflection("cached").as_deref(), Some("кэшед"));
    }

    #[test]
    fn compose_inflection_keeps_plain_s_for_non_sibilant_stems() {
        // A restored stem that is not sibilant takes the plain "s" plural: the
        // shorter suffix composes "feature" rather than "фичаес".
        let en = EnglishNormalizer::new();
        assert_eq!(en.compose_inflection("features").as_deref(), Some("фичас"));
        assert_eq!(
            en.compose_inflection("supports").as_deref(),
            Some("саппортс")
        );
    }

    #[test]
    fn compose_inflection_does_not_restore_silent_e_for_ally() {
        // "-ally" attaches to an "-al" base ("practical" → "practically"), so a
        // restored silent "e" would compose the wrong base; the helper must not
        // compose "practice" + "ally" ("практисалли").
        let en = EnglishNormalizer::new();
        assert_eq!(en.compose_inflection("practically"), None);
    }

    #[test]
    fn compose_inflection_reads_ies_from_y_stem() {
        // "-ies" is the plural of "-y": the base reading drops its trailing "й"
        // because the plural adds a single "с".
        let en = EnglishNormalizer::new();
        assert_eq!(
            en.compose_inflection("repositories").as_deref(),
            Some("репозиторис")
        );
    }

    #[test]
    fn compose_inflection_returns_none_for_abbreviations() {
        // A pure abbreviation has no recognized suffix with a curated stem, so
        // the helper must not transliterate it — otherwise letter-by-letter
        // readings would regress. A stem that is not a "-y" base must not
        // compose through the "-ies" branch either ("species" from "spec").
        let en = EnglishNormalizer::new();
        assert_eq!(en.compose_inflection("html"), None);
        assert_eq!(en.compose_inflection("API"), None);
        assert_eq!(en.compose_inflection("json"), None);
        assert_eq!(en.compose_inflection("species"), None);
    }
}
