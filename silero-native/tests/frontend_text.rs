//! Golden test: the Rust `prepare_text_input` port must reproduce the
//! upstream Python frontend output symbol-for-symbol.
//!
//! Fixtures come from `tests/tools/gen_frontend_fixtures.py` (real v5_ru
//! model) — regenerate them (via `tests/tools/regenerate_fixtures.sh`) if the
//! upstream reference changes.

use std::collections::{HashMap, HashSet};

use serde::Deserialize;
use silero_native::frontend::text::{build_sequence, prepare_text_input};

#[derive(Deserialize)]
struct FixtureFile {
    symbols: String,
    sos_token: String,
    eos_token: String,
    symbol_to_id: HashMap<String, i64>,
    cases: Vec<FixtureCase>,
}

#[derive(Deserialize)]
struct FixtureCase {
    input: String,
    sentence: String,
    clean_sentence: String,
    has_text: bool,
    sequence: Vec<i64>,
}

#[test]
fn prepare_text_input_matches_upstream_golden() {
    let raw = include_str!("fixtures/frontend/prepare_text_input.json");
    let fixtures: FixtureFile = serde_json::from_str(raw).expect("fixture JSON must parse");
    let symbols_tail: HashSet<char> = fixtures.symbols.chars().skip(3).collect();

    for case in &fixtures.cases {
        let out = prepare_text_input(&case.input, &symbols_tail);
        assert_eq!(
            out.sentence, case.sentence,
            "sentence mismatch for input {:?}",
            case.input
        );
        assert_eq!(
            out.clean_sentence, case.clean_sentence,
            "clean_sentence mismatch for input {:?}",
            case.input
        );
        assert_eq!(
            out.has_text, case.has_text,
            "has_text mismatch for input {:?}",
            case.input
        );
        let sequence = build_sequence(
            &out.sentence,
            &fixtures.symbol_to_id,
            &fixtures.sos_token,
            &fixtures.eos_token,
        )
        .expect("sequence must build");
        assert_eq!(
            sequence.ids, case.sequence,
            "sequence mismatch for input {:?}",
            case.input
        );
    }
}
