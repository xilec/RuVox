# Delta: text-pipeline

## ADDED Requirements

### Requirement: Engine text profile

The system SHALL describe what each engine can read in a data-only text
profile containing: the language codes the engine accepts natively; the
fragment route (see the next requirement); whether the engine reads digits
natively; whether it reads units natively; its stress support (`None`, `Auto`,
`Manual`); the codepoints it accepts (`Any` or an explicit alphabet); its
chunking limit and pause policy (see `text-chunking`); and the markup syntax it
wants around native spans (`None`, `RuEnTags`, `LangTag`).

The profile SHALL contain no behavior — no callbacks, no trait objects — and
SHALL be reachable two ways: from an engine instance through
`TtsEngine::text_profile()`, and from the static per-engine registry keyed by
`EngineKind` that the pipeline and tests use without an engine in hand. The
registry SHALL declare a profile for every `EngineKind` variant, and the
definition SHALL live outside the application crate so that an engine crate can
share it.

Invariant: a profile that routes any Latin fragment `Native` SHALL list at
least one language other than Russian; declaring native Latin reading without a
language to read it in is a specification error, not a runtime state.

Invariant: `units: Expanded` SHALL require `digits: Expanded` — a unit reading
spells its number out ("пятьдесят процентов"), so the pipeline cannot expand a
unit while the engine reads the number.

Invariant: `identifier: Native` SHALL require `latin_prose: Native` — an engine
that reads identifiers natively reads Latin by definition, and the opposite
combination would hand the same engine a Latin token in one place and its
Russian reading in another.

A profile that violates an invariant SHALL be rejected by `TextProfile::validate`
rather than handled at runtime.

#### Scenario: Every engine kind has a profile

- GIVEN the registry
- WHEN it is asked for each `EngineKind` variant
- THEN a profile is returned for every variant, and the profiles differ only
  where the engines genuinely differ

#### Scenario: Native routing requires a non-Russian language

- GIVEN a profile with `latin_prose: Native` and `languages: ["ru"]`
- WHEN the profile invariants are checked
- THEN the check fails

#### Scenario: Incoherent profile combinations are rejected

- GIVEN a profile with `digits: Native` and `units: Expanded`
- WHEN the profile invariants are checked
- THEN the check fails, because a unit reading needs the number in words
- AND a profile with `identifier: Native` and `latin_prose: Narrate` fails too

#### Scenario: Profile is data, not behavior

- GIVEN a profile value
- WHEN it is copied, serialized for a generation snapshot, or compared
- THEN no closure, engine handle or I/O is involved

### Requirement: Fragment route

The profile SHALL decide, per fragment kind, who reads the fragment:
`Narrate` turns the fragment into Russian text inside the pipeline (the
compensation a Russian-only engine needs); `Native` leaves the fragment intact
and lets the adapter read it in its own language.

This change SHALL route two kinds: a Latin run in prose and a code identifier
(including identifiers split out of identifiers, per the existing code-identifier
requirement). Abbreviations, URLs/emails and fenced code blocks SHALL NOT carry
their own route until evidence for one exists; the route struct SHALL grow one
field per kind when that evidence arrives.

The Latin prose block is one unit of compensation for a Russian-only reader:
transliteration, the built-in term dictionary, abbreviation readings and
single-letter naming SHALL all be gated by the prose route together, because a
single route cannot honestly claim that an engine reads "Kubernetes" natively
but needs "API" spelled out in Cyrillic.

URLs, emails and fenced code blocks SHALL keep their structural narration
regardless of the prose route for this change: their reading is punctuation
words ("точка", "слэш") and block markers, not a transliteration of a prose
word. Reading them natively needs a route of their own (#311).

A fragment routed `Native` SHALL be annotated (see the span requirement) and
SHALL NOT be rewritten by any narrating phase. A fragment routed `Narrate`
SHALL NOT be annotated as native, and SHALL NOT be presented to the adapter as
something to read.

#### Scenario: Identifier routed Native survives verbatim

- GIVEN a profile with the identifier route set to `Native`
- WHEN the pipeline processes "Вызови getUserData() через API"
- THEN "getUserData" stays Latin in the output, carries an identifier span with
  its language, and no Russian reading of it is emitted

#### Scenario: Prose Latin run routed Native survives verbatim

- GIVEN a profile with the prose route set to `Native` and English among its
  languages
- WHEN the pipeline processes "Kubernetes нужен кластер"
- THEN "Kubernetes" stays Latin and is annotated as a native prose run

#### Scenario: Narrating profile transliterates as today

- GIVEN the reference (Silero) profile
- WHEN the pipeline processes "Вызови getUserData() через API"
- THEN the output is "Вызови гет юзер дата() через эй пи ай" and no native span
  is produced

#### Scenario: Unrouted kinds keep their structural narration

- GIVEN a profile that routes prose and identifiers `Native`
- WHEN the pipeline processes a text containing a URL and a fenced code block
- THEN those fragments are narrated in Russian exactly as today

#### Scenario: Abbreviations follow the prose route

- GIVEN a profile that routes prose `Native`
- WHEN the pipeline processes "API нужен отдельно"
- THEN "API" stays Latin, because abbreviation reading is part of the same
  compensation as transliteration and no separate route exists yet

### Requirement: Shared, gated and adapter blocks

Every normalization rule SHALL have exactly one owner, and that owner SHALL run
at most once per fragment. The owners SHALL be:

- **Shared blocks** (run for every profile, because they decide structure or
  identify fragments rather than pronunciation): BOM removal, quote and dash
  normalization, whitespace normalization, Markdown structure, extraction of
  fenced code blocks, inline code, URLs/emails/IPs/paths, operators, special
  symbols, user-dictionary application, punctuation post-processing, and the
  final trim.
- **Gated blocks** (run only when the profile asks): the Latin prose block
  (transliteration, term dictionary, abbreviations, single-letter naming) and
  the Russian narration of an identifier; digit expansion — numbers, dates,
  times, ranges, versions, leading-dot decimals and tilde-approximation — for
  `digits: Expanded`; unit expansion — sizes, percentages and percentage
  ranges — for `units: Expanded`; chunking (see `text-chunking`).
- **Adapter steps** (performed by the engine implementation, never by the
  pipeline): filtering against the engine charset, collapsing newlines before a
  model tokenizer that cannot handle them, rendering markup around native
  spans, and issuing the synthesis calls. Stress rendering is the next
  consumer of this boundary and arrives with its own change (#312).

The gated number and unit blocks SHALL be skipped as a whole when the profile
declares the corresponding capability `Native`: a skipped block SHALL leave its
fragment untouched, and no other block SHALL re-process it.

#### Scenario: Gated digits block leaves a number untouched

- GIVEN a profile with `digits: Native`
- WHEN the pipeline processes "42 сервера"
- THEN the output contains "42" unchanged and the number phase did not run on it

#### Scenario: Units and digits are independent

- GIVEN a profile with `digits: Expanded` and `units: Native`
- WHEN the pipeline processes "рост 12.5% и 42 сервера"
- THEN "42" is read as "сорок два" while "12.5%" stays as it is, sign included

#### Scenario: A number is not split from its unit sign

- GIVEN a profile with `units: Native`
- WHEN the pipeline runs its number block on "рост 50%"
- THEN the number is left untouched: "50" is not read on its own, because the
  unit phase that owns the fragment did not run and the "%" would be stranded

#### Scenario: Nothing is applied twice

- GIVEN any profile and a text that reaches the transliteration block
- WHEN the pipeline processes it
- THEN applying the pipeline a second time to its output changes nothing that
  the first run produced (transliteration and number expansion are idempotent),
  and fragments already narrated are not re-read by a later phase

#### Scenario: Adapter step is not a pipeline phase

- GIVEN a profile whose engine drops codepoints outside its alphabet
- WHEN the pipeline processes a text containing such a codepoint
- THEN the pipeline does not remove it; the adapter filters it

A profile-level assertion that the output already fits the declared alphabet is
not part of this change: the Silero alphabet lives in the model bundle
(`frontend.json`, loaded only when `SILERO_NATIVE_BUNDLE` is set), so the check
belongs to the bundle-gated engine suite. `charset` is declared data that no
pipeline phase reads.

### Requirement: Span annotation output

The pipeline output SHALL carry, besides the normalized text and its
`CharMapping`, a list of span annotations. Each span SHALL state its kind (code
identifier, prose Latin run, dictionary override), its language when the
fragment is native, and its range (see `position-mapping` for the coordinate
space and the invariants).

Annotations SHALL express decisions, never markup: no span SHALL contain tag
syntax, and the normalized text SHALL remain the exact synthesis input.

#### Scenario: Dictionary override in a narrating profile

- GIVEN the reference profile and a user dictionary entry that rewrites a
  standalone Latin token
- WHEN the pipeline processes a text containing that token
- THEN a dictionary-override span covers the replacement in the output

An override applied *inside* a narrated identifier is not annotated
separately: the identifier phase replaces the whole identifier as one unit, so
its `char_map` entry maps every replacement codepoint to the whole original
identifier. Producing a part-level span would require part-wise replacement and
would change the mapping of every identifier in the corpus — that is #308's
territory, and this change keeps the mapping stable.

#### Scenario: No markup leaks into the text

- GIVEN any profile and any fixture
- WHEN the pipeline processes it
- THEN the normalized text contains no tag inserted by the pipeline, and every
  span range falls inside the text

## MODIFIED Requirements

### Requirement: Pipeline entry points and integration

The system SHALL normalize all text through `TTSPipeline` before TTS
synthesis. `TTSPipeline::process_for_engine(input, &profile)` SHALL be the
primary entry point and SHALL return the normalized text, its `CharMapping` and
its span annotations. `TTSPipeline::process_with_char_mapping(input)` SHALL
remain as a compatibility wrapper equivalent to
`process_for_engine(input, REFERENCE_PROFILE)`, returning text and mapping
only, where the reference profile is the Silero one. The system SHALL NOT
expose a bare `process(input)` that discards the mapping.

The pipeline SHALL be shared as `Arc<Mutex<TTSPipeline>>` in `AppState` and
invoked from the `add_text_entry` / `add_clipboard_entry` commands
(normalization before synthesis) and from `preview_normalize` (normalization
preview dialog). Both call sites SHALL pass the profile of the engine that is
active at that moment, captured in the same critical section that produces the
normalized text and the code-block narration mode, so that a racing engine
switch cannot falsify the recorded settings. The preview SHALL therefore show
the text the active engine will actually be asked to speak.

When normalization yields an empty string, the system SHALL reject synthesis
with the user-visible error "нормализация вернула пустой текст".

#### Scenario: Normalization before synthesis

- GIVEN the user adds the text "Вызови getUserData() через API"
- WHEN `add_text_entry` runs the normalization phase with the reference profile
- THEN the pipeline produces "Вызови гет юзер дата() через эй пи ай"
  (parentheses pass through verbatim; only the identifier and the English
  word are normalized) and synthesis proceeds with the normalized text

#### Scenario: Empty input

- GIVEN an empty input string
- WHEN `process_for_engine` is called
- THEN the system returns an empty string, an empty `CharMapping` and no spans,
  without running any phase

#### Scenario: Whitespace-only input rejected

- GIVEN input that contains only whitespace
- WHEN the pipeline processes it
- THEN the normalized result is empty and the synthesis command fails with
  "нормализация вернула пустой текст"

#### Scenario: process and process_with_char_mapping are consistent

- GIVEN any input text
- WHEN both `process_for_engine(input, REFERENCE_PROFILE)` and
  `process_with_char_mapping(input)` are called on it
- THEN both return the same text and the same `CharMapping`

#### Scenario: Preview follows the active engine

- GIVEN the active engine's profile routes prose Latin runs `Native`
- WHEN `preview_normalize` runs
- THEN the previewed text keeps the Latin run instead of transliterating it

### Requirement: Fixed phase order

The system SHALL execute normalization phases in a strictly fixed order
(`src-tauri/src/pipeline/mod.rs`): BOM removal; fenced code blocks; quote
normalization; dash normalization; whitespace normalization; inline code;
Markdown structure; URLs/emails/IPs/paths; sizes; dates and times; percentage
ranges; percentages; ranges; versions; leading-dot decimals; operators; special
symbols; code identifiers; user-dictionary pre-pass; English words; numbers;
whitespace post-processing; span resolution. The order is load-bearing: URLs
MUST be consumed before the number phase so an IP is not torn into four
numbers, percentage ranges MUST be consumed before percentages so the trailing
"%" is not read separately from its range, percentages MUST be consumed before
versions so a decimal percentage is not torn apart by the version phase,
versions MUST precede bare numbers, leading-dot decimals MUST follow versions so
"1.5" keeps its version reading and MUST precede bare numbers so the fraction
digits are consumed before the number phase runs, code identifiers MUST be split
before English words, the user-dictionary pre-pass MUST run after code-identifier
splitting (so dictionary entries apply to split parts of identifiers, never to
whole identifiers) and before English-word resolution (so user entries win over
every built-in table), abbreviations MUST be resolved before transliteration,
and multi-character operators MUST be processed longest-first.

The span resolution phase SHALL run last, after the final trim, so that every
span range refers to the text the engine will receive. A gated phase that the
profile declines SHALL be skipped without changing the order of the phases that
do run, and skipping it SHALL NOT let a later phase consume its fragment.

#### Scenario: IP address is not split into numbers

- GIVEN the input "Пинг 192.168.1.1"
- WHEN the pipeline processes it with the reference profile
- THEN the address is read as "сто девяносто два точка сто шестьдесят восемь
  точка один точка один" and not as four unrelated numbers

#### Scenario: Version is read as a whole

- GIVEN the input "версия v2.3.1"
- WHEN the pipeline processes it with the reference profile
- THEN the version is read as "два точка три точка один" and the version
  phase consumes the region before the number phase runs

#### Scenario: Decimal percentage is not consumed by the version phase

- GIVEN the input "Рост на 12.5% за квартал."
- WHEN the pipeline processes it with the reference profile
- THEN the percentage is read as "двенадцать точка пять процентов" and no
  bare "%" remains in the output

#### Scenario: camelCase split before English processing

- GIVEN the input "getUserData"
- WHEN the pipeline processes it with the reference profile
- THEN the identifier is split by the code-identifier phase into "гет юзер
  дата" instead of being transliterated as one opaque word

#### Scenario: Empty user dictionary leaves output unchanged

- GIVEN an empty user dictionary and any input from the existing golden
  fixtures
- WHEN the pipeline processes it
- THEN the output is byte-identical to the output without the dictionary
  feature

#### Scenario: Span resolution runs after the final trim

- GIVEN an input whose normalized text has leading and trailing whitespace
- WHEN the pipeline processes it
- THEN every span range lies within the trimmed text and ends before its
  codepoint count
