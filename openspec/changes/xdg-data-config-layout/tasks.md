# Tasks: xdg-data-config-layout

## 1. Paths

- [x] 1.1 Split `paths.rs` into `data_root()` / `config_root()` / `legacy_cache_root()`; Windows behavior unchanged; update the `unix_roots_keep_the_ruvox_dir_name` test to pin `~/.local/share/ruvox` and `~/.config/ruvox` shape via env-overridable dirs resolution

## 2. Storage service

- [x] 2.1 Add split-root construction: production `new()` resolves both roots; keep single-dir `with_cache_dir()` for tests; add `with_data_and_config_dirs(data, config)`
- [x] 2.2 Implement legacy-layout migration per design (per-item, audio → config → history order, rename with copy fallback, remove empty legacy dir, log-and-continue on failure); unit tests: fresh migration, idempotency, partial completion, failure-tolerance, empty-dir removal
- [x] 2.3 Config `.bak` recovery in `load_config` mirroring history behavior; unit tests: corrupted JSON → `.bak` + defaults; unreadable file → defaults

## 3. IPC & UI surface

- [x] 3.1 `get_cache_dir` command returns the data root; check all `cache_dir().join("audio")` call sites still resolve correctly
- [x] 3.2 Settings screen: label "Папка кэша" → "Папка данных"

## 4. Gates

- [x] 4.1 `cargo test --manifest-path src-tauri/Cargo.toml`, `just lint`, `pnpm typecheck`, `pnpm test:unit`, ttsd pytest — green
- [x] 4.2 Manual pass (dev build, leftmost monitor, sandboxed XDG env): legacy layout migrates on launch; history/audio visible in app; corrupted config.json recovers via .bak; Settings shows new data dir path

## 5. Wrap-up

- [ ] 5.1 Update delta specs if implementation revealed deviations; validate with `openspec validate --specs --strict`
- [ ] 5.2 Archive change (sync specs), pre-PR reviewer pass, PR, merge
