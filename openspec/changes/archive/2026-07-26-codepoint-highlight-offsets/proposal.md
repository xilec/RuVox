# Proposal: Codepoint-based highlight offsets (#77)

## Summary

The frontend rendered `data-orig-start` / `data-orig-end` span attributes in
UTF-16 code units, while the Rust pipeline's `char_map` (and therefore
`WordTimestamp.original_pos`) is defined in Unicode codepoints
(position-mapping spec, pinned in #52). For BMP-only text the two coincide;
after any astral character (emoji, mathematical alphanumerics) every span
offset drifts by +1 per astral char and word highlighting matches the wrong
span (or none) for the rest of the document.

Bring the frontend offset chain in line with the codepoint contract:
`wrapWordsWithOrigPos` (`src/lib/wordSpans.ts`) tracks a codepoint cursor,
and both callers (`plainToWordHtml` in `src/lib/plainTextHtml.ts`,
`renderMarkdown` in `src/lib/markdown.ts`) pass codepoint-based start
offsets.

## Capabilities

- `word-highlight` (modified)

## Non-goals

- No changes to the Rust side — its codepoint contract is unchanged and
  already correct.
- No changes to span-matching logic (`findSpanByOrigPos` exact-match /
  containment fallback) — with correct offsets it works as designed.
- No rendering/visual changes for BMP-only documents (behavior identical).

## Approach

Minimal, mechanical: a codepoint cursor next to the UTF-16 index in
`wrapWordsWithOrigPos`; `Array.from(...).length` for line/token lengths in
the two callers (markdown keeps its UTF-16 `indexOf` cursor for search
mechanics and maintains a parallel codepoint cursor). Vitest coverage with
astral characters for all three modules.
