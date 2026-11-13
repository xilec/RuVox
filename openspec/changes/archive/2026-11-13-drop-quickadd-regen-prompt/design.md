## Context

The viewer quick-add flow is: select one word in the text → «В словарь» → the
dictionary editor opens prefilled → the user writes the pronunciation and the
entry saves immediately → the editor closes and the user selects the next
word. The implementation also raised a bespoke `RegeneratePrompt` whenever the
session had written to the dictionary, offering «Переозвучить сейчас?» for the
entry that was selected when the editor opened.

That offer was introduced with the quick-add flow so a dictionary fix would
not leave the recording sounding wrong. In practice the entry context menu
already exposes the same regeneration (with its preview dialog), and adding
several words in a row produces a modal after every close — the user called it
noise rather than help. The offer also required bookkeeping (the captured
entry, a "was written" flag, a special close path, a bespoke modal that is
sequenced after the editor's exit transition) that exists only for it.

## Goals / Non-Goals

**Goals**

- Closing the dictionary editor after a viewer quick-add save does not ask
  anything and does not synthesize.
- Regeneration stays reachable as before: the entry's context menu
  («Перегенерировать аудио») and its existing preview dialog.
- Remove the now-dead surface instead of leaving a dormant component: the
  prompt, its strings, its state and the `onSaved` callback that only fed it.

**Non-Goals**

- Touching the queue context menu, the regeneration command or its preview.
- Changing save/import, the editor's preview controls, or the Settings path.
- Indicating stale audio in the queue: the entry card keeps its current state
  until the user regenerates.

## Decisions

**Remove the offer rather than make it opt-out or throttle it.** A setting or
"don't ask again" flag would still leave the modal in the common path for
first-time users and add state to persist; the context menu already gives the
same action on demand. The user explicitly asked to drop the check, and there
is no data-loss risk: the old recording stays playable and the dictionary
change is already persisted.

**Delete the dead plumbing with the component.** `RegeneratePrompt` is a
bespoke modal only this flow opened (it had to be sequenced after the editor's
close transition, which is why it is not `modals.openConfirmModal`). Keeping it
unused would leave an untested exported component, its four strings and the
`onSaved` callback in `DictionaryModal` — the callback loses its only
production consumer, so it goes too. The modal's `dictionaryRevision` counter
and the Settings restoration in `closeDictionary` are unrelated and stay.

**No new state for "stale" audio.** The queue does not track which recordings
predate a dictionary change today; adding a badge would be a separate feature
and the user did not ask for it. The regeneration entry point is unchanged.

**Replace the requirement through a removal plus an addition.** The change
schema applies MODIFIED requirements as whole-block replacements and refuses
to drop scenarios that the current spec still has, so the offer scenarios
cannot simply disappear from "Viewer quick-add". Removing the requirement and
re-adding it in the paired change `viewer-quickadd-no-offer` keeps the schema
satisfied and the final spec correct; both changes archive together in one
branch, so no committed state ever lacks the requirement.

## Risks / Trade-offs

- [A user who relied on the prompt must open the context menu] → the action is
  one click away on the entry, and its preview dialog makes the result
  explicit before the old audio is replaced.
- [Removing `onSaved` changes `DictionaryModal`'s API] → the prop was optional
  and its only production caller is AppShell; the component's own test for it
  is removed with the prop.
- [The offer's absence could look like a regression to someone who never used
  the context menu] → the user guide already documents both paths; it is
  updated to point at the context menu.
- [A behavior change split over two changes] → each change carries the reason
  and the paired change's name; the removal has no independent value without
  the re-add, and they are reviewed and merged as one branch.

## Migration Plan

None: no persisted state changes, no data migration. Reverting the change
restores the previous prompt without affecting stored entries or audio.

## Open Questions

None.
