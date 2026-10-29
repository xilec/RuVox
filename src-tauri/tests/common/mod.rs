//! Shared helpers for integration tests under `src-tauri/tests/`.
//!
//! This is a plain module (`tests/common/mod.rs`), not a standalone
//! `tests/common.rs` file, so cargo does not compile it as its own test
//! binary -- it is only included via `mod common;` in the crates that need it.

// Each test binary includes this module and uses a subset of it; without the
// allow, `-D warnings` turns the unused half into build failures.
#![allow(dead_code)]

use std::path::PathBuf;

/// Resolve a path relative to the `src-tauri` crate root, tolerating both
/// working directories integration tests may be invoked from: `src-tauri/`
/// (the manifest dir, the default for `cargo test --manifest-path
/// src-tauri/Cargo.toml`) or the workspace root.
pub fn resolve_test_path(rel: &str) -> PathBuf {
    let from_crate = PathBuf::from(rel);
    if from_crate.exists() {
        from_crate
    } else {
        PathBuf::from("src-tauri").join(rel)
    }
}

// ── Golden fixture access ─────────────────────────────────────────────────────

use std::fs;

use ruvox_tauri_lib::dictionary::{UserDictionary, parse_import, validate_entry};
use ruvox_tauri_lib::pipeline::normalizers::code_blocks::CodeBlockMode;

/// Directory holding the pipeline golden corpus.
pub fn fixtures_dir() -> PathBuf {
    resolve_test_path("tests/fixtures/pipeline")
}

/// Sorted case names derived from `*.input.txt` fixtures.
pub fn collect_case_names() -> Vec<String> {
    let dir = fixtures_dir();
    let mut names: Vec<String> = fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("cannot read fixtures dir {}: {}", dir.display(), e))
        .filter_map(|entry| {
            let entry = entry.expect("dir entry error");
            let name = entry.file_name();
            let s = name.to_string_lossy();
            s.ends_with(".input.txt")
                .then(|| s.trim_end_matches(".input.txt").to_string())
        })
        .collect();
    names.sort();
    names
}

/// Reads `<case>.<ext>` with the fixture convention of one trailing newline.
pub fn read_fixture_text(case: &str, ext: &str) -> String {
    let path = fixtures_dir().join(format!("{}.{}", case, ext));
    fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("cannot read fixture {}: {}", path.display(), e))
        .trim_end_matches('\n')
        .to_string()
}

/// Optional `<case>.mode.txt` sidecar pinning the code block narration mode.
pub fn read_mode_sidecar(case: &str) -> Option<CodeBlockMode> {
    let path = fixtures_dir().join(format!("{}.mode.txt", case));
    let text = fs::read_to_string(&path).ok()?;
    match text.trim() {
        "brief" => Some(CodeBlockMode::Brief),
        "read" => Some(CodeBlockMode::Full),
        other => panic!("invalid mode sidecar {}: {:?}", path.display(), other),
    }
}

/// Optional `<case>.dict.toml` sidecar loading user-dictionary entries.
pub fn read_dict_sidecar(case: &str) -> UserDictionary {
    let path = fixtures_dir().join(format!("{}.dict.toml", case));
    let text = match fs::read_to_string(&path) {
        Ok(text) => text,
        Err(_) => return UserDictionary::default(),
    };
    let mut dict = UserDictionary::default();
    for entry in parse_import(&text).expect("valid dict sidecar TOML") {
        validate_entry(&entry.from, &entry.to).expect("valid dict sidecar entry");
        dict.insert(entry);
    }
    dict
}

/// Every profile the golden harness knows, keyed by the name used both in
/// `RUVOX_GOLDEN_PROFILES` and in `tests/fixtures/pipeline/<profile>/`.
pub fn all_profiles() -> Vec<(&'static str, &'static tts_contract::TextProfile)> {
    vec![
        ("silero", &ruvox_tauri_lib::tts::profiles::SILERO_PROFILE),
        (
            "native_en",
            &ruvox_tauri_lib::tts::profiles::NATIVE_EN_PROFILE,
        ),
    ]
}

/// Profile names the profile-invariant fixtures run under:
/// `RUVOX_GOLDEN_PROFILES` (comma-separated) or every known profile.
pub fn selected_profile_names() -> Vec<&'static str> {
    let known = all_profiles();
    let Ok(raw) = std::env::var("RUVOX_GOLDEN_PROFILES") else {
        return known.into_iter().map(|(name, _)| name).collect();
    };
    let mut names = Vec::new();
    for name in raw.split(',').map(str::trim).filter(|n| !n.is_empty()) {
        let Some((known_name, _)) = known.iter().find(|(known, _)| *known == name) else {
            let available: Vec<&str> = known.iter().map(|(known, _)| *known).collect();
            panic!("unknown golden profile {name:?}; available: {available:?}");
        };
        names.push(*known_name);
    }
    names
}

/// The profile behind a harness name.
pub fn profile_by_name(name: &str) -> &'static tts_contract::TextProfile {
    let known = all_profiles();
    match known.iter().find(|(known, _)| *known == name) {
        Some((_, profile)) => profile,
        None => panic!("unknown golden profile {name:?}"),
    }
}

/// Cases pinned to one profile: `tests/fixtures/pipeline/<profile>/<case>.*`.
///
/// Returns `(profile, case path relative to the fixtures dir)`.
pub fn collect_profile_cases() -> Vec<(String, String)> {
    let dir = fixtures_dir();
    let known: Vec<&str> = all_profiles().into_iter().map(|(name, _)| name).collect();
    let mut cases = Vec::new();
    let entries = match fs::read_dir(&dir) {
        Ok(entries) => entries,
        Err(_) => return cases,
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if !known.contains(&name.as_str()) || !entry.path().is_dir() {
            continue;
        }
        let nested = match fs::read_dir(entry.path()) {
            Ok(nested) => nested,
            Err(_) => continue,
        };
        let mut names: Vec<String> = nested
            .flatten()
            .filter_map(|nested_entry| {
                let file = nested_entry.file_name().to_string_lossy().to_string();
                file.ends_with(".input.txt")
                    .then(|| format!("{name}/{}", file.trim_end_matches(".input.txt")))
            })
            .collect();
        names.sort();
        cases.extend(names.into_iter().map(|case| (name.clone(), case)));
    }
    cases.sort();
    cases
}
