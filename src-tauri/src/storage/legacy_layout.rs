//! One-shot migration from the legacy single-root layout
//! (`~/.cache/ruvox/`, change `xdg-data-config-layout`, issue #222) into the
//! two-root data/config layout.
//!
//! Per-item semantics: an item (`audio/`, `config.json`, `history.json`)
//! moves only when its destination does not already exist, which makes the
//! sweep idempotent and tolerant of partially completed earlier runs. Items
//! move in the order audio → config → history so that entry validation during
//! history load never sees a moved `history.json` against not-yet-moved
//! audio. Failures are logged and never abort startup.

use std::fs;
use std::path::{Path, PathBuf};

/// Move every still-present legacy item into its new root and drop the legacy
/// directory when it ends up empty.
pub(crate) fn migrate_legacy_layout(data_dir: &Path, config_dir: &Path) {
    run(crate::paths::legacy_cache_root(), data_dir, config_dir);
}

fn run(legacy_root: Option<PathBuf>, data_dir: &Path, config_dir: &Path) {
    let Some(legacy) = legacy_root else {
        return;
    };
    if !legacy.exists() {
        return;
    }

    // Order matters: audio must be in place before history loads (see module doc).
    let items: [(&str, &Path); 3] = [
        ("audio", &data_dir.join("audio")),
        ("config.json", &config_dir.join("config.json")),
        ("history.json", &data_dir.join("history.json")),
    ];

    for (name, dst) in items {
        let src = legacy.join(name);
        if !src.exists() {
            continue;
        }
        if dst.exists() {
            tracing::warn!(
                "legacy layout migration: {} already exists, leaving {} in place",
                dst.display(),
                src.display()
            );
            continue;
        }
        match move_item(&src, dst) {
            Ok(()) => {
                tracing::info!(
                    "legacy layout migration: {} -> {}",
                    src.display(),
                    dst.display()
                )
            }
            Err(e) => tracing::error!(
                "legacy layout migration: failed to move {} to {}: {e}; leaving it in place",
                src.display(),
                dst.display()
            ),
        }
    }

    remove_dir_if_empty(&legacy);
}

fn move_item(src: &Path, dst: &Path) -> std::io::Result<()> {
    let parent = dst.parent().ok_or_else(|| {
        std::io::Error::other(format!("destination has no parent: {}", dst.display()))
    })?;
    fs::create_dir_all(parent)?;

    match fs::rename(src, dst) {
        Ok(()) => Ok(()),
        // Cross-filesystem rename fails; fall back to a full copy, deleting
        // the source only after the copy succeeded.
        Err(rename_err) => {
            copy_recursive(src, dst).map_err(|copy_err| {
                std::io::Error::other(format!(
                    "rename ({rename_err}) and copy fallback ({copy_err}) both failed"
                ))
            })?;
            remove_item(src)
        }
    }
}

fn copy_recursive(src: &Path, dst: &Path) -> std::io::Result<()> {
    if src.is_dir() {
        fs::create_dir_all(dst)?;
        for entry in fs::read_dir(src)? {
            let entry = entry?;
            copy_recursive(&entry.path(), &dst.join(entry.file_name()))?;
        }
        Ok(())
    } else {
        fs::copy(src, dst).map(|_| ())
    }
}

fn remove_item(path: &Path) -> std::io::Result<()> {
    if path.is_dir() {
        fs::remove_dir_all(path)
    } else {
        fs::remove_file(path)
    }
}

fn remove_dir_if_empty(dir: &Path) {
    let is_empty = match fs::read_dir(dir) {
        Ok(mut entries) => entries.next().is_none(),
        Err(e) => {
            tracing::warn!(
                "legacy layout migration: cannot inspect {}: {e}",
                dir.display()
            );
            return;
        }
    };
    if is_empty {
        match fs::remove_dir(dir) {
            Ok(()) => tracing::info!("legacy layout migration: removed empty {}", dir.display()),
            Err(e) => tracing::warn!(
                "legacy layout migration: could not remove empty {}: {e}",
                dir.display()
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn moves_all_items_and_removes_empty_legacy_dir() {
        let tmp = TempDir::new().unwrap();
        let legacy = tmp.path().join("cache").join("ruvox");
        let data = tmp.path().join("data");
        let config = tmp.path().join("config");

        fs::create_dir_all(legacy.join("audio")).unwrap();
        fs::write(legacy.join("history.json"), r#"{"version":1,"entries":[]}"#).unwrap();
        fs::write(legacy.join("config.json"), "{}").unwrap();
        fs::write(legacy.join("audio").join("x.opus"), b"OggS").unwrap();

        run(Some(legacy.clone()), &data, &config);

        assert_eq!(
            fs::read_to_string(data.join("history.json")).unwrap(),
            r#"{"version":1,"entries":[]}"#
        );
        assert_eq!(
            fs::read_to_string(config.join("config.json")).unwrap(),
            "{}"
        );
        assert_eq!(
            fs::read(data.join("audio").join("x.opus")).unwrap(),
            b"OggS"
        );
        assert!(!legacy.exists(), "empty legacy dir must be removed");
    }

    #[test]
    fn completes_partial_earlier_migration() {
        let tmp = TempDir::new().unwrap();
        let legacy = tmp.path().join("ruvox");
        let data = tmp.path().join("data");
        let config = tmp.path().join("config");

        // A previous run crashed after moving history.json only; audio +
        // config remain in the legacy dir alongside its stale copy.
        fs::create_dir_all(&data).unwrap();
        fs::write(data.join("history.json"), r#"{"version":1,"entries":[]}"#).unwrap();

        fs::create_dir_all(legacy.join("audio")).unwrap();
        fs::write(legacy.join("history.json"), "stale leftover").unwrap();
        fs::write(legacy.join("config.json"), "{}").unwrap();
        fs::write(legacy.join("audio").join("y.opus"), b"OggS").unwrap();

        run(Some(legacy.clone()), &data, &config);

        assert_eq!(
            fs::read_to_string(data.join("history.json")).unwrap(),
            r#"{"version":1,"entries":[]}"#,
            "already-migrated history stays untouched"
        );
        assert_eq!(
            fs::read_to_string(legacy.join("history.json")).unwrap(),
            "stale leftover",
            "existing destination wins; the stale source is kept for inspection"
        );
        assert_eq!(
            fs::read_to_string(config.join("config.json")).unwrap(),
            "{}"
        );
        assert_eq!(
            fs::read(data.join("audio").join("y.opus")).unwrap(),
            b"OggS"
        );
    }

    #[test]
    fn leaves_unexpected_content_in_legacy_dir() {
        let tmp = TempDir::new().unwrap();
        let legacy = tmp.path().join("ruvox");
        let data = tmp.path().join("data");
        let config = tmp.path().join("config");

        fs::create_dir_all(legacy.join("audio")).unwrap();
        fs::write(legacy.join("history.json"), r#"{"version":1,"entries":[]}"#).unwrap();
        fs::write(legacy.join("something-else.txt"), "keep me").unwrap();

        run(Some(legacy.clone()), &data, &config);

        // Known items moved...
        assert!(data.join("history.json").exists());
        // ...but unknown content keeps the legacy dir alive for inspection.
        assert!(legacy.exists());
        assert!(legacy.join("something-else.txt").exists());
    }

    #[test]
    fn no_op_without_legacy_dir() {
        let tmp = TempDir::new().unwrap();
        let data = tmp.path().join("data");
        let config = tmp.path().join("config");

        run(Some(tmp.path().join("nonexistent")), &data, &config);

        assert!(!data.exists());
        assert!(!config.exists());
    }

    #[test]
    fn no_op_when_legacy_root_unresolvable() {
        let tmp = TempDir::new().unwrap();
        let data = tmp.path().join("data");
        let config = tmp.path().join("config");

        run(None, &data, &config);

        assert!(!data.exists());
        assert!(!config.exists());
    }
}
