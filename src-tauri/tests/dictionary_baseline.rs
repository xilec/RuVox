//! General-vocabulary transliteration snapshot.
//!
//! `tests/fixtures/transliteration/dictionary_baseline.csv` snapshots the reading of
//! general/canonical vocabulary through `EnglishNormalizer::normalize`. It is a
//! *snapshot*, not a specification: any fallback change legitimately moves rows, so
//! the fixture is regenerated from its word column with `UPDATE_BASELINE=1`:
//!
//! ```text
//! UPDATE_BASELINE=1 cargo test --manifest-path src-tauri/Cargo.toml --test dictionary_baseline
//! ```
//!
//! The regeneration only rewrites readings; words are never added or removed
//! automatically. Always review the diff before committing it — a regenerated row is a
//! decision, not an observation.
//!
//! Inflected and derived forms are covered separately by `inflected_forms.rs`; those
//! readings are curated by hand and deliberately cannot be regenerated.

mod common;

use ruvox_tauri_lib::pipeline::normalizers::english::EnglishNormalizer;

const FIXTURE: &str = "tests/fixtures/transliteration/dictionary_baseline.csv";
const HEADER: &str = "word,current";
const UPDATE_ENV: &str = "UPDATE_BASELINE";

fn is_update_mode() -> bool {
    std::env::var(UPDATE_ENV).as_deref() == Ok("1")
}

fn fixture_path() -> std::path::PathBuf {
    common::resolve_test_path(FIXTURE)
}

fn fixture() -> Vec<(String, String)> {
    let path = fixture_path();
    let content =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    let mut lines = content.lines();
    assert_eq!(lines.next(), Some(HEADER), "{FIXTURE}: unexpected header");
    lines
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            let (word, recorded) = line
                .split_once(',')
                .unwrap_or_else(|| panic!("{FIXTURE}: malformed row {line:?}"));
            (word.to_owned(), recorded.to_owned())
        })
        .collect()
}

#[test]
fn dictionary_baseline() {
    let rows = fixture();
    // A header-only fixture would make this test vacuous — and would let update
    // mode rewrite the file down to its header.
    assert!(!rows.is_empty(), "{FIXTURE}: no rows");
    let mut normalizer = EnglishNormalizer::default();

    if is_update_mode() {
        // Regenerate every reading in place, preserving the word order.
        let mut regenerated = String::from(HEADER);
        for (word, _) in &rows {
            regenerated.push('\n');
            regenerated.push_str(word);
            regenerated.push(',');
            regenerated.push_str(&normalizer.normalize(word, false));
        }
        regenerated.push('\n');
        let path = fixture_path();
        std::fs::write(&path, regenerated)
            .unwrap_or_else(|e| panic!("write {}: {e}", path.display()));
        eprintln!(
            "UPDATE_BASELINE=1: regenerated {} readings in {}",
            rows.len(),
            path.display()
        );
        return;
    }

    for (word, recorded) in rows {
        assert_eq!(
            normalizer.normalize(&word, false),
            recorded,
            "baseline mismatch for {word}"
        );
    }
}
