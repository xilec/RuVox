# Tasks — numeric span normalization

## 1. Span scan and parse

- [x] 1.1 Add `NumericSpanNormalizer` in `tts-normalize/src/pipeline/numeric/mod.rs`
      and the `Span`/`Body`/`Trailing` model: scan the text once, skip a digit
      another owner keeps, collect `(start, end, reading)`, and apply the
      accepted readings as one `replace_byte_ranges` batch.
- [x] 1.2 Implement the digit-candidate parser in `parse.rs`: space and NBSP
      three-digit groups, comma fractions, dot groups, slash groups, times,
      ISO/European/slashed dates, year-month, numeric ranges, ordinal and
      version suffixes, mixed fractions.
- [x] 1.3 Parse the trailing construct a number owns: percentage, plus suffix,
      currency symbol, scale word, money abbreviation, year abbreviation,
      degree Celsius, unit tokens; gate the whole family with `expand_units`.
- [x] 1.4 Parse candidates that start with a symbol or an infix: currency
      prefixes, `+`/`-`/`−` signs, `~`/`≈`/`±`, leading dot, `v`-prefixed
      versions, `г.` city/year homograph, `№` and textual references, phones,
      uppercase Roman centuries.
- [x] 1.5 Add start guards so a digit glued to a letter, an underscore, a
      slash, a letter-prefixed hyphen or a dotted label is never partially
      consumed by the layer.

## 2. Morphology and lexicon

- [x] 2.1 Add `morphology.rs`: cardinal numerals with gender/case agreement,
      the natural `count` form that drops a lone leading "одна/один" for a
      scale word, and the scale-word declension.
- [x] 2.2 Add ordinal numerals in all genders, numbers and cases, including
      the `-тысячный` compound and the stressed `-ой` class.
- [x] 2.3 Add decade words and fractional numerals (`полтора`, `с половиной`,
      `N целых M сотых`, simple `1/2` fractions).
- [x] 2.4 Add `lexicon.rs`: unit and currency forms, scale words, genitive and
      nominative months, year-context words, noun genders, reference prefixes
      and ordinal heads, version suffixes.

## 3. Reading

- [x] 3.1 Read numbers: cardinals with head-noun agreement, grouped digits,
      scale words, signs, zero-padded codes, decimal fractions (natural with a
      physical unit, technical "точка" for bare decimals and size units),
      slash and mixed fractions.
- [x] 3.2 Read years and ordinals: year case from the following year word or
      from a month/season/`с … по …` context, the 1900–1999 "тысяча" form,
      ordinal suffixes with gender/number/case, the ambiguous `-й`/`-х`
      resolution, head-noun ordinals, decades and Roman centuries.
- [x] 3.3 Read dates and times: label nominative vs genitive after
      `от/до/с/со/из/около` vs accusative after `по`, the `с … по …` pair,
      written dates, two-digit years, year-month (not a range), clock values,
      time ranges, invalid-time fall-through.
- [x] 3.4 Read ranges and percentages: genitive bounds, year ranges with
      `годы`/`гг.`, adjacent-count hyphen pairs, decimal bounds, shared
      unit/percentage/currency trailing, percentage agreement, decimal
      percentages, approximation words and signs.
- [x] 3.5 Read money and units: currency symbol before or after the number,
      money words, scale words with an optional currency, physical units and
      speed units with their spoken tails, unit agreement by count.
- [x] 3.6 Read phones, references, identifiers and versions: phone groups,
      `№`, `стр.`, `табл.`, `рис.`, `гл.`, `тел.`, `ИНН`, `арт.`, `г. Москва`,
      dotted versions with pre-release suffixes.

## 4. Pipeline wiring and phase removal

- [x] 4.1 Insert the numeric span phase after URLs/emails/IPs/paths and before
      operators; gate it on `digits: Expanded` and pass `units: Expanded` to
      the scanner.
- [x] 4.2 Delete the size, date, time, percentage-range, percentage, range,
      version, leading-dot-decimal and bare-number regex phases, the tilde
      approximation regex, and their helpers (`SIZE_UNITS`, `unit_owns`,
      `sub_number_phase`, `process_numbers_tracked`), then drop the legacy
      numeric reading methods from `normalizers/numbers.rs`
      (`normalize_date`, `normalize_time`, `normalize_size`, `normalize_float`,
      `normalize_version`, `normalize_range`, `normalize_percentage`);
      `normalize_number` stays for the URL/IP and Markdown-list callers.
- [x] 4.3 Move dash normalization after the numeric span phase and comment why
      the order is load-bearing (an already-replaced en dash overlaps the
      range span and the reading is dropped).
- [x] 4.4 Keep the URL, e-mail, IP and file-path regex phases before the
      numeric phase, with a comment that explains address ownership.
- [x] 4.5 Keep the alphanumeric-token phase after the numeric phase so
      "x86"/"v2" are never read as a number plus a stray letter.

## 5. Corpus, goldens and docs

- [x] 5.1 Keep the five corpus categories (`cardinals`, `dates`,
      `money_codes`, `ordinals`, `ranges_percentages`) as the behavior spec:
      256 cases, 57 locked and 199 target.
- [x] 5.2 `corpus_locked_matches_current_behavior` green.
- [x] 5.3 `corpus_target_rate` green at 199/199 (`--ignored`).
- [x] 5.4 Refresh the app-level numeric goldens
      (`leading_dot_decimal`, `range_years`, `tilde_approx`,
      `tilde_approx_space`).
- [x] 5.5 Correct the year-1900 corpus case to the "тысяча девятисотом" form.
- [x] 5.6 Refresh the pipeline unit expectation for the grouped number
      ("42 142" is one number).
- [x] 5.7 Add the `[Unreleased]` CHANGELOG entry.

## 6. Validation

- [x] 6.1 `cargo fmt --check`.
- [x] 6.2 `cargo clippy --manifest-path tts-normalize/Cargo.toml -- -D warnings`.
- [x] 6.3 `cargo test --manifest-path tts-normalize/Cargo.toml`.
- [x] 6.4 `cargo test --manifest-path tts-normalize/Cargo.toml --test corpus`
      (locked set).
- [x] 6.5 `cargo test --manifest-path tts-normalize/Cargo.toml --test corpus --
      --ignored --nocapture corpus_target_rate` (199/199).
- [x] 6.6 `cargo test --manifest-path src-tauri/Cargo.toml --test golden`.
- [x] 6.7 1 MB runtime against main: 8.77 s vs 11.83 s, within the 1.5x cap.
- [x] 6.8 `pnpm dlx @fission-ai/openspec@1.6.0 validate
      numeric-span-normalization --strict`.
