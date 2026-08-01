//! Integration test: load the real exported bundle (tmp/bundle-v5 by default,
//! override with SILERO_NATIVE_BUNDLE). Skipped silently when the bundle is
//! absent — CI machines without the ~230 MB bundle still run the unit tests.

use silero_native::bundle::{Manifest, Sessions};

mod common;

#[test]
fn valid_bundle_loads_all_sessions() {
    let Some(dir) = common::gated_bundle_dir() else {
        return;
    };
    let manifest = Manifest::load(&dir).expect("manifest must parse");
    manifest
        .verify(&dir)
        .expect("bundle files must match sha256");
    Sessions::open(&dir, &manifest).expect("all six sessions must open");
}
