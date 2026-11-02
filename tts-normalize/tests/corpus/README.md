# Number, ordinal and date corpus

Phase E of epic #343: the evaluation harness that phases B and C are measured
against. One TOML file per category; the file stem is the category name.

## Case schema

```toml
[[case]]
id = "dates/iso-release"          # unique across the whole corpus
subcategory = "iso"               # coverage bucket inside the category
status = "locked"                 # `locked` or `target`
input = "Релиз 2024-12-31"
expected = "Релиз тридцать первое декабря две тысячи двадцать четвёртого года"
note = "optional: why this reading, or which ambiguity it pins"
```

`expected` is the full normalized text under the default Russian engine profile
(`silero-native`), not just the number fragment, so a case also pins that the
surrounding words and punctuation survive.

## Statuses

- `locked` — the pipeline already reads the case correctly. The
  `corpus_locked_matches_current_behavior` test fails when one of these drifts,
  which is the regression guard for phases B and C.
- `target` — the natural reading the epic wants. The `corpus_target_rate` test
  is `#[ignore]`d so the known gap does not turn CI red before the epic lands;
  the same test is the yardstick for comparing the B and C branches.

When the winning implementation lands, the target cases it covers flip to
`locked` and stay in the corpus as regression guards.

## Reading policy

Natural spoken Russian is the reference, not a word-by-word transliteration.

- Years are ordinals: "в 2024 году" → "в две тысячи двадцать четвёртом году".
  Years 1900–1999 drop the "одна тысяча" prefix. A lone scale word drops
  "одна/один": "1000 рублей" → "тысяча рублей", "1 000 000" → "миллион".
- Dates: the day is a neuter ordinal, the year a genitive ordinal. A date used
  as a label reads nominative ("Релиз 2024-12-31"); after "от"/"до" it takes
  the genitive, after "с … по …" the genitive/accusative pair.
- Decades and centuries are ordinals: "в 1990-х" → "в девяностых",
  "в XX веке" → "в двадцатом веке".
- Ordinal suffixes (`-й`, `-я`, `-е`, `-го`, `-му`, `-м`, `-ми`, `-ю`, and the
  plural `-х`) carry gender, number and case: "5-й этаж" → "пятый этаж".
  `N-х` is ambiguous: an ordinal when a plural head noun follows ("в 5-х
  классах" → "в пятых классах"), a cardinal genitive otherwise ("из 2-х
  частей" → "из двух частей").
- Cardinal numerals agree with the head noun in gender and number: "42 книги" →
  "сорок две книги", "1 рубль" → "один рубль".
- Bare decimals keep the technical register ("3.14" → "три точка четырнадцать",
  "99.9%" → "девяносто девять точка девять процентов"); a decimal with a
  physical unit reads naturally ("1,5 кг" → "полтора килограмма", "2,5 км" →
  "два с половиной километра"), and other fractions read "N целых M
  десятых/сотых".
- Ranges read "от … до …" with both bounds in the genitive ("5–10 кг" →
  "от пяти до десяти килограммов"). A year range followed by "годы"/"гг."
  reads "с … по … год". A hyphen between two counts no greater than ten that
  is followed by a counted noun keeps the hyphenated reading ("2-3 раза" →
  "два-три раза").
- Phones are read in groups ("+7 999 123-45-67" → "плюс семь девятьсот
  девяносто девять сто двадцать три сорок пять шестьдесят семь"); IDs, ИНН
  and article numbers are read digit by digit; reference numbers (№, стр.,
  табл.) are read as numbers, and a chapter reference as an ordinal.
- Currency and unit symbols are read as words with the correct case and number
  ("5$" → "пять долларов", "1 %" → "один процент", "50 коп." → "пятьдесят
  копеек").

## Running

```bash
# Locked regression guard (runs in CI with the rest of the crate tests).
cargo test --manifest-path tts-normalize/Cargo.toml --test corpus

# Full yardstick with the per-category pass rate.
cargo test --manifest-path tts-normalize/Cargo.toml --test corpus -- --ignored --nocapture corpus_target_rate

# Runtime side: a fixed sample assembled from the corpus (256 KB by default,
# overridable for the branch comparison; the ratio is what matters).
RUVOX_CORPUS_SAMPLE_BYTES=1000000 cargo test --manifest-path tts-normalize/Cargo.toml \
  --test corpus -- --nocapture corpus_runtime_within_budget
```

## Coverage review

The corpus is reviewed for coverage and naturalness before merge: every
category has cases in several buckets, no bucket dominates, and every
`expected` string is what a Russian speaker would actually say. Most inputs are
realistic phrases with context words; a minority are bare tokens kept for
format coverage. Add a case when a real text exposes a reading the corpus
misses.

## Legacy and robustness cases

Some `locked` cases pin behavior that is intentionally not natural speech:

- Markdown list markers ("1. Первый пункт" → "первое:Первый пункт") follow the
  `text-pipeline` spec.
- An invalid clock time ("В 25:00 встреча" → "В двадцать пять:ноль встреча")
  pins the documented fall-through to the number phase, not a desired reading.
