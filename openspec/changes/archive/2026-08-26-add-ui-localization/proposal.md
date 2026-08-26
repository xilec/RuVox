# Proposal: add-ui-localization

## Why

The UI mixes hardcoded Russian and English strings (e.g. "Add" in `AppShell`,
"Read Now" in `PreviewDialog`, "Mermaid diagram" in `TextViewer`), and every
backend error toast surfaces a pre-formatted Russian message. English-speaking
users cannot use the app in their language, and the string sprawl makes any
future copy change a cross-cutting edit.

## What Changes

- Introduce a frontend localization layer: RU (default) + EN string catalogs,
  a translation helper (`t`) backed by a locale store, no third-party i18n
  dependency.
- New persisted setting `UIConfig.language` (`"ru"` / `"en"`, default `"ru"`),
  exposed as a language selector in Settings.
- Migrate all user-visible frontend strings (components, dialogs, lib
  notification helpers) to the catalogs.
- **BREAKING** (wire format): command errors become
  `{ type, code, params?, message? }` — a machine-readable site code plus
  interpolation params replaces the human-readable Russian `message` as the
  primary payload. The frontend translates known codes via the catalogs;
  unknown codes fall back to `message`, then to a generic per-`type` string.
  Backend error sites carry codes instead of Russian text.
- Code-highlight theme follows the active color scheme (light/dark) instead of
  the statically imported light theme.

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `ipc-commands`: the Command Error Format requirement changes — errors carry
  `code`/`params` instead of a Russian `message`; `UIConfig` gains
  `language`.
- `storage`: the Config File Schema requirement gains the `language` field.
- `ui`: the Settings dialog requirement gains the language selector; a new
  localization requirement covers catalogs, the `t` helper, locale switching,
  and highlight-theme switching.

## Impact

- `src-tauri/src/storage/schema.rs` (+ language field, patch, tests);
  `src-tauri/src/commands/mod.rs` (error sites → codes; ~61 sites).
- `src/lib/errors.ts` (formatError → localized), new `src/i18n/` catalogs +
  `src/stores/locale.ts`, migration across components/dialogs/libs,
  `src/main.tsx` (highlight theme), `Settings.tsx` (selector).
- Wire-format break: anything matching on `CommandError.message` must move to
  `code`/`params` — covered repo-internally; external consumers none.
- Tests: Rust schema/command tests updated for the new shape; TS unit tests
  for `t`, `formatError`, and locale switching.

## Non-goals

- Localizing engine-internal diagnostic detail (reqwest/ort/piper strings) —
  those surface as `message` fallbacks verbatim.
- More than two languages; RTL layouts; translating docs/README.
