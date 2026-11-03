# Tasks — Settings sections with search

## 1. OpenSpec

- [x] 1.1 Proposal, design and the `ui` "Settings dialog" delta.

## 2. Navigation and layout

- [x] 2.1 Pure section/row catalogue with both-catalog search terms and the
  field-to-section mapping (`src/lib/settingsSections.ts`).
- [x] 2.2 Two-pane layout: section nav with the search field, independently
  scrolling content, sticky footer; modal about 900 px, capped by the viewport.
- [x] 2.3 Section bodies moved to `SettingsSections.tsx` without changing any
  control or the save semantics.
- [x] 2.4 Below about 700 px the nav collapses into a `Select` above the
  content.

## 3. Search, dirty, footer

- [x] 3.1 Search filters rows by RU and EN labels plus synonyms; the matched
  substring is highlighted; sections without matches hide; no-results state.
- [x] 3.2 Enter jumps to the first match (switch section, scroll into view,
  no submit); Esc clears the query first and closes the dialog only afterwards.
- [x] 3.3 Per-section dirty dots and «Изменено: N» in the footer, counted per
  field against the loaded config; the coerced engine is not a change.
- [x] 3.4 Footer «Сброс» (factory defaults) and «Сохранить»; the X discards.
- [x] 3.5 Nested models/cleanup/dictionary dialogs keep working and return to
  the originating section; the cleanup `closeOnEscape` gating stays.

## 4. i18n and docs

- [x] 4.1 RU/EN keys for the section titles, search, dirty and no-results text.
- [x] 4.2 CHANGELOG `[Unreleased]` entry.

## 5. Validation

- [x] 5.1 Unit tests: every control reachable exactly once; the same
  `UIConfigPatch` for the same edits; search by RU and EN labels; dirty dots and
  the footer count; language switch relabels; nested dialogs return to the
  section; sticky footer; narrow select.
- [x] 5.2 `pnpm typecheck`, `pnpm test:unit`, `just lint`.
- [x] 5.3 `openspec validate --specs --strict` and
  `openspec validate rework-settings-sections --strict`.
