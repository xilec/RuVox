//! Shared helpers for the bundle-gated integration tests: one source of
//! truth for locating the exported model bundle.

use std::path::PathBuf;

/// Bundle directory: `SILERO_NATIVE_BUNDLE` env override, else the dev
/// default `<repo>/tmp/bundle-v5` (tests run with CWD = crate root).
pub fn bundle_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("SILERO_NATIVE_BUNDLE") {
        return PathBuf::from(dir);
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../tmp/bundle-v5")
}

/// Bundle dir when a manifest is present; `None` otherwise — the caller
/// skips silently so CI machines without the ~230 MB bundle still run the
/// unit tests.
pub fn gated_bundle_dir() -> Option<PathBuf> {
    let dir = bundle_dir();
    if dir.join("manifest.json").exists() {
        Some(dir)
    } else {
        eprintln!("bundle not found, skipping (set SILERO_NATIVE_BUNDLE)");
        None
    }
}
