//! Span invariants over the golden corpus.
//!
//! The `position-mapping` spec requires, for every profile: spans sorted and
//! non-overlapping, non-empty, inside the normalized text, carrying a language
//! exactly when the engine reads them, and no markup leaking into the text.
//!
//! Shared fixtures run under every harness profile; a fixture inside
//! `tests/fixtures/pipeline/<profile>/` runs under that profile only.
//!
//! Run with:
//!   cargo test --manifest-path src-tauri/Cargo.toml --test span_invariants

use ruvox_tauri_lib::pipeline::{Span, SpanKind, TTSPipeline};

mod common;

/// `(case path, profiles it must hold under)`.
fn cases_with_profiles() -> Vec<(String, Vec<&'static str>)> {
    let selected = common::selected_profile_names();
    let mut cases: Vec<(String, Vec<&'static str>)> = common::collect_case_names()
        .into_iter()
        .map(|path| (path, selected.clone()))
        .collect();
    cases.extend(
        common::collect_profile_cases()
            .into_iter()
            .map(|(profile, path)| (path, vec![profile])),
    );
    cases
}

#[test]
fn span_invariants_hold_for_every_fixture_under_its_profiles() {
    let cases = cases_with_profiles();
    assert!(!cases.is_empty(), "no golden fixtures found");

    let mut checked = 0;
    for (case, profiles) in &cases {
        let input = common::read_fixture_text(case, "input.txt");
        let dict = common::read_dict_sidecar(case);
        let mode = common::read_mode_sidecar(case);

        for profile_name in profiles {
            let mut pipeline = TTSPipeline::new();
            if let Some(mode) = mode {
                pipeline.set_code_block_mode(mode);
            }
            pipeline.set_user_dictionary(dict.clone());
            let out = pipeline.process_for_engine(&input, common::profile_by_name(profile_name));
            let label = format!("{case}/{profile_name}");
            assert_span_invariants(&out.text, &out.spans, &out.mapping.char_map, &label);
            checked += 1;
        }
    }
    assert!(checked >= cases.len());
}

fn assert_span_invariants(text: &str, spans: &[Span], char_map: &[(usize, usize)], label: &str) {
    let len = text.chars().count();
    assert_eq!(
        char_map.len(),
        len,
        "{label}: char_map must stay one entry per output codepoint"
    );

    let mut previous_end = 0;
    for span in spans {
        assert!(
            span.start < span.end,
            "{label}: empty span reported: {span:?}"
        );
        assert!(
            span.end <= len,
            "{label}: span past the normalized text: {span:?} (len {len})"
        );
        assert!(
            span.start >= previous_end,
            "{label}: spans must be sorted and non-overlapping: {span:?}"
        );
        previous_end = span.end;

        match span.kind {
            SpanKind::CodeIdentifier | SpanKind::ProseLatin | SpanKind::CodeBlock => assert_eq!(
                span.lang,
                Some("en"),
                "{label}: a native span carries the language the engine reads it in"
            ),
            SpanKind::DictionaryOverride => assert_eq!(
                span.lang, None,
                "{label}: the pipeline rewrote the fragment, so it has no language"
            ),
        }
    }

    // Markup is the adapter's business: nothing tagged may reach the text.
    for marker in ["<ru>", "</ru>", "<lang="] {
        assert!(
            !text.contains(marker),
            "{label}: pipeline emitted markup {marker:?} into the synthesis text"
        );
    }
}
