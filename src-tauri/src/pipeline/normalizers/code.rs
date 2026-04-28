use std::collections::HashMap;

/// Number words for small integers used inside code identifiers.
/// Larger numbers are spelled out using a general routine.
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
        56 => "пятьдесят шесть".to_string(),
        60 => "шестьдесят".to_string(),
        64 => "шестьдесят четыре".to_string(),
        70 => "семьдесят".to_string(),
        80 => "восемьдесят".to_string(),
        90 => "девяносто".to_string(),
        100 => "сто".to_string(),
        128 => "сто двадцать восемь".to_string(),
        200 => "двести".to_string(),
        256 => "двести пятьдесят шесть".to_string(),
        512 => "пятьсот двенадцать".to_string(),
        1000 => "тысяча".to_string(),
        _ => {
            // Generic fallback: spell digit by digit
            n.to_string()
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
                .join(" ")
        }
    }
}

/// Normalizes code identifiers (camelCase, PascalCase, snake_case, kebab-case).
///
/// Cross-dependencies with NumberNormalizer (R2) and AbbreviationNormalizer (R4)
/// will be wired in R9 (pipeline integration). For now, this module contains
/// a self-sufficient implementation:
///   - number_to_russian() for numeric parts
///   - abbreviation detection (all-caps, 2+ chars) with letter-by-letter spelling
pub struct CodeIdentifierNormalizer {
    code_words: HashMap<&'static str, &'static str>,
    translit_map: HashMap<char, &'static str>,
}

impl CodeIdentifierNormalizer {
    pub fn new() -> Self {
        let mut code_words: HashMap<&'static str, &'static str> = HashMap::new();

        // Common verbs
        code_words.insert("get", "гет");
        code_words.insert("set", "сет");
        code_words.insert("is", "из");
        code_words.insert("has", "хэз");
        code_words.insert("can", "кэн");
        code_words.insert("on", "он");
        code_words.insert("off", "офф");
        code_words.insert("add", "адд");
        code_words.insert("remove", "ремув");
        code_words.insert("delete", "делит");
        code_words.insert("create", "криейт");
        code_words.insert("update", "апдейт");
        code_words.insert("find", "файнд");
        code_words.insert("search", "сёрч");
        code_words.insert("load", "лоуд");
        code_words.insert("save", "сейв");
        code_words.insert("read", "рид");
        code_words.insert("write", "райт");
        code_words.insert("send", "сенд");
        code_words.insert("receive", "ресив");
        code_words.insert("fetch", "фетч");
        code_words.insert("parse", "парс");
        code_words.insert("format", "формат");
        code_words.insert("convert", "конверт");
        code_words.insert("transform", "трансформ");
        code_words.insert("validate", "валидейт");
        code_words.insert("check", "чек");
        code_words.insert("handle", "хендл");
        code_words.insert("process", "процесс");
        code_words.insert("execute", "экзекьют");
        code_words.insert("run", "ран");
        code_words.insert("start", "старт");
        code_words.insert("stop", "стоп");
        code_words.insert("init", "инит");
        code_words.insert("close", "клоуз");
        code_words.insert("open", "оупен");
        code_words.insert("click", "клик");
        code_words.insert("change", "чейндж");
        code_words.insert("submit", "сабмит");
        code_words.insert("reset", "ризет");
        code_words.insert("clear", "клир");
        code_words.insert("show", "шоу");
        code_words.insert("hide", "хайд");
        code_words.insert("toggle", "тоггл");
        code_words.insert("enable", "энейбл");
        code_words.insert("disable", "дизейбл");
        code_words.insert("calculate", "калькулейт");
        code_words.insert("compute", "компьют");
        code_words.insert("render", "рендер");
        code_words.insert("mount", "маунт");
        code_words.insert("unmount", "анмаунт");
        code_words.insert("dispatch", "диспатч");
        code_words.insert("emit", "эмит");
        code_words.insert("listen", "лисен");
        code_words.insert("subscribe", "сабскрайб");
        code_words.insert("unsubscribe", "ансабскрайб");
        code_words.insert("connect", "коннект");
        code_words.insert("disconnect", "дисконнект");
        code_words.insert("encode", "энкоуд");
        code_words.insert("decode", "декоуд");
        // Common nouns
        code_words.insert("user", "юзер");
        code_words.insert("data", "дата");
        code_words.insert("item", "айтем");
        code_words.insert("list", "лист");
        code_words.insert("array", "эррей");
        code_words.insert("object", "обджект");
        code_words.insert("value", "вэлью");
        code_words.insert("key", "кей");
        code_words.insert("name", "нейм");
        code_words.insert("id", "ай ди");
        code_words.insert("type", "тайп");
        code_words.insert("size", "сайз");
        code_words.insert("count", "каунт");
        code_words.insert("index", "индекс");
        code_words.insert("length", "ленгс");
        code_words.insert("status", "статус");
        code_words.insert("state", "стейт");
        code_words.insert("error", "эррор");
        code_words.insert("message", "мессадж");
        code_words.insert("result", "резалт");
        code_words.insert("response", "респонс");
        code_words.insert("request", "реквест");
        code_words.insert("event", "ивент");
        code_words.insert("action", "экшн");
        code_words.insert("handler", "хендлер");
        code_words.insert("callback", "коллбэк");
        code_words.insert("promise", "промис");
        code_words.insert("function", "функшн");
        code_words.insert("method", "метод");
        code_words.insert("class", "класс");
        code_words.insert("instance", "инстанс");
        code_words.insert("module", "модуль");
        code_words.insert("component", "компонент");
        code_words.insert("element", "элемент");
        code_words.insert("node", "ноуд");
        code_words.insert("child", "чайлд");
        code_words.insert("parent", "парент");
        code_words.insert("root", "рут");
        code_words.insert("path", "пас");
        code_words.insert("url", "ю ар эл");
        code_words.insert("file", "файл");
        code_words.insert("folder", "фолдер");
        code_words.insert("directory", "директори");
        code_words.insert("config", "конфиг");
        code_words.insert("settings", "сеттингс");
        code_words.insert("options", "опшнс");
        code_words.insert("params", "парамс");
        code_words.insert("args", "аргс");
        code_words.insert("props", "пропс");
        code_words.insert("attr", "аттр");
        code_words.insert("attribute", "атрибьют");
        code_words.insert("context", "контекст");
        code_words.insert("session", "сешн");
        code_words.insert("token", "токен");
        code_words.insert("cache", "кэш");
        code_words.insert("store", "стор");
        code_words.insert("service", "сервис");
        code_words.insert("client", "клиент");
        code_words.insert("server", "сервер");
        code_words.insert("database", "датабейз");
        code_words.insert("connection", "коннекшн");
        code_words.insert("query", "квери");
        code_words.insert("table", "тейбл");
        code_words.insert("column", "колумн");
        code_words.insert("row", "роу");
        code_words.insert("record", "рекорд");
        code_words.insert("field", "филд");
        code_words.insert("form", "форм");
        code_words.insert("input", "инпут");
        code_words.insert("output", "аутпут");
        code_words.insert("button", "баттон");
        code_words.insert("link", "линк");
        code_words.insert("image", "имадж");
        code_words.insert("text", "текст");
        code_words.insert("content", "контент");
        code_words.insert("body", "боди");
        code_words.insert("header", "хедер");
        code_words.insert("footer", "футер");
        code_words.insert("nav", "нав");
        code_words.insert("menu", "меню");
        code_words.insert("sidebar", "сайдбар");
        code_words.insert("modal", "модал");
        code_words.insert("popup", "попап");
        code_words.insert("tooltip", "тултип");
        code_words.insert("loader", "лоудер");
        code_words.insert("spinner", "спиннер");
        code_words.insert("icon", "айкон");
        code_words.insert("logo", "лого");
        code_words.insert("avatar", "аватар");
        code_words.insert("badge", "бэдж");
        code_words.insert("tag", "тэг");
        code_words.insert("label", "лейбл");
        code_words.insert("title", "тайтл");
        code_words.insert("description", "дескрипшн");
        code_words.insert("info", "инфо");
        code_words.insert("details", "детейлс");
        code_words.insert("summary", "саммари");
        code_words.insert("total", "тотал");
        code_words.insert("price", "прайс");
        code_words.insert("amount", "эмаунт");
        code_words.insert("balance", "бэлэнс");
        code_words.insert("date", "дейт");
        code_words.insert("time", "тайм");
        code_words.insert("timestamp", "таймстэмп");
        code_words.insert("version", "вёршн");
        code_words.insert("hash", "хэш");
        code_words.insert("string", "стринг");
        code_words.insert("number", "намбер");
        code_words.insert("boolean", "булеан");
        code_words.insert("null", "налл");
        code_words.insert("undefined", "андефайнд");
        code_words.insert("true", "тру");
        code_words.insert("false", "фолс");
        code_words.insert("const", "конст");
        code_words.insert("var", "вар");
        code_words.insert("let", "лет");
        code_words.insert("def", "деф");
        code_words.insert("print", "принт");
        code_words.insert("return", "ретёрн");
        code_words.insert("import", "импорт");
        code_words.insert("export", "экспорт");
        code_words.insert("from", "фром");
        code_words.insert("async", "эсинк");
        code_words.insert("await", "эвейт");
        code_words.insert("try", "трай");
        code_words.insert("catch", "кэтч");
        code_words.insert("throw", "сроу");
        code_words.insert("new", "нью");
        code_words.insert("this", "зис");
        code_words.insert("self", "селф");
        code_words.insert("super", "супер");
        code_words.insert("extends", "экстендс");
        code_words.insert("implements", "имплементс");
        code_words.insert("interface", "интерфейс");
        code_words.insert("abstract", "абстракт");
        code_words.insert("static", "статик");
        code_words.insert("public", "паблик");
        code_words.insert("private", "прайвит");
        code_words.insert("protected", "протектед");
        code_words.insert("final", "файнал");
        code_words.insert("override", "оверрайд");
        code_words.insert("virtual", "виртуал");
        // Common adjectives
        code_words.insert("valid", "вэлид");
        code_words.insert("invalid", "инвэлид");
        code_words.insert("active", "эктив");
        code_words.insert("inactive", "инэктив");
        code_words.insert("enabled", "энейблд");
        code_words.insert("disabled", "дизейблд");
        code_words.insert("visible", "визибл");
        code_words.insert("hidden", "хидден");
        code_words.insert("selected", "селектед");
        code_words.insert("focused", "фокусд");
        code_words.insert("loading", "лоудинг");
        code_words.insert("loaded", "лоудед");
        code_words.insert("pending", "пендинг");
        code_words.insert("success", "саксесс");
        code_words.insert("failed", "фейлд");
        code_words.insert("empty", "эмпти");
        code_words.insert("full", "фулл");
        code_words.insert("old", "олд");
        code_words.insert("first", "фёрст");
        code_words.insert("last", "ласт");
        code_words.insert("next", "некст");
        code_words.insert("prev", "прев");
        code_words.insert("previous", "привиас");
        code_words.insert("current", "каррент");
        code_words.insert("default", "дефолт");
        code_words.insert("custom", "кастом");
        code_words.insert("primary", "праймари");
        code_words.insert("secondary", "секондари");
        code_words.insert("main", "мейн");
        code_words.insert("base", "бейз");
        code_words.insert("max", "макс");
        code_words.insert("min", "мин");
        code_words.insert("all", "олл");
        code_words.insert("none", "нан");
        code_words.insert("any", "эни");
        code_words.insert("some", "сам");
        code_words.insert("my", "май");
        code_words.insert("your", "юр");
        code_words.insert("our", "ауэр");
        code_words.insert("to", "ту");
        code_words.insert("by", "бай");
        code_words.insert("with", "виз");
        code_words.insert("for", "фор");
        code_words.insert("of", "оф");
        code_words.insert("in", "ин");
        code_words.insert("out", "аут");
        code_words.insert("up", "ап");
        code_words.insert("down", "даун");
        code_words.insert("no", "ноу");
        code_words.insert("not", "нот");
        code_words.insert("or", "ор");
        code_words.insert("and", "энд");
        code_words.insert("if", "иф");
        code_words.insert("else", "элс");
        code_words.insert("then", "зен");
        code_words.insert("when", "вен");
        code_words.insert("where", "вер");
        code_words.insert("while", "вайл");
        code_words.insert("do", "ду");
        code_words.insert("case", "кейс");
        code_words.insert("switch", "свитч");
        code_words.insert("break", "брейк");
        code_words.insert("continue", "континью");
        // Common patterns
        code_words.insert("authenticated", "аутентикейтед");
        code_words.insert("timeout", "таймаут");
        code_words.insert("repository", "репозитори");
        code_words.insert("controller", "контроллер");
        code_words.insert("manager", "менеджер");
        code_words.insert("factory", "фэктори");
        code_words.insert("builder", "билдер");
        code_words.insert("adapter", "адаптер");
        code_words.insert("wrapper", "врэппер");
        code_words.insert("helper", "хелпер");
        code_words.insert("util", "утил");
        code_words.insert("utils", "утилз");
        code_words.insert("common", "коммон");
        code_words.insert("shared", "шэрд");
        code_words.insert("global", "глобал");
        code_words.insert("local", "локал");
        code_words.insert("links", "линкс");
        code_words.insert("dir", "дир");
        code_words.insert("awesome", "авесом");
        code_words.insert("package", "пакет");
        code_words.insert("dom", "дом");
        code_words.insert("router", "роутер");
        code_words.insert("react", "риакт");
        code_words.insert("vue", "вью");
        code_words.insert("variable", "вэриабл");
        code_words.insert("side", "сайд");
        code_words.insert("dry", "драй");
        code_words.insert("pip", "пип");
        code_words.insert("install", "инсталл");
        // Python specific
        code_words.insert("str", "стр");
        code_words.insert("repr", "репр");
        code_words.insert("len", "лен");
        code_words.insert("dict", "дикт");
        code_words.insert("int", "инт");
        code_words.insert("float", "флоат");
        code_words.insert("bool", "бул");
        // Abbreviations with special pronunciation
        code_words.insert("api", "эй пи ай");
        code_words.insert("html", "эйч ти эм эл");
        code_words.insert("http", "эйч ти ти пи");
        code_words.insert("sql", "эс кью эл");
        code_words.insert("utf", "ю ти эф");
        code_words.insert("sha", "ша");
        code_words.insert("json", "джейсон");
        // Common words
        code_words.insert("hello", "хелло");
        code_words.insert("world", "ворлд");
        code_words.insert("plus", "плас");
        code_words.insert("foo", "фу");
        code_words.insert("bar", "бар");
        code_words.insert("baz", "баз");
        code_words.insert("test", "тест");
        code_words.insert("example", "экзампл");
        code_words.insert("demo", "демо");
        code_words.insert("sample", "сэмпл");
        code_words.insert("x", "икс");
        code_words.insert("y", "игрек");
        code_words.insert("z", "зет");
        code_words.insert("a", "эй");
        code_words.insert("b", "би");
        code_words.insert("i", "ай");
        code_words.insert("j", "джей");
        code_words.insert("k", "кей");
        code_words.insert("n", "эн");
        code_words.insert("m", "эм");

        let mut translit_map: HashMap<char, &'static str> = HashMap::new();
        translit_map.insert('a', "а");
        translit_map.insert('b', "б");
        translit_map.insert('c', "к");
        translit_map.insert('d', "д");
        translit_map.insert('e', "е");
        translit_map.insert('f', "ф");
        translit_map.insert('g', "г");
        translit_map.insert('h', "х");
        translit_map.insert('i', "и");
        translit_map.insert('j', "дж");
        translit_map.insert('k', "к");
        translit_map.insert('l', "л");
        translit_map.insert('m', "м");
        translit_map.insert('n', "н");
        translit_map.insert('o', "о");
        translit_map.insert('p', "п");
        translit_map.insert('q', "к");
        translit_map.insert('r', "р");
        translit_map.insert('s', "с");
        translit_map.insert('t', "т");
        translit_map.insert('u', "у");
        translit_map.insert('v', "в");
        translit_map.insert('w', "в");
        translit_map.insert('x', "кс");
        translit_map.insert('y', "й");
        translit_map.insert('z', "з");

        Self {
            code_words,
            translit_map,
        }
    }

    /// Convert camelCase or PascalCase identifier to speakable Russian text.
    pub fn normalize_camel_case(&self, identifier: &str) -> String {
        if identifier.is_empty() {
            return identifier.to_string();
        }
        let parts = self.split_camel_case(identifier);
        self.transliterate_parts(&parts)
    }

    /// Convert snake_case identifier to speakable Russian text.
    /// Handles dunder methods (__init__) and private prefixes (_name).
    pub fn normalize_snake_case(&self, identifier: &str) -> String {
        if identifier.is_empty() {
            return identifier.to_string();
        }
        let stripped = identifier.trim_matches('_');
        if stripped.is_empty() {
            return identifier.to_string();
        }
        let parts: Vec<&str> = stripped.split('_').filter(|p| !p.is_empty()).collect();
        self.transliterate_parts(&parts)
    }

    /// Convert kebab-case identifier to speakable Russian text.
    pub fn normalize_kebab_case(&self, identifier: &str) -> String {
        if identifier.is_empty() {
            return identifier.to_string();
        }
        let parts: Vec<&str> = identifier.split('-').filter(|p| !p.is_empty()).collect();
        self.transliterate_parts(&parts)
    }

    fn split_camel_case<'a>(&self, identifier: &'a str) -> Vec<&'a str> {
        // Port of Python regex:
        // r"[A-Z]?[a-z]+|[A-Z]+(?=[A-Z][a-z]|\d|$)|[A-Z]+(?![a-z])|\d+"
        //
        // Strategy: scan the string and emit slices.
        let bytes = identifier.as_bytes();
        let len = bytes.len();
        let mut parts: Vec<&str> = Vec::new();
        let mut i = 0usize;

        while i < len {
            let start = i;
            let ch = bytes[i] as char;

            if ch.is_ascii_digit() {
                // Consume digit run
                while i < len && (bytes[i] as char).is_ascii_digit() {
                    i += 1;
                }
                parts.push(&identifier[start..i]);
            } else if ch.is_ascii_uppercase() {
                // Peek ahead to decide how many uppercase chars to consume
                // Count consecutive uppercase chars
                let up_start = i;
                while i < len && (bytes[i] as char).is_ascii_uppercase() {
                    i += 1;
                }
                let up_end = i;
                let up_count = up_end - up_start;

                if up_count == 1 {
                    // Single uppercase: might be start of TitleCase word
                    // Consume following lowercase chars
                    while i < len && (bytes[i] as char).is_ascii_lowercase() {
                        i += 1;
                    }
                    parts.push(&identifier[start..i]);
                } else {
                    // Multiple uppercase chars: e.g. "HTML", "API"
                    // If followed by lowercase, last uppercase belongs to next word
                    if i < len && (bytes[i] as char).is_ascii_lowercase() {
                        // e.g. "HTMLParser" → "HTML" + "Parser"
                        // backtrack one char — that uppercase starts the next word
                        let acronym_end = up_end - 1;
                        if acronym_end > up_start {
                            parts.push(&identifier[up_start..acronym_end]);
                        }
                        // Now consume the remaining uppercase + lowercase
                        i = acronym_end;
                        let word_start = i;
                        if i < len && (bytes[i] as char).is_ascii_uppercase() {
                            i += 1;
                        }
                        while i < len && (bytes[i] as char).is_ascii_lowercase() {
                            i += 1;
                        }
                        parts.push(&identifier[word_start..i]);
                    } else {
                        // Pure acronym at end or before digit/boundary: emit whole run
                        parts.push(&identifier[up_start..up_end]);
                    }
                }
            } else if ch.is_ascii_lowercase() {
                // Consume lowercase run (handles initial lowercase word)
                while i < len && (bytes[i] as char).is_ascii_lowercase() {
                    i += 1;
                }
                parts.push(&identifier[start..i]);
            } else {
                i += 1; // skip non-alphanumeric
            }
        }

        parts.into_iter().filter(|p| !p.is_empty()).collect()
    }

    fn transliterate_parts(&self, parts: &[&str]) -> String {
        let result: Vec<String> = parts
            .iter()
            .map(|part| {
                let part_lower = part.to_lowercase();

                if part.chars().all(|c| c.is_ascii_digit()) {
                    // Numeric part
                    let n: u64 = part.parse().unwrap_or(0);
                    number_to_russian(n)
                } else if let Some(&translation) = self.code_words.get(part_lower.as_str()) {
                    translation.to_string()
                } else if part.len() >= 2 && part.chars().all(|c| c.is_ascii_uppercase()) {
                    // All-caps abbreviation not in CODE_WORDS: spell letter by letter
                    self.spell_abbreviation(part)
                } else {
                    self.basic_transliterate(&part_lower)
                }
            })
            .collect();

        result.join(" ")
    }

    /// Spell abbreviation letter-by-letter using English letter names.
    fn spell_abbreviation(&self, abbrev: &str) -> String {
        let letter_names: &[(&str, &str)] = &[
            ("A", "эй"),
            ("B", "би"),
            ("C", "си"),
            ("D", "ди"),
            ("E", "и"),
            ("F", "эф"),
            ("G", "джи"),
            ("H", "эйч"),
            ("I", "ай"),
            ("J", "джей"),
            ("K", "кей"),
            ("L", "эл"),
            ("M", "эм"),
            ("N", "эн"),
            ("O", "о"),
            ("P", "пи"),
            ("Q", "кью"),
            ("R", "ар"),
            ("S", "эс"),
            ("T", "ти"),
            ("U", "ю"),
            ("V", "ви"),
            ("W", "дабл ю"),
            ("X", "икс"),
            ("Y", "вай"),
            ("Z", "зет"),
        ];

        let map: HashMap<&str, &str> = letter_names.iter().copied().collect();

        abbrev
            .chars()
            .map(|c| {
                let s: String = c.to_uppercase().collect();
                map.get(s.as_str()).copied().unwrap_or("?")
            })
            .collect::<Vec<_>>()
            .join(" ")
    }

    fn basic_transliterate(&self, word: &str) -> String {
        // Accumulate into a String to avoid Box::leak memory leak for unmapped chars.
        let mut result = String::new();
        for c in word.chars() {
            match self.translit_map.get(&c) {
                Some(s) => result.push_str(s),
                None => result.push(c),
            }
        }
        result
    }
}

impl Default for CodeIdentifierNormalizer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn normalizer() -> CodeIdentifierNormalizer {
        CodeIdentifierNormalizer::new()
    }

    // --- CamelCase ---

    #[test]
    fn camel_get_user_data() {
        assert_eq!(
            normalizer().normalize_camel_case("getUserData"),
            "гет юзер дата"
        );
    }

    #[test]
    fn camel_my_variable() {
        assert_eq!(
            normalizer().normalize_camel_case("myVariable"),
            "май вэриабл"
        );
    }

    #[test]
    fn camel_is_valid() {
        assert_eq!(normalizer().normalize_camel_case("isValid"), "из вэлид");
    }

    #[test]
    fn camel_has_value() {
        assert_eq!(normalizer().normalize_camel_case("hasValue"), "хэз вэлью");
    }

    #[test]
    fn camel_on_click() {
        assert_eq!(normalizer().normalize_camel_case("onClick"), "он клик");
    }

    #[test]
    fn camel_on_change() {
        assert_eq!(normalizer().normalize_camel_case("onChange"), "он чейндж");
    }

    #[test]
    fn camel_handle_submit() {
        assert_eq!(
            normalizer().normalize_camel_case("handleSubmit"),
            "хендл сабмит"
        );
    }

    #[test]
    fn camel_fetch_data() {
        assert_eq!(normalizer().normalize_camel_case("fetchData"), "фетч дата");
    }

    #[test]
    fn camel_parse_json() {
        assert_eq!(
            normalizer().normalize_camel_case("parseJSON"),
            "парс джейсон"
        );
    }

    #[test]
    fn camel_to_string() {
        assert_eq!(normalizer().normalize_camel_case("toString"), "ту стринг");
    }

    #[test]
    fn camel_get_user_data_from_server() {
        assert_eq!(
            normalizer().normalize_camel_case("getUserDataFromServer"),
            "гет юзер дата фром сервер"
        );
    }

    #[test]
    fn camel_calculate_total_price() {
        assert_eq!(
            normalizer().normalize_camel_case("calculateTotalPrice"),
            "калькулейт тотал прайс"
        );
    }

    #[test]
    fn camel_is_user_authenticated() {
        assert_eq!(
            normalizer().normalize_camel_case("isUserAuthenticated"),
            "из юзер аутентикейтед"
        );
    }

    #[test]
    fn camel_parse_html_content() {
        assert_eq!(
            normalizer().normalize_camel_case("parseHTMLContent"),
            "парс эйч ти эм эл контент"
        );
    }

    #[test]
    fn camel_get_api_response() {
        assert_eq!(
            normalizer().normalize_camel_case("getAPIResponse"),
            "гет эй пи ай респонс"
        );
    }

    #[test]
    fn camel_load_json_data() {
        assert_eq!(
            normalizer().normalize_camel_case("loadJSONData"),
            "лоуд джейсон дата"
        );
    }

    #[test]
    fn camel_create_url_path() {
        assert_eq!(
            normalizer().normalize_camel_case("createURLPath"),
            "криейт ю ар эл пас"
        );
    }

    #[test]
    fn camel_get_user2_data() {
        assert_eq!(
            normalizer().normalize_camel_case("getUser2Data"),
            "гет юзер два дата"
        );
    }

    #[test]
    fn camel_item1_name() {
        assert_eq!(
            normalizer().normalize_camel_case("item1Name"),
            "айтем один нейм"
        );
    }

    // --- PascalCase ---

    #[test]
    fn pascal_user_service() {
        assert_eq!(
            normalizer().normalize_camel_case("UserService"),
            "юзер сервис"
        );
    }

    #[test]
    fn pascal_data_repository() {
        assert_eq!(
            normalizer().normalize_camel_case("DataRepository"),
            "дата репозитори"
        );
    }

    #[test]
    fn pascal_http_client() {
        assert_eq!(
            normalizer().normalize_camel_case("HttpClient"),
            "эйч ти ти пи клиент"
        );
    }

    #[test]
    fn pascal_api_controller() {
        assert_eq!(
            normalizer().normalize_camel_case("ApiController"),
            "эй пи ай контроллер"
        );
    }

    #[test]
    fn pascal_database_connection() {
        assert_eq!(
            normalizer().normalize_camel_case("DatabaseConnection"),
            "датабейз коннекшн"
        );
    }

    #[test]
    fn pascal_event_handler() {
        assert_eq!(
            normalizer().normalize_camel_case("EventHandler"),
            "ивент хендлер"
        );
    }

    #[test]
    fn pascal_file_manager() {
        assert_eq!(
            normalizer().normalize_camel_case("FileManager"),
            "файл менеджер"
        );
    }

    #[test]
    fn pascal_config_loader() {
        assert_eq!(
            normalizer().normalize_camel_case("ConfigLoader"),
            "конфиг лоудер"
        );
    }

    // --- SnakeCase ---

    #[test]
    fn snake_get_user_data() {
        assert_eq!(
            normalizer().normalize_snake_case("get_user_data"),
            "гет юзер дата"
        );
    }

    #[test]
    fn snake_my_variable() {
        assert_eq!(
            normalizer().normalize_snake_case("my_variable"),
            "май вэриабл"
        );
    }

    #[test]
    fn snake_is_valid() {
        assert_eq!(normalizer().normalize_snake_case("is_valid"), "из вэлид");
    }

    #[test]
    fn snake_has_value() {
        assert_eq!(normalizer().normalize_snake_case("has_value"), "хэз вэлью");
    }

    #[test]
    fn snake_on_click() {
        assert_eq!(normalizer().normalize_snake_case("on_click"), "он клик");
    }

    #[test]
    fn snake_handle_submit() {
        assert_eq!(
            normalizer().normalize_snake_case("handle_submit"),
            "хендл сабмит"
        );
    }

    #[test]
    fn snake_fetch_data() {
        assert_eq!(normalizer().normalize_snake_case("fetch_data"), "фетч дата");
    }

    #[test]
    fn snake_parse_json() {
        assert_eq!(
            normalizer().normalize_snake_case("parse_json"),
            "парс джейсон"
        );
    }

    #[test]
    fn snake_get_user_data_from_server() {
        assert_eq!(
            normalizer().normalize_snake_case("get_user_data_from_server"),
            "гет юзер дата фром сервер"
        );
    }

    #[test]
    fn snake_calculate_total_price() {
        assert_eq!(
            normalizer().normalize_snake_case("calculate_total_price"),
            "калькулейт тотал прайс"
        );
    }

    #[test]
    fn snake_user_2_data() {
        assert_eq!(
            normalizer().normalize_snake_case("user_2_data"),
            "юзер два дата"
        );
    }

    #[test]
    fn snake_item_1_name() {
        assert_eq!(
            normalizer().normalize_snake_case("item_1_name"),
            "айтем один нейм"
        );
    }

    #[test]
    fn snake_dunder_init() {
        assert_eq!(normalizer().normalize_snake_case("__init__"), "инит");
    }

    #[test]
    fn snake_dunder_str() {
        assert_eq!(normalizer().normalize_snake_case("__str__"), "стр");
    }

    #[test]
    fn snake_dunder_repr() {
        assert_eq!(normalizer().normalize_snake_case("__repr__"), "репр");
    }

    #[test]
    fn snake_dunder_len() {
        assert_eq!(normalizer().normalize_snake_case("__len__"), "лен");
    }

    #[test]
    fn snake_private_method() {
        assert_eq!(
            normalizer().normalize_snake_case("_private_method"),
            "прайвит метод"
        );
    }

    #[test]
    fn snake_private_attr() {
        assert_eq!(
            normalizer().normalize_snake_case("__private_attr"),
            "прайвит аттр"
        );
    }

    // --- KebabCase ---

    #[test]
    fn kebab_my_component() {
        assert_eq!(
            normalizer().normalize_kebab_case("my-component"),
            "май компонент"
        );
    }

    #[test]
    fn kebab_button_primary() {
        assert_eq!(
            normalizer().normalize_kebab_case("button-primary"),
            "баттон праймари"
        );
    }

    #[test]
    fn kebab_nav_bar() {
        assert_eq!(normalizer().normalize_kebab_case("nav-bar"), "нав бар");
    }

    #[test]
    fn kebab_side_menu() {
        assert_eq!(normalizer().normalize_kebab_case("side-menu"), "сайд меню");
    }

    #[test]
    fn kebab_header_logo() {
        assert_eq!(
            normalizer().normalize_kebab_case("header-logo"),
            "хедер лого"
        );
    }

    #[test]
    fn kebab_footer_links() {
        assert_eq!(
            normalizer().normalize_kebab_case("footer-links"),
            "футер линкс"
        );
    }

    #[test]
    fn kebab_output_dir() {
        assert_eq!(
            normalizer().normalize_kebab_case("output-dir"),
            "аутпут дир"
        );
    }

    #[test]
    fn kebab_config_file() {
        assert_eq!(
            normalizer().normalize_kebab_case("config-file"),
            "конфиг файл"
        );
    }

    #[test]
    fn kebab_no_cache() {
        assert_eq!(normalizer().normalize_kebab_case("no-cache"), "ноу кэш");
    }

    #[test]
    fn kebab_dry_run() {
        assert_eq!(normalizer().normalize_kebab_case("dry-run"), "драй ран");
    }

    #[test]
    fn kebab_my_awesome_package() {
        assert_eq!(
            normalizer().normalize_kebab_case("my-awesome-package"),
            "май авесом пакет"
        );
    }

    #[test]
    fn kebab_react_dom() {
        assert_eq!(normalizer().normalize_kebab_case("react-dom"), "риакт дом");
    }

    #[test]
    fn kebab_vue_router() {
        assert_eq!(
            normalizer().normalize_kebab_case("vue-router"),
            "вью роутер"
        );
    }

    // --- SCREAMING_SNAKE_CASE (MixedIdentifiers) ---

    #[test]
    fn screaming_max_value() {
        assert_eq!(normalizer().normalize_snake_case("MAX_VALUE"), "макс вэлью");
    }

    #[test]
    fn screaming_default_timeout() {
        assert_eq!(
            normalizer().normalize_snake_case("DEFAULT_TIMEOUT"),
            "дефолт таймаут"
        );
    }

    #[test]
    fn screaming_api_base_url() {
        assert_eq!(
            normalizer().normalize_snake_case("API_BASE_URL"),
            "эй пи ай бейз ю ар эл"
        );
    }
}
