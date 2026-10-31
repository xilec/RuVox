//! Profile classification of the golden corpus.
//!
//! `RUVOX_GOLDEN_PROFILES` decides which text profiles the corpus runs under;
//! without it the flat corpus runs under its classified set
//! ([`common::flat_corpus_profile_names`]). This test reports, for every
//! fixture, whether its output is the same under every selected profile
//! (profile-invariant, stays in `tests/fixtures/pipeline/`) or differs
//! (profile-specific, belongs in `tests/fixtures/pipeline/<profile>/` with its
//! own expected files).
//!
//! Run the classifier with:
//!   cargo test --manifest-path src-tauri/Cargo.toml --test golden_profiles -- --ignored --nocapture

use ruvox_tauri_lib::pipeline::TTSPipeline;

mod common;

/// One profile's result for a case: its name, text and codepoint mapping.
type ProfileOutput = (&'static str, String, Vec<(usize, usize)>);

#[test]
#[ignore = "classifier: prints the move list instead of asserting"]
fn classify_corpus_by_profile() {
    let cases = common::collect_case_names();
    let profiles: Vec<(&'static str, &tts_contract::TextProfile)> =
        common::selected_profile_names()
            .into_iter()
            .map(|name| (name, common::profile_by_name(name)))
            .collect();
    let mut invariant = Vec::new();
    let mut specific: Vec<(String, Vec<&str>)> = Vec::new();

    for case in &cases {
        let input = common::read_fixture_text(case, "input.txt");
        let dict = common::read_dict_sidecar(case);
        let mode = common::read_mode_sidecar(case);
        let english = common::read_english_reading_sidecar(case);

        let mut outputs: Vec<ProfileOutput> = Vec::new();
        for (name, profile) in &profiles {
            let mut pipeline = TTSPipeline::new();
            if let Some(mode) = mode {
                pipeline.set_code_block_mode(mode);
            }
            if let Some(english) = english {
                pipeline.set_english_reading(english);
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

/// Authoring helper for the per-profile fixtures: rewrites the expected text
/// and char map of one case from the current pipeline, then the case is
/// reviewed by hand.
///
///   RUVOX_FIXTURE_CASE=native_en/dictionary_stress \
///     cargo test --test golden_profiles -- --ignored write_fixture_case
///
/// Without the variable it defaults to `native_en/native_blocks`.
#[test]
#[ignore = "fixture authoring helper"]
fn write_fixture_case() {
    let case = std::env::var("RUVOX_FIXTURE_CASE")
        .unwrap_or_else(|_| "native_en/native_blocks".to_string());
    let (profile_name, case_name) = case
        .split_once('/')
        .unwrap_or_else(|| panic!("expected <profile>/<case>, got {case:?}"));
    let input = common::read_fixture_text(&format!("{profile_name}/{case_name}"), "input.txt");
    let mut pipeline = TTSPipeline::new();
    pipeline.set_user_dictionary(common::read_dict_sidecar(&format!(
        "{profile_name}/{case_name}"
    )));
    if let Some(mode) = common::read_mode_sidecar(&format!("{profile_name}/{case_name}")) {
        pipeline.set_code_block_mode(mode);
    }
    if let Some(reading) =
        common::read_english_reading_sidecar(&format!("{profile_name}/{case_name}"))
    {
        pipeline.set_english_reading(reading);
    }
    let out = pipeline.process_for_engine(&input, common::profile_by_name(profile_name));

    let dir = common::fixtures_dir().join(profile_name);
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
    std::fs::write(dir.join(format!("{case_name}.expected.txt")), expected_text)
        .expect("write expected text");
    std::fs::write(dir.join(format!("{case_name}.char_map.json")), char_map)
        .expect("write char map");
}
