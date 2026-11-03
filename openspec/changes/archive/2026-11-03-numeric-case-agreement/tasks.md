## 1. Noun case forms

- [x] 1.1 Derive dative and prepositional forms for units, currencies,
      percentages and scale words from the nominative base, with an explicit
      fallback for indeclinable nouns and fixed units.
- [x] 1.2 Route the preposition case through the numeral and the noun in
      `read_unit`, `read_currency`, `read_percent`, `read_scale` and the
      fraction tails.
- [x] 1.3 Keep dates and ordinal suffixes on their existing case path.

## 2. Fractions

- [x] 2.1 Keep the unit in the singular genitive after an adjectival fraction
      ("около 1/2 кг" → "около одной второй килограмма").
- [x] 2.2 Decline the unit with the surrounding case after a simple fraction
      ("к 1,5 кг" → "к полутора килограммам").

## 3. Magnitude consistency

- [x] 3.1 Read dotted groups above the cardinal limit, exact 10^12 and
      unparsable groups digit by digit, through one magnitude helper shared with
      `version_digits`.

## 4. Corpus and tests

- [x] 4.1 Add locked cases for the dative and prepositional readings of units,
      currency, percentages, scale words and fractions.
- [x] 4.2 Add locked cases for a long dotted group, exact 10^12, a leading-zero
      pre-release suffix, a technical fraction with a unit and the year-range
      veto.

## 5. Validation

- [x] 5.1 `cargo fmt -p tts-normalize` and
      `cargo clippy -p tts-normalize --all-targets -- -D warnings` are clean.
- [x] 5.2 `cargo test -p tts-normalize` passes; the locked corpus and the
      target corpus (199/199) stay green.
- [x] 5.3 `cargo test --manifest-path src-tauri/Cargo.toml` passes, golden
      fixtures unchanged.
