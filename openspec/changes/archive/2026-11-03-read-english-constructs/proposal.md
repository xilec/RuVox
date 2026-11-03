## Why

The English-context rule (#369) covers a standalone cardinal only. A number that
owns a unit, percentage, currency, date, time, version, range, reference or
ordinal suffix still reads as a Russian phrase inside English prose on the
narrated route:

| Input | Current normalized text |
|---|---|
| `the API grew 50%` | English prose transliterated + `пятьдесят процентов` |
| `shipped 100 MB` | English prose transliterated + `сто мегабайт` |
| `version 1.2.3` | English prose transliterated + `один точка два точка три` |
| `on 12.05.2024` | English prose transliterated + `двенадцатое мая две тысячи двадцать четвёртого года` |
| `1st release` | English prose transliterated + `первый релиз` |

There is no engine boundary on this route, but the construct is the only Russian
fragment in the phrase — and the fragment the listener most needs to parse.

## What Changes

- The context rule applies per numeric construct, not only per bare number: the
  construct span is claimed as one whole (`100 MB`, `12.05.2024`, `10–20%`,
  `1.2.3`, `№ 5`), and the neighbour and sentence tests run against the span
  boundaries.
- A new English construct lexicon supplies the words the numeric layer lacks:
  unit names with English plural agreement, percent, currency, month names,
  ordinals, and the wording of dates, times, ranges, versions, references and
  year-months.
- On the narrated route an English-context construct is replaced with the
  transliterated English reading through the same path as the bare numbers
  (`50%` → «фифти персент», `100 MB` → «уан хандред мегабайтс»).
- Dates, year-months and year ranges reuse the English year register.
- Russian context, the native route and every construct outside the list keep
  their current readings.

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `text-pipeline`: the Numbers, Sizes, durations, and versions, Dates and times,
  and Ranges and percentages requirements gain the English construct reading.

## Non-goals

- The native register of these constructs (#372).
- Compound and mixed constructs (`$5 per MB`, `10–20% of 1 GB`) and arithmetic.
- Phone numbers, Roman numerals, city abbreviations, comma decimals, slash
  fractions and pre-release version suffixes keep their Russian readings.

## Impact

- `tts-normalize`: `numeric/english.rs` becomes `numeric/english/` with a
  `constructs` submodule; `numeric/lexicon.rs` gains the unit/currency/scale
  identities the English table needs; the numeric layer's claim path grows a
  construct branch.
- Fixtures: narrated construct fixtures (English context and the Russian
  counterpart); the golden corpus and the perf budgets stay green.
- `CHANGELOG.md` `[Unreleased]` note.
