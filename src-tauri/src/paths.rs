//! Per-user data root resolution.
//!
//! Windows layout note: the NSIS per-user installer puts the program into
//! `%LOCALAPPDATA%\<productName>` (`RuVox`), and the uninstaller's
//! "Delete the application data" checkbox removes
//! `%APPDATA%\<identifier>` and `%LOCALAPPDATA%\<identifier>`. Keeping
//! app data under the identifier dir keeps it out of the install dir and
//! makes the checkbox work with no NSIS customization (change
//! `2026-08-19-windows-data-dir`, issue #200).

use std::path::PathBuf;

/// App data dir name on Windows — MUST equal `identifier` in
/// `tauri.conf.json`; the NSIS uninstaller deletes exactly this dir.
#[cfg(any(windows, test))]
const WINDOWS_DATA_DIR_NAME: &str = "com.ruvox.app";

/// Root for history/config/audio (the storage cache root).
pub fn storage_root() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        Some(dirs::data_local_dir()?.join(WINDOWS_DATA_DIR_NAME))
    }
    #[cfg(not(windows))]
    {
        Some(dirs::cache_dir()?.join("ruvox"))
    }
}

/// Root for TTS voice/model downloads (Piper voices, silero-native bundle).
pub fn voices_root() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        Some(
            dirs::data_local_dir()?
                .join(WINDOWS_DATA_DIR_NAME)
                .join("voices"),
        )
    }
    #[cfg(not(windows))]
    {
        Some(dirs::data_local_dir()?.join("ruvox").join("voices"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The NSIS uninstaller's data-deletion checkbox targets the bundle
    /// identifier dir; pinning the constant to `tauri.conf.json` keeps the
    /// two from drifting apart.
    #[test]
    fn windows_data_dir_matches_bundle_identifier() {
        let conf = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/tauri.conf.json"))
            .expect("tauri.conf.json readable");
        let json: serde_json::Value = serde_json::from_str(&conf).expect("tauri.conf.json parses");
        assert_eq!(
            json["identifier"].as_str().unwrap(),
            WINDOWS_DATA_DIR_NAME,
            "tauri.conf.json identifier changed — update WINDOWS_DATA_DIR_NAME"
        );
    }

    #[test]
    #[cfg(not(windows))]
    fn unix_roots_keep_the_ruvox_dir_name() {
        // Guards against an accidental rename of the long-standing Linux
        // locations (~/.cache/ruvox, ~/.local/share/ruvox/voices) — existing
        // installs depend on them.
        assert!(storage_root().unwrap().ends_with("ruvox"));
        let voices = voices_root().unwrap();
        assert!(voices.ends_with("voices"));
        assert!(voices.parent().unwrap().ends_with("ruvox"));
    }
}
