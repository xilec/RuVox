//! Shared helpers for integration tests under `src-tauri/tests/`.
//!
//! This is a plain module (`tests/common/mod.rs`), not a standalone
//! `tests/common.rs` file, so cargo does not compile it as its own test
//! binary -- it is only included via `mod common;` in the crates that need it.

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
