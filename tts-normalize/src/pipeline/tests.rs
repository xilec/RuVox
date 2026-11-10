use super::*;
use crate::dictionary::DictionaryEntry;

// ── Unit-level sanity tests ────────────────────────────────────────────────

#[test]
fn pipeline_empty_input() {
    let mut p = TTSPipeline::new();
    assert_eq!(p.process_text(""), "");
}

#[test]
fn pipeline_plain_russian() {
    let mut p = TTSPipeline::new();
    let input = "Привет мир";
    assert_eq!(p.process_text(input), "Привет мир");
}

#[test]
fn pipeline_invalid_time_falls_through_to_numbers() {
    // Invariant: the time parser rejects an out-of-range clock value, leaving
    // the digits for the number reader, so "25:00" is not narrated as a clock.
    let mut p = TTSPipeline::new();
    assert_eq!(
        p.process_text("В 25:00 встреча"),
        "В двадцать пять:ноль встреча"
    );
}

#[test]
fn pipeline_number_inline() {
    let mut p = TTSPipeline::new();
    assert_eq!(p.process_text("Версия 3"), "Версия три");
}

#[test]
fn pipeline_mermaid_marker() {
    let mut p = TTSPipeline::new();
    let input = "```mermaid\ngraph TD\nA-->B\n```";
    assert_eq!(p.process_text(input), "Тут мермэйд диаграмма");
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
    assert_eq!(p.process_text(input), "далее следует пример кода на пайтон");
    p.set_code_block_mode(CodeBlockMode::Full);
    assert_eq!(
        p.process_text(input),
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
    let direct = p.process_text(input);
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
    assert_eq!(p.process_text("Встреча в 5."), "Встреча в пять.");
    assert_eq!(
        p.process_text("Сначала пункт 3. Потом пункт 4."),
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
        p.process_text("Остаток 3.14 в конце"),
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
    assert_eq!(p.process_text("Вес .5 кг"), "Вес ноль точка пять кг");
    assert_eq!(
        p.process_text(".75 вероятность"),
        "ноль точка семь пять вероятность"
    );
}

#[test]
fn pipeline_leading_dot_decimal_after_letter_is_untouched() {
    // A dot preceded by a letter belongs to a dotted label ("example.5"),
    // not a decimal — the leading-dot phase must skip it, and the number
    // phase guard keeps the digit unexpanded as before.
    let mut p = TTSPipeline::new();
    assert_eq!(p.process_text("файл example.5"), "файл экзампл.5");
}

#[test]
fn pipeline_float_keeps_version_reading() {
    // "1.5" is consumed by the version phase before the leading-dot
    // phase runs — no double-consume, no "ноль" prefix.
    let mut p = TTSPipeline::new();
    assert_eq!(p.process_text("Точность 1.5"), "Точность один точка пять");
}

#[test]
fn pipeline_percentage_range_is_read_as_a_unit() {
    // Regression (#112): after the percentages-before-ranges phase swap
    // the trailing "20%" was consumed alone, leaving "10-" with a bare
    // dash. The percentage-range phase must claim "N-M%" as one region.
    let mut p = TTSPipeline::new();
    assert_eq!(
        p.process_text("Рост на 10-20% за квартал"),
        "Рост на от десяти до двадцати процентов за квартал"
    );
    // Whitespace around the dash and before "%" is tolerated.
    assert_eq!(
        p.process_text("10 - 20 %"),
        "от десяти до двадцати процентов"
    );
}

#[test]
fn pipeline_percentage_range_glued_to_letter_is_not_a_range() {
    // A digit glued to a letter is not a range bound: "\b" rejects the
    // percentage-range match, so no "от … до …" reading. The letter-digit
    // token is narrated as a whole and the plain percentage phase still
    // claims "20%" on its own (the "-" provides its boundary).
    let mut p = TTSPipeline::new();
    assert_eq!(
        p.process_text("метрика x10-20%"),
        "метрика икс десять-двадцать процентов"
    );
}

#[test]
fn pipeline_plain_range_and_percentage_unchanged() {
    // The new phase must not steal plain ranges or plain percentages.
    let mut p = TTSPipeline::new();
    assert_eq!(
        p.process_text("диапазон 10-20"),
        "диапазон от десяти до двадцати"
    );
    assert_eq!(
        p.process_text("загрузка 20%"),
        "загрузка двадцать процентов"
    );
}

#[test]
fn pipeline_number_adjacent_to_letter_is_narrated() {
    // A letter+digit token is narrated by the English phase: the letter run
    // and the digit run each get their existing reading (#313).
    let mut p = TTSPipeline::new();
    assert_eq!(
        p.process_text("Запусти v1 сейчас"),
        "Запусти ви один сейчас"
    );
    assert_eq!(
        p.process_text("Координата x2 равна"),
        "Координата икс два равна"
    );
    assert_eq!(
        p.process_text("Файл app2 лежит тут"),
        "Файл апп два лежит тут"
    );
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
    assert_eq!(p.process_text("user use"), "юзер юз");
    assert_eq!(p.process_text("use user"), "юз юзер");
    assert_eq!(p.process_text("tests test"), "тестс тест");
    assert_eq!(p.process_text("test tests"), "тест тестс");
}

#[test]
fn pipeline_phrase_words_read_from_the_dictionary() {
    // Regression (#287): a phrase used to be transliterated word by word
    // with the wrong readings ("стак трейс"); its words are curated now, so
    // the reading does not depend on how the words are separated.
    let mut p = TTSPipeline::new();
    assert_eq!(p.process_text("stack trace"), "стэк трейс");
    assert_eq!(p.process_text("stack\ntrace"), "стэк\nтрейс");
    assert_eq!(p.process_text("stack  trace"), "стэк трейс");
    assert_eq!(p.process_text("stack\ttrace"), "стэк трейс");
    assert_eq!(
        p.process_text("Перед merge request нужен code review."),
        "Перед мёрдж реквест нужен код ревью."
    );
    assert_eq!(
        p.process_text("hot fix и live reload, dry run и edge case"),
        "хот фикс и лайв релоуд, драй ран и эдж кейс"
    );
}

#[test]
fn pipeline_phrase_words_respect_user_dictionary() {
    // The user dictionary pre-pass runs before English resolution, so an
    // overridden word wins over its curated reading.
    let mut p = TTSPipeline::new();
    p.user_dictionary.insert(DictionaryEntry {
        from: "stack".to_string(),
        to: "стек".to_string(),
        forms: Vec::new(),
    });
    assert_eq!(p.process_text("stack trace"), "стек трейс");
}

#[test]
fn pipeline_number_embedded_in_earlier_number() {
    // Regression (#75): same literal-replace bug in the numbers phase —
    // "1" replaced inside "10", "42" inside "142".
    let mut p = TTSPipeline::new();
    assert_eq!(p.process_text("10:1"), "десять:один");
    assert_eq!(
        p.process_text("Счёт 10:1 в нашу пользу."),
        "Счёт десять:один в нашу пользу."
    );
    // A space-separated three-digit group is a thousands separator, so
    // "42 142" is one number; the regression this test guards (replacing
    // "42" inside "142") still shows in the ungrouped "142 42" above.
    assert_eq!(p.process_text("142 42"), "сто сорок два сорок два");
    assert_eq!(p.process_text("42 142"), "сорок две тысячи сто сорок два");
}

#[test]
fn pipeline_single_latin_letters_read_by_name() {
    // Lone letters used to stay Latin (re_english_words required 2+
    // letters) and were silently dropped by Silero.
    let mut p = TTSPipeline::new();
    assert_eq!(
        p.process_text("Переменная x равна 5"),
        "Переменная икс равна пять"
    );
    assert_eq!(p.process_text("пункты a и I"), "пункты эй и ай");
}

#[test]
fn pipeline_single_letters_not_tracked_as_unknown() {
    // Letter-name spelling is a dictionary lookup, not a transliteration
    // fallback — it must not enter the unknown-words map.
    let mut p = TTSPipeline::new();
    p.process_text("Переменная x равна 5");
    assert!(p.english_normalizer.get_unknown_words().is_empty());
}

#[test]
fn pipeline_word_like_all_caps_read_as_words() {
    // Long vowel-rich all-caps tokens are words, not initialisms: they are
    // transliterated instead of spelled letter by letter, and follow the
    // tracking rules of any transliterated word.
    let mut p = TTSPipeline::new();
    // "y" counts as a vowel, so "SYSTEM" is a word too. One call: the
    // unknown-words map is cleared at the start of every process call.
    assert_eq!(
        p.process_text("DEVELOPER LINUX UBUNTU JAVASCRIPT SYSTEM"),
        "девелопер линукс убунту джаваскрипт систем"
    );
    let unknown = p.english_normalizer.get_unknown_words();
    for word in ["developer", "linux", "ubuntu", "javascript", "system"] {
        assert!(unknown.contains_key(word), "{word} must be tracked");
    }
}

#[test]
fn every_noop_entry_keeps_its_all_caps_reading() {
    // The entries whose curated reading equals the raw transliteration only
    // carry information where transliteration is not consulted — above
    // all, they stop an all-caps token from being spelled out. Pin every
    // one of them, so deleting a guard cannot stay invisible to tests.
    use crate::pipeline::normalizers::english::{IT_TERMS, transliterate_simple};

    let mut p = TTSPipeline::new();
    let guards: Vec<(&str, &str)> = IT_TERMS
        .iter()
        .filter(|(word, reading)| transliterate_simple(word) == **reading)
        .map(|(word, reading)| (*word, *reading))
        .collect();
    assert!(guards.len() >= 28, "guard set shrank unexpectedly");
    for (word, reading) in guards {
        assert_eq!(
            p.process_text(&word.to_uppercase()),
            reading,
            "{word} lost its all-caps reading"
        );
    }
}

#[test]
fn pipeline_caps_guard_entries_keep_short_all_caps_words_readable() {
    // These words have no useful lowercase information (the transliteration
    // already matches the curated reading) but their entries keep an
    // uppercase token from falling through to letter-by-letter spelling;
    // #292 removed every other no-op, so this is the only reason they stay.
    let mut p = TTSPipeline::new();
    assert_eq!(
        p.process_text("TEST DATA APP CLASS WEB STRING"),
        "тест дата апп класс веб стринг"
    );
}

#[test]
fn pipeline_short_or_vowel_poor_all_caps_stay_spelled() {
    // The heuristic must not swallow the abbreviation class it exists to
    // distinguish words from.
    let mut p = TTSPipeline::new();
    assert_eq!(
        p.process_text("API HTML SQL"),
        "эй пи ай эйч ти эм эл эс кью эл"
    );
    // "SPARK" is absent from every table and one vowel in five letters is
    // below the threshold, so the length/vowel boundary itself is locked.
    assert_eq!(p.process_text("SPARK"), "эс пи эй ар кей");
    // "JSON" is short but also curated: the table wins before the rule.
    assert_eq!(p.process_text("JSON"), "джейсон");
}

#[test]
fn pipeline_curated_readings_outrank_word_like_rule() {
    // Word-like tokens that are curated keep their curated reading and are
    // not transliterated.
    let mut p = TTSPipeline::new();
    // One call, so the empty-map assertion below covers every token: if
    // the word-like rule ran before the curated tables, all three would be
    // transliterated and tracked.
    assert_eq!(
        p.process_text("KUBERNETES OAUTH WINDOWS MACOS BROWSER"),
        "кубернетис о аус виндовс мак оу эс браузэр"
    );
    assert!(p.english_normalizer.get_unknown_words().is_empty());
}

#[test]
fn pipeline_short_all_caps_guards_stay_words() {
    // These four-letter tokens are too short for the word-like all-caps
    // heuristic, so the curated entries are the only thing that keeps them
    // from being spelled letter by letter (#306 review).
    let mut p = TTSPipeline::new();
    assert_eq!(
        p.process_text("FILE PAGE TYPE BYTE SIZE LINE SITE MIME PIPE HOME LIVE SLACK"),
        "файл пейдж тайп байт сайз лайн сайт майм пайп хоум лайв слэк"
    );
}

#[test]
fn pipeline_all_caps_inflected_it_terms_composed() {
    // Regression (#288): the all-uppercase abbreviation branch used to run
    // before inflection composition, so an all-caps inflected form of a
    // known IT term was spelled letter by letter. Composition now runs
    // first and is case-independent, matching the mixed-case path.
    let mut p = TTSPipeline::new();
    assert_eq!(p.process_text("SUPPORTING"), "саппортинг");
    assert_eq!(p.process_text("BUILDING"), "билдинг");
    assert_eq!(p.process_text("PACKAGES"), "пэкэджес");
    // Composition is a curated lookup, not the transliteration fallback,
    // so the composed form must not enter the unknown-words map.
    assert!(p.english_normalizer.get_unknown_words().is_empty());
}

#[test]
fn pipeline_composed_forms_not_tracked_while_fallback_is() {
    // Spec: "Inflected IT-term form composed" — a composed form is read from
    // the curated stem and never lands in the unknown-words map. Pin the
    // whole chain (mixed case, several suffixes), and in the same test prove
    // the map still receives a genuine fallback reading: otherwise "map is
    // empty" would also pass if tracking had been disabled entirely.
    let mut p = TTSPipeline::new();
    assert_eq!(
        p.process_text("Supporting supports using caches libraries Compilation"),
        "саппортинг саппортс юзинг кэшес лайбрарис компайлашн"
    );
    assert!(p.english_normalizer.get_unknown_words().is_empty());

    p.process_text("xyzzy");
    assert!(
        p.english_normalizer
            .get_unknown_words()
            .contains_key("xyzzy"),
        "the transliteration fallback must still report unknown words"
    );
}

#[test]
fn pipeline_pure_abbreviations_keep_letter_readings() {
    // The composition helper fires only on a recognized suffix over a
    // curated stem: pure abbreviations still read letter by letter (or via
    // their `AS_WORD` entry) and are never recorded as unknown words.
    let mut p = TTSPipeline::new();
    assert_eq!(p.process_text("API"), "эй пи ай");
    assert_eq!(p.process_text("CPU"), "си пи ю");
    assert_eq!(p.process_text("JSON"), "джейсон");
    assert_eq!(p.process_text("HTML"), "эйч ти эм эл");
    assert_eq!(p.process_text("SQL"), "эс кью эл");
    assert_eq!(p.process_text("XYZ"), "икс вай зет");
    assert!(p.english_normalizer.get_unknown_words().is_empty());
}

// ── Scheme-less URLs ─────────────────────────────────────────────────────

#[test]
fn pipeline_schemeless_www_domain() {
    let mut p = TTSPipeline::new();
    assert_eq!(
        p.process_text("Сайт www.example.com недоступен"),
        "Сайт ввв точка экзампл точка ком недоступен"
    );
}

#[test]
fn pipeline_schemeless_bare_domain_with_path() {
    let mut p = TTSPipeline::new();
    assert_eq!(
        p.process_text("документация на docs.python.org/3/tutorial"),
        "документация на докс точка пайтон точка орг слэш три слэш тьюториал"
    );
}

#[test]
fn pipeline_schemeless_domain_keeps_sentence_punctuation() {
    // The sentence-ending dot is not part of the domain and must not be
    // read as "точка".
    let mut p = TTSPipeline::new();
    assert_eq!(
        p.process_text("Смотри example.com, там документация."),
        "Смотри экзампл точка ком, там документация."
    );
}

#[test]
fn pipeline_schemeless_skips_email_domain() {
    // The email pass consumes the whole address first; the domain inside
    // must not be re-matched as a bare domain.
    let mut p = TTSPipeline::new();
    assert_eq!(
        p.process_text("Пишите на user@example.com"),
        "Пишите на юзер собака экзампл точка ком"
    );
}

#[test]
fn pipeline_schemeless_false_positive_guards() {
    // Filenames (suffix not in TLD_MAP) and versions (numeric last label)
    // must not be detected as bare domains.
    let mut p = TTSPipeline::new();
    assert_eq!(
        p.process_text("открой file.txt и config.yaml"),
        "открой файл.тэкст и конфиг.ямл"
    );
    assert_eq!(
        p.process_text("версия 1.2.3"),
        "версия один точка два точка три"
    );
}

#[test]
fn pipeline_schemeless_skips_path_internal_segment() {
    // "site.dev" inside a file path is a directory, not a domain.
    let mut p = TTSPipeline::new();
    assert_eq!(
        p.process_text("Файл /home/site.dev/main.py лежит тут"),
        "Файл слэш хоум слэш сайт точка дев слэш мэйн точка пи лежит тут"
    );
    // Same guard for Windows-style separators: "app.dev" after a
    // backslash must not be read as a domain. (Backslash paths are not
    // matched by re_path today at all — pre-existing gap, out of scope.)
    assert_eq!(
        p.process_text(r"путь C:\projects\app.dev\main.py тут"),
        "путь си:\\проджектс\\апп.дев\\мэйн.пи тут"
    );
}

// ── Profile gating ────────────────────────────────────────────────────────

/// The strictest and the most permissive profiles the pipeline supports.
fn native_profile() -> &'static TextProfile {
    &crate::profiles::NATIVE_EN_PROFILE
}

#[test]
fn identifier_ranges_merge_the_patterns_claims() {
    // The kebab pattern claims the whole token while the camel pattern
    // claims the identifier nested inside it; a later phase must see one
    // range covering both, or its binary search misses the outer one.
    assert_eq!(
        find_identifier_ranges("my-app-getUserData-flag"),
        vec![(0, 23)]
    );
    assert_eq!(
        find_identifier_ranges("getUserData и my_snake_case"),
        vec![(0, 11), (15, 28)]
    );
}

#[test]
fn overlap_check_uses_half_open_ranges() {
    let ranges = vec![(0, 3), (7, 10)];
    assert!(overlaps_ranges(&ranges, 2, 8));
    assert!(overlaps_ranges(&ranges, 8, 9));
    assert!(!overlaps_ranges(&ranges, 3, 7));
    assert!(!overlaps_ranges(&ranges, 10, 12));
}

#[test]
fn native_profile_keeps_prose_latin() {
    let mut p = TTSPipeline::new();
    let out = p.process_for_engine("Kubernetes нужен кластер", native_profile());
    assert_eq!(out.text, "Kubernetes нужен кластер");
}

#[test]
fn native_profile_keeps_identifiers_and_abbreviations() {
    let mut p = TTSPipeline::new();
    let out = p.process_for_engine("Вызови getUserData() через API", native_profile());
    assert_eq!(out.text, "Вызови getUserData() через API");
}

#[test]
fn identifier_route_is_independent_of_prose_route() {
    let mut p = TTSPipeline::new();
    let mut profile = *native_profile();
    profile.route.identifier = Reading::Narrate;
    let out = p.process_for_engine("getUserData и Kubernetes", &profile);
    assert_eq!(out.text, "гет юзер дата и Kubernetes");
}

// ── English reading setting ──────────────────────────────────────────────

#[test]
fn english_reading_auto_follows_the_engine_capability() {
    let mut p = TTSPipeline::new();
    let native = p.process_for_engine(
        "Kubernetes нужен кластер",
        &crate::profiles::TERATTS_PROFILE,
    );
    assert_eq!(native.text, "Kubernetes нужен кластер");
    assert_eq!(native.spans.len(), 1, "spans: {:?}", native.spans);

    let narrated =
        p.process_for_engine("Kubernetes нужен кластер", &crate::profiles::SILERO_PROFILE);
    assert_eq!(narrated.text, "кубернетис нужен кластер");
    assert!(narrated.spans.is_empty(), "spans: {:?}", narrated.spans);
}

#[test]
fn english_reading_native_falls_back_when_the_engine_cannot_read_latin() {
    let mut p = TTSPipeline::new();
    p.set_english_reading(EnglishReading::Native);
    let out = p.process_for_engine("Kubernetes нужен кластер", &crate::profiles::SILERO_PROFILE);
    assert_eq!(out.text, "кубернетис нужен кластер");
    assert!(out.spans.is_empty(), "spans: {:?}", out.spans);
}

#[test]
fn english_reading_narrated_transliterates_on_a_capable_engine() {
    let mut p = TTSPipeline::new();
    p.set_english_reading(EnglishReading::Narrated);
    let out = p.process_for_engine(
        "Kubernetes нужен кластер",
        &crate::profiles::TERATTS_PROFILE,
    );
    assert_eq!(out.text, "кубернетис нужен кластер");
    assert!(out.spans.is_empty(), "spans: {:?}", out.spans);
    assert_eq!(
        out.mapping.char_map.len(),
        out.text.chars().count(),
        "highlighting keeps one mapping entry per output codepoint"
    );
}

#[test]
fn narrated_prose_keeps_native_identifiers() {
    let mut p = TTSPipeline::new();
    p.set_english_reading(EnglishReading::Narrated);
    let out = p.process_for_engine(
        "Kubernetes и getUserData()",
        &crate::profiles::TERATTS_PROFILE,
    );
    assert_eq!(out.text, "кубернетис и getUserData()");
    assert_eq!(out.spans.len(), 1, "spans: {:?}", out.spans);
    let span = &out.spans[0];
    assert_eq!(span.kind, SpanKind::CodeIdentifier);
    assert_eq!(span.lang, Some("en"));
    assert_eq!(span_text(&out.text, span), "getUserData");
    assert_eq!(out.mapping.char_map.len(), out.text.chars().count());
}

#[test]
fn narrated_prose_keeps_native_identifiers_after_a_special_term() {
    // "C++" is rewritten before the English matches are collected; the
    // protected identifier ranges must be taken after that rewrite or the
    // shifted offsets would let the phase transliterate the identifier.
    let mut p = TTSPipeline::new();
    p.set_english_reading(EnglishReading::Narrated);
    let out = p.process_for_engine("C++ и getUserData()", &crate::profiles::TERATTS_PROFILE);
    assert_eq!(out.text, "си плюс плюс и getUserData()");
    assert_eq!(out.spans.len(), 1, "spans: {:?}", out.spans);
    assert_eq!(span_text(&out.text, &out.spans[0]), "getUserData");
}

#[test]
fn narrated_prose_keeps_native_identifiers_nested_in_a_kebab_token() {
    // The whole token is one identifier range: without merging the kebab
    // and camel claims, `flag` would slip past the overlap check and be
    // transliterated while the rest stayed Latin.
    let mut p = TTSPipeline::new();
    p.set_english_reading(EnglishReading::Narrated);
    let out = p.process_for_engine("my-app-getUserData-flag", &crate::profiles::TERATTS_PROFILE);
    assert_eq!(out.text, "my-app-getUserData-flag");
    assert_eq!(out.spans.len(), 1, "spans: {:?}", out.spans);
    assert_eq!(
        span_text(&out.text, &out.spans[0]),
        "my-app-getUserData-flag"
    );
}

#[test]
fn native_setting_keeps_both_native_routes_on_a_capable_engine() {
    let mut p = TTSPipeline::new();
    p.set_english_reading(EnglishReading::Native);
    let out = p.process_for_engine(
        "Kubernetes и getUserData()",
        &crate::profiles::TERATTS_PROFILE,
    );
    assert_eq!(out.text, "Kubernetes и getUserData()");
    let kinds: Vec<_> = out.spans.iter().map(|span| span.kind).collect();
    assert_eq!(kinds, vec![SpanKind::ProseLatin, SpanKind::CodeIdentifier]);
}

#[test]
fn native_digits_leave_numbers_untouched() {
    let mut p = TTSPipeline::new();
    let out = p.process_for_engine("42 сервера и версия 1.2.3", native_profile());
    assert_eq!(out.text, "42 сервера и версия 1.2.3");
}

#[test]
fn native_units_expand_digits_but_keep_the_percentage() {
    let mut p = TTSPipeline::new();
    let mut profile = *native_profile();
    profile.digits = Digits::Expanded;
    let out = p.process_for_engine("рост 50% и 42 сервера", &profile);
    assert_eq!(out.text, "рост 50% и сорок два сервера");
}

#[test]
fn native_units_leave_every_unit_fragment_untouched() {
    // The unit phases own these fragments as a whole; with units gated off,
    // the digit phases must not consume the number and strand the unit.
    let mut p = TTSPipeline::new();
    let mut profile = *native_profile();
    profile.digits = Digits::Expanded;
    for input in ["12.5%", "10-20%", "50 %", "1.5 GB", "512 KB", "2 min"] {
        let out = p.process_for_engine(input, &profile);
        assert_eq!(out.text, input, "{input} must stay with its unit");
    }
    // Untouched fragments outside a unit still expand.
    let out = p.process_for_engine("12.5 и 7 штук", &profile);
    assert_eq!(out.text, "двенадцать точка пять и семь штук");
}

#[test]
fn markdown_list_numbers_follow_the_digits_route() {
    let mut p = TTSPipeline::new();
    let mut profile = *native_profile();
    profile.digits = Digits::Expanded;
    let out = p.process_for_engine("1. Первое", &profile);
    assert_eq!(out.text, "первое:Первое");

    let out = p.process_for_engine("1. Первое", native_profile());
    assert_eq!(out.text, "1. Первое", "the marker itself is a number");
}

#[test]
fn a_glued_link_keeps_its_number_readable() {
    let mut p = TTSPipeline::new();
    let out = p.process_for_engine(
        "CVE.[12](#fn:12)\n\nТекст",
        &crate::profiles::TERATTS_PROFILE,
    );
    assert_eq!(out.text, "CVE. двенадцать\n\nТекст");
}

#[test]
fn markdown_blockquote_markers_are_not_read() {
    let mut p = TTSPipeline::new();
    let out = p.process_for_engine("> цитата\n>> вложенная", native_profile());
    assert_eq!(out.text, "цитата\nвложенная");
    let out = p.process_for_engine("сравнение a > b", native_profile());
    assert_eq!(
        out.text, "сравнение a  больше  b",
        "an inline comparison is narrated, not read as a blockquote marker"
    );
}

#[test]
fn user_dictionary_applies_under_a_native_profile() {
    let mut p = TTSPipeline::new();
    let mut dict = UserDictionary::default();
    dict.insert(DictionaryEntry {
        from: "kubernetes".to_string(),
        to: "кубернетес".to_string(),
        forms: Vec::new(),
    });
    p.set_user_dictionary(dict);
    let out = p.process_for_engine("Kubernetes кластер", native_profile());
    assert_eq!(out.text, "кубернетес кластер");
}

#[test]
fn narrated_fragments_are_not_read_twice() {
    // A second pass must not re-read what the first already narrated: URLs and
    // identifiers are the fragments whose narration emits new Latin.
    let mut p = TTSPipeline::new();
    for input in [
        "https://example.com/path/to/page",
        "Вызови getUserData сейчас",
        "Рост 50% и 1.5 GB, дата 12.05.2024",
    ] {
        let once = p.process_text(input);
        let twice = p.process_text(&once);
        assert_eq!(once, twice, "second pass changed {input:?}");
    }
}

#[test]
fn transliteration_is_not_applied_twice() {
    let mut p = TTSPipeline::new();
    let once = p
        .process_for_engine(
            "Вызови getUserData() через API",
            &crate::profiles::SILERO_PROFILE,
        )
        .text;
    assert_eq!(once, "Вызови гет юзер дата() через эй пи ай");
    let again = p
        .process_for_engine(&once, &crate::profiles::SILERO_PROFILE)
        .text;
    assert_eq!(again, once);
}

#[test]
fn number_expansion_is_not_applied_twice() {
    let mut p = TTSPipeline::new();
    let once = p
        .process_for_engine("42 сервера", &crate::profiles::SILERO_PROFILE)
        .text;
    assert_eq!(once, "сорок два сервера");
    let again = p
        .process_for_engine(&once, &crate::profiles::SILERO_PROFILE)
        .text;
    assert_eq!(again, once);
}

// ── Span annotations ──────────────────────────────────────────────────────

/// The output slice a span covers, so assertions read in text terms.
fn span_text(text: &str, span: &Span) -> String {
    text.chars()
        .skip(span.start)
        .take(span.end - span.start)
        .collect()
}

#[test]
fn alphanumeric_abbreviations_are_narrated_on_both_routes() {
    let mut p = TTSPipeline::new();
    for profile in [&crate::profiles::TERATTS_PROFILE, native_profile()] {
        let out = p.process_for_engine("IPv6 и HTTP/2", profile);
        assert!(
            !out.text.contains("IPv") && !out.text.contains("HTTP"),
            "mixed-script fragment left: {:?}",
            out.text
        );
        assert!(
            out.text.contains("шесть") && out.text.contains("два"),
            "digits missing: {:?}",
            out.text
        );
    }
}

#[test]
fn letter_digit_tokens_are_narrated_with_the_profile_route() {
    let mut p = TTSPipeline::new();
    let cases = [
        ("p95", "пи девяносто пять"),
        ("v2", "ви два"),
        ("x86", "икс восемьдесят шесть"),
        ("sha256", "ша двести пятьдесят шесть"),
        ("mp3", "эм пэ три"),
        ("H264", "эйч двести шестьдесят четыре"),
        ("3d", "три ди"),
    ];
    for (input, expected) in cases {
        let out = p.process_for_engine(input, &crate::profiles::SILERO_PROFILE);
        assert_eq!(out.text, expected, "input: {input:?}");
        assert!(
            !out.text.chars().any(|c| c.is_ascii_alphabetic()),
            "Latin left for a narrating profile: {:?}",
            out.text
        );
    }

    let native = p.process_for_engine("p95 и sha256", native_profile());
    assert_eq!(native.text, "p95 и sha256");
    let kinds: Vec<_> = native.spans.iter().map(|s| s.kind).collect();
    assert_eq!(
        kinds,
        vec![SpanKind::CodeIdentifier, SpanKind::CodeIdentifier]
    );
    assert_eq!(span_text(&native.text, &native.spans[0]), "p95");
    assert_eq!(span_text(&native.text, &native.spans[1]), "sha256");
}

#[test]
fn letter_digit_token_respects_the_digits_route() {
    let mut p = TTSPipeline::new();
    let profile = TextProfile {
        digits: Digits::Native,
        units: Units::Native,
        ..crate::profiles::SILERO_PROFILE
    };
    let out = p.process_for_engine("p95", &profile);
    assert_eq!(out.text, "пи 95");
}

#[test]
fn narrating_profile_produces_no_spans() {
    let mut p = TTSPipeline::new();
    let out = p.process_for_engine(
        "Вызови getUserData() через API",
        &crate::profiles::SILERO_PROFILE,
    );
    assert!(out.spans.is_empty(), "unexpected spans: {:?}", out.spans);
}

#[test]
fn native_prose_run_is_annotated_with_its_language() {
    let mut p = TTSPipeline::new();
    let out = p.process_for_engine("Kubernetes нужен", native_profile());
    assert_eq!(out.spans.len(), 1, "spans: {:?}", out.spans);
    let span = &out.spans[0];
    assert_eq!(span.kind, SpanKind::ProseLatin);
    assert_eq!(span.lang, Some("en"));
    assert_eq!(span_text(&out.text, span), "Kubernetes");
}

#[test]
fn native_identifier_and_prose_are_annotated_separately() {
    let mut p = TTSPipeline::new();
    let out = p.process_for_engine("Вызови getUserData через API", native_profile());
    let kinds: Vec<_> = out.spans.iter().map(|s| s.kind).collect();
    assert_eq!(kinds, vec![SpanKind::CodeIdentifier, SpanKind::ProseLatin]);
    assert_eq!(span_text(&out.text, &out.spans[0]), "getUserData");
    assert_eq!(span_text(&out.text, &out.spans[1]), "API");
}

#[test]
fn native_code_block_is_kept_verbatim_and_annotated() {
    let mut p = TTSPipeline::new();
    p.set_code_block_mode(CodeBlockMode::Full);
    let input = "Пример:\n\n```rust\nlet x = 42;\nif x == 0 { return; }\n```";
    let out = p.process_for_engine(input, &crate::profiles::TERATTS_PROFILE);

    assert_eq!(out.text, "Пример:\n\nlet x = 42; if x == 0 { return; }");
    assert!(
        !out.text
            .chars()
            .any(|c| { matches!(c as u32, 0xE000..=0xF8FF) }),
        "a private-use placeholder leaked into the output"
    );
    assert_eq!(out.spans.len(), 1, "spans: {:?}", out.spans);
    let span = &out.spans[0];
    assert_eq!(span.kind, SpanKind::CodeBlock);
    assert_eq!(span.lang, Some("en"));
    assert_eq!(
        span_text(&out.text, span),
        "let x = 42; if x == 0 { return; }"
    );

    // Every output codepoint of the block maps to the whole source block:
    // the same collapse the narrated-full path has.
    let block_start = input[..input.find("```rust").unwrap()].chars().count();
    let block_end = input.chars().count();
    for entry in &out.mapping.char_map[span.start..span.end] {
        assert_eq!(*entry, (block_start, block_end));
    }
}

#[test]
fn brief_mode_wins_over_a_native_code_route() {
    let mut p = TTSPipeline::new();
    p.set_code_block_mode(CodeBlockMode::Brief);
    let out = p.process_for_engine(
        "```rust\nlet x = 42;\n```",
        &crate::profiles::TERATTS_PROFILE,
    );

    assert_eq!(out.text, "далее следует пример кода на раст");
    assert!(out.spans.is_empty(), "spans: {:?}", out.spans);
}

#[test]
fn piper_narrates_prose_but_keeps_code() {
    let mut p = TTSPipeline::new();
    p.set_code_block_mode(CodeBlockMode::Full);
    let out = p.process_for_engine(
        "Kubernetes:\n```rust\nlet x = 42;\n```",
        &crate::profiles::PIPER_PROFILE,
    );

    assert_eq!(out.text, "кубернетис:\nlet x = 42;");
    assert_eq!(out.spans.len(), 1, "spans: {:?}", out.spans);
    assert_eq!(out.spans[0].kind, SpanKind::CodeBlock);
}

#[test]
fn dictionary_does_not_rewrite_native_code() {
    let mut p = TTSPipeline::new();
    p.set_code_block_mode(CodeBlockMode::Full);
    let mut dict = UserDictionary::default();
    dict.insert(DictionaryEntry {
        from: "let".to_string(),
        to: "лет".to_string(),
        forms: Vec::new(),
    });
    p.set_user_dictionary(dict);
    let out = p.process_for_engine(
        "```rust\nlet x = 42;\n```",
        &crate::profiles::TERATTS_PROFILE,
    );

    assert_eq!(out.text, "let x = 42;");
}

#[test]
fn every_native_block_is_restored_in_place() {
    let mut p = TTSPipeline::new();
    p.set_code_block_mode(CodeBlockMode::Full);
    let out = p.process_for_engine(
        "```rust\nlet a = 1;\n```\nсередина\n```rust\nlet b = 2;\n```",
        &crate::profiles::TERATTS_PROFILE,
    );

    assert_eq!(out.text, "let a = 1;\nсередина\nlet b = 2;");
    assert_eq!(out.spans.len(), 2, "spans: {:?}", out.spans);
    assert_eq!(span_text(&out.text, &out.spans[0]), "let a = 1;");
    assert_eq!(span_text(&out.text, &out.spans[1]), "let b = 2;");
}

#[test]
fn pasted_private_use_glyphs_do_not_confuse_the_restore() {
    let mut p = TTSPipeline::new();
    p.set_code_block_mode(CodeBlockMode::Full);
    let out = p.process_for_engine(
        "\u{e000}\u{e000}\n```rust\nlet a = 1;\n```",
        &crate::profiles::TERATTS_PROFILE,
    );

    assert_eq!(out.text, "\u{e000}\u{e000}\nlet a = 1;");
    assert_eq!(out.spans.len(), 1, "spans: {:?}", out.spans);
    assert_eq!(span_text(&out.text, &out.spans[0]), "let a = 1;");
}

#[test]
fn dictionary_override_is_annotated_in_output_coordinates() {
    let mut p = TTSPipeline::new();
    let mut dict = UserDictionary::default();
    dict.insert(DictionaryEntry {
        from: "docker".to_string(),
        to: "доккер".to_string(),
        forms: Vec::new(),
    });
    p.set_user_dictionary(dict);
    let out = p.process_for_engine("Запусти docker сейчас", &crate::profiles::SILERO_PROFILE);
    assert_eq!(out.text, "Запусти доккер сейчас");
    assert_eq!(out.spans.len(), 1, "spans: {:?}", out.spans);
    assert_eq!(out.spans[0].kind, SpanKind::DictionaryOverride);
    assert_eq!(out.spans[0].lang, None);
    assert_eq!(span_text(&out.text, &out.spans[0]), "доккер");
}

#[test]
fn spans_shift_with_earlier_replacements() {
    // The code block expands before the Latin word is annotated, so the
    // span must follow the output text, not its original offset.
    let mut p = TTSPipeline::new();
    let out = p.process_for_engine("```rust\nlet a = 1;\n```\nKubernetes тут", native_profile());
    assert_eq!(out.spans.len(), 1, "spans: {:?}", out.spans);
    assert_eq!(span_text(&out.text, &out.spans[0]), "Kubernetes");
}

#[test]
fn empty_input_produces_no_spans() {
    let mut p = TTSPipeline::new();
    assert!(p.process_for_engine("", native_profile()).spans.is_empty());
    assert!(
        p.process_for_engine("   \n\t ", native_profile())
            .spans
            .is_empty()
    );
}

#[test]
fn dictionary_override_inside_a_native_identifier_drops_the_identifier_span() {
    // "get-user" is one kebab identifier, and the dictionary replaces only
    // its second part: the identifier annotation would claim "get-юзер" as
    // Latin, so it gives way to the override instead.
    let mut p = TTSPipeline::new();
    let mut dict = UserDictionary::default();
    dict.insert(DictionaryEntry {
        from: "user".to_string(),
        to: "юзер".to_string(),
        forms: Vec::new(),
    });
    p.set_user_dictionary(dict);
    let out = p.process_for_engine("вызови get-user сейчас", native_profile());
    assert_eq!(out.text, "вызови get-юзер сейчас");
    // "юзер" gives way to the override; what is left of the identifier is a
    // plain Latin run, which the engine reads natively.
    let kinds: Vec<(SpanKind, String)> = out
        .spans
        .iter()
        .map(|span| (span.kind, span_text(&out.text, span)))
        .collect();
    assert_eq!(
        kinds,
        vec![
            (SpanKind::ProseLatin, "get".to_string()),
            (SpanKind::DictionaryOverride, "юзер".to_string())
        ]
    );
    assert!(
        out.spans
            .iter()
            .all(|span| span.lang.is_some() == (span.kind == SpanKind::ProseLatin)),
        "only the Latin run carries a language: {:?}",
        out.spans
    );
}

#[test]
fn dictionary_override_wins_over_a_native_identifier_span() {
    // The identifier and the dictionary both claim this fragment; the more
    // specific decision must win and the overlap must not survive as two
    // spans (the recordings happen on either side of the dictionary pass).
    let mut p = TTSPipeline::new();
    let mut dict = UserDictionary::default();
    dict.insert(DictionaryEntry {
        from: "getuserdata".to_string(),
        to: "получить данные".to_string(),
        forms: Vec::new(),
    });
    p.set_user_dictionary(dict);
    let out = p.process_for_engine("Вызови getUserData сейчас", native_profile());
    assert_eq!(out.spans.len(), 1, "spans: {:?}", out.spans);
    assert_eq!(out.spans[0].kind, SpanKind::DictionaryOverride);
    assert_eq!(span_text(&out.text, &out.spans[0]), "получить данные");
}

// ── Dictionary stress marks (change `dictionary-stress-marks`) ──────────

fn dictionary_with_mark() -> UserDictionary {
    let mut dict = UserDictionary::default();
    dict.insert(DictionaryEntry {
        from: "moskva".to_string(),
        to: "М+осква".to_string(),
        forms: Vec::new(),
    });
    dict
}

#[test]
fn stress_mark_reaches_a_profile_that_declares_it() {
    let mut p = TTSPipeline::new();
    p.set_user_dictionary(dictionary_with_mark());
    let out = p.process_for_engine("в moskva сегодня", &crate::profiles::SILERO_NATIVE_PROFILE);
    assert_eq!(out.text, "в М+осква сегодня");
    // The mark travels with the replacement, so the override span still
    // covers the whole word and maps back to the original token.
    assert_eq!(out.spans.len(), 1, "spans: {:?}", out.spans);
    assert_eq!(span_text(&out.text, &out.spans[0]), "М+осква");
    let word_start = "в ".chars().count();
    let ranges: Vec<(usize, usize)> =
        out.mapping.char_map[word_start..word_start + "М+осква".chars().count()].to_vec();
    for (s, e) in ranges {
        let mapped: String = out.mapping.original.chars().skip(s).take(e - s).collect();
        assert_eq!(
            mapped, "moskva",
            "every replacement character, the mark included, maps onto the word"
        );
    }
}

#[test]
fn the_stress_decision_is_rederived_on_every_run() {
    // Switching engines on one pipeline must not inherit the previous
    // run's decision: the flag is set from the profile on each call.
    let mut p = TTSPipeline::new();
    p.set_user_dictionary(dictionary_with_mark());
    let input = "в moskva сегодня";

    let marked = p
        .process_for_engine(input, &crate::profiles::SILERO_NATIVE_PROFILE)
        .text;
    assert_eq!(marked, "в М+осква сегодня");

    let stripped = p
        .process_for_engine(input, &crate::profiles::SILERO_PROFILE)
        .text;
    assert_eq!(stripped, "в Москва сегодня");
    assert!(!stripped.contains('+'), "no mark leaks: {stripped:?}");

    let marked_again = p
        .process_for_engine(input, &crate::profiles::SILERO_NATIVE_PROFILE)
        .text;
    assert_eq!(
        marked_again, marked,
        "the previous run's decision must not stick to the pipeline"
    );
}

// ── Cyrillic dictionary keys (change `dictionary-cyrillic-keys`) ────────

fn dictionary_with_cyrillic_mark() -> UserDictionary {
    let mut dict = UserDictionary::default();
    dict.insert(DictionaryEntry {
        from: "замок".to_string(),
        to: "зам+ок".to_string(),
        forms: Vec::new(),
    });
    dict
}

#[test]
fn a_cyrillic_entry_replaces_its_exact_word() {
    let mut p = TTSPipeline::new();
    p.set_user_dictionary(dictionary_with_cyrillic_mark());
    let out = p.process_for_engine(
        "старый замок стоит",
        &crate::profiles::SILERO_NATIVE_PROFILE,
    );
    assert_eq!(out.text, "старый зам+ок стоит");
    assert_eq!(out.spans.len(), 1, "spans: {:?}", out.spans);
    assert_eq!(out.spans[0].kind, SpanKind::DictionaryOverride);
    assert_eq!(
        out.spans[0].lang, None,
        "a Russian word needs no language tag"
    );
    assert_eq!(span_text(&out.text, &out.spans[0]), "зам+ок");
}

#[test]
fn a_single_form_entry_matches_case_insensitively_but_not_inflections() {
    let mut p = TTSPipeline::new();
    p.set_user_dictionary(dictionary_with_cyrillic_mark());
    let upper = p.process_for_engine(
        "старый ЗАМОК стоит",
        &crate::profiles::SILERO_NATIVE_PROFILE,
    );
    assert_eq!(
        upper.text, "старый зам+ок стоит",
        "the typed case is replaced"
    );
    let inflected =
        p.process_for_engine("старого замка нет", &crate::profiles::SILERO_NATIVE_PROFILE);
    assert_eq!(
        inflected.text, "старого замка нет",
        "an inflected form is a different key"
    );
    assert!(inflected.spans.is_empty(), "spans: {:?}", inflected.spans);
}

#[test]
fn listed_inflected_forms_are_replaced_and_unlisted_ones_are_left_alone() {
    let mut p = TTSPipeline::new();
    let mut dict = UserDictionary::default();
    dict.insert(DictionaryEntry::with_forms(
        "замок",
        "зам+ок",
        ["замка", "замке"],
    ));
    p.set_user_dictionary(dict);

    let out = p.process_for_engine(
        "старого замка и замке нет, а замки стоят",
        &crate::profiles::SILERO_NATIVE_PROFILE,
    );
    assert_eq!(out.text, "старого зам+ок и зам+ок нет, а замки стоят");
    assert_eq!(out.spans.len(), 2, "spans: {:?}", out.spans);
    for span in &out.spans {
        assert_eq!(span.kind, SpanKind::DictionaryOverride);
        assert_eq!(span_text(&out.text, span), "зам+ок");
    }
}

#[test]
fn a_cyrillic_entry_strips_the_mark_without_stress_support() {
    let mut p = TTSPipeline::new();
    p.set_user_dictionary(dictionary_with_cyrillic_mark());
    let out = p.process_for_engine("старый замок стоит", &crate::profiles::SILERO_PROFILE);
    // Stripping a mark from the entry's own word yields the input text, so
    // the recorded span is what proves the entry fired at all.
    assert_eq!(out.text, "старый замок стоит");
    assert!(!out.text.contains('+'), "no mark leaks: {:?}", out.text);
    assert_eq!(out.spans.len(), 1, "spans: {:?}", out.spans);
    assert_eq!(out.spans[0].kind, SpanKind::DictionaryOverride);
    assert_eq!(span_text(&out.text, &out.spans[0]), "замок");
}

#[test]
fn a_latin_only_dictionary_leaves_russian_words_alone() {
    // The Cyrillic scan is skipped entirely; this pins that the skip cannot
    // change the output for a dictionary that has no Cyrillic key.
    let mut p = TTSPipeline::new();
    let mut dict = UserDictionary::default();
    dict.insert(DictionaryEntry {
        from: "docker".to_string(),
        to: "доккер".to_string(),
        forms: Vec::new(),
    });
    p.set_user_dictionary(dict);
    let out = p.process_for_engine("Запусти docker и замок", &crate::profiles::SILERO_PROFILE);
    assert_eq!(out.text, "Запусти доккер и замок");
}

#[test]
fn entries_of_both_scripts_apply_in_text_order() {
    // The two candidate scans each yield a sorted run; the merged list must
    // stay sorted because the replacement applies back to front.
    let mut p = TTSPipeline::new();
    let mut dict = UserDictionary::default();
    dict.insert(DictionaryEntry {
        from: "docker".to_string(),
        to: "доккер".to_string(),
        forms: Vec::new(),
    });
    dict.insert(DictionaryEntry {
        from: "замок".to_string(),
        to: "зам+ок".to_string(),
        forms: Vec::new(),
    });
    p.set_user_dictionary(dict);

    let cyrillic_first = p.process_for_engine(
        "старый замок и docker",
        &crate::profiles::SILERO_NATIVE_PROFILE,
    );
    assert_eq!(cyrillic_first.text, "старый зам+ок и доккер");

    let latin_first = p.process_for_engine(
        "docker и старый замок",
        &crate::profiles::SILERO_NATIVE_PROFILE,
    );
    assert_eq!(latin_first.text, "доккер и старый зам+ок");
}

#[test]
fn the_pre_pass_pattern_covers_every_letter_the_validator_accepts() {
    // The validator, the pre-pass regex, and the TS mirror each spell out
    // the Russian alphabet; this pins the two Rust copies to one another,
    // so a letter cannot be accepted and then never matched.
    for c in ('а'..='я').chain(['ё', 'Ё']).chain('А'..='Я') {
        let word = format!("{c}{c}{c}");
        assert!(
            crate::dictionary::validate_entry(&word, &word).is_ok(),
            "the validator rejects {c:?}"
        );
        assert_eq!(
            re_user_dict_token_cyrillic()
                .find(&word)
                .map(|m| m.as_str()),
            Some(word.as_str()),
            "the pre-pass does not match a word built from {c:?}"
        );
    }
}

#[test]
fn e_and_yo_are_different_keys() {
    let mut dict = UserDictionary::default();
    dict.insert(DictionaryEntry {
        from: "все".to_string(),
        to: "вс+е".to_string(),
        forms: Vec::new(),
    });
    assert_eq!(dict.resolve("все", true).as_deref(), Some("вс+е"));
    assert_eq!(
        dict.resolve("всё", true),
        None,
        "ё is a different letter, so the entry must not fire on it"
    );
}

// ── Hyphenated dictionary keys (change `add-hyphenated-dictionary-keys`) ─

fn dictionary_with_hyphenated(entries: &[(&str, &str)]) -> UserDictionary {
    let mut dict = UserDictionary::default();
    for (from, to) in entries {
        dict.insert(DictionaryEntry::new(*from, *to));
    }
    dict
}

#[test]
fn a_hyphenated_entry_replaces_the_whole_token() {
    // The code-identifier phase would split "UTF-8" into "UTF" + "8", so
    // the whole-token pass has to resolve it first.
    let mut p = TTSPipeline::new();
    p.set_user_dictionary(dictionary_with_hyphenated(&[("UTF-8", "у тэф восемь")]));
    let out = p.process_for_engine("Вызови UTF-8 сейчас", &crate::profiles::SILERO_PROFILE);
    assert_eq!(out.text, "Вызови у тэф восемь сейчас");
    assert_eq!(out.spans.len(), 1, "spans: {:?}", out.spans);
    assert_eq!(out.spans[0].kind, SpanKind::DictionaryOverride);
    assert_eq!(out.spans[0].lang, None);
    assert_eq!(span_text(&out.text, &out.spans[0]), "у тэф восемь");
}

#[test]
fn a_hyphenated_entry_matches_case_insensitively() {
    let mut p = TTSPipeline::new();
    p.set_user_dictionary(dictionary_with_hyphenated(&[("UTF-8", "у тэф восемь")]));
    let upper = p.process_for_engine("UTF-8", &crate::profiles::SILERO_PROFILE);
    assert_eq!(upper.text, "у тэф восемь");
    let lower = p.process_for_engine("utf-8", &crate::profiles::SILERO_PROFILE);
    assert_eq!(lower.text, "у тэф восемь");
    let mixed = p.process_for_engine("Utf-8", &crate::profiles::SILERO_PROFILE);
    assert_eq!(mixed.text, "у тэф восемь");
}

#[test]
fn a_whole_token_entry_outranks_its_parts() {
    let mut p = TTSPipeline::new();
    p.set_user_dictionary(dictionary_with_hyphenated(&[
        ("e", "и"),
        ("mail", "мэйл"),
        ("e-mail", "имейл"),
    ]));
    let out = p.process_for_engine("напиши e-mail", &crate::profiles::SILERO_PROFILE);
    assert_eq!(out.text, "напиши имейл");
}

#[test]
fn part_entries_still_apply_without_a_whole_token_entry() {
    let mut p = TTSPipeline::new();
    p.set_user_dictionary(dictionary_with_hyphenated(&[("utf", "у тэф")]));
    let out = p.process_for_engine("UTF-8", &crate::profiles::SILERO_PROFILE);
    assert_eq!(out.text, "у тэф восемь");
}

#[test]
fn a_hyphenated_entry_does_not_rewrite_a_longer_identifier() {
    // A partial replacement would leave a mixed-script fragment
    // ("имейл-signature") that no later phase owns.
    let mut p = TTSPipeline::new();
    p.set_user_dictionary(dictionary_with_hyphenated(&[("e-mail", "имейл")]));
    let out = p.process_for_engine("e-mail-signature", &crate::profiles::SILERO_PROFILE);
    assert!(!out.text.contains("имейл"), "text: {:?}", out.text);
    assert!(
        out.spans
            .iter()
            .all(|span| span.kind != SpanKind::DictionaryOverride),
        "spans: {:?}",
        out.spans
    );
}

#[test]
fn a_hyphenated_entry_applies_under_a_native_identifier_route() {
    // The native route never runs the kebab normalizer, so the override has
    // to come from the pass that runs ahead of the identifier phase.
    let mut p = TTSPipeline::new();
    p.set_user_dictionary(dictionary_with_hyphenated(&[("e-mail", "имейл")]));
    let out = p.process_for_engine("напиши e-mail и getUserData", native_profile());
    assert_eq!(out.text, "напиши имейл и getUserData");
    let spans: Vec<(SpanKind, String)> = out
        .spans
        .iter()
        .map(|span| (span.kind, span_text(&out.text, span)))
        .collect();
    assert_eq!(
        spans,
        vec![
            (SpanKind::DictionaryOverride, "имейл".to_string()),
            (SpanKind::CodeIdentifier, "getUserData".to_string()),
        ],
        "the override claims its token, the rest of the identifiers stay native"
    );
    assert_eq!(out.spans[1].lang, Some("en"));
}

#[test]
fn a_hyphenated_cyrillic_entry_replaces_the_whole_word() {
    let mut p = TTSPipeline::new();
    p.set_user_dictionary(dictionary_with_hyphenated(&[("по-русски", "по русски")]));
    let out = p.process_for_engine("говори по-русски", &crate::profiles::SILERO_NATIVE_PROFILE);
    assert_eq!(out.text, "говори по русски");
    assert_eq!(out.spans.len(), 1, "spans: {:?}", out.spans);
    assert_eq!(span_text(&out.text, &out.spans[0]), "по русски");
}

#[test]
fn inline_code_resolves_a_hyphenated_entry_on_the_whole_token() {
    // Inline code is normalized in phase 5, before any dictionary pass, so
    // the kebab normalizer has to carry the same whole-token lookup.
    let mut p = TTSPipeline::new();
    p.set_user_dictionary(dictionary_with_hyphenated(&[("e-mail", "имейл")]));
    let out = p.process_for_engine("поле `e-mail` пустое", &crate::profiles::SILERO_PROFILE);
    assert_eq!(out.text, "поле имейл пустое");
}

#[test]
fn the_hyphenated_pattern_matches_every_letter_the_validator_accepts() {
    // The validator and the candidate pattern each spell out the alphabets;
    // this pins them to one another, so a letter cannot be accepted and
    // then never matched.
    for c in ('a'..='z')
        .chain('A'..='Z')
        .chain('а'..='я')
        .chain(['ё', 'Ё'])
        .chain('А'..='Я')
    {
        let word = format!("{c}{c}-{c}{c}");
        assert!(
            crate::dictionary::validate_entry(&word, &word).is_ok(),
            "the validator rejects {c:?}"
        );
        assert_eq!(
            re_user_dict_hyphenated().find(&word).map(|m| m.as_str()),
            Some(word.as_str()),
            "the whole-token pass does not match a token built from {c:?}"
        );
    }
}

/// The engine text must carry no markup characters: TeraTTS reads `<` and
/// `>` only as its own tag syntax and rejects the segment, while the
/// alphabet-gated engines silently drop the character and lose the
/// comparison or redirect it belonged to. Asserted across every shipping
/// profile because no profile reads a stray bracket natively.
#[test]
fn pipeline_never_emits_markup_characters_for_prose() {
    let cases = [
        "Перенаправьте вывод в 2>/dev/null",
        "Значение a > b",
        "Значение a < b",
        "Тег <div> тут",
        "Метод возвращает `Cow<str>` при потере",
        "Смотри <https://example.com/ путь>",
        "если a >= b",
        "если a <= b",
        "стрелка -> туда",
    ];
    for profile in [
        &profiles::SILERO_PROFILE,
        &profiles::SILERO_NATIVE_PROFILE,
        &profiles::PIPER_PROFILE,
        &profiles::TERATTS_PROFILE,
    ] {
        for input in cases {
            let mut p = TTSPipeline::new();
            let text = p.process_for_engine(input, profile).text;
            assert!(
                !text.contains('<') && !text.contains('>'),
                "profile left markup in {text:?} for input {input:?}"
            );
        }
    }
}

/// The incident text (2026-11-10): the inline-code phase narrates each
/// identifier part, so `Cow<str>` becomes `ков<стр>` — produced inside an
/// already-replaced region, where `TrackedText` refuses every later edit. The
/// prose bracket phase therefore could not see it, and TeraTTS rejected the
/// whole generation with `text_invalid_tags`. The reading and the position it
/// maps back to are pinned here; the case is also part of the cross-profile
/// markup sweep above.
#[test]
fn pipeline_reads_brackets_inside_narrated_inline_code() {
    let input = "Метод возвращает `Cow<str>` при потере";
    let mut p = TTSPipeline::new();
    let out = p.process_for_engine(input, &profiles::TERATTS_PROFILE);
    assert_eq!(
        out.text, "Метод возвращает ков меньше стр больше при потере",
        "markup from the inline-code phase reached the engine text"
    );

    // The reading points back at the bracket inside the code fragment. It
    // inherits the inline-code replacement's range — every codepoint of a
    // replacement maps to the whole replaced source range, so the exact
    // bracket position is not recoverable here.
    let bracket = input[..input.find('<').unwrap()].chars().count();
    let reading = out.text[..out.text.find("меньше").unwrap()].chars().count();
    let range = out
        .mapping
        .get_original_range(reading, reading + "меньше".chars().count());
    assert!(
        range.0 <= bracket && bracket < range.1,
        "the reading maps to {range:?}, which does not cover the bracket at {bracket}"
    );
}

/// The incident text: a shell redirect with no spaces around the bracket.
/// The exact reading is pinned here (and by the `teratts/shell_redirect`
/// golden fixture) so a wording change shows up in review.
#[test]
fn pipeline_reads_a_shell_redirect_bracket() {
    let mut p = TTSPipeline::new();
    assert_eq!(
        p.process_text("Перенаправьте вывод в 2>/dev/null"),
        "Перенаправьте вывод в два больше слэш дев слэш налл"
    );
}

/// Multi-character operators keep their own reading: the bracket phase
/// runs after the operator phase and never splits `>=` into `>` + `=`.
#[test]
fn pipeline_operator_phase_owns_multi_character_brackets() {
    let mut p = TTSPipeline::new();
    // Compared with collapsed whitespace: a narrated reading is inserted
    // space-padded, and the padding is an implementation detail.
    let reading = p
        .process_text("если a >= b")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    assert_eq!(reading, "если эй больше или равно би");
}

// ── English-context numbers ──────────────────────────────────────────────

#[test]
fn english_context_numbers_read_in_english_when_narrated() {
    let mut p = TTSPipeline::new();
    let out = p.process_for_engine(
        "In 2024 we shipped 3 releases.",
        &crate::profiles::SILERO_PROFILE,
    );
    assert!(
        out.text.contains("ту саузенд твенти-фо"),
        "narrated reading: {}",
        out.text
    );
    assert!(out.text.contains(" фри "), "narrated reading: {}", out.text);
    assert!(out.spans.is_empty(), "spans: {:?}", out.spans);
}

#[test]
fn english_context_numbers_are_spelled_on_the_native_route() {
    let mut p = TTSPipeline::new();
    let out = p.process_for_engine(
        "In 2024 we shipped 3 releases.",
        &crate::profiles::TERATTS_PROFILE,
    );
    assert_eq!(out.text, "In twenty twenty-four we shipped three releases.");
    let texts: Vec<String> = out
        .spans
        .iter()
        .map(|span| span_text(&out.text, span))
        .collect();
    assert_eq!(
        texts.join("|"),
        "In|twenty twenty-four|we|shipped|three|releases"
    );
    assert!(
        out.spans
            .iter()
            .all(|span| span.kind == SpanKind::ProseLatin && span.lang == Some("en")),
        "spans: {:?}",
        out.spans
    );
}

#[test]
fn english_context_number_at_the_sentence_start_is_english() {
    let mut p = TTSPipeline::new();
    let out = p.process_for_engine("2024 was the year.", &crate::profiles::TERATTS_PROFILE);
    assert_eq!(out.text, "twenty twenty-four was the year.");
    assert!(
        out.spans
            .iter()
            .any(|span| span_text(&out.text, span) == "twenty twenty-four"),
        "spans: {:?}",
        out.spans
    );
}

#[test]
fn digits_are_transparent_behind_a_comma() {
    let mut p = TTSPipeline::new();
    let out = p.process_for_engine("In 2024, 3 releases.", &crate::profiles::TERATTS_PROFILE);
    assert_eq!(out.text, "In twenty twenty-four, three releases.");
    let texts: Vec<String> = out
        .spans
        .iter()
        .map(|span| span_text(&out.text, span))
        .collect();
    assert!(
        texts.iter().any(|text| text == "twenty twenty-four"),
        "spans: {texts:?}"
    );
    assert!(texts.iter().any(|text| text == "three"), "spans: {texts:?}");
}

#[test]
fn cyrillic_sentence_keeps_the_number_russian() {
    let mut p = TTSPipeline::new();
    let out = p.process_for_engine("Установи Python 3.", &crate::profiles::SILERO_PROFILE);
    assert!(
        out.text.ends_with(" три."),
        "narrated reading: {}",
        out.text
    );
    assert!(out.spans.is_empty(), "spans: {:?}", out.spans);
}

#[test]
fn identifier_beside_a_number_is_not_english_prose() {
    let mut p = TTSPipeline::new();
    let out = p.process_for_engine("getUserData 3", &crate::profiles::TERATTS_PROFILE);
    assert_eq!(out.text, "getUserData три");
    assert_eq!(out.spans.len(), 1, "spans: {:?}", out.spans);
    assert_eq!(span_text(&out.text, &out.spans[0]), "getUserData");
}

#[test]
fn constructs_read_in_english_in_english_prose() {
    let mut p = TTSPipeline::new();
    let percent = p.process_for_engine("In 5% we trust", &crate::profiles::SILERO_PROFILE);
    assert!(percent.text.contains("файв пёрсэнт"), "{}", percent.text);

    let money = p.process_for_engine("In $5 we trust", &crate::profiles::SILERO_PROFILE);
    assert!(money.text.contains("файв долларс"), "{}", money.text);
}

#[test]
fn a_bare_approximated_number_keeps_the_russian_reading() {
    let mut p = TTSPipeline::new();
    let approx = p.process_for_engine("In ~5 we trust", &crate::profiles::SILERO_PROFILE);
    assert!(approx.text.contains("около"), "{}", approx.text);
    assert!(!approx.text.contains("файв"), "{}", approx.text);
}

#[test]
fn function_words_use_the_curated_readings() {
    let mut p = TTSPipeline::new();
    for input in ["The API and the cache", "THE API AND the cache"] {
        let out = p.process_for_engine(input, &crate::profiles::SILERO_PROFILE);
        assert!(
            out.text.contains("зэ эй пи ай энд зэ кэш"),
            "{input}: {}",
            out.text
        );
    }
}

#[test]
fn a_construct_in_a_cyrillic_sentence_keeps_the_russian_reading() {
    let mut p = TTSPipeline::new();
    let out = p.process_for_engine("Установи 5 кг.", &crate::profiles::SILERO_PROFILE);
    assert!(out.text.contains("пять килограммов"), "{}", out.text);
}

#[test]
fn the_native_route_spells_the_construct() {
    let mut p = TTSPipeline::new();
    let out = p.process_for_engine("In 5% we trust", &crate::profiles::TERATTS_PROFILE);
    assert_eq!(out.text, "In five percent we trust");
    let spans: Vec<_> = out
        .spans
        .iter()
        .filter(|span| span_text(&out.text, span) == "five percent")
        .collect();
    assert_eq!(spans.len(), 1, "spans: {:?}", out.spans);
    assert_eq!(spans[0].kind, crate::pipeline::spans::SpanKind::ProseLatin);
    assert_eq!(spans[0].lang, Some("en"));
}

#[test]
fn a_spelled_construct_maps_back_to_the_source_span() {
    let mut p = TTSPipeline::new();
    let out = p.process_for_engine("In 5% we trust", &crate::profiles::TERATTS_PROFILE);
    let reading = "five percent";
    let start = out.text.find(reading).expect("the construct is spelled");
    let reading_chars = out.text[..start].chars().count();
    let end = reading_chars + reading.chars().count();
    assert!(
        out.mapping.char_map[reading_chars..end]
            .iter()
            .all(|&(from, to)| (from, to) == (3, 5)),
        "mapping: {:?}",
        &out.mapping.char_map[reading_chars..end]
    );
}

#[test]
fn a_russian_context_construct_keeps_the_russian_reading_on_the_native_route() {
    let mut p = TTSPipeline::new();
    let out = p.process_for_engine("Установи 5 кг.", &crate::profiles::TERATTS_PROFILE);
    assert!(out.text.contains("пять килограммов"), "{}", out.text);
}

#[test]
fn the_native_route_spells_every_construct_family() {
    let cases = [
        ("In 5% we trust", "In five percent we trust", "five percent"),
        (
            "Loaded 100 MB today",
            "Loaded one hundred megabytes today",
            "one hundred megabytes",
        ),
        (
            "Version 1.2.3 shipped",
            "Version one point two point three shipped",
            "one point two point three",
        ),
        (
            "It landed on 12.05.2024 today",
            "It landed on the twelfth of May, twenty twenty-four today",
            "the twelfth of May, twenty twenty-four",
        ),
        (
            "It grew 10-20 builds",
            "It grew ten to twenty builds",
            "ten to twenty",
        ),
        (
            "The clock showed 09:30 today",
            "The clock showed nine thirty today",
            "nine thirty",
        ),
        ("Paid 5 € today", "Paid five euros today", "five euros"),
        (
            "In 2024-12 we shipped",
            "In December twenty twenty-four we shipped",
            "December twenty twenty-four",
        ),
        ("See № 5 here", "See number five here", "number five"),
        ("The 1st release", "The first release", "first"),
    ];
    for (input, expected, reading) in cases {
        let mut p = TTSPipeline::new();
        let out = p.process_for_engine(input, &crate::profiles::TERATTS_PROFILE);
        assert_eq!(out.text, expected, "{input}");
        let spans: Vec<_> = out
            .spans
            .iter()
            .filter(|span| span_text(&out.text, span) == reading)
            .collect();
        assert_eq!(spans.len(), 1, "{input}: spans {:?}", out.spans);
        assert_eq!(
            spans[0].kind,
            crate::pipeline::spans::SpanKind::ProseLatin,
            "{input}"
        );
        assert_eq!(spans[0].lang, Some("en"), "{input}");
    }
}

#[test]
fn a_native_sentence_with_several_constructs_gets_one_span_each() {
    let mut p = TTSPipeline::new();
    let out = p.process_for_engine(
        "Version 1.2.3 shipped on 12.05.2024 in 10-20 builds.",
        &crate::profiles::TERATTS_PROFILE,
    );
    assert_eq!(
        out.text,
        "Version one point two point three shipped on the twelfth of May, twenty twenty-four in ten to twenty builds."
    );
    let spans: Vec<_> = out
        .spans
        .iter()
        .map(|span| span_text(&out.text, span))
        .collect();
    for reading in [
        "one point two point three",
        "the twelfth of May, twenty twenty-four",
        "ten to twenty",
    ] {
        assert_eq!(
            spans.iter().filter(|text| *text == reading).count(),
            1,
            "spans: {spans:?}"
        );
    }
}

#[test]
fn a_spelled_date_is_not_relabelled_as_an_identifier() {
    let mut p = TTSPipeline::new();
    let out = p.process_for_engine(
        "The release landed on 12.05.2024.",
        &crate::profiles::TERATTS_PROFILE,
    );
    assert!(out.text.contains("twenty twenty-four"), "{}", out.text);
    assert!(
        !out.spans
            .iter()
            .any(|span| span.kind == crate::pipeline::spans::SpanKind::CodeIdentifier),
        "spans: {:?}",
        out.spans
    );
}

#[test]
fn an_english_ordinal_reads_on_the_narrated_route() {
    let mut p = TTSPipeline::new();
    let out = p.process_for_engine("In the 1st release", &crate::profiles::SILERO_PROFILE);
    assert_eq!(out.text, "ин зэ фёст релиз");
}

#[test]
fn an_identifier_in_the_sentence_does_not_veto_a_construct() {
    let mut p = TTSPipeline::new();
    let out = p.process_for_engine("In 100 MB of getUserData", &crate::profiles::SILERO_PROFILE);
    assert!(out.text.contains("уан хандрэд мегабайтс"), "{}", out.text);
    assert!(!out.text.contains("100"), "{}", out.text);
}

#[test]
fn zero_padded_pair_is_still_a_cardinal() {
    let mut p = TTSPipeline::new();
    let out = p.process_for_engine("In 07 we trust", &crate::profiles::SILERO_PROFILE);
    assert!(out.text.contains("сэвен"), "{}", out.text);

    let native = p.process_for_engine("In 07 we trust", &crate::profiles::TERATTS_PROFILE);
    assert_eq!(native.text, "In seven we trust");
    assert!(
        native
            .spans
            .iter()
            .any(|span| span_text(&native.text, span) == "seven"),
        "spans: {:?}",
        native.spans
    );
}

#[test]
fn grouped_cardinal_reads_in_english() {
    let mut p = TTSPipeline::new();
    let out = p.process_for_engine("In 10 000 words", &crate::profiles::SILERO_PROFILE);
    assert!(out.text.contains("тен саузенд"), "{}", out.text);
}

#[test]
fn a_list_marker_does_not_veto_the_item_number() {
    let mut p = TTSPipeline::new();
    let out = p.process_for_engine("1. 2024 was the year.", &crate::profiles::TERATTS_PROFILE);
    assert!(out.text.contains("первое"), "marker: {}", out.text);
    assert_eq!(out.text, "первое:twenty twenty-four was the year.");
    assert!(
        out.spans
            .iter()
            .any(|span| span_text(&out.text, span) == "twenty twenty-four"),
        "spans: {:?}",
        out.spans
    );
}

#[test]
fn a_url_reading_is_not_english_prose() {
    let mut p = TTSPipeline::new();
    let out = p.process_for_engine(
        "Check https://example.com 2024 now",
        &crate::profiles::SILERO_PROFILE,
    );
    assert!(
        out.text.contains("две тысячи двадцать четыре"),
        "{}",
        out.text
    );
}

#[test]
fn a_spelled_reading_maps_back_to_the_digits() {
    let mut p = TTSPipeline::new();
    let out = p.process_for_engine("In 2024 we shipped.", &crate::profiles::TERATTS_PROFILE);
    let reading = "twenty twenty-four";
    let start = out.text.find(reading).expect("the number is spelled");
    let reading_chars = out.text[..start].chars().count();
    let reading_len = reading.chars().count();
    for entry in &out.mapping.char_map[reading_chars..reading_chars + reading_len] {
        assert_eq!(
            *entry,
            (3, 7),
            "every spelled character maps to the source digits"
        );
    }
    assert!(
        out.spans
            .iter()
            .any(|span| span_text(&out.text, span) == reading),
        "one prose span covers the whole reading: {:?}",
        out.spans
    );
}

#[test]
fn a_number_glued_to_a_hyphenated_word_is_not_an_identifier() {
    let mut p = TTSPipeline::new();
    let out = p.process_for_engine("In 2024-only builds.", &crate::profiles::TERATTS_PROFILE);
    assert_eq!(out.text, "In twenty twenty-four-only builds.");
    assert!(
        out.spans.iter().any(|span| {
            span.kind == SpanKind::ProseLatin && span_text(&out.text, span) == "twenty twenty-four"
        }),
        "spans: {:?}",
        out.spans
    );
    assert!(
        out.spans
            .iter()
            .all(|span| span.kind != SpanKind::CodeIdentifier),
        "a spelled number must not be relabelled: {:?}",
        out.spans
    );
}
