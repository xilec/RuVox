# Proposal: windows-data-dir

Fixes #200 (blocks v0.3.0). Found during the v0.3.0 VM verification pass.

## Problem

On Windows all app data (config, history, audio cache, downloaded voices)
lives in `%LOCALAPPDATA%\ruvox` — the same directory the NSIS per-user
installer uses as `$INSTDIR` (`%LOCALAPPDATA%\<productName>`). Two
consequences, both verified on a Win10 22H2 VM:

- The uninstaller's "Delete the application data" checkbox removes
  `%APPDATA%\<identifier>` and `%LOCALAPPDATA%\<identifier>`
  (`com.ruvox.app`) — directories the app never uses — so user data is
  left behind even when the user asked to delete it.
- User data sits inside the install dir; uninstalling without the
  checkbox leaves a `$INSTDIR` that mixes leftover data with a removed
  program.

## Change

On Windows, resolve the per-user data root as
`dirs::data_local_dir()/<bundle identifier>`
(`%LOCALAPPDATA%\com.ruvox.app`) for both the storage root and the voices
root, instead of `…\ruvox`. Non-Windows layouts are unchanged
(`dirs::cache_dir()/ruvox` for storage, `dirs::data_local_dir()/ruvox`
for voices).

This matches the Tauri NSIS uninstaller convention, so the stock
"Delete the application data" checkbox cleans the data root with no NSIS
customization, and the install dir no longer holds user data.

No migration: v0.3.0 is the first Windows release, there are no Windows
installs with data in the old location. Linux/macOS paths are untouched,
so no migration there either.

## Out of scope

- Moving the NSIS install dir itself (stays `%LOCALAPPDATA%\RuVox`).
- Roaming vs Local split: everything stays under LocalAppData (audio
  cache and voice models can be hundreds of MB and must not roam).
