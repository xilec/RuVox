# Design: Upcoming-word highlighting in gaps

## Context

`findActiveTimestamp` binary-searches the word active at `positionSec`. Its
post-loop `lo` index already is the answer for gaps: by the loop invariant
every word before `lo` has `end <= positionSec` and every word from `lo` on
has `start > positionSec`, so `lo` is exactly the closest upcoming word.
The dead `if`/`return -1` block suggests the author intended to use it and
never did. All three timestamp producers (ttsd, silero-native
char-proportional, piper) emit lists sorted by `start`; nothing asserted
this at the boundary.

## Decision

- Gap → `return lo` (upcoming word). Past the last word's end (`lo ==
  timestamps.length`) → -1, as before.
- The invariant is documented on `findActiveTimestamp` and enforced by a
  dev-only `debugAssertSortedTimestamps()` invoked once per timestamp load
  (both load sites in `TextViewer.tsx`), not per `playback_position` event.

## Rejected alternatives

- **Keep the previous word highlighted through the gap** (common in
  read-along UIs): contradicts the original code's documented intent
  ("closest upcoming word") and the issue's framing; highlighting ahead of
  speech also doubles as a visual prefetch cue. Can be revisited if the
  ahead-of-time highlight feels wrong in practice.
- **Assert inside `findActiveTimestamp`**: it runs on every
  `playback_position` event; an O(n) check per tick is wasteful when the
  list never changes between loads. Checking at load time covers the same
  invariant once per entry.
- **Throw on unsorted input instead of `console.assert`**: unsorted input
  is a producer bug, but crashing playback highlighting in a dev build
  blocks unrelated manual testing; a loud console assertion is enough.

## Testing

Update `wordHighlight.test.ts`: gap/before-first cases now expect the
upcoming index; the unsorted-input case pins the new (still wrong, still
documented) answer. New cases: gap picks the *immediate* next word among
several; position past the last word stays -1; `debugAssertSortedTimestamps`
fires `console.assert` on unsorted input and stays silent on sorted input.
