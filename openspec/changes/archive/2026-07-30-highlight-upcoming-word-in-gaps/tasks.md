# Tasks: Highlight the upcoming word in gaps

## Implementation

- [x] `src/lib/wordHighlight.ts` — `findActiveTimestamp` returns the closest
  upcoming word's index in gaps (`return lo` when `lo < len`); JSDoc
  documents the sorted-by-`start` invariant.
- [x] `src/lib/wordHighlight.ts` — new dev-only
  `debugAssertSortedTimestamps()` (`import.meta.env.DEV` gate,
  `console.assert` per out-of-order pair).
- [x] `src/components/TextViewer.tsx` — call `debugAssertSortedTimestamps`
  at both timestamp-load sites (prefetch effect and `playbackStarted`
  handler).

## Tests

- [x] Update `wordHighlight.test.ts`: before-first and gap cases expect the
  upcoming index; unsorted case pins the new documented answer.
- [x] New cases: gap picks the immediate next word among several; past the
  last word stays -1; `debugAssertSortedTimestamps` asserts on unsorted
  input and is silent on sorted input.

## Validation

- [x] `nix develop -c pnpm test:unit` green.
- [x] `nix develop -c just lint` green.
- [x] openspec validate highlight-upcoming-word-in-gaps --strict green.
