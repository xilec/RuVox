//! Profile classification of the golden corpus.
//!
//! `RUVOX_GOLDEN_PROFILES` decides which text profiles the corpus runs under.
//! This test reports, for every fixture, whether its output is the same under
//! every profile (profile-invariant, stays in `tests/fixtures/pipeline/`) or
//! differs (profile-specific, belongs in `tests/fixtures/pipeline/<profile>/`
//! with its own expected files).
//!
//! Run the classifier with:
//!   cargo test --manifest-path src-tauri/Cargo.toml --test golden_profiles -- --ignored --nocapture

use ruvox_tauri_lib::pipeline::TTSPipeline;
use ruvox_tauri_lib::tts::profiles::NATIVE_EN_PROFILE;

mod common;

/// One profile's result for a case: its name, text and codepoint mapping.
type ProfileOutput = (&'static str, String, Vec<(usize, usize)>);

#[test]
#[ignore = "classifier: prints the move list instead of asserting"]
fn classify_corpus_by_profile() {
    let cases = common::collect_case_names();
    let profiles = common::all_profiles();
    let mut invariant = Vec::new();
    let mut specific: Vec<(String, Vec<&str>)> = Vec::new();

    for case in &cases {
        let input = common::read_fixture_text(case, "input.txt");
        let dict = common::read_dict_sidecar(case);
        let mode = common::read_mode_sidecar(case);

        let mut outputs: Vec<ProfileOutput> = Vec::new();
        for (name, profile) in &profiles {
            let mut pipeline = TTSPipeline::new();
            if let Some(mode) = mode {
                pipeline.set_code_block_mode(mode);
            }
            pipeline.set_user_dictionary(dict.clone());
            let out = pipeline.process_for_engine(&input, profile);
            outputs.push((name, out.text, out.mapping.char_map));
        }

        let first = &outputs[0];
        if outputs
            .iter()
            .all(|(_, text, map)| *text == first.1 && *map == first.2)
        {
            invariant.push(case.clone());
        } else {
            let differs: Vec<&str> = outputs
                .iter()
                .filter(|(_, text, map)| *text != first.1 || *map != first.2)
                .map(|(name, _, _)| *name)
                .collect();
            specific.push((case.clone(), differs));
        }
    }

    println!("invariant: {}", invariant.len());
    for case in &invariant {
        println!("  keep {case}");
    }
    println!("profile-specific: {}", specific.len());
    for (case, differs) in &specific {
        println!("  move {case} -> {}", differs.join(","));
    }
    assert_eq!(invariant.len() + specific.len(), cases.len());
}

/// Authoring helper for the `native_en/` fixture: rewrites its expected text
/// and char map from the current pipeline, then the case is reviewed by hand.
///
///   cargo test --test golden_profiles -- --ignored write_native_en_case
#[test]
#[ignore = "fixture authoring helper"]
fn write_native_en_case() {
    let dir = common::fixtures_dir().join("native_en");
    let input = common::read_fixture_text("native_en/native_blocks", "input.txt");
    let mut pipeline = TTSPipeline::new();
    let out = pipeline.process_for_engine(&input, &NATIVE_EN_PROFILE);

    let mapping = serde_json::json!({
        "original": out.mapping.original,
        "transformed": out.mapping.transformed,
        "char_map": out.mapping.char_map,
    });
    let expected_text = format!("{}\n", out.text);
    let char_map = format!(
        "{}\n",
        serde_json::to_string_pretty(&mapping).expect("serialize mapping")
    );
    std::fs::write(dir.join("native_blocks.expected.txt"), expected_text)
        .expect("write expected text");
    std::fs::write(dir.join("native_blocks.char_map.json"), char_map).expect("write char map");
}
