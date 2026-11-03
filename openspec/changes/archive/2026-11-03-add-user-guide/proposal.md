# Add a user guide with ASCII UI mockups and a hotkey reference

## Why

RuVox has no single user-facing text that explains the application end to end:
text → preview → queue → playback. `README.md` is a flat feature list,
`openspec/specs/` is developer-facing, and screenshots go stale faster than
they can be updated. Users also have no trustworthy hotkey reference: the
`UIConfig::player_hotkeys` map still carries defaults for ten shortcuts, but
nothing consumes it — the player hardcodes three keys and Settings exposes no
hotkey controls.

## What Changes

- Add `docs/user-guide.md` — a Russian, user-facing guide with one short
  section per topic (first launch, adding text, preview, queue, playback,
  settings, dictionary, tray, hotkeys, troubleshooting), linked from
  `README.md` in one click.
- Use fenced ASCII mockups instead of screenshots: box drawing, Russian UI
  labels, window chrome elided, roughly 80 columns, and only the control the
  section is about.
- Document only hotkeys that actually work (`Space`, `Left`/`Right`,
  `Ctrl/Cmd+F`, `Ctrl/Cmd+A`/`Ctrl/Cmd+C` on a link, `Esc`) and remove the
  unused `player_hotkeys` config field instead of promising keys that do
  nothing.
- Add a hard rule to `ai/rules/conventions.md` (with a pointer from
  `AGENTS.md`): a PR that changes UI structure or observable feature behavior
  updates `docs/user-guide.md` in the same PR, ASCII mockup and hotkey list
  included; a stale guide is a review finding.
- Record the `ipc-commands` `UIConfig` shape without `player_hotkeys` as an
  OpenSpec delta and archive the change in the same PR.
- Add `[Unreleased]` CHANGELOG entries: the guide under Added, the removed
  setting under Removed.

## Capabilities

### New Capabilities

(none — this is documentation plus removal of an unused config field)

### Modified Capabilities

- `ipc-commands`: `UIConfig` no longer carries `player_hotkeys`; a legacy
  `config.json` with that key still loads (unknown fields are ignored) and the
  key disappears on the next save.

## Impact

- Affected specs: `ipc-commands`.
- Affected code: `src-tauri/src/storage/schema.rs`,
  `src-tauri/src/commands/mod.rs`, `src-tauri/src/commands/tests.rs`,
  `src/lib/tauri.ts`, and the UI test config mocks that carried the field.
- Affected docs: `docs/user-guide.md` (new), `README.md`, `README.en.md`,
  `docs/index.md`, `ai/rules/conventions.md`, `AGENTS.md`, `CHANGELOG.md`.
- No runtime behavior change beyond dropping an unused config key: existing
  `config.json` files keep working, and the removed key is dropped on the next
  save.

## Non-goals

- English translation of the guide.
- In-app onboarding, tooltips or help overlays.
- Screenshots.
- Configurable player hotkeys — reintroduce them only when users ask.
- Developer documentation and the OpenSpec specs themselves (only the
  `ipc-commands` delta is added, then archived).
