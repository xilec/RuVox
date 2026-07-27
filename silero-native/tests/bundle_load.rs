//! Integration test: load the real exported bundle (tmp/bundle-v5 by default,
//! override with SILERO_NATIVE_BUNDLE). Skipped silently when the bundle is
//! absent — CI machines without the ~230 MB bundle still run the unit tests.

use std::path::PathBuf;

use silero_native::bundle::{Manifest, Sessions};

fn bundle_dir() -> Option<PathBuf> {
    if let Ok(dir) = std::env::var("SILERO_NATIVE_BUNDLE") {
        return Some(PathBuf::from(dir));
    }
    // Tests run with CWD = crate root; the dev bundle lives in <repo>/tmp.
    Some(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../tmp/bundle-v5"))
}

#[test]
fn valid_bundle_loads_all_sessions() {
    let dir = match bundle_dir() {
        Some(d) if d.join("manifest.json").exists() => d,
        _ => {
            eprintln!("bundle not found, skipping (set SILERO_NATIVE_BUNDLE)");
            return;
        }
    };
    let manifest = Manifest::load(&dir).expect("manifest must parse");
    manifest.verify(&dir).expect("bundle files must match sha256");
    Sessions::open(&dir, &manifest).expect("all six sessions must open");
}
