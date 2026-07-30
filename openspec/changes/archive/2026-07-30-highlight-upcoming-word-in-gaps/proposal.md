# Proposal: Highlight the upcoming word in gaps between words (#78)

## Summary

`findActiveTimestamp` returns -1 for playback positions in a gap between
words, so the highlight blinks out during every pause. A leftover comment
("find the closest upcoming word") shows the intended behavior was never
implemented — both post-loop branches return -1.

Resolve gap positions to the closest upcoming word (the next word lights up
ahead of its interval), document the sorted-by-`start` invariant the binary
search relies on, and assert it in development builds where timestamps are
loaded.

## Capabilities

- `word-highlight` (modified — Active word detection)

## Non-goals

- No threshold on the gap size: any position before the next word's start
  highlights that word, however long the pause.
- No runtime validation in production builds: the sorted invariant is
  guaranteed by all current producers (ttsd, piper, silero-native) and is
  asserted in dev builds only.
- No change to span lookup, styling, or auto-scroll.

## Approach

- `findActiveTimestamp`: after the binary-search loop, `lo` already points
  at the first word with `start > positionSec` — return it instead of -1;
  return -1 only when `lo` is past the end.
- Document the sorted invariant in the function's JSDoc.
- New exported `debugAssertSortedTimestamps()` (dev-only, `console.assert`)
  called at both timestamp-load sites in `TextViewer.tsx`.
