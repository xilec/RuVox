## Context

Two quick-add entry points share one gate: `isSingleSourceToken` in
`src/lib/userDictionary.ts` — the preview dialog's «В словарь» action and the
viewer context menu item. The canonical predicate already lives in
`user-dictionary` "Dictionary entry semantics" and matches both that helper
and the Rust validator in `tts-normalize/src/dictionary/mod.rs`: one word of
Latin or Russian Cyrillic letters and digits, at least one letter, at most 64
characters, all letters from one script, ASCII hyphens only as internal
separators between non-empty alnum segments.

The two quick-add requirements restated that predicate instead of referencing
it, and drifted: the viewer text drops the one-script clause (so `IPзамок`
reads as valid) and the preview text calls "punctuation" invalid (so `UTF-8`
reads as invalid). #348 already synced the viewer text, so the fix is a
MODIFIED-delta change, not an edit of `openspec/specs/`.

## Goals / Non-Goals

**Goals:**

- One canonical token predicate; both quick-add requirements reference it.
- Menu placement stated once, in `viewer-copy-actions`.
- Scenarios lock the two shapes that drifted (`UTF-8`, `IPзамок`).

**Non-Goals:**

- Changing the predicate, the gate or any behavior.
- Rewording "Dictionary entry semantics" (it is already accurate).

## Decisions

**D1. "Dictionary entry semantics" stays the single gloss.** Both quick-add
requirements say "exactly one valid source token as defined by "Dictionary
entry semantics"" and drop their restatements. Rejected: copy the gloss into
all three places and keep them in sync by hand — that is exactly how the
current drift happened.

**D2. Placement lives in `viewer-copy-actions` "Viewer context menu".** That
requirement already defines the menu items and their order (and references
"Viewer quick-add" for applicability). The user-dictionary requirement keeps
the applicability rule and points at the copy-actions requirement for
placement; its scenarios assert the item's presence, not its position.
Rejected: keep the order sentence in both — a second place to edit and another
drift source.

**D3. No code change.** `isSingleSourceToken` trims the selection and accepts
`LATIN_TOKEN_RE`/`CYRILLIC_TOKEN_RE` matches: `UTF-8` and `по-русски` pass,
`IPзамок` fails (each regex is single-script), digit-only tokens fail (the
lookahead requires a letter), and a leading, trailing or doubled hyphen fails
(every segment must be non-empty). The Rust validator follows the same shape.
The unit tests in `src/lib/userDictionary.test.ts` and
`src/components/ViewerContextMenu.test.tsx` already cover these cases; the
delta only records them.

## Risks / Trade-offs

- Cross-spec references are textual: renaming either requirement must update
  the reference. A MODIFIED delta header that no longer matches the main spec
  fails `openspec validate --strict`.
- Moving the order out of `user-dictionary` loses one redundant assertion; the
  `viewer-copy-actions` scenario "Menu on a single word" keeps it.
