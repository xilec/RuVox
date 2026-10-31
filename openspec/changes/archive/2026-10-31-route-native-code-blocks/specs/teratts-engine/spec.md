# Delta: teratts-engine

## MODIFIED Requirements

### Requirement: Language tags and text validation

The model requires every fragment to be wrapped in a language tag. The adapter
SHALL wrap each language segment in its own top-level tag: `<ru>…</ru>` for
Russian and `<en>…</en>` for English. Tags SHALL be top-level only: nested
tags, an unknown tag name, an unbalanced tag, or a stray `<`/`>` outside a tag
MUST fail with a typed invalid-text error rather than reach the model. The
pipeline SHALL NOT emit tags; wrapping is the adapter's job, and a segment's
language is chosen from its span annotation, never guessed from the text.

A native code-block span is the one place where segment text may legitimately
contain the tag delimiters. The adapter SHALL replace `<` and `>` inside a
code-block span with spaces before wrapping, because code may contain generics
or comparisons (`Vec<T>`, `a < b`) and the model interprets those characters
only as tag syntax. The codepoint count SHALL stay unchanged so spans, word
timestamps and the `CharMapping` remain valid, and the substitution SHALL be
limited to code-block spans: every other stray `<`/`>` still fails with the
typed error.

#### Scenario: Russian text is wrapped

- GIVEN the normalized text "привет мир"
- WHEN the adapter prepares it for synthesis
- THEN the model input is `<ru>привет мир</ru>`

#### Scenario: English segment is wrapped

- GIVEN a segment with language `en` and text "Kubernetes"
- WHEN the adapter prepares it for synthesis
- THEN the model input is `<en>Kubernetes</en>`

#### Scenario: Nested tags are rejected

- GIVEN the text `<ru>привет <en>world</en></ru>`
- WHEN the adapter validates it
- THEN it fails with a typed invalid-text error

#### Scenario: Stray angle bracket is rejected

- GIVEN a Russian text `привет < мир`
- WHEN the adapter validates it
- THEN it fails with a typed invalid-text error

#### Scenario: Angle brackets in native code are neutralised

- GIVEN a normalized text with a code-block span whose text is `Vec<T>`
- WHEN the adapter prepares the segments for synthesis
- THEN the English segment reads `Vec T` inside the `<en>` tag, its codepoint
  count is unchanged, and synthesis does not fail

### Requirement: No double normalization

The adapter SHALL NOT expand numbers, transliterate Latin text, substitute
symbols or units, or run `num2words`. Those decisions belong to the pipeline
and are declared by the text profile (`digits: Expanded`, `units: Expanded`,
and the per-kind fragment routes, which may be `Native`). The only text
transformations the adapter performs are tag wrapping, neutralising tag
delimiters inside a native code-block span, RUAccent dictionary accentuation,
NFKD and the codepoint gate.

#### Scenario: Expanded numbers are not expanded again

- GIVEN the pipeline output "двадцать пять процентов"
- WHEN the adapter prepares it
- THEN the text still reads "двадцать пять процентов" and contains no digits

#### Scenario: Transliterated Latin is not transliterated again

- GIVEN the pipeline output "эй пи ай"
- WHEN the adapter prepares it
- THEN the text is unchanged apart from tags and stress marks

#### Scenario: Native Latin is not transliterated

- GIVEN the pipeline output "Kubernetes" in an English segment
- WHEN the adapter prepares it
- THEN the text is unchanged apart from tags and stress marks
