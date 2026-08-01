//! Golden parity test: the Rust accentor + homosolver port must reproduce
//! the upstream torch pipeline output symbol-for-symbol on real homographs,
//! ё-words, explicit stress markers and dash tokens.
//!
//! Fixtures come from `tests/tools/gen_accentor_fixtures.py` (real v5_ru
//! model, references taken after `unpack_q_model` — regenerate via
//! `tests/tools/regenerate_fixtures.sh`).
//! Skipped when the bundle is absent (set SILERO_NATIVE_BUNDLE to override).

use serde::Deserialize;
use silero_native::frontend::accentor::Accentor;
use silero_native::frontend::homosolver::HomoSolver;
use silero_native::frontend::text::{build_sequence, prepare_text_input};
use silero_native::frontend::FrontendConfig;

mod common;

#[derive(Deserialize)]
struct FixtureFile {
    cases: Vec<FixtureCase>,
}

#[derive(Deserialize)]
struct FixtureCase {
    input: String,
    prepared: String,
    homosolved: String,
    accented: String,
    sequence: Vec<i64>,
}

#[test]
fn accentor_pipeline_matches_upstream_golden() {
    let Some(dir) = common::gated_bundle_dir() else {
        return;
    };
    let config = FrontendConfig::load(&dir).expect("frontend.json must parse");
    let open = |name: &str| {
        ort::session::Session::builder()
            .and_then(|mut b| b.commit_from_file(dir.join(name)))
            .expect("session must open")
    };
    let homosolver = HomoSolver::load(&dir, &config.homosolver, open("homosolver.onnx"))
        .expect("homosolver must load");
    let accentor = Accentor::load(&dir, &config.accentor, open("accentor_tensor.onnx"))
        .expect("accentor must load");

    let raw = include_str!("fixtures/frontend/accentor.json");
    let fixtures: FixtureFile = serde_json::from_str(raw).expect("fixture JSON must parse");
    let symbols_tail = config.symbols_tail();
    let empty: std::collections::HashSet<String> = Default::default();

    for case in &fixtures.cases {
        let prepared = prepare_text_input(&case.input, &symbols_tail);
        assert_eq!(
            prepared.sentence, case.prepared,
            "prepared mismatch for {:?}",
            case.input
        );
        let homosolved = homosolver
            .resolve(&prepared.sentence, true, true, true)
            .expect("homosolver must run");
        assert_eq!(
            homosolved, case.homosolved,
            "homosolved mismatch for {:?}",
            case.input
        );
        let accented = accentor
            .accentuate(&homosolved, true, true, true, &empty, &empty)
            .expect("accentor must run");
        assert_eq!(
            accented, case.accented,
            "accented mismatch for {:?}",
            case.input
        );
        let sequence = build_sequence(
            &accented,
            &config.symbol_to_id,
            &config.sos_token,
            &config.eos_token,
        )
        .expect("sequence must build");
        assert_eq!(
            sequence, case.sequence,
            "sequence mismatch for {:?}",
            case.input
        );
    }
}
