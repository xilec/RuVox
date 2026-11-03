## Why

The pronunciation dictionary can be extended only from the normalization
preview: select a word in the original pane and press «В словарь». The preview
exists only in the add and regeneration flows, so fixing a wrong reading
noticed while listening and following the text in the viewer requires the
context menu → «Перегенерировать аудио» → select the word in the dialog →
«В словарь» → cancel the regeneration detour. That is the most common moment
the problem appears — during playback with the viewer open.

The viewer already has a custom context menu (`viewer-copy-actions`), and the
dictionary editor with prefill (`DictionaryModal` + `initialFrom`) plus the
shared token validator (`isSingleSourceToken`) already exist; only the entry
point and the follow-up re-synthesis are missing.

## What Changes

- The viewer context menu gains «В словарь» when the selection inside the
  viewer is exactly one valid source token, in every display mode (plain,
  Markdown, HTML). Multi-word, invalid and empty selections keep the
  copy-only menu; the item is rendered before the copy actions.
- Choosing it opens the existing dictionary editor prefilled with the selected
  word, exactly like the preview quick-add: the scope selector, the add form
  and the immediate-save path are unchanged.
- The editor reports a successful write back to the shell (a new `onSaved`
  callback fired after each successful `save_dictionary`/import). When the
  editor was opened from the viewer and at least one write succeeded, closing
  it offers «Переозвучить сейчас?»: confirming runs the existing
  `regenerate_entry` for the entry that was selected when the editor opened,
  declining changes nothing, and a failed regeneration keeps the previous
  audio playable.
- Opening and closing the editor never touches the player; only the confirmed
  regeneration stops the current playback (the existing backend behavior).
- RU and EN catalogs get the new strings; component and integration tests
  cover the menu gate in all three modes, the prefill, the save, the prompt
  and both prompt decisions, and the untouched playback on open/close.
- No backend, IPC or storage changes: `save_dictionary` and `regenerate_entry`
  already cover persistence and re-synthesis.

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `user-dictionary`: a new "Viewer quick-add" requirement defines the menu
  entry point, the prefilled editor, the post-save regeneration offer and its
  decision semantics (confirm/decline, stale-audio retention), and that the
  editor does not interrupt playback.
- `viewer-copy-actions`: the "Viewer context menu" requirement gains the
  dictionary item and its applicability rule (a single valid source token,
  rendered before the copy items); the existing copy items are unchanged.

## Impact

- Affected code: `src/components/ViewerContextMenu.tsx`,
  `src/components/TextViewer.tsx`, `src/components/AppShell.tsx`,
  `src/dialogs/DictionaryModal.tsx`, `src/dialogs/RegeneratePrompt.tsx`
  (new), `src/i18n/ru.ts`, `src/i18n/en.ts`, component and integration
  tests, `CHANGELOG.md`.
- No IPC command, storage format or normalization change: the change reuses
  `save_dictionary` and `regenerate_entry`.
- User-visible: a word noticed as mispronounced while reading can be added to
  the dictionary without leaving the viewer, with an immediate offer to
  re-synthesize the entry.

## Non-goals

- Multi-word dictionary entries (#278).
- Automatic re-synthesis on any dictionary change — regeneration stays a
  confirmed, per-entry action.
- Editing the pronunciation inline in the viewer (the existing editor is
  reused as-is).
- A dedicated hotkey on the selection: layout-dependent (Cyrillic layouts) and
  undiscoverable; can be layered on later.
- Changing the player behavior, the queue regeneration flow or the preview
  quick-add.
