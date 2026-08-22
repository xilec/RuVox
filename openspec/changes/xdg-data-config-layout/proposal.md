# Proposal: xdg-data-config-layout

## Why

On Linux the storage service keeps history, config, and audio under `~/.cache/ruvox/`
(`dirs::cache_dir()`), a location system cache cleaners (systemd-tmpfiles, bleachbit)
are allowed to wipe — a user can lose all narration history to a routine cleanup.
Windows already stores data outside volatile locations (#200/#201); Linux is the last
platform on the legacy layout.

## What Changes

- Split the single per-user storage root into two roots:
  - **data root** (history.json, audio/) → `dirs::data_local_dir()/ruvox` on Linux
    (`~/.local/share/ruvox/`); unchanged (`%LOCALAPPDATA%\com.ruvox.app`) on Windows.
  - **config root** (config.json) → `dirs::config_dir()/ruvox` on Linux
    (`~/.config/ruvox/`); unchanged on Windows (stays next to data in
    `%LOCALAPPDATA%\com.ruvox.app`).
- One-shot startup migration from the legacy `~/.cache/ruvox/` layout: move
  `history.json`, `audio/`, and `config.json` into the new roots; idempotent, tolerant
  of partial previous migrations; removes migrated items from the old dir and deletes
  it when empty.
- Add `.bak` recovery for a corrupted `config.json` (rename to `config.json.bak`,
  fall back to defaults) — same behavior `history.json` already has.
- Settings screen label updated ("Папка кэша" → "Папка данных"); `get_cache_dir`
  IPC command keeps its name and now returns the data root.

## Capabilities

- **Modified Capabilities:**
  - `storage` — cache directory layout requirement is rewritten for the two-root
    XDG layout; new migration requirement; corrupted-config recovery scenarios added
    alongside the existing corrupted-history recovery.
  - `ipc-commands` — `get_cache_dir` returns the per-user data directory (the
    directory that holds `history.json` and `audio/`) instead of "the cache directory".
- **New Capabilities:** none.

## Impact

- `src-tauri/src/paths.rs` — root resolution split (`data_root()`, `config_root()`,
  `legacy_cache_root()`); unit tests updated for the new Linux locations.
- `src-tauri/src/storage/service.rs` — constructor resolves two roots, runs the legacy
  migration, adds config `.bak` recovery; test constructor keeps the single-dir form
  so existing tests stay meaningful; a public `with_data_and_config_dirs(data, config)`
  constructor serves split-layout/migration tests directly (no change needed in
  `storage/test_util.rs`).
- `src/dialogs/Settings.tsx` — label text only.
- Specs: `openspec/specs/storage/spec.md`, `openspec/specs/ipc-commands/spec.md`.
- No dependency changes; no schema/format changes to any persisted file.

## Non-goals

- No change to file formats or schemas (history.json / config.json contents).
- No Windows path changes (already correct since #200/#201).
- No migration UI or user prompt — migration is silent and automatic.
- Startup failure handling when storage cannot be opened at all (#223, separate change).
