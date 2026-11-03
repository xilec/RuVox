# Tasks

## 1. Construct wording

- [x] 1.1 Add a `UnitKind` to the unit table and English forms to `Currency` and
      `Scale`, so the English wording keys by identity instead of the Russian
      word.
- [x] 1.2 Move `numeric/english.rs` to `numeric/english/mod.rs`; add
      `numeric/english/constructs.rs` with the construct word tables (units,
      currencies, months, ordinals, function words) and their Cyrillic readings.
- [x] 1.3 Add `english::ordinal` and the wording readers: percent, unit,
      currency, scale, ordinal, time, dotted/version, date, year-month, range,
      reference, and the sign/approximation prefixes.

## 2. Routing

- [x] 2.1 Claim a construct as one span on the narrated route when the #369
      context rule holds, respecting `expand_units`, and leave a bare integer to
      the existing claim predicate.
- [x] 2.2 Extend `needs_context` to construct claims and keep `english_context`
      running on the construct span boundaries.

## 3. Tests

- [x] 3.1 Unit tests for the wording: percent, unit plural agreement, currency,
      months, ordinals, time, version, date, year-month, range, reference,
      sign/approximation, and the out-of-scope shapes returning `None`.
- [x] 3.2 Pipeline tests: an English-context construct reads in English on the
      narrated route; Russian context stays byte-identical; the native route
      keeps the Russian reading; the sentence guard keeps "Установи 5 кг.";
      identifiers next to a construct are not prose.
- [x] 3.3 Perf: construct-heavy text stays within the near-linear budget.

## 4. Fixtures and docs

- [x] 4.1 Silero fixtures: one English paragraph with a percentage, a size unit,
      a version, a range and a date read in English; one Russian counterpart
      with the same constructs unchanged.
- [x] 4.2 `CHANGELOG.md` `[Unreleased]` note.

## 5. Validation

- [x] 5.1 `cargo test -p tts-normalize` (lib and corpus suites).
- [x] 5.2 `cargo clippy -p tts-normalize --all-targets -- -D warnings` and
      `cargo fmt --all`.
- [x] 5.3 Golden fixtures: all 119 cases.
- [x] 5.4 Perf budgets.
- [x] 5.5 Archive the change and sync the delta specs.
