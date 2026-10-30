//! On-demand download of the Silero Native model bundle.
//!
//! Thin wrapper over the shared [`crate::tts::bundle_download`]: it supplies
//! the release URL and maps the silero-native manifest (which has no separate
//! asset name — its release assets are flat, so `path` is the asset) into the
//! generic file list. Resume, `.partial`, sha256 verification and the
//! `bundle_download_*` events all live in the shared downloader.

use std::path::Path;

use silero_native::bundle::Manifest;

use crate::tts::TtsError;
use crate::tts::bundle_download::{self, BundleFile};
use crate::tts::supervisor::Emitter;

/// Base URL of the GitHub Release that hosts the bundle files. The release
/// may not exist yet — a 404 surfaces as a typed `bundle_download_failed`
/// error (and a `bundle_download_finished { ok: false }` event), never a
/// panic or a half-installed bundle.
pub const BUNDLE_RELEASE_BASE_URL: &str =
    "https://github.com/xilec/RuVox/releases/download/silero-models-v5_ru";

/// Download and verify the whole bundle into `bundle_dir`.
pub async fn download_bundle(bundle_dir: &Path, emitter: &Emitter) -> Result<(), TtsError> {
    bundle_download::download(
        bundle_dir,
        "silero_native",
        BUNDLE_RELEASE_BASE_URL,
        |bytes| {
            Manifest::parse(bytes)
                .map(|manifest| {
                    manifest
                        .files
                        .into_iter()
                        .map(|file| BundleFile {
                            asset: file.path.clone(),
                            path: file.path,
                            size: file.size,
                            sha256: file.sha256,
                        })
                        .collect()
                })
                .map_err(|e| e.to_string())
        },
        emitter,
    )
    .await
}
