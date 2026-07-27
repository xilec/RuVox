# Tasks: Codepoint-based highlight offsets

## Implementation

- [x] `src/lib/wordSpans.ts`: `wrapWordsWithOrigPos` tracks a codepoint
  cursor next to the UTF-16 index; data-orig-* emitted in codepoints.
- [x] `src/lib/plainTextHtml.ts`: per-line offset accumulation in
  codepoints.
- [x] `src/lib/markdown.ts`: parallel codepoint cursor next to the UTF-16
  `indexOf` cursor; codepoint start offset passed to
  `wrapWordsWithOrigPos`.

## Tests

- [x] `wordSpans.test.ts`: astral case asserts codepoint offsets (replaces
  the test that pinned the old UTF-16 behavior).
- [x] `plainTextHtml.test.ts`: astral character before a newline — second
  line offset in codepoints.
- [x] `markdown.test.ts` (new): astral before a word; repeated fragments
  after an astral char get distinct codepoint positions.

## Validation

- [x] `nix develop -c pnpm test:unit` — 69/69 green.
- [x] `nix develop -c pnpm typecheck` + `nix develop -c pnpm lint` green.
- [x] `nix develop -c pnpm dlx @fission-ai/openspec@1.6.0 validate
  codepoint-highlight-offsets --strict` green.
