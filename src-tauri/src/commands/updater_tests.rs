use super::updater_supported_with;

// `cargo test` runs on Linux CI (the windows-build job is compile-only),
// so the Windows branch is pinned by a cfg-gated test for Windows
// runners and by compilation everywhere else.
#[test]
#[cfg(target_os = "linux")]
fn appimage_env_presence_decides_on_linux() {
    assert!(updater_supported_with(Some(std::ffi::OsString::from(
        "/opt/RuVox.AppImage"
    ))));
    assert!(!updater_supported_with(None));
}

#[test]
#[cfg(target_os = "windows")]
fn updater_is_always_supported_on_windows() {
    assert!(updater_supported_with(None));
    assert!(updater_supported_with(Some(std::ffi::OsString::from(
        "C:\\apps\\RuVox.exe"
    ))));
}
