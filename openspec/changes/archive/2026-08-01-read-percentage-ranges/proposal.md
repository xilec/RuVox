# Proposal: Read percentage ranges "10-20%" as "от десяти до двадцати процентов" (#112)

## Summary

After the #76 phase swap (Percentages before Ranges), "10-20%" reads as
"десять-двадцать процентов": the percentage phase consumes "20%" first, the
range phase never sees "10-20", and the dash is left bare. The pre-#76
output ("от десяти до двадцати%") was equally broken — a bare "%" Silero
cannot read. Neither variant is acceptable.

Read percentage ranges as a unit: "10-20%" → "от десяти до двадцати
процентов".

## Capabilities

- `text-pipeline` (modified — Ranges and percentages; Fixed phase order)

## Non-goals

- Decimal bounds ("10.5-20.5%") — the plain range phase is integer-only
  too; falls back to the current (imperfect) behavior, unchanged.
- No change to plain ranges ("10-20") or plain percentages ("20%").

## Approach

New `re_percentage_range()` (`\b(\d+)\s*-\s*(\d+)\s*%`) and a Percentage
ranges phase immediately before Percentages: `normalize_range` on the
`N-M` part + the fixed genitive-plural "процентов" (always correct after
"до <genitive>"). Unit tests + golden fixture trio `range_percent.*`.
