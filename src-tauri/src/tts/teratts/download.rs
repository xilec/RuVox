//! On-demand download of the TeraTTSv2 model bundle.
//!
//! Thin wrapper over the shared [`crate::tts::bundle_download`]: it supplies
//! the release URL and the manifest parser. The TeraTTS manifest is already
//! the generic shape (bundle path + flat release asset + size + sha256), so
//! the mapping is a field copy; the format version is checked here before any
//! file is fetched, so a bundle written for a newer app cannot half-install.

use std::path::Path;

use crate::tts::TtsError;
use crate::tts::bundle_download::{self, BundleFile};
use crate::tts::supervisor::Emitter;

/// Base URL of the GitHub Release that hosts the bundle files. The release may
/// not exist yet — a 404 surfaces as a typed `bundle_download_failed` error
/// (and a `bundle_download_finished { ok: false }` event), never a panic or a
/// half-installed bundle.
pub const BUNDLE_RELEASE_BASE_URL: &str =
    "https://github.com/xilec/RuVox/releases/download/teratts-models-v1";

/// Download and verify the whole bundle into `bundle_dir`.
pub async fn download_bundle(bundle_dir: &Path, emitter: &Emitter) -> Result<(), TtsError> {
    bundle_download::download(
        bundle_dir,
        "teratts",
        BUNDLE_RELEASE_BASE_URL,
        |bytes| {
            let manifest: teratts::bundle::Manifest =
                serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
            if manifest.format_version != teratts::bundle::FORMAT_VERSION {
                return Err(format!(
                    "unsupported bundle format {} (expected {})",
                    manifest.format_version,
                    teratts::bundle::FORMAT_VERSION
                ));
            }
            Ok(manifest
                .files
                .into_iter()
                .map(|file| BundleFile {
                    path: file.path,
                    asset: file.asset,
                    size: file.size,
                    sha256: file.sha256,
                })
                .collect())
        },
        emitter,
    )
    .await
}
