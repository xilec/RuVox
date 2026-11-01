# Delta: text-pipeline

## MODIFIED Requirements

### Requirement: Operators and special symbols

The system SHALL replace the multi-character operators `===`, `!==`, `->`,
`=>`, `>=`, `<=`, `!=`, `==`, `&&`, `||` with their spoken forms ("===" →
"строго равно", "->" → "стрелка"), processing them longest-first via
`TRACKED_OPERATOR_KEYS`; a single `=` MUST NOT be replaced at this phase so
mathematical formulas stay intact. The system SHALL replace Greek letters
(`GREEK_LETTERS`), math symbols (`MATH_SYMBOLS`), and arrow symbols
(`ARROW_SYMBOLS`) with their Russian names, surrounded by spaces. A tilde
directly before a number SHALL be read as "около" ("~46" → "около 46").

The same phase SHALL read a bare `<` and `>` that survived the operator phase
as "меньше" and "больше", also surrounded by spaces, so a comparison, a shell
redirect or a pasted markup tag is spoken instead of reaching the engine.
Multi-character operators take priority: they are consumed by the operator
phase that runs first, so only the remaining single brackets are read here.
Invariant: the text handed to an engine SHALL NOT contain a stray `<` or `>`
outside a native code-block span (those spans are restored after every
narrating phase, and the TeraTTS adapter neutralises their tag delimiters
before wrapping).

#### Scenario: Strict equality operator

- GIVEN the input "a === b"
- WHEN the pipeline processes it
- THEN the operator is read as "строго равно" and not as three separate
  equals signs

#### Scenario: Greek letters

- GIVEN the input "α = β"
- WHEN the pipeline processes it
- THEN the letters are read as "альфа" and "бета" and the single "=" is left
  for later phases, not consumed by the operator phase

#### Scenario: Tilde as approximately

- GIVEN the input "~46 секунд"
- WHEN the pipeline processes it
- THEN the tilde is read as "около" before the number

#### Scenario: Comparison operator keeps its two-character reading

- GIVEN the input "если a >= b"
- WHEN the pipeline processes it
- THEN the pair is read as "больше или равно" and never as a separate
  "больше" plus "равно"

#### Scenario: Shell redirect is narrated

- GIVEN the input "grep -r key /root/.ssh 2>/dev/null"
- WHEN the pipeline processes it with any engine profile
- THEN the redirect's `>` is read as "больше" and the normalized text
  contains no `<` or `>`; under `silero`, `piper` and `teratts` the whole
  fragment reads "два больше слэш дев слэш налл", while
  `native_en` keeps the digit native

#### Scenario: Pasted markup tag is narrated

- GIVEN the input "Тег <div> тут"
- WHEN the pipeline processes it with any engine profile
- THEN the output contains neither `<` nor `>`: the opening bracket reads
  "меньше" in front of the tag name and the closing one reads "больше"

#### Scenario: Native code block keeps its tag delimiters

- GIVEN a fenced code block routed `Native` whose content is `Vec<T>`
- WHEN the pipeline processes it
- THEN the bracket narration does not touch the restored code text, and only
  the adapter neutralises the delimiters before wrapping
