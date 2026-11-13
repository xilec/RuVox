## Why

After a dictionary quick-add from the viewer (select a word → «В словарь» →
save), closing the editor asks «Переозвучить сейчас?». When several words are
being added in a row the question appears after every save, while the user is
still on their way back to the text to select the next word — it interrupts
more than it helps. Regeneration of a recording is already one click away in
the entry's own context menu, so the automatic offer adds a second, redundant
path to the same command.

## What Changes

- Remove the post-save regeneration offer (`RegeneratePrompt`) and the viewer
  quick-add bookkeeping that raised it: closing the dictionary editor never
  regenerates anything and never issues a player or regeneration command.
- Keep the «В словарь» context-menu action, the editor prefill and the
  immediate-save path unchanged.
- Keep regeneration available through the entry's context menu (the existing
  «Перегенерировать аудио» path with its preview dialog).
- Drop the now-unused `dictionary.quickadd.regen.*` strings and the
  `DictionaryModal.onSaved` callback that existed only to mark a quick-add
  session as written.
- The `user-dictionary` "Viewer quick-add" requirement is removed here and
  re-added without the offer scenarios in the paired change
  `viewer-quickadd-no-offer`: the change schema refuses to drop scenarios from
  a MODIFIED requirement, so the requirement is replaced through a removal
  plus an addition.

## Non-goals

- Changing regeneration itself: preview, command, error handling, stale-audio
  behavior and the queue's context menu stay as they are.
- Changing the dictionary editor's save/import paths, its preview controls or
  the Settings entry point.
- Any automatic re-synthesis on dictionary change: audio stays stale until the
  user regenerates it explicitly.

## Capabilities

### Modified Capabilities

- `user-dictionary`: "Viewer quick-add" no longer offers to re-synthesize the
  entry after the editor closes; regeneration is explicit, from the entry's
  context menu.

## Impact

- `src/components/AppShell.tsx`: remove the offer state, the quick-add refs and
  the `RegeneratePrompt` element; `closeDictionary` only restores Settings and
  bumps the dictionary revision.
- `src/dialogs/RegeneratePrompt.tsx`: deleted.
- `src/dialogs/DictionaryModal.tsx`: drop the `onSaved` prop (no consumer left).
- `src/i18n/{ru,en}.ts`: remove `dictionary.quickadd.regen.*`.
- Tests: `AppShell.dictionary.test.tsx` loses the offer flows and gains a
  "closing after a save offers nothing" check; the `DictionaryModal` test for
  `onSaved` is removed.
- `docs/user-guide.md`: the two paragraphs promising the offer are updated.
- `openspec/specs/user-dictionary/spec.md`: the requirement comes back from
  the paired change `viewer-quickadd-no-offer` with the offer scenarios gone.
