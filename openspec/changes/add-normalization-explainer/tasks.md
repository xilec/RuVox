# Tasks: add-normalization-explainer

## 1. UI copy in the preview dialog

- [x] 1.1 Add `preview.explain.*` keys to `src/i18n/ru.ts` and `src/i18n/en.ts`:
      the short explainer line, the tooltip/popover full text, the README link
      label, and the help icon aria-label.
- [x] 1.2 `src/dialogs/PreviewDialog.tsx` + `PreviewDialog.module.css`: render
      the explainer line between the header and the panes; add the header help
      `ActionIcon` with a click-toggled `Popover` (full text + README link)
      placed before the close button, with an aria-label and without breaking
      the header drag handle.
- [x] 1.3 Wire the README link through `openUrl` from
      `@tauri-apps/plugin-opener`; a rejected open shows a red error
      notification (no swallowed promise).

## 2. Tests

- [x] 2.1 Extend/`PreviewDialog` unit tests: explainer line is visible on
      open; help control toggles the popover with the full text; link click
      calls the opener with the README URL; strings follow the active locale.

## 3. Docs

- [x] 3.1 `README.md`: add the «Нормализация» section (what is rewritten; how
      to steer — source-format selector, `ruvox-code` directives, Mermaid
      marker); link the «Возможности» bullet to it; regenerate the
      `README.en.md` mirror from it.
- [ ] 3.2 Propose the additive `CHANGELOG.md` `[Unreleased]` entry as a diff
      for user approval (human-owned file; do not commit without approval).

## 4. Validation

- [x] 4.1 `nix develop -c just lint` and `nix develop -c just test` green.
- [ ] 4.2 Manual pass (checklist to the user): open the Add flow with real
      clipboard text — explainer visible, popover opens, README link opens
      the browser at the right section, layout intact at the minimum window
      size, both locales.
