# Proposal: Read leading-dot decimals as "ноль точка …" (#147)

## Summary

A bare decimal fraction without the integer part (".5" in "Вес .5 кг") is
never read: the `preceded_ok` guard in `process_numbers_tracked` skips any
number directly preceded by a dot, so the digit survives to Silero, which
cannot read it. This is the mirror case of #111 (terminal dot), deliberately
deferred there because a naive fix would read ".5" as a bare "пять".

Read leading-dot decimals as proper decimals: ".5" → "ноль точка пять"
(decision made with the user over "точка пять" and wontfix).

## Capabilities

- `text-pipeline` (added — leading-dot decimals; modified — fixed phase
  order)

## Non-goals

- Comma form (",5") — not reported, not handled anywhere else either.
- Dots preceded by a letter, digit, underscore, dot, or path separator stay
  untouched: those are float tails ("1.5"), dotted labels ("example.5"),
  version chains, and path fragments owned by earlier phases.
- No change to how "0.5" (with explicit zero) is read — already covered by
  the version/float path.

## Approach

New pipeline phase right after Versions (before Operators): match
`(^|[^\p{L}\p{N}_./\\])\.(\d+)` and replace with the boundary char(s) plus
`normalize_float("0." + digits)` — reusing the existing float reading so
".5" and "0.5" sound identical. Unit tests in `pipeline/mod.rs` pin the
reading, the text-start case, and the excluded letter-preceded case.
