# Tasks — align default engine contract

## 1. Specs

- [x] 1.1 Add the `ipc-commands` MODIFIED delta: the `engine` field comment
  says `"teratts" (default)`, and the `get_config` defaults scenario expects
  `engine: "teratts"`.
- [x] 1.2 Add the `ui` MODIFIED delta: the `Reset restores factory defaults`
  scenario records the engine returning to the storage default through the
  availability coercion.
- [x] 1.3 Confirm the Rust default and the Settings reset path already match
  the corrected text; no code change.

## 2. Validation

- [x] 2.1 `openspec validate align-default-engine-contract --strict`.
- [x] 2.2 Archive the change, then `openspec validate --specs --strict`
  (27/27).
- [x] 2.3 Diff the two synced requirements against the delta: only the engine
  comment, the reset THEN clause and the `get_config` scenario value change.
- [x] 2.4 Commit the archive with the synced specs.
