//! Tests for the dictionary entry model and TOML store.

use super::store;
use super::*;
use std::fs;

fn entry(from: &str, to: &str) -> DictionaryEntry {
    DictionaryEntry {
        from: from.to_string(),
        to: to.to_string(),
    }
}

// ── Validation ─────────────────────────────────────────────────────────────

#[test]
fn accepts_plain_word() {
    assert!(validate_entry("GitHub", "гитхаб").is_ok());
}

#[test]
fn accepts_alnum_token_with_letter() {
    assert!(validate_entry("IPv6", "айпи ви шесть").is_ok());
    assert!(validate_entry("x86", "икс восемьдесят шесть").is_ok());
}

#[test]
fn accepts_a_russian_word() {
    assert!(validate_entry("замок", "зам+ок").is_ok());
    assert!(validate_entry("Замок", "зам+ок").is_ok(), "case is fine");
    assert!(
        validate_entry("ёлка", "+Ёлка").is_ok(),
        "ё is a Russian letter"
    );
    assert!(
        validate_entry("А4", "а четыре").is_ok(),
        "letters and digits"
    );
}

#[test]
fn rejects_a_multi_word_key() {
    assert!(matches!(
        validate_entry("Иванов Пётр", "иванов пётр"),
        Err(DictionaryError::InvalidFrom)
    ));
}

#[test]
fn rejects_a_mixed_script_key() {
    assert!(matches!(
        validate_entry("IPзамок", "ип замок"),
        Err(DictionaryError::InvalidFrom)
    ));
}

#[test]
fn rejects_a_non_russian_cyrillic_letter() {
    // Ukrainian "і" is outside the Russian alphabet the pipeline narrates.
    assert!(matches!(
        validate_entry("кіт", "кот"),
        Err(DictionaryError::InvalidFrom)
    ));
}

#[test]
fn rejects_digit_only_from() {
    assert!(matches!(
        validate_entry("123", "сто двадцать три"),
        Err(DictionaryError::InvalidFrom)
    ));
}

#[test]
fn rejects_empty_from() {
    assert!(matches!(
        validate_entry("", "что-то"),
        Err(DictionaryError::InvalidFrom)
    ));
}

#[test]
fn rejects_punctuated_from() {
    assert!(matches!(
        validate_entry("UTF-8", "у тэф восемь"),
        Err(DictionaryError::InvalidFrom)
    ));
    assert!(matches!(
        validate_entry("a.b", "а точка б"),
        Err(DictionaryError::InvalidFrom)
    ));
}

#[test]
fn rejects_overlong_from() {
    let long = "a".repeat(MAX_FROM_LEN + 1);
    assert!(matches!(
        validate_entry(&long, "длинно"),
        Err(DictionaryError::InvalidFrom)
    ));
}

#[test]
fn rejects_empty_to() {
    assert!(matches!(
        validate_entry("docker", ""),
        Err(DictionaryError::InvalidTo)
    ));
}

#[test]
fn rejects_overlong_to() {
    let long = "а".repeat(MAX_TO_LEN + 1);
    assert!(matches!(
        validate_entry("docker", &long),
        Err(DictionaryError::InvalidTo)
    ));
}

// ── Map semantics ──────────────────────────────────────────────────────────

#[test]
fn insert_returns_updated_vs_added() {
    let mut dict = UserDictionary::default();
    assert!(!dict.insert(entry("GitHub", "гитхаб")), "first insert adds");
    assert!(dict.insert(entry("github", "хаб")), "same key replaces");
    assert_eq!(dict.len(), 1);
    assert_eq!(dict.get("github"), Some("хаб"));
}

#[test]
fn lookup_expects_lowercased_word() {
    let mut dict = UserDictionary::default();
    dict.insert(entry("IPv6", "айпи ви шесть"));
    assert_eq!(dict.get("ipv6"), Some("айпи ви шесть"));
    // `get` documents a lowercased word (call sites lower-case once); raw
    // mixed-case input is a caller contract violation and must not match.
    assert_eq!(dict.get("IPV6"), None);
}

#[test]
fn iteration_is_sorted_by_lowercased_key() {
    let mut dict = UserDictionary::default();
    dict.insert(entry("nginx", "энджинкс"));
    dict.insert(entry("Docker", "докер"));
    let froms: Vec<&str> = dict.iter().map(|e| e.from.as_str()).collect();
    assert_eq!(
        froms,
        ["Docker", "nginx"],
        "docker < nginx case-insensitively"
    );
}

#[test]
fn replace_all_clears_previous_entries() {
    let mut dict = UserDictionary::default();
    dict.insert(entry("old", "старое"));
    dict.replace_all([entry("new", "новое")]);
    assert_eq!(dict.len(), 1);
    assert_eq!(dict.get("new"), Some("новое"));
    assert_eq!(dict.get("old"), None);
}

// ── Store: load/save round-trip ────────────────────────────────────────────

#[test]
fn save_then_load_round_trips_entries() {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = DictionaryStore::new(dir.path().join("user_dictionary.toml"));
    let mut dict = UserDictionary::default();
    dict.insert(entry("GitHub", "гитхаб"));
    dict.insert(entry("IPv6", "айпи ви шесть"));
    store.save(&dict).expect("save");

    let loaded = store.load();
    let got: Vec<(String, String)> = loaded
        .iter()
        .map(|e| (e.from.clone(), e.to.clone()))
        .collect();
    assert_eq!(
        got,
        [
            ("GitHub".to_string(), "гитхаб".to_string()),
            ("IPv6".into(), "айпи ви шесть".into())
        ]
    );
}

#[test]
fn missing_file_loads_empty() {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = DictionaryStore::new(dir.path().join("absent.toml"));
    assert!(store.load().is_empty());
}

#[test]
fn saved_file_has_version_and_entries_table() {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = DictionaryStore::new(dir.path().join("user_dictionary.toml"));
    let mut dict = UserDictionary::default();
    dict.insert(entry("kubectl", "куб контрол"));
    store.save(&dict).expect("save");

    let raw = fs::read_to_string(store.path()).expect("read");
    assert!(raw.contains("version = 1"), "version field: {raw}");
    assert!(raw.contains("[entries]"), "entries table: {raw}");
    assert!(raw.contains("kubectl = \"куб контрол\""), "entry: {raw}");
}

// ── Store: dedupe and invalid entries on load ──────────────────────────────

#[test]
fn case_differing_duplicate_keys_collapse_last_in_file_order_wins() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("user_dictionary.toml");
    fs::write(
        &path,
        "version = 1\n\n[entries]\nGit = \"гит\"\ngit = \"город\"\n",
    )
    .expect("write fixture");

    let dict = DictionaryStore::new(path).load();
    assert_eq!(dict.len(), 1);
    assert_eq!(dict.get("git"), Some("город"), "last in file order wins");
}

#[test]
fn invalid_entries_are_skipped_on_load() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("user_dictionary.toml");
    // Non-ASCII keys need TOML quoting; "123" is a valid bare key.
    fs::write(
        &path,
        "version = 1\n\n[entries]\n\"Иванов Пётр\" = \"иванов\"\ndocker = \"докер\"\n123 = \"число\"\n",
    )
    .expect("write fixture");

    let dict = DictionaryStore::new(path).load();
    assert_eq!(dict.len(), 1);
    assert_eq!(dict.get("docker"), Some("докер"));
}

// ── Store: corruption recovery ─────────────────────────────────────────────

#[test]
fn corrupted_file_is_backed_up_and_dictionary_starts_empty() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("user_dictionary.toml");
    fs::write(&path, "this is [ not toml").expect("write fixture");

    let dict = DictionaryStore::new(path.clone()).load();
    assert!(dict.is_empty());
    assert!(!path.exists(), "corrupted file renamed away");
    let bak = path.with_extension("toml.bak");
    assert!(bak.exists(), "backup created");
}

#[test]
fn foreign_format_version_loads_empty_without_backup() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("user_dictionary.toml");
    fs::write(&path, "version = 99\n\n[entries]\ndocker = \"докер\"\n").expect("write fixture");

    let dict = DictionaryStore::new(path.clone()).load();
    assert!(dict.is_empty());
    assert!(path.exists(), "foreign file left untouched");
}

// ── Import parse / export ──────────────────────────────────────────────────

#[test]
fn parse_import_returns_entries_in_document_order_unvalidated() {
    let raw =
        "version = 1\n\n[entries]\nzzz = \"зэд\"\nDocker = \"докер\"\n\"Иванов\" = \"иванов\"\n";
    let entries = store::parse_import(raw).expect("parses");
    let froms: Vec<&str> = entries.iter().map(|e| e.from.as_str()).collect();
    assert_eq!(
        froms,
        ["zzz", "Docker", "Иванов"],
        "document order, not pre-validated"
    );
}

#[test]
fn parse_import_rejects_invalid_toml_with_typed_error() {
    assert!(matches!(
        store::parse_import("not toml ["),
        Err(DictionaryError::Parse(_))
    ));
}

#[test]
fn export_writes_valid_dictionary_file() {
    let dir = tempfile::tempdir().expect("tempdir");
    let mut dict = UserDictionary::default();
    dict.insert(entry("GitHub", "гитхаб"));
    let out = dir.path().join("exported.toml");

    store::export_to(&dict, &out).expect("export");

    let entries = store::parse_import(&fs::read_to_string(&out).expect("read")).expect("reparse");
    assert_eq!(entries, vec![entry("GitHub", "гитхаб")]);
}

// ── Stress marks ───────────────────────────────────────────────────────────

#[test]
fn accepts_a_mark_before_a_vowel() {
    assert!(validate_entry("moskva", "М+осква").is_ok());
    assert!(validate_entry("zamok", "зам+ок").is_ok(), "lowercase vowel");
    assert!(
        validate_entry("yolka", "+Ёлка").is_ok(),
        "a mark at the start is fine before a vowel"
    );
}

#[test]
fn rejects_a_malformed_mark() {
    for to in ["Моск+ва", "М+оск+ва", "+Москва", "Москва+", "М+осква+"]
    {
        assert!(
            matches!(
                validate_entry("moskva", to),
                Err(DictionaryError::InvalidStress)
            ),
            "{to:?} must be rejected"
        );
    }
}

#[test]
fn the_spoken_form_never_contains_the_mark() {
    assert_eq!(spoken_form("М+осква").expect("valid"), "Москва");
    assert_eq!(spoken_form("зам+ок").expect("valid"), "замок");
    assert_eq!(spoken_form("Москва").expect("valid"), "Москва");
    assert!(matches!(
        spoken_form("Моск+ва"),
        Err(DictionaryError::InvalidStress)
    ));
}

#[test]
fn the_length_limit_counts_the_spoken_form() {
    let spoken = "а".repeat(MAX_TO_LEN);
    assert!(
        validate_entry("x", &format!("{spoken}+а")).is_err(),
        "over the limit"
    );
    let at_limit = "б".repeat(MAX_TO_LEN - 1);
    assert!(
        validate_entry("x", &format!("{at_limit}+а")).is_ok(),
        "the mark itself does not count"
    );
}

#[test]
fn resolve_keeps_the_mark_only_when_asked() {
    let mut dict = UserDictionary::default();
    dict.insert(entry("moskva", "М+осква"));
    assert_eq!(dict.resolve("moskva", false).as_deref(), Some("Москва"));
    assert_eq!(dict.resolve("moskva", true).as_deref(), Some("М+осква"));
}

#[test]
fn a_malformed_mark_in_the_file_is_skipped() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("user_dictionary.toml");
    fs::write(
        &path,
        "version = 1\n\n[entries]\nmoskva = \"Моск+ва\"\nzamok = \"зам+ок\"\n",
    )
    .expect("write");
    let dict = DictionaryStore::new(path).load();
    assert_eq!(dict.len(), 1, "only the well-formed entry survives");
    assert!(dict.contains("zamok"));
}

#[test]
fn the_mark_survives_export_and_reimport() {
    let dir = tempfile::tempdir().expect("tempdir");
    let mut dict = UserDictionary::default();
    dict.insert(entry("Москва", "М+осква"));
    let out = dir.path().join("exported.toml");

    store::export_to(&dict, &out).expect("export");
    let entries = store::parse_import(&fs::read_to_string(&out).expect("read")).expect("reparse");

    assert_eq!(entries, vec![entry("Москва", "М+осква")]);
}

#[test]
fn a_cyrillic_key_round_trips_through_the_file() {
    // TOML quotes a non-bare key on write; if it did not, the loader would
    // reject the file it just wrote.
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("user_dictionary.toml");
    let mut dict = UserDictionary::default();
    dict.insert(entry("замок", "зам+ок"));
    store::export_to(&dict, &path).expect("export");

    let raw = fs::read_to_string(&path).expect("read");
    assert!(
        raw.contains("\"замок\" = \"зам+ок\""),
        "a Cyrillic key is quoted: {raw}"
    );
    let loaded = DictionaryStore::new(path).load();
    assert_eq!(loaded.get("замок"), Some("зам+ок"));
}
