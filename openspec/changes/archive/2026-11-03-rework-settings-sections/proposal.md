## Why

Settings is one long scroll: engine, voices, reading, dictionary, notifications,
preview, cache/logs, tray/theme/language and updates share a single column, the
save button lives at the very bottom, and there is no way to jump to a setting
(#355). The dialog has outgrown a comfortable single column now that the reading
typography controls and the dictionary block live there too.

## What Changes

- Two-pane modal of about 900 px (never wider than the viewport): a section nav
  with the search field on the left, an independently scrolling content pane on
  the right, and a footer pinned below both panes with «Сброс» and «Сохранить».
- Seven sections grouping every existing control: «Движок и голос», «Чтение
  текста», «Уведомления», «Хранилище», «Интерфейс», «Словарь», «О программе».
  The engine coercion alert, the model-download entry, the reading width/scale
  controls from #354/#357, the data/logs folders and the nested dialogs all keep
  their current behavior.
- Search above the nav: filters the rows by labels from **both** the RU and EN
  catalogs plus a small synonym map; the matched substring is highlighted,
  sections without matches disappear, Enter jumps to the first match, Esc clears
  the query before closing the dialog, and an empty result shows a no-results
  state.
- Dirty state: a dot on every section owning a changed field and «Изменено: N»
  (counted per field) in the footer, computed against the loaded config; the
  coerced engine does not mark «Движок и голос» dirty.
- Below about 700 px the nav collapses into a `Select` above the content.
- Tests covering the layout contract (row reachability, the identical
  `UIConfigPatch`, search in both languages, dirty markers, language relabel,
  nested dialogs, sticky footer, narrow select), the `ui` delta spec and a
  CHANGELOG `[Unreleased]` entry.

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `ui`: "Settings dialog" gains the two-pane section layout, the cross-catalog
  search, the per-section dirty markers, the sticky footer and the narrow-window
  section select, with scenarios pinning each behavior.

## Impact

- Affected code: `src/dialogs/Settings.tsx` (orchestrator), new
  `src/dialogs/SettingsSections.tsx` (section bodies), new
  `src/dialogs/Settings.module.css` (layout), new `src/lib/settingsSections.ts`
  (section catalogue, search index, dirty mapping), `src/i18n/ru.ts`,
  `src/i18n/en.ts`, `src/dialogs/Settings.test.tsx`, new
  `src/dialogs/Settings.sections.test.tsx`, new
  `src/lib/settingsSections.test.ts`, `openspec/`, `CHANGELOG.md`.
- No IPC, config-schema or Rust changes: the submitted `UIConfigPatch`, its
  validation and the engine coercion stay exactly as before.
- User-visible: findable settings, section-level unsaved markers, a save button
  that never scrolls away, and a nav usable in a narrow window.

## Non-goals

- Autosave, per-section discard or per-profile settings.
- Any new setting, or a change to validation, engine coercion and save
  semantics.
- Moving Settings out of the modal or splitting it into routes.
- New UI strings beyond the section titles, search, dirty and no-results labels
  in RU and EN.
