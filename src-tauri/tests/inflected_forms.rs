//! Curated readings for inflected and derived forms.
//!
//! `tests/fixtures/transliteration/inflected_forms.csv` lists forms whose readings
//! were checked by hand. Unlike `dictionary_baseline.rs`, this fixture is **not** a
//! snapshot: auto-regenerating it would bake in the composition artifacts
//! (`строкас`, `функцияс`, `кейсс`, `подс`, ...) that this coverage exists to catch.
//! There is deliberately no `UPDATE_BASELINE` mode here; when a row fails, decide by
//! ear whether the composition logic or the expected reading is wrong and edit the
//! row explicitly.
//!
//! Scope: forms built from curated stems — plural `-s`/`-es`, `-ed`, `-ing`, `-er`,
//! `-able`, plus the silent-e / `-ies` stems restored by the composition step.
//! Semantic-base plurals (`strings`, `functions`, `arrays`, `objects`, `methods`,
//! `classes`, `indexes`, `services`, `cases`, `packages`, `modules`) are intentionally
//! absent: their composition still produces artifacts and is revisited in #292.

mod common;

use ruvox_tauri_lib::pipeline::normalizers::english::EnglishNormalizer;

const FIXTURE: &str = "tests/fixtures/transliteration/inflected_forms.csv";
const HEADER: &str = "word,expected";

fn fixture() -> Vec<(String, String)> {
    let path = common::resolve_test_path(FIXTURE);
    let content =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    let mut lines = content.lines();
    assert_eq!(lines.next(), Some(HEADER), "{FIXTURE}: unexpected header");
    lines
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            let (word, expected) = line
                .split_once(',')
                .unwrap_or_else(|| panic!("{FIXTURE}: malformed row {line:?}"));
            (word.to_owned(), expected.to_owned())
        })
        .collect()
}

#[test]
fn inflected_forms() {
    let rows = fixture();
    // A header-only fixture would make this test vacuous.
    assert!(!rows.is_empty(), "{FIXTURE}: no rows");
    let mut normalizer = EnglishNormalizer::default();
    for (word, expected) in rows {
        assert_eq!(
            normalizer.normalize(&word, false),
            expected,
            "curated reading changed for {word}"
        );
    }
}
