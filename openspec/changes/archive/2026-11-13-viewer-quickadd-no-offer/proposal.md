## Why

The paired change `drop-quickadd-regen-prompt` removes the viewer's
post-save regeneration offer from the application. Removing it from the spec
cannot be expressed as a MODIFIED requirement: the change schema applies a
MODIFIED block as a whole-block replacement and refuses to drop scenarios the
current spec still has, and one change may not both remove and add the same
requirement. So the paired change removes "Viewer quick-add" and this change
re-adds it without the six offer scenarios, keeping the rest of the contract
(menu action, editor prefill, immediate save, playback untouched).

## What Changes

- Add the `user-dictionary` "Viewer quick-add" requirement back with the same
  menu, editor and save contract and without the post-save regeneration offer.
- Keep the existing scenarios for the menu action, the prefill and the save;
  replace the offer scenarios with one that asserts the quiet close.

## Non-goals

- Any behavior change: the implementation ships in the paired change
  `drop-quickadd-regen-prompt`; this change only restores the requirement.
- The regeneration path itself (context menu, preview, command).

## Capabilities

### Modified Capabilities

- `user-dictionary`: "Viewer quick-add" is re-added without the post-save
  regeneration offer.

## Impact

- `openspec/specs/user-dictionary/spec.md`: the requirement returns after the
  paired change removed it. No code, tests or docs change here.
