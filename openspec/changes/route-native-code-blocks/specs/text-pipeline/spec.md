# Delta: text-pipeline

## MODIFIED Requirements

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

Invariant: a profile that routes any fragment kind `Native` SHALL list at
least one language other than Russian; declaring a native reading without a
language to read it in is a specification error, not a runtime state. The
first non-Russian language is also the language a native span carries, so the
code-block route follows the same requirement as the prose and identifier
routes.

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
- AND a profile with only `code_block: Native` and `languages: ["ru"]` fails
  the same way

#### Scenario: Incoherent profile combinations are rejected

- GIVEN a profile with `digits: Native` and `units: Expanded`
- WHEN the profile invariants are checked
- THEN the check fails, because a unit reading needs the number in words
- AND a profile with `identifier: Native` and `latin_prose: Narrate` fails too

#### Scenario: Native code without native prose is valid

- GIVEN a profile with `code_block: Native`, `latin_prose: Narrate`,
  `identifier: Narrate` and a non-Russian language
- WHEN the profile invariants are checked
- THEN the profile is accepted, because code reading and prose reading are
  independent decisions

#### Scenario: Profile is data, not behavior

- GIVEN a profile value
- WHEN it is copied, serialized for a generation snapshot, or compared
- THEN no closure, engine handle or I/O is involved

### Requirement: Fragment route

The profile SHALL decide, per fragment kind, who reads the fragment:
`Narrate` turns the fragment into Russian text inside the pipeline (the
compensation a Russian-only engine needs); `Native` leaves the fragment intact
and lets the adapter read it in its own language.

The route SHALL cover three kinds: a Latin run in prose, a code identifier
(including identifiers split out of identifiers, per the existing
code-identifier requirement), and a fenced code block. Abbreviations and
URLs/emails SHALL NOT carry their own route until evidence for one exists; the
route struct SHALL grow one field per kind when that evidence arrives.

The Latin prose block is one unit of compensation for a Russian-only reader:
transliteration, the built-in term dictionary, abbreviation readings and
single-letter naming SHALL all be gated by the prose route together, because a
single route cannot honestly claim that an engine reads "Kubernetes" natively
but needs "API" spelled out in Cyrillic.

URLs and emails SHALL keep their structural narration regardless of the prose
route: their reading is punctuation words ("точка", "слэш"), not a
transliteration of a prose word. Reading them natively needs a route of their
own (#311).

The code-block route SHALL apply only in the read narration mode (see the
fenced code block requirement). In brief mode a `Native` block is still
announced, because the mode is the user's decision not to read code at all,
and the route only chooses the reader.

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

#### Scenario: Code block routed Native survives verbatim

- GIVEN a profile with the code-block route set to `Native`, English among its
  languages, and the read mode
- WHEN the pipeline processes a text containing
  "```rust\nlet x = 42;\nif x == 0 { return; }\n```"
- THEN the output contains "let x = 42; if x == 0 { return; }" — the digit and
  the operator are not read in Russian — and a code-block span with language
  `en` covers that text

#### Scenario: Brief mode outranks a Native code route

- GIVEN a profile with the code-block route set to `Native` and the brief mode
- WHEN the pipeline processes the same text
- THEN the block is replaced with the announcement and carries no native span

#### Scenario: Narrating profile transliterates as today

- GIVEN the reference (Silero) profile
- WHEN the pipeline processes "Вызови getUserData() через API"
- THEN the output is "Вызови гет юзер дата() через эй пи ай" and no native span
  is produced

#### Scenario: Unrouted kinds keep their structural narration

- GIVEN a profile that routes prose, identifiers and code blocks `Native`
- WHEN the pipeline processes a text containing a URL and an email
- THEN those fragments are narrated in Russian exactly as today

#### Scenario: Abbreviations follow the prose route

- GIVEN a profile that routes prose `Native`
- WHEN the pipeline processes "API нужен отдельно"
- THEN "API" stays Latin, because abbreviation reading is part of the same
  compensation as transliteration and no separate route exists yet

### Requirement: Fenced code blocks

The system SHALL process fenced code blocks (` ```lang ... ``` `) before all
other content phases. A block tagged `mermaid` SHALL be replaced with the
exact marker string "Тут мермэйд диаграмма" and MUST NOT be read aloud, in
every narration mode and under every profile.

For every other block the pipeline's code block narration mode decides whether
the block is read at all, and the active profile's code-block route decides
who reads it. The application sets the mode from the `code_block_mode` config
value (`"brief"` default, `"read"` optional; the wiring lives in the
`ipc-commands` capability):

- **Brief** (`"brief"`): the block is replaced with "далее следует пример
  кода на <язык>" (language looked up in `LANGUAGE_NAMES`) or "далее следует
  блок кода" when no language tag is present. The route is not consulted.
- **Full, `Narrate`** (`"read"`): block contents are tokenized and read aloud
  with identifiers, operators, brackets, and integer literals normalized to
  spoken Russian.
- **Full, `Native`** (`"read"`): the block's content is handed to the engine
  as text. Its surrounding whitespace is trimmed and internal whitespace runs
  are collapsed to one space; the `CharMapping` and word timestamps keep
  pointing into the original fenced block.

A `Native` block SHALL NOT be rewritten by any narrating phase: the English,
identifier, operator, symbol, number and user-dictionary phases SHALL NOT see
it, and a `Native` block SHALL NOT be annotated as native prose or as
identifiers. Its codepoints map to the whole source fenced block, which is the
same granularity the full Russian narration already has.

The pipeline's implicit default mode SHALL be Brief, matching the config
default, so a freshly constructed pipeline never contradicts the product
default. The former inline mode-switch directives (`<!-- ruvox-code: … -->`)
SHALL no longer be interpreted: such comments are processed as ordinary text
by the symbol phases exactly like any other punctuation, and the narration
mode is owned solely by the setting.

#### Scenario: Mermaid block replaced with marker

- GIVEN the input "```mermaid\ngraph TD\nA-->B\n```"
- WHEN the pipeline processes it in any mode
- THEN the entire block is replaced with "Тут мермэйд диаграмма"

#### Scenario: Brief mode is the default

- GIVEN a pipeline constructed without an explicit mode and the input
  "```python\nprint('hi')\n```"
- WHEN the pipeline processes it
- THEN the block is replaced with "далее следует пример кода на пайтон"

#### Scenario: Brief mode without a language tag

- GIVEN the pipeline in Brief mode and the input "```\nx = 1\n```"
- WHEN the pipeline processes it
- THEN the block is replaced with "далее следует блок кода"

#### Scenario: Code block read in full mode

- GIVEN the reference (Silero) profile, the pipeline configured in Full mode
  and the input "```python\nprint('hi')\n```"
- WHEN the pipeline processes it
- THEN the block content is read as "принт открывающая скобка хи
  закрывающая скобка"

#### Scenario: Native code keeps its content and line breaks do not glue words

- GIVEN the TeraTTS profile, the read mode and the input
  "```python\ndef hello():\n    print('hi')\n```"
- WHEN the pipeline processes it
- THEN the output is "def hello(): print('hi')" — one space between the two
  lines, no Russian reading of any token

#### Scenario: Mode-switch directive

- GIVEN the pipeline in Full mode and the text
  "<!-- ruvox-code: brief -->\n```python\nprint('hi')\n```"
- WHEN the pipeline processes it
- THEN the block is still read in full (the directive comment does not change
  the mode) and the comment is normalized as ordinary text; the exact output
  is pinned by a golden fixture

### Requirement: Span annotation output

The pipeline output SHALL carry, besides the normalized text and its
`CharMapping`, a list of span annotations. Each span SHALL state its kind (code
identifier, prose Latin run, fenced code block, dictionary override), its
language when the fragment is native, and its range (see `position-mapping`
for the coordinate space and the invariants).

A fragment SHALL be annotated only when the active profile routes its kind
`Native`; a narrating profile SHALL produce no native span for it. The span's
language SHALL be the non-Russian language the profile declares (for TeraTTS
and Piper, `en`). A dictionary-override span SHALL carry no language and SHALL
stay Russian regardless of the prose route. A native code-block span SHALL
cover exactly the code text handed to the engine.

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

#### Scenario: Native prose and identifier spans

- GIVEN the TeraTTS profile
- WHEN the pipeline processes `Kubernetes и getUserData()`
- THEN `Kubernetes` carries a prose-Latin span with language `en` and
  `getUserData` carries a code-identifier span with language `en`

#### Scenario: Native code-block span

- GIVEN the TeraTTS profile, the read mode and a text containing a fenced
  code block
- WHEN the pipeline processes it
- THEN one code-block span with language `en` covers exactly the code text in
  the output, and no prose or identifier span overlaps it

#### Scenario: Narrating profile emits no native spans

- GIVEN the Silero profile
- WHEN the pipeline processes the same text
- THEN every Latin fragment is transliterated and no span carries a language

#### Scenario: No markup leaks into the text

- GIVEN any profile and any fixture
- WHEN the pipeline processes it
- THEN the normalized text contains no tag inserted by the pipeline, and every
  span range falls inside the text
