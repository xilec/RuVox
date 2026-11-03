# Tasks — viewer dictionary quick-add

## 1. OpenSpec

- [x] 1.1 Proposal, design, tasks and deltas for `user-dictionary`
  ("Viewer quick-add") and `viewer-copy-actions` ("Viewer context menu").
- [x] 1.2 `openspec validate viewer-dictionary-quick-add --strict` and
  `openspec validate --specs --strict`.

## 2. Frontend

- [x] 2.1 `ViewerContextMenu`: optional `onAddToDictionary(word)` prop; the
  «В словарь» item renders for a single valid source token (shared
  `isSingleSourceToken`) before the copy actions.
- [x] 2.2 `TextViewer`: forward `onAddToDictionary` to the context menu.
- [x] 2.3 `DictionaryModal`: optional `onSaved` callback fired after every
  successful dictionary write (submit, delete, import).
- [x] 2.4 `AppShell`: open the editor prefilled from the viewer, remember the
  entry and whether the session wrote anything, and raise the regeneration
  offer on close only for a viewer session with a write; confirm reuses
  `handleRegenConfirm`, cancel changes nothing.
- [x] 2.5 `RegeneratePrompt`: the «Переозвучить сейчас?» confirmation with
  «Переозвучить» / «Не сейчас».
- [x] 2.6 RU/EN strings for the offer.
- [x] 2.7 No player command on open/close; only the confirmed regeneration
  reaches `regenerate_entry`.

## 3. Tests

- [x] 3.1 `ViewerContextMenu`: the item appears for a Latin and a Cyrillic
  token, comes before the copy items, hides for multi-word and invalid
  selections, and dispatches the trimmed word.
- [x] 3.2 `TextViewer`: the item appears in plain, Markdown and HTML display
  modes and dispatches the word.
- [x] 3.3 `DictionaryModal`: `initialFrom` prefills the add form and is
  consumed once; `onSaved` fires after a successful save and not without one.
- [x] 3.4 `AppShell` integration: the menu opens the editor prefilled; the
  save persists via `save_dictionary`; the offer appears only after a save and
  only on close; confirming runs `regenerate_entry` with the captured entry;
  declining issues nothing; opening/closing issues no playback command.

## 4. Validation

- [x] 4.1 `pnpm typecheck`
- [x] 4.2 `pnpm test:unit`
- [x] 4.3 `just lint`
- [x] 4.4 `openspec validate --specs --strict`, then archive the change and
  sync the specs.
- [x] 4.5 CHANGELOG `[Unreleased]` note.
