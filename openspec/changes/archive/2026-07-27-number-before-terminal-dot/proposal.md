# Proposal: Read numbers before a sentence-ending dot (#111)

## Summary

The number phase skips any integer immediately followed by `.`, because the
manual lookaround guard (`process_numbers_tracked`,
`src-tauri/src/pipeline/mod.rs:698`) treats every dot as a decimal/version
separator owned by earlier phases. A plain sentence-ending period is not a
separator: `Встреча в 5.` keeps the digit, and Silero silently drops it in
the audio.

Tighten the guard: a trailing dot disqualifies a number only when it is a
real separator, i.e. directly followed by another digit (`3.14`, `1.2.3`).
A dot followed by whitespace, end of text, or a non-digit character is
terminal punctuation, and the number is read normally.

## Capabilities

- `text-pipeline` (modified)

## Non-goals

- No change to the leading-dot side of the guard (`preceded_ok`): `.5`-style
  fragments keep today's behavior.
- No handling of decimal commas or new number formats — this is only about
  the terminal-dot false positive.
- No changes for inputs already consumed by earlier phases (dates, versions,
  floats via the version path) — they never reach the number guard.

## Approach

One-condition change in the `followed_ok` check plus regression unit tests
and a golden fixture; see tasks.md. The alternative (also rewriting
`preceded_ok` symmetrically) was rejected as out of scope and riskier
(`.5` decimals would start being read as bare numbers).
