/// Integration test harness for TTSPipeline golden fixtures.
///
/// Reads every `*.input.txt` in `tests/fixtures/pipeline/`, runs
/// `TTSPipeline::process_for_engine` with the fixture's profile (shared cases run
/// under every profile in `RUVOX_GOLDEN_PROFILES`, `<profile>/` folders pin one),
/// and compares both the normalised text and the character mapping against the
/// corresponding `*.expected.txt` and `*.char_map.json` fixture files.
///
/// Run with:
///   cargo test --manifest-path src-tauri/Cargo.toml --test golden
use ruvox_tauri_lib::pipeline::TTSPipeline;
use serde::Deserialize;
use similar::{ChangeTag, TextDiff};
use std::fs;

mod common;

// ── JSON fixture schema ────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct CharMapFixture {
    original: String,
    transformed: String,
    char_map: Vec<[usize; 2]>,
}

// ── Fixture access ─────────────────────────────────────────────────────────────

fn read_char_map_fixture(case: &str) -> CharMapFixture {
    let path = common::fixtures_dir().join(format!("{}.char_map.json", case));
    let json = fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("cannot read char_map fixture {}: {}", path.display(), e));
    serde_json::from_str(&json)
        .unwrap_or_else(|e| panic!("cannot parse char_map fixture {}: {}", path.display(), e))
}

// ── Diff helpers ───────────────────────────────────────────────────────────────

/// Produce a unified diff string between `expected` and `actual` text.
fn text_diff(expected: &str, actual: &str) -> String {
    let diff = TextDiff::from_lines(expected, actual);
    let mut output = String::new();
    for change in diff.iter_all_changes() {
        let prefix = match change.tag() {
            ChangeTag::Delete => "-",
            ChangeTag::Insert => "+",
            ChangeTag::Equal => " ",
        };
        output.push_str(&format!("{}{}", prefix, change));
    }
    output
}

/// Show the first `limit` mismatched char_map entries with context.
fn char_map_diff(expected: &[[usize; 2]], actual: &[(usize, usize)], limit: usize) -> String {
    let mut output = String::new();
    let max_len = expected.len().max(actual.len());
    let mut shown = 0;

    for i in 0..max_len {
        let exp = expected.get(i).copied();
        let act = actual.get(i).copied();
        let exp_pair = exp.map(|a| (a[0], a[1]));
        if exp_pair != act {
            output.push_str(&format!(
                "  char_map[{}]: expected {:?}, got {:?}\n",
                i,
                exp.map(|a| [a[0], a[1]]),
                act
            ));
            shown += 1;
            if shown >= limit {
                let remaining = max_len - i - 1;
                if remaining > 0 {
                    output.push_str(&format!("  ... and {} more mismatches\n", remaining));
                }
                break;
            }
        }
    }

    if expected.len() != actual.len() {
        output.push_str(&format!(
            "  char_map length: expected {}, got {}\n",
            expected.len(),
            actual.len()
        ));
    }

    output
}

// ── Main harness ───────────────────────────────────────────────────────────────

/// One run of the corpus: which profile, and which fixture case.
///
/// A case under `tests/fixtures/pipeline/<profile>/` runs only under that
/// profile; a case at the top level is profile-invariant and therefore runs
/// under every profile in [`common::selected_profile_names`].
struct GoldenCase {
    path: String,
    profile: String,
}

fn collect_cases() -> Vec<GoldenCase> {
    let mut cases = Vec::new();
    for profile in common::selected_profile_names() {
        for path in common::collect_case_names() {
            cases.push(GoldenCase {
                path,
                profile: profile.to_string(),
            });
        }
    }
    for (profile, path) in common::collect_profile_cases() {
        cases.push(GoldenCase {
            path,
            profile: profile.to_string(),
        });
    }
    cases
}

#[test]
fn golden_fixtures() {
    let cases = collect_cases();
    assert!(
        !cases.is_empty(),
        "no fixture cases found in {}",
        common::fixtures_dir().display()
    );

    let mut failures: Vec<String> = Vec::new();

    for case in &cases {
        let case_name = &case.path;
        let profile = common::profile_by_name(&case.profile);
        let input = common::read_fixture_text(case_name, "input.txt");
        let expected_text = common::read_fixture_text(case_name, "expected.txt");
        let expected_map = read_char_map_fixture(case_name);

        let mut pipeline = TTSPipeline::new();
        if let Some(mode) = common::read_mode_sidecar(case_name) {
            pipeline.set_code_block_mode(mode);
        }
        pipeline.set_user_dictionary(common::read_dict_sidecar(case_name));
        let processed = pipeline.process_for_engine(&input, profile);
        let actual_text = processed.text;
        let actual_mapping = processed.mapping;

        let mut case_errors: Vec<String> = Vec::new();

        // ── Text comparison ──────────────────────────────────────────────────
        if actual_text != expected_text {
            let diff = text_diff(&expected_text, &actual_text);
            case_errors.push(format!(
                "text mismatch:\n--- expected\n+++ actual\n{}",
                diff
            ));
        }

        // ── original field comparison ────────────────────────────────────────
        // The fixture stores the original input; verify consistency.
        if actual_mapping.original != expected_map.original {
            case_errors.push(format!(
                "mapping.original mismatch:\n  expected: {:?}\n  got:      {:?}",
                expected_map.original, actual_mapping.original
            ));
        }

        // ── transformed field comparison ─────────────────────────────────────
        if actual_mapping.transformed != expected_map.transformed {
            let diff = text_diff(&expected_map.transformed, &actual_mapping.transformed);
            case_errors.push(format!(
                "mapping.transformed mismatch:\n--- expected\n+++ actual\n{}",
                diff
            ));
        }

        // ── char_map comparison ──────────────────────────────────────────────
        let expected_pairs: Vec<[usize; 2]> = expected_map.char_map.clone();
        let actual_pairs: &[(usize, usize)] = &actual_mapping.char_map;

        let maps_match = expected_pairs.len() == actual_pairs.len()
            && expected_pairs
                .iter()
                .zip(actual_pairs.iter())
                .all(|(e, a)| e[0] == a.0 && e[1] == a.1);

        if !maps_match {
            let diff = char_map_diff(&expected_pairs, actual_pairs, 10);
            case_errors.push(format!("char_map mismatch:\n{}", diff));
        }

        if !case_errors.is_empty() {
            failures.push(format!(
                "\n=== FAIL: {} under {} ===\n{}",
                case_name,
                case.profile,
                case_errors.join("\n")
            ));
        }
    }

    let total = cases.len();
    let failed = failures.len();
    let passed = total - failed;

    if !failures.is_empty() {
        panic!(
            "{}/{} golden cases FAILED:\n{}",
            failed,
            total,
            failures.join("\n")
        );
    }

    // Emit pass count for visibility in test output.
    eprintln!("golden_fixtures: {}/{} passed", passed, total);
}
