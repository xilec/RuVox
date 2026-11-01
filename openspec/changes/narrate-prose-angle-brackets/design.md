## Context

The pipeline's phase order ends its symbol work with two passes: the operator
phase (`TRACKED_OPERATOR_KEYS`, multi-character operators only) and the
special-symbols phase (Greek letters, math symbols, arrows, the "~" suffix).
Single `<`/`>` are in the `SYMBOLS` table — used by inline-code and identifier
normalization — but no prose phase ever consults it for them.

The failure that surfaced this: a TeraTTS regeneration of an article about an
SSH honeypot aborted with `text_invalid_tags` on
`… /home/*/.ssh 2>/dev/null …`. The path phase had already turned `/dev/null`
into "слэш дев слэш налл" and the number phase turned `2` into "два", so the
text handed to the engine read "два>слэш дев слэш налл". TeraTTS treats `<`/`>`
as tag syntax, so its adapter rejected the segment (`teratts-engine` spec: a
stray delimiter is a typed error). Silero and Piper never complained because
their alphabet/charset gates drop the character — the same text was silently
losing the redirect.

The two engines therefore disagree about the same pipeline output: one hard
error, the others silent data loss. That is a pipeline bug, not an adapter one.

## Goals / Non-Goals

**Goals:**

- The narrated text never contains a bare `<`/`>` outside a native code span.
- The symbols are *spoken*, not dropped: a comparison stays a comparison and a
  shell redirect stays audible.
- Multi-character operators keep their existing, more specific readings.
- Native code blocks are untouched (they are restored after the narrating
  phases and the TeraTTS adapter already neutralises their delimiters).

**Non-Goals:**

- A shell-redirect vocabulary (`2>` → "редирект", `>>` → "дописать"). Single
  brackets only; `>>` reads as two "больше".
- Covering other ASCII symbols the alphabet gate drops (`|`, `$`, `_`, `[`).
- Changing the TeraTTS validation or making the adapter lenient for prose.
- Stripping Markdown autolink delimiters (`<https://…>`).

## Decisions

### D1. Narrate the brackets inside the existing special-symbols phase

The narration is a loop over the two brackets placed right after the
Greek/math/arrow loop in the same phase (Phase 14), using `TrackedText::replace`
with a space-surrounded reading, exactly like its neighbours. The reading itself
comes from `SymbolNormalizer`'s shared `SYMBOLS` table — the same one inline
code and identifiers already use — so the wording has one home (a review
follow-up replaced the initial literal pair; the output is byte-identical).

Rejected: adding `<`/`>` to `TRACKED_OPERATOR_KEYS`. That list is "operators
with a longer form worth matching first" and its entries are matched *before*
the symbol phase; putting single characters there would blur that meaning and
would still miss `<`/`>` when the operator loop is diffed against the spec's
operator set. Keeping the rule in the symbol phase leaves the phase order —
which the spec pins — untouched.

Rejected: a new dedicated phase. It would force an edit to the "Fixed phase
order" requirement and its enumerated list for no behavioral gain.

### D2. Space-surrounded readings, matching the symbol/operator convention

`" {} "` padding is what the neighbouring operator and symbol rules emit; the
final whitespace post-processing collapses the runs. Using a bare replacement
would glue the reading to the neighbouring token ("двабольшеслэш") and break
word boundaries in the mapping.

### D3. The adapter stays strict

TeraTTS must keep failing on a stray `<`/`>`: it is the only signal that a
pipeline path forgot a fragment, and stripping in the adapter would silently
reproduce the Silero behaviour the project is trying to leave behind. The bug
is fixed where the text is produced.

Rejected: neutralising `<`/`>` everywhere in the TeraTTS adapter (the
code-block neutraliser extended to prose). It would convert a loud, fixable
gap into silent loss and hide the next regression.

### D4. Structural lock: a fixture plus a corpus-level invariant test

- A golden fixture pinned to the `teratts` profile (`teratts/shell_redirect`)
  carries the incident line and pins the exact output and the character map. It
  cannot live in the flat corpus: the line contains digits and Latin tokens whose
  readings legitimately differ per profile (`2` is narrated under `silero` but
  kept native under `native_en`), so only a profile-specific case can assert one
  expected string. The bug is TeraTTS-specific anyway — that is the engine that
  rejected the text.
- The existing `silero/markdown_code_block_directive_legacy` fixture is
  re-baked: its input is a legacy `ruvox-code` HTML comment that the code-block
  handler deliberately leaves in the text ("the comment survives as text for the
  symbol phases", see `legacy_directive_does_not_switch_mode`), so the leading
  `<` now reads "меньше" instead of reaching the engine as a literal. The
  source-level `markdown_blockquote_markers_are_not_read` test is updated for the
  same reason: an inline comparison is narrated now, not read as a blockquote
  marker.
- A unit test drives the public `process_for_engine` over a handful of
  markup-shaped inputs (redirect, comparison, pasted tag, autolink) under every
  shipping profile and asserts the result has no `<`/`>` left. The test states
  the invariant the old code violated rather than the single input's output, so a
  new phase that reintroduces a stray bracket fails it.

## Risks / Trade-offs

- A `<` in prose becomes "меньше" even when it was meant as a quotation mark or
  a decorative bracket; the alternatives (drop, error) are worse.
- A pasted HTML-looking tag now reads "меньше див больше" instead of being
  dropped by the engine — longer, but faithful.
- Markdown autolinks (`<https://…>`) read the brackets out loud. Accepted for
  now; a dedicated Markdown rule is a separate change.
- The `silero/markdown_code_block_directive_legacy` golden output changes (a
  pasted HTML comment now starts with "меньше"). It is the only corpus case
  whose output moves: every other fixture that contains `<`/`>` has them in
  native code spans or in Markdown blockquote markers that are stripped before
  this phase.

## Migration Plan

None — no stored data, API or profile changes. Existing history entries are
re-normalized on the next synthesis or regeneration, so a text that used to
fail on TeraTTS becomes synthesizable without user action.
