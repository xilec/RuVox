//! Shared helpers for the examples: one source of truth for locating the
//! exported model bundle.

use std::path::PathBuf;

/// Bundle dir: first CLI arg, else `SILERO_NATIVE_BUNDLE`, else the dev
/// default `<repo>/tmp/bundle-v5`.
pub fn bundle_dir_arg() -> PathBuf {
    std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            std::env::var("SILERO_NATIVE_BUNDLE")
                .map(PathBuf::from)
                .unwrap_or_else(|_| {
                    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../tmp/bundle-v5")
                })
        })
}
