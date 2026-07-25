# Design: Fix number normalization bugs

## Bug #76: decimal percentages

**Root cause.** Phase order in `TTSPipeline::process_with_char_mapping`
(`src-tauri/src/pipeline/mod.rs`): Versions (phase 10, `re_version`
`(?i)\bv?(\d+\.\d+...)\b`) runs before Percentages (phase 12). The `v?`
prefix is optional, so a bare `12.5` inside `12.5%` matches as a "version"
and is consumed before the percentage regex
(`\b(\d+(?:\.\d+)?)\s*%`) can see it. `%` is present in no symbol table, so
it survives into the TTS input unreadable.

**Chosen fix:** swap the two phases — Percentages before Versions. A version
immediately followed by `%` is meaningless, and there is precedent: Sizes
already runs before Versions for the same reason.

**Rejected alternatives:**

- *Filter version matches followed by `%` in the closure* — the `regex`
  crate has no lookahead and `sub` closures cannot see past the match;
  rebuilding the phase as a collect+filter loop like
  `process_numbers_tracked` is more code and more risk for the same result.
- *Tighten `re_version` (require `v` prefix or ≥ 2 dots)* — breaks intended
  behavior pinned by the `number_decimal` fixture (`3.14` is read via the
  version path as "три точка четырнадцать"). Most invasive option.

## Bug #75: value-based replacement corrupts overlapping numbers

**Root cause.** `process_numbers_tracked` (mod.rs:626-672) collects matches
by byte offset on a snapshot but applies them by value:
`tracked.replace(original, &replacement)`. `TrackedText::replace` replaces
*all* occurrences of the literal. For `10:1` the matches are `10` (0-2) and
`1` (3-4); applied in reverse, `replace("1", "один")` also rewrites the `1`
inside `10`, producing `один0:один`, after which `replace("10", ...)` finds
nothing. Any input with a number that is a substring/prefix of another
(`20:0` → `2ноль:ноль`, `33 3` → `тритри три`) is affected.

**Chosen fix:** apply replacements positionally with the existing
`TrackedText::replace_byte_range(start, end, &replacement)`
(tracked_text.rs:177-229) — one-line change at the call site. The method has
the same overlap guards and no-op skip; the reverse iteration order keeps
earlier byte offsets valid. Precedent: markdown link stripping already uses
this pattern (mod.rs:546-551).

**Rejected alternatives:**

- *New sub-with-positions API in `TrackedText`* — redundant,
  `replace_byte_range` already exists and covers the need.

## Notes

- The `14:30` → `четырнадцать:30` secondary repro from #75 does not
  reproduce on current `main` (the `time_hm` fixture added in #67 covers
  valid `HH:MM` and is green); it was a pre-#67 snapshot. No action needed.
- The golden fixture `percentage_decimal` currently codifies bug #76
  (expected output contains the bare `%`); it is updated to the corrected
  output as part of this change.
