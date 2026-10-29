//! Span invariants over the golden corpus.
//!
//! The `position-mapping` spec requires, for every profile: spans sorted and
//! non-overlapping, non-empty, inside the normalized text, carrying a language
//! exactly when the engine reads them, and no markup leaking into the text.
//!
//! Run with:
//!   cargo test --manifest-path src-tauri/Cargo.toml --test span_invariants

use ruvox_tauri_lib::pipeline::{SpanKind, TTSPipeline};
use ruvox_tauri_lib::tts::profiles::{NATIVE_EN_PROFILE, SILERO_PROFILE};

mod common;

/// Every profile the pipeline is expected to handle: the reference
/// Russian-only engine and the most permissive one.
const PROFILE_NAMES: [&str; 2] = ["silero", "native_en"];

#[test]
fn span_invariants_hold_for_every_fixture_under_every_profile() {
    let cases = common::collect_case_names();
    assert!(!cases.is_empty(), "no golden fixtures found");

    let mut checked = 0;
    for case in &cases {
        let input = common::read_fixture_text(case, "input.txt");
        let dict = common::read_dict_sidecar(case);
        let mode = common::read_mode_sidecar(case);

        for profile in PROFILE_NAMES {
            let mut pipeline = TTSPipeline::new();
            if let Some(mode) = mode {
                pipeline.set_code_block_mode(mode);
            }
            pipeline.set_user_dictionary(dict.clone());
            let out = match profile {
                "silero" => pipeline.process_for_engine(&input, &SILERO_PROFILE),
                _ => pipeline.process_for_engine(&input, &NATIVE_EN_PROFILE),
            };
            let label = format!("{case}/{profile}");
            assert_span_invariants(&out.text, &out.spans, &out.mapping.char_map, &label);
            checked += 1;
        }
    }
    assert_eq!(checked, cases.len() * PROFILE_NAMES.len());
}

fn assert_span_invariants(
    text: &str,
    spans: &[ruvox_tauri_lib::pipeline::Span],
    char_map: &[(usize, usize)],
    label: &str,
) {
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
            SpanKind::CodeIdentifier | SpanKind::ProseLatin => assert_eq!(
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
