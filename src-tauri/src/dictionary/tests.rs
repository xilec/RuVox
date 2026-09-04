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
fn rejects_cyrillic_from() {
    assert!(matches!(
        validate_entry("Иванов", "иванов"),
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
        "version = 1\n\n[entries]\n\"Иванов\" = \"иванов\"\ndocker = \"докер\"\n123 = \"число\"\n",
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
