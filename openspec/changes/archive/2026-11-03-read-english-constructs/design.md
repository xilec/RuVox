## Context

The numeric span layer already turns a construct into one span (`numeric/parse.rs`)
and reads it in Russian (`numeric/read.rs`). #369 added the English-context
decision and the English cardinal reader (`numeric/english.rs`), and #370 made
the native route spell the reading; both light up only for a standalone integer.
The layer now needs the same decision for shapes whose reading is not one
cardinal: a unit, percentage, currency, date, time, version, range, reference or
ordinal.

## Goals / Non-Goals

**Goals**

- One construct span keeps its shape and is read in English as a whole when the
  #369 rule holds for that span.
- The narrated route renders the reading through the existing curated readings
  (approximated English in Cyrillic), with no new route or engine change.
- The wording is data: one table per construct family, and the same wording can
  serve the native route later (#372).

**Non-Goals**

- Native construct rendering (#372); compound/mixed constructs; arithmetic;
  phone numbers, Roman numerals, city abbreviations, comma decimals, slash
  fractions, pre-release version suffixes and the `г.`/`гг.` abbreviations.

## Decisions

- **A construct reading is Latin English words, transliterated by the route.**
  `constructs::read` returns the same kind of string `english::cardinal` returns
  (`"fifty percent"`), and the narrated arm calls `english::transliterate`. The
  native construct change (#372) can reuse the Latin string without touching the
  wording.
- **The construct words extend the existing reading table.** `english::reading`
  chains the construct tables from `constructs` (unit words, currencies, months,
  ordinals, function words) instead of putting a second lookup path next to it,
  so the "every emitted word has a reading" invariant stays one test.
- **Construct claims are narrated-only in this change.** `constructs::read` runs
  only when the route is `Narrate`; the native route keeps the Russian reading
  until #372, exactly as the issue asks.
- **Measurements respect `expand_units`.** Percent, unit, currency and scale
  constructs are claimed only when the profile expands units, mirroring the
  Russian reader's `owns_measurement` guard; dates, times, versions, ranges,
  references and ordinals expand regardless.
- **A shape is a construct only when it is not a bare integer.** A plain
  integer stays with #369's claim predicate (and its code/zero-padded guards);
  `constructs::read` returns `None` for it, so the two predicates cannot race.
- **Wording.**
  - Units: English singular at exactly one, plural otherwise ("1 MB" → "one
    megabyte", "50 MB" → "fifty megabytes"); fixed units keep their name ("em",
    "rem", "vee aitch", "vee double you"); `°C` is "degree(s) Celsius",
    "км/ч" is "kilometers per hour", "м/с" is "meters per second".
  - Percent: invariant "percent" ("1 percent", "50 percent").
  - Currency: singular at one, plural otherwise ("$5" → "five dollars", "$1" →
    "one dollar"); the symbol position does not change the wording.
  - Scale: invariant "thousand"/"million"/"billion" ("5 млн" → "five million").
  - Dates: "the <day ordinal> of <Month>, <year register>" — day-first matches
    the source order and stays unambiguous ("12.05.2024" → "the twelfth of May,
    twenty twenty-four"); without a year the trailing part is dropped.
  - Year-month: "<Month> <year register>" ("2024-12" → "December twenty
    twenty-four").
  - Times: "<hour> <minutes>" with "oh" before a one-digit minute and the hour
    alone at zero minutes ("09:30" → "nine thirty", "09:05" → "nine oh five");
    seconds append "and <n> seconds" when non-zero.
  - Versions and dot decimals: groups joined by " point ", a leading-zero group
    read digit by digit ("1.2.3" → "one point two point three", "1.02" → "one
    point zero two").
  - Ranges: "<left> to <right>"; a bound that is a year uses the year register
    ("2020–2024" → "twenty twenty to twenty twenty-four"), otherwise cardinals.
  - References: "number five", "page fifteen", "table three", "figure three",
    "chapter two"; phone, tax-id and article references stay Russian.
  - Signs and approximations prefix the reading ("minus", "plus", "about",
    "roughly", "plus-minus") for a construct that is claimed for another reason;
    a bare signed number stays Russian, as in #369.
- **Submodule layout.** `numeric/english.rs` becomes `numeric/english/mod.rs`
  (cardinal reader, readings, year register) plus `numeric/english/constructs.rs`
  (shapes and the construct word tables). `numeric/lexicon.rs` gains a
  `UnitKind` on `UnitForms` and English forms on `Currency`/`Scale`, so the
  English table keys by identity, not by matching the Russian word.

## Risks / Trade-offs

- **Transliteration quality** of new words is judgement, and the readings are
  the product: the table is curated and locked by a test that every emitted word
  has a reading.
- **The claim gets wider**: a sentence that looked Russian could now read a
  construct in English. The #369 sentence guard (no Cyrillic word) covers it,
  and the Russian context tests stay.
- **Allocation cost** for construct wording in documents that then fail the
  context check; the perf budgets include integer runs where the construct
  predicate returns early (not a bare integer), so the added cost is bounded by
  the number of construct spans.

## Migration Plan

No data or API migration: the change is text normalization only. The narrated
readings change for English-context constructs; Russian text is byte-identical.
The golden fixture corpus is extended, not rewritten, apart from the new
fixtures.

## Open Questions

None — the wording decisions above close the issue's open questions.
