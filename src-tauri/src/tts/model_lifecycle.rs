//! Versioned model-bundle lifecycle shared by Silero Native and TeraTTSv2.
//!
//! The channel is a small JSON document per bundle family. It names the latest
//! bundle version, the release manifest URL, the manifest sha256, and the
//! download size. Channel checks are throttled and offline-safe: a failure is
//! cached for the throttle window and never blocks an installed bundle.
//!
//! Installs are staged next to the bundle directory, verified through the
//! shared downloader, then swapped in with a rename. The previous directory is
//! restored if the swap fails; it is removed only after the new bundle is in
//! place.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;
use std::time::{Duration, Instant};

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::tts::TtsError;
use crate::tts::bundle_download::{self, BundleFile};
use crate::tts::supervisor::Emitter;

pub const SILERO_NATIVE: &str = "silero_native";
pub const TERATTS: &str = "teratts";
pub const BUNDLE_ASSETS: [&str; 2] = [SILERO_NATIVE, TERATTS];
pub const CHANNEL_TTL: Duration = Duration::from_secs(24 * 60 * 60);

const DEFAULT_CHANNEL_BASE: &str =
    "https://raw.githubusercontent.com/xilec/RuVox/main/model-channels";

/// One published channel document.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct Channel {
    pub format_version: u32,
    pub asset: String,
    pub bundle_version: String,
    pub manifest_url: String,
    pub manifest_sha256: String,
    pub size_bytes: u64,
    #[serde(default)]
    pub published_at: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ChannelSnapshot {
    pub channels: BTreeMap<String, Channel>,
    pub error: Option<String>,
    pub checked_at: Option<String>,
}

#[derive(Debug)]
struct CacheEntry {
    at: Instant,
    snapshot: ChannelSnapshot,
}

static CHANNEL_CACHE: LazyLock<Mutex<Option<CacheEntry>>> = LazyLock::new(|| Mutex::new(None));

pub fn channel_base() -> String {
    std::env::var("RUVOX_MODEL_CHANNEL_BASE")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| DEFAULT_CHANNEL_BASE.to_string())
}

fn channel_url(asset: &str) -> String {
    format!("{}/{asset}.json", channel_base().trim_end_matches('/'))
}

/// Return the cached channel snapshot, refreshing it when `force` is set or
/// the cache is older than 24 hours. A failed refresh still returns any cached
/// channels; the error is reported separately for the UI.
pub async fn channels(force: bool) -> ChannelSnapshot {
    if !force {
        if let Some(entry) = CHANNEL_CACHE.lock().as_ref() {
            if entry.at.elapsed() < CHANNEL_TTL {
                return entry.snapshot.clone();
            }
        }
    }

    let client = match reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(20))
        .build()
    {
        Ok(client) => client,
        Err(e) => {
            return ChannelSnapshot {
                channels: BTreeMap::new(),
                error: Some(e.to_string()),
                checked_at: Some(now_iso()),
            };
        }
    };

    let mut channels = BTreeMap::new();
    let mut errors = Vec::new();
    for asset in BUNDLE_ASSETS {
        let url = channel_url(asset);
        match fetch_channel(&client, &url).await {
            Ok(channel) if channel.asset == asset => {
                channels.insert(asset.to_string(), channel);
            }
            Ok(channel) => errors.push(format!(
                "{asset}: channel names asset {} instead of {asset}",
                channel.asset
            )),
            Err(e) => errors.push(format!("{asset}: {e}")),
        }
    }
    let snapshot = ChannelSnapshot {
        channels,
        error: (!errors.is_empty()).then(|| errors.join("; ")),
        checked_at: Some(now_iso()),
    };
    *CHANNEL_CACHE.lock() = Some(CacheEntry {
        at: Instant::now(),
        snapshot: snapshot.clone(),
    });
    snapshot
}

async fn fetch_channel(client: &reqwest::Client, url: &str) -> Result<Channel, String> {
    let response = client
        .get(url)
        .send()
        .await
        .map_err(|e| format!("{url}: {e}"))?;
    if !response.status().is_success() {
        return Err(format!("{url}: HTTP {}", response.status()));
    }
    let bytes = response.bytes().await.map_err(|e| format!("{url}: {e}"))?;
    let channel: Channel =
        serde_json::from_slice(&bytes).map_err(|e| format!("{url}: invalid channel JSON: {e}"))?;
    if channel.format_version != 1 {
        return Err(format!(
            "{url}: unsupported channel format {}",
            channel.format_version
        ));
    }
    Ok(channel)
}

fn now_iso() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

/// Metadata written next to `manifest.json` after a successful install. The
/// Silero manifest predates bundle versioning, so this is its only record of
/// which channel version produced the directory.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InstallRecord {
    pub bundle_version: String,
    pub manifest_sha256: String,
    pub size_bytes: u64,
    pub installed_at: String,
}

pub const INSTALL_RECORD: &str = "install.json";

pub fn read_install_record(bundle_dir: &Path) -> Option<InstallRecord> {
    let bytes = std::fs::read(bundle_dir.join(INSTALL_RECORD)).ok()?;
    serde_json::from_slice(&bytes).ok()
}

pub fn write_install_record(bundle_dir: &Path, record: &InstallRecord) -> std::io::Result<()> {
    let bytes = serde_json::to_vec_pretty(record).map_err(std::io::Error::other)?;
    let tmp = bundle_dir.join(format!("{INSTALL_RECORD}.partial"));
    std::fs::write(&tmp, bytes)?;
    std::fs::rename(tmp, bundle_dir.join(INSTALL_RECORD))
}

/// Recursive size of a directory; a missing path is zero bytes.
pub fn dir_size(path: &Path) -> u64 {
    let Ok(meta) = std::fs::symlink_metadata(path) else {
        return 0;
    };
    if !meta.is_dir() {
        return meta.len();
    }
    let Ok(entries) = std::fs::read_dir(path) else {
        return 0;
    };
    entries
        .filter_map(Result::ok)
        .map(|entry| dir_size(&entry.path()))
        .sum()
}

/// Staging directory for an asset, next to the final bundle directory.
pub fn staging_dir(bundle_dir: &Path) -> PathBuf {
    sibling(bundle_dir, "staging")
}

fn backup_dir(bundle_dir: &Path) -> PathBuf {
    sibling(bundle_dir, "previous")
}

fn deleting_dir(bundle_dir: &Path) -> PathBuf {
    sibling(bundle_dir, "deleting")
}

fn sibling(bundle_dir: &Path, suffix: &str) -> PathBuf {
    let name = bundle_dir
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "bundle".to_string());
    bundle_dir.with_file_name(format!("{name}.{suffix}"))
}

/// Download into the staging directory and atomically replace `bundle_dir`.
/// The previous directory survives until the new one is renamed into place.
#[allow(clippy::too_many_arguments)]
pub async fn install(
    bundle_dir: &Path,
    asset: &str,
    manifest_url: &str,
    manifest_sha256: Option<&str>,
    parse_manifest: impl Fn(&[u8]) -> Result<Vec<BundleFile>, String>,
    channel_version: &str,
    emitter: &Emitter,
) -> Result<InstallRecord, TtsError> {
    let staging = staging_dir(bundle_dir);
    let parent = bundle_dir.parent().ok_or_else(|| TtsError::Ttsd {
        code: "bundle_path_unsafe".to_string(),
        message: "bundle directory has no parent".to_string(),
    })?;
    tokio::fs::create_dir_all(parent)
        .await
        .map_err(TtsError::Ipc)?;
    if let Some(backup) = existing_dir(&backup_dir(bundle_dir)) {
        let _ = tokio::fs::remove_dir_all(backup).await;
    }

    bundle_download::download_from_manifest_url(
        &staging,
        asset,
        manifest_url,
        manifest_sha256,
        parse_manifest,
        emitter,
    )
    .await?;

    let size_bytes = dir_size(&staging);
    let record = InstallRecord {
        bundle_version: channel_version.to_string(),
        manifest_sha256: manifest_sha256.unwrap_or_default().to_string(),
        size_bytes,
        installed_at: now_iso(),
    };
    write_install_record(&staging, &record).map_err(TtsError::Ipc)?;

    swap_in(bundle_dir, &staging).await?;
    Ok(record)
}

fn existing_dir(path: &Path) -> Option<&Path> {
    path.is_dir().then_some(path)
}

async fn swap_in(bundle_dir: &Path, staging: &Path) -> Result<(), TtsError> {
    let backup = backup_dir(bundle_dir);
    let had_current = bundle_dir.exists();
    if had_current {
        if backup.exists() {
            tokio::fs::remove_dir_all(&backup)
                .await
                .map_err(TtsError::Ipc)?;
        }
        tokio::fs::rename(bundle_dir, &backup)
            .await
            .map_err(TtsError::Ipc)?;
    }
    if let Err(e) = tokio::fs::rename(staging, bundle_dir).await {
        if had_current {
            let _ = tokio::fs::rename(&backup, bundle_dir).await;
        }
        return Err(TtsError::Ipc(e));
    }
    if had_current {
        let _ = tokio::fs::remove_dir_all(&backup).await;
    }
    Ok(())
}

/// Remove an installed bundle (or a Piper voice directory). The directory is
/// first renamed to a deleting sibling so a partial removal cannot be mistaken
/// for an installed asset; the rename is rolled back if it fails.
pub async fn remove_dir(path: &Path) -> Result<u64, TtsError> {
    if !path.exists() {
        return Ok(0);
    }
    let bytes = dir_size(path);
    let deleting = deleting_dir(path);
    if deleting.exists() {
        tokio::fs::remove_dir_all(&deleting)
            .await
            .map_err(TtsError::Ipc)?;
    }
    tokio::fs::rename(path, &deleting)
        .await
        .map_err(TtsError::Ipc)?;
    if let Err(e) = tokio::fs::remove_dir_all(&deleting).await {
        let _ = tokio::fs::rename(&deleting, path).await;
        return Err(TtsError::Ipc(e));
    }
    Ok(bytes)
}

/// Persisted user choices that are not part of the main config.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ModelState {
    #[serde(default)]
    pub skipped: BTreeMap<String, String>,
}

impl ModelState {
    pub fn load(path: &Path) -> Self {
        std::fs::read(path)
            .ok()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or_default()
    }

    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let bytes = serde_json::to_vec_pretty(self).map_err(std::io::Error::other)?;
        let tmp = path.with_extension("json.partial");
        std::fs::write(&tmp, bytes)?;
        std::fs::rename(tmp, path)
    }

    pub fn skip(&mut self, asset: &str, version: &str) {
        self.skipped.insert(asset.to_string(), version.to_string());
    }

    pub fn is_skipped(&self, asset: &str, version: &str) -> bool {
        self.skipped.get(asset).is_some_and(|v| v == version)
    }
}

/// Installed metadata for one bundle directory.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InstalledBundle {
    pub installed: bool,
    pub version: Option<String>,
    pub model_id: Option<String>,
    pub upstream_revision: Option<String>,
    pub bundled_voices: Vec<String>,
    pub bytes: u64,
    pub error: Option<String>,
}

pub fn inspect_bundle(asset: &str, bundle_dir: &Path) -> InstalledBundle {
    let record = read_install_record(bundle_dir);
    let bytes = dir_size(bundle_dir);
    match asset {
        SILERO_NATIVE => match silero_native::bundle::Manifest::load(bundle_dir) {
            Ok(manifest) => InstalledBundle {
                installed: true,
                version: record
                    .map(|r| r.bundle_version)
                    .or_else(|| Some(manifest.model_id.clone())),
                model_id: Some(manifest.model_id),
                upstream_revision: None,
                bundled_voices: silero_native::frontend::FrontendConfig::load(bundle_dir)
                    .map(|frontend| frontend.speakers)
                    .unwrap_or_default(),
                bytes,
                error: None,
            },
            Err(e) => InstalledBundle {
                installed: false,
                bytes,
                error: Some(e.to_string()),
                ..Default::default()
            },
        },
        TERATTS => match teratts::bundle::Manifest::load(bundle_dir) {
            Ok(manifest) => InstalledBundle {
                installed: true,
                version: record
                    .map(|r| r.bundle_version)
                    .or_else(|| Some(manifest.bundle_version.to_string())),
                model_id: Some(manifest.model_id),
                upstream_revision: Some(manifest.upstream_revision),
                bundled_voices: manifest.voices.into_iter().map(|voice| voice.id).collect(),
                bytes,
                error: None,
            },
            Err(e) => InstalledBundle {
                installed: false,
                bytes,
                error: Some(e.to_string()),
                ..Default::default()
            },
        },
        _ => InstalledBundle {
            installed: false,
            bytes,
            error: Some(format!("unknown bundle asset {asset}")),
            ..Default::default()
        },
    }
}

/// One row of the model inventory returned to Settings.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ModelAssetStatus {
    pub id: String,
    pub kind: String,
    pub installed: bool,
    pub active: bool,
    pub installed_version: Option<String>,
    pub model_id: Option<String>,
    pub upstream_revision: Option<String>,
    pub bundled_voices: Vec<String>,
    pub installed_bytes: u64,
    pub latest_version: Option<String>,
    pub latest_bytes: Option<u64>,
    pub update_available: bool,
    pub skipped: bool,
    pub deletable: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ModelStatus {
    pub assets: Vec<ModelAssetStatus>,
    pub total_bytes: u64,
    pub channel_error: Option<String>,
    pub checked_at: Option<String>,
}

/// Convert a channel snapshot + installed rows into the final status. Kept
/// pure so the command layer can assemble installed rows without duplicating
/// update/skip logic.
pub fn status_from_rows(
    rows: Vec<ModelAssetStatus>,
    channels: &ChannelSnapshot,
    state: &ModelState,
) -> ModelStatus {
    let mut assets = rows;
    for row in &mut assets {
        if let Some(channel) = channels.channels.get(&row.id) {
            row.latest_version = Some(channel.bundle_version.clone());
            row.latest_bytes = Some(channel.size_bytes);
            let version_differs =
                row.installed_version.as_deref() != Some(channel.bundle_version.as_str());
            // A deferral only applies while the newer version is still not
            // installed. Once the user installs it manually, the row must
            // stop showing a stale "update deferred" badge.
            let deferred = version_differs && state.is_skipped(&row.id, &channel.bundle_version);
            row.update_available = row.installed && version_differs && !deferred;
            row.skipped = deferred;
        }
    }
    let total_bytes = assets.iter().map(|a| a.installed_bytes).sum();
    ModelStatus {
        assets,
        total_bytes,
        channel_error: channels.error.clone(),
        checked_at: channels.checked_at.clone(),
    }
}

/// sha256 of an in-memory manifest, used by tests and by callers that already
/// fetched the channel.
pub fn sha256_bytes(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn channel_url_uses_env_override() {
        // SAFETY: the test process is single-threaded for this variable and
        // no other test reads it concurrently.
        unsafe { std::env::set_var("RUVOX_MODEL_CHANNEL_BASE", "http://localhost:1/channels/") };
        assert_eq!(
            channel_url("teratts"),
            "http://localhost:1/channels/teratts.json"
        );
        // The URL uses the asset id, so the published file must be named
        // `silero_native.json`, not `silero-native.json`.
        assert_eq!(
            channel_url("silero_native"),
            "http://localhost:1/channels/silero_native.json"
        );
        unsafe { std::env::remove_var("RUVOX_MODEL_CHANNEL_BASE") };
    }

    #[test]
    fn channel_json_parses_and_requires_format_one() {
        let channel: Channel = serde_json::from_str(
            r#"{
                "format_version": 1,
                "asset": "teratts",
                "bundle_version": "2",
                "manifest_url": "https://example/manifest.json",
                "manifest_sha256": "abc",
                "size_bytes": 42
            }"#,
        )
        .unwrap();
        assert_eq!(channel.bundle_version, "2");
        assert_eq!(channel.size_bytes, 42);

        let wrong = serde_json::from_str::<Channel>(
            r#"{
                "format_version": 2,
                "asset": "teratts",
                "bundle_version": "2",
                "manifest_url": "https://example/manifest.json",
                "manifest_sha256": "abc",
                "size_bytes": 42
            }"#,
        );
        assert!(
            wrong.is_ok(),
            "format is checked by fetch_channel, not serde"
        );
    }

    #[test]
    fn sha256_bytes_matches_known_value() {
        assert_eq!(
            sha256_bytes(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn model_state_roundtrips_and_skips() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("model_state.json");
        let mut state = ModelState::default();
        state.skip("teratts", "2");
        state.save(&path).unwrap();
        let loaded = ModelState::load(&path);
        assert!(loaded.is_skipped("teratts", "2"));
        assert!(!loaded.is_skipped("teratts", "1"));
    }

    #[test]
    fn dir_size_is_recursive_and_missing_is_zero() {
        let dir = tempfile::TempDir::new().unwrap();
        fs::create_dir_all(dir.path().join("a/b")).unwrap();
        fs::write(dir.path().join("a/b/file"), b"12345").unwrap();
        assert_eq!(dir_size(dir.path()), 5);
        assert_eq!(dir_size(&dir.path().join("missing")), 0);
    }

    #[tokio::test]
    async fn swap_in_replaces_bundle_and_removes_backup() {
        let dir = tempfile::TempDir::new().unwrap();
        let bundle = dir.path().join("teratts");
        let staging = staging_dir(&bundle);
        fs::create_dir_all(&bundle).unwrap();
        fs::write(bundle.join("old"), b"old").unwrap();
        fs::create_dir_all(&staging).unwrap();
        fs::write(staging.join("new"), b"new").unwrap();
        swap_in(&bundle, &staging).await.unwrap();
        assert_eq!(fs::read(bundle.join("new")).unwrap(), b"new");
        assert!(!bundle.join("old").exists());
        assert!(!backup_dir(&bundle).exists());
        assert!(!staging.exists());
    }

    #[tokio::test]
    async fn swap_in_restores_previous_bundle_when_staging_is_missing() {
        let dir = tempfile::TempDir::new().unwrap();
        let bundle = dir.path().join("teratts");
        fs::create_dir_all(&bundle).unwrap();
        fs::write(bundle.join("old"), b"old").unwrap();
        let staging = staging_dir(&bundle);
        assert!(swap_in(&bundle, &staging).await.is_err());
        assert_eq!(fs::read(bundle.join("old")).unwrap(), b"old");
        assert!(!backup_dir(&bundle).exists());
    }

    #[tokio::test]
    async fn remove_dir_reports_size_and_removes_tree() {
        let dir = tempfile::TempDir::new().unwrap();
        let bundle = dir.path().join("teratts");
        fs::create_dir_all(&bundle).unwrap();
        fs::write(bundle.join("a"), b"12345").unwrap();
        assert_eq!(remove_dir(&bundle).await.unwrap(), 5);
        assert!(!bundle.exists());
        assert_eq!(remove_dir(&bundle).await.unwrap(), 0);
    }

    #[test]
    fn install_record_roundtrips() {
        let dir = tempfile::TempDir::new().unwrap();
        let record = InstallRecord {
            bundle_version: "2".to_string(),
            manifest_sha256: "abc".to_string(),
            size_bytes: 42,
            installed_at: "2026-10-30T00:00:00Z".to_string(),
        };
        write_install_record(dir.path(), &record).unwrap();
        assert_eq!(read_install_record(dir.path()), Some(record));
    }

    #[test]
    fn status_marks_update_and_skip() {
        let rows = vec![ModelAssetStatus {
            id: TERATTS.to_string(),
            kind: "bundle".to_string(),
            installed: true,
            active: false,
            installed_version: Some("1".to_string()),
            model_id: None,
            upstream_revision: None,
            bundled_voices: Vec::new(),
            installed_bytes: 10,
            latest_version: None,
            latest_bytes: None,
            update_available: false,
            skipped: false,
            deletable: true,
            error: None,
        }];
        let channels = ChannelSnapshot {
            channels: BTreeMap::from([(
                TERATTS.to_string(),
                Channel {
                    format_version: 1,
                    asset: TERATTS.to_string(),
                    bundle_version: "2".to_string(),
                    manifest_url: "http://example/manifest.json".to_string(),
                    manifest_sha256: "x".to_string(),
                    size_bytes: 20,
                    published_at: None,
                },
            )]),
            error: None,
            checked_at: None,
        };
        let state = ModelState::default();
        let status = status_from_rows(rows.clone(), &channels, &state);
        assert!(status.assets[0].update_available);
        assert_eq!(status.assets[0].latest_bytes, Some(20));

        let mut skipped = ModelState::default();
        skipped.skip(TERATTS, "2");
        let status = status_from_rows(rows, &channels, &skipped);
        assert!(!status.assets[0].update_available);
        assert!(status.assets[0].skipped);
    }

    #[test]
    fn skip_badge_clears_after_the_deferred_version_is_installed() {
        let mut rows = vec![ModelAssetStatus {
            id: TERATTS.to_string(),
            kind: "bundle".to_string(),
            installed: true,
            active: false,
            installed_version: Some("2".to_string()),
            model_id: None,
            upstream_revision: None,
            bundled_voices: Vec::new(),
            installed_bytes: 10,
            latest_version: None,
            latest_bytes: None,
            update_available: false,
            skipped: false,
            deletable: true,
            error: None,
        }];
        let channels = ChannelSnapshot {
            channels: BTreeMap::from([(
                TERATTS.to_string(),
                Channel {
                    format_version: 1,
                    asset: TERATTS.to_string(),
                    bundle_version: "2".to_string(),
                    manifest_url: "https://example/manifest.json".to_string(),
                    manifest_sha256: "abc".to_string(),
                    size_bytes: 20,
                    published_at: None,
                },
            )]),
            error: None,
            checked_at: None,
        };
        let mut state = ModelState::default();
        state.skip(TERATTS, "2");
        let status = status_from_rows(rows.clone(), &channels, &state);
        assert!(!status.assets[0].update_available);
        assert!(!status.assets[0].skipped);

        // Keep the fixture used: mutating it verifies the installed version
        // is what drives the badge, not the persisted deferral alone.
        rows[0].installed_version = Some("1".to_string());
        let status = status_from_rows(rows, &channels, &state);
        assert!(status.assets[0].skipped);
    }
}
