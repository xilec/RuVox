# Proposal: Fix number normalization bugs (#75, #76)

## Summary

Fix two pre-existing bugs in the number-related phases of the text
normalization pipeline that produce corrupted or silently lost speech:

1. **#76 — decimal percentages lose the `%`:** the Versions phase runs before
   the Percentages phase and consumes `12.5` out of `12.5%`, leaving a bare
   `%` that Silero cannot read. `Рост на 12.5%` becomes
   `Рост на двенадцать точка пять%` instead of
   `Рост на двенадцать точка пять процентов`.
2. **#75 — multi-digit numbers corrupted by value-based replacement:**
   `process_numbers_tracked` applies replacements by string value
   (`TrackedText::replace`), which replaces *every* occurrence of the matched
   substring. When one match is a substring of another (`10` and `1` in
   `10:1`), the first replacement corrupts the second:
   `10:1` → `один0:один` instead of `десять:один`.

## Capabilities

- `text-pipeline` (modified)

## Non-goals

- The same replace-by-value pattern in `process_english_tracked` (latent bug
  with overlapping English words, e.g. `uninstalled install`) — separate
  issue, out of scope.
- The latent `followed_ok` skip of numbers before a sentence-ending dot
  (`Встреча в 5.` keeps the digit) — separate issue, out of scope.
- No changes to normalization output for inputs not affected by the two bugs;
  all existing golden fixtures except `percentage_decimal` (which currently
  codifies bug #76) must keep passing unchanged.

## Approach

- Run the Percentages phase before the Versions phase (phase swap, no regex
  changes).
- Apply number replacements positionally via the existing
  `TrackedText::replace_byte_range` instead of by value.
- Update the `percentage_decimal` golden fixture to the corrected output and
  add regression coverage for the `10:1` ratio case.
