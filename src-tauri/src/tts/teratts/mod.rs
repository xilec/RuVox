//! TeraTTSv2 engine: in-process ONNX inference through the `teratts` crate.

use std::path::Path;

use tracing::debug;

pub mod download;
pub mod engine;

pub use engine::TeraTtsEngine;

/// Voice ids shipped by the installed bundle, in manifest order.
///
/// Feeds the Settings voice selector. A missing, unreadable or incompatible
/// manifest yields an empty list instead of an error: the availability probe
/// already carries the coded reason the engine is unusable, and a catalogue
/// read must never turn into a Settings load failure.
pub fn voices(bundle_dir: &Path) -> Vec<String> {
    match teratts::bundle::Manifest::load(bundle_dir) {
        Ok(manifest) => manifest.voices.into_iter().map(|voice| voice.id).collect(),
        Err(error) => {
            debug!(%error, "no TeraTTS voice catalogue");
            Vec::new()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::write_teratts_manifest;

    #[test]
    fn reports_manifest_voices_in_order() {
        let dir = tempfile::TempDir::new().expect("tempdir");
        write_teratts_manifest(
            dir.path(),
            teratts::bundle::FORMAT_VERSION,
            &[
                ("ru_m5", "ru"),
                ("ru_f1", "ru"),
                ("eng_f3", "en"),
                ("eng_f5", "en"),
            ],
            &[],
        );

        assert_eq!(
            voices(dir.path()),
            vec!["ru_m5", "ru_f1", "eng_f3", "eng_f5"]
        );
    }

    #[test]
    fn missing_bundle_yields_an_empty_catalogue() {
        let dir = tempfile::TempDir::new().expect("tempdir");

        assert!(voices(dir.path()).is_empty());
    }

    #[test]
    fn incompatible_manifest_yields_an_empty_catalogue() {
        let dir = tempfile::TempDir::new().expect("tempdir");
        write_teratts_manifest(
            dir.path(),
            teratts::bundle::FORMAT_VERSION + 1,
            &[("ru_m5", "ru")],
            &[],
        );

        assert!(voices(dir.path()).is_empty());
    }
}
