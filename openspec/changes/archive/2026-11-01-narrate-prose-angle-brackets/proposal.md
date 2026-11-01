## Why

A TeraTTS regeneration hard-failed with `text_invalid_tags` because the
normalized text still contained a bare `>` from a shell redirect
(`… /home/*/.ssh 2>/dev/null crontab -l …`). The pipeline narrates multi-character
operators and the Greek/math/arrow symbol sets, but a single `<` or `>` in
prose has no phase at all: it reaches the engine verbatim. TeraTTS reads those
characters only as tag syntax and rejects the text, while the alphabet-gated
engines (Silero, Piper) silently drop them — so the same text either fails or
loses the comparison/redirect it belonged to, which is exactly the information
loss the normalization layer exists to prevent.

## What Changes

- The special-symbols phase additionally reads a bare `<` and `>` in prose as
  "меньше" and "больше" (space-surrounded, like the existing symbol and
  operator readings), so no engine ever receives markup characters from
  narrated text.
- Multi-character operators keep priority: `>=`, `<=`, `->`, `=>` and the rest
  are consumed by the operator phase that runs before the symbol phase, so the
  single-bracket rule only sees what is left.
- Fenced code blocks routed `Native` are unaffected: they sit behind
  placeholders during every narrating phase and are restored afterwards, and
  the TeraTTS adapter keeps neutralising tag delimiters inside a code-block
  span.
- A golden fixture pins the shell-redirect line from the incident, and unit
  tests pin the narration for comparisons, a pasted tag, a redirect and an
  autolink across every shipping profile.
- TeraTTS's own validation stays strict: a stray `<`/`>` still fails with a
  typed error rather than being silently stripped by the adapter.

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `text-pipeline`: the "Operators and special symbols" requirement gains the
  bare angle-bracket readings and the invariant that narrated prose carries no
  markup characters.

## Impact

- Affected code: `src-tauri/src/pipeline/mod.rs` (special-symbols phase).
- Fixtures: a new `teratts/shell_redirect` golden case for the redirect line
  (profile-pinned: the line's digits and Latin tokens read differently under
  another profile) plus inline assertions for the comparison/tag cases.
- User-visible: texts containing `<`/`>` (shell redirects, comparisons, pasted
  tags) are now read aloud instead of failing on TeraTTS or losing the symbol
  on the other engines.

## Non-goals

- Reading `>>`/`<<` as shell append/heredoc syntax (they stay two single
  brackets; a dedicated redirect vocabulary is a separate decision).
- Narrating other ASCII symbols that the alphabet gate currently drops
  (`|`, `$`, `_`, brackets) — a broader symbol-coverage change.
- Adapter-side stripping of stray markup for prose: that would hide pipeline
  gaps and lose the meaning of the character.
- Stripping Markdown autolink delimiters (`<https://…>`); they are narrated
  like any other bracket pair until a dedicated Markdown rule exists.
- Changing HTML ingestion (`html_source`) or the TeraTTS tag validation.
