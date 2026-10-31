# Delta: text-pipeline

## MODIFIED Requirements

### Requirement: Engine text profile

The system SHALL describe what each engine can read in a data-only text
profile containing: the language codes the engine accepts natively; whether the
engine reads Latin prose natively (`reads_latin_natively`); the fragment route
for identifiers and fenced code blocks (see the next requirement); whether the
engine reads digits natively; whether it reads units natively; its stress
support (`None`, `Auto`, `Manual`); the codepoints it accepts (`Any` or an
explicit alphabet); its chunking limit and pause policy (see `text-chunking`);
and the markup syntax it wants around native spans (`None`, `RuEnTags`,
`LangTag`).

The profile SHALL contain no behavior — no callbacks, no trait objects — and
SHALL be reachable two ways: from an engine instance through
`TtsEngine::text_profile()`, and from the static per-engine registry keyed by
`EngineKind` that the pipeline and tests use without an engine in hand. The
registry SHALL declare a profile for every `EngineKind` variant, and the
definition SHALL live outside the application crate so that an engine crate can
share it.

Invariant: `reads_latin_natively` SHALL require `"en"` among the profile's
languages — the flag promises a native English reading, and a profile that
cannot name the language it reads has no way to annotate the fragment.

Invariant: a profile that routes an identifier or a fenced code block `Native`
SHALL list at least one language other than Russian; declaring a native reading
without a language to read it in is a specification error, not a runtime state.
The first non-Russian language is also the language a native span carries.

Invariant: `units: Expanded` SHALL require `digits: Expanded` — a unit reading
spells its number out ("пятьдесят процентов"), so the pipeline cannot expand a
unit while the engine reads the number.

Invariant: `identifier: Native` SHALL require `reads_latin_natively` — an
engine that reads identifiers natively reads Latin by definition, so a profile
may not claim the identifier route without the capability that justifies it.

A profile that violates an invariant SHALL be rejected by `TextProfile::validate`
rather than handled at runtime.

#### Scenario: Every engine kind has a profile

- GIVEN the registry
- WHEN it is asked for each `EngineKind` variant
- THEN a profile is returned for every variant, and the profiles differ only
  where the engines genuinely differ

#### Scenario: Native routing requires a non-Russian language

- GIVEN a profile with `code_block: Native` and `languages: ["ru"]`
- WHEN the profile invariants are checked
- THEN the check fails
- AND a profile with only `identifier: Native` and `languages: ["ru"]` fails
  the same way

#### Scenario: Native Latin capability requires English

- GIVEN a profile with `reads_latin_natively: true` and `languages: ["ru"]`
- WHEN the profile invariants are checked
- THEN the check fails, because the profile promises a language it does not
  list
- AND the same profile with `languages: ["ru", "en"]` is accepted

#### Scenario: Incoherent profile combinations are rejected

- GIVEN a profile with `digits: Native` and `units: Expanded`
- WHEN the profile invariants are checked
- THEN the check fails, because a unit reading needs the number in words
- AND a profile with `identifier: Native` and `reads_latin_natively: false`
  fails too

#### Scenario: Native code without native prose is valid

- GIVEN a profile with `code_block: Native`, `identifier: Narrate`,
  `reads_latin_natively: false` and a non-Russian language
- WHEN the profile invariants are checked
- THEN the profile is accepted, because code reading and Latin prose reading
  are independent capabilities

#### Scenario: Profile is data, not behavior

- GIVEN a profile value
- WHEN it is copied, serialized for a generation snapshot, or compared
- THEN no closure, engine handle or I/O is involved

### Requirement: Fragment route

The profile SHALL decide, per fragment kind, who reads the fragment:
`Narrate` turns the fragment into Russian text inside the pipeline (the
compensation a Russian-only engine needs); `Native` leaves the fragment intact
and lets the adapter read it in its own language.

The route SHALL cover two kinds: a code identifier (including identifiers
split out of identifiers, per the existing code-identifier requirement), and a
fenced code block. A Latin run in prose SHALL NOT be a profile route: the
profile declares only whether the engine can read it
(`reads_latin_natively`), and the user's English reading setting decides
whether it should (see the English reading setting requirement). Abbreviations
and URLs/emails SHALL NOT carry their own route until evidence for one exists;
the route struct SHALL grow one field per kind when that evidence arrives.

The Latin prose block is one unit of compensation for a Russian-only reader:
transliteration, the built-in term dictionary, abbreviation readings and
single-letter naming SHALL all be gated by the effective prose route together,
because a single route cannot honestly claim that an engine reads "Kubernetes"
natively but needs "API" spelled out in Cyrillic.

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
something to read. A native identifier SHALL stay verbatim even when the
effective prose route is `Narrate`: the identifier route is the profile's
decision and the English block SHALL NOT re-read it.

#### Scenario: Identifier routed Native survives verbatim

- GIVEN a profile with the identifier route set to `Native`
- WHEN the pipeline processes "Вызови getUserData() через API"
- THEN "getUserData" stays Latin in the output, carries an identifier span with
  its language, and no Russian reading of it is emitted

#### Scenario: Prose Latin run routed Native survives verbatim

- GIVEN a profile that reads Latin natively and the effective prose route
  `Native`
- WHEN the pipeline processes "Kubernetes нужен кластер"
- THEN "Kubernetes" stays Latin and is annotated as a native prose run

#### Scenario: Native identifier survives a narrated prose route

- GIVEN a profile that reads Latin natively and routes identifiers `Native`,
  and the English reading setting `narrated`
- WHEN the pipeline processes "Kubernetes и getUserData()"
- THEN "Kubernetes" is transliterated with no native span, while "getUserData"
  stays Latin and carries an identifier span

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

- GIVEN a profile that routes identifiers and code blocks `Native`
- WHEN the pipeline processes a text containing a URL and an email
- THEN those fragments are narrated in Russian exactly as today

#### Scenario: Abbreviations follow the prose route

- GIVEN a profile that reads Latin natively and the effective prose route
  `Native`
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
- **Gated blocks** (run only when the profile or the resolved English reading
  asks): the Latin prose block (transliteration, term dictionary,
  abbreviations, single-letter naming) whenever the effective prose route is
  `Narrate`, and the Russian narration of an identifier when the profile routes
  it `Narrate`; digit expansion — numbers, dates, times, ranges, versions,
  leading-dot decimals and tilde-approximation — for `digits: Expanded`; unit
  expansion — sizes, percentages and percentage ranges — for
  `units: Expanded`; chunking (see `text-chunking`).
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
identifier, prose Latin run, fenced code block, dictionary override), its
language when the fragment is native, and its range (see `position-mapping`
for the coordinate space and the invariants).

A fragment SHALL be annotated only when the pipeline leaves it to the engine: a
prose Latin run only under the effective prose route `Native` (see the English
reading setting requirement), an identifier or a code block only when the
profile routes its kind `Native`; a narrated fragment SHALL produce no native
span. The span's language SHALL be the non-Russian language the profile
declares (for TeraTTS and Piper, `en`). A dictionary-override span SHALL carry
no language and SHALL stay Russian regardless of the prose route. A native
code-block span SHALL cover exactly the code text handed to the engine.

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

- GIVEN a profile that reads Latin natively and the effective prose route
  `Native`
- WHEN the pipeline processes `Kubernetes и getUserData()`
- THEN `Kubernetes` carries a prose-Latin span with language `en` and
  `getUserData` carries a code-identifier span with language `en`

#### Scenario: Narrated prose keeps the identifier span

- GIVEN a profile that reads Latin natively and routes identifiers `Native`,
  and the English reading setting `narrated`
- WHEN the pipeline processes `Kubernetes и getUserData()`
- THEN no prose-Latin span is produced, and the output still carries the
  code-identifier span over the verbatim `getUserData`

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

## ADDED Requirements

### Requirement: English reading setting

The system SHALL persist a user preference for how Latin prose is read as
`UIConfig.english_reading`: `"auto"` (default), `"native"` or `"narrated"`.
Unknown values SHALL canonicalize to `"auto"` on load and on patch, the same
way `code_block_mode` does.

At the start of every synthesis the pipeline SHALL resolve one effective prose
route from that setting and the active profile's `reads_latin_natively`
capability:

| setting | engine reads Latin natively | effective route |
|---------|-----------------------------|-----------------|
| `auto` | yes | `Native` |
| `auto` | no | `Narrate` |
| `native` | yes | `Native` |
| `native` | no | `Narrate` (fallback) |
| `narrated` | any | `Narrate` |

`auto` SHALL reproduce the behaviour the engine had before this setting
existed, so an existing installation does not change on upgrade. Requesting
`native` on an engine without the capability SHALL fall back to narration
without failing and without rewriting the stored value. The setting SHALL be
independent of the selected engine: a choice made under one engine SHALL
survive switching to another and SHALL apply to the next synthesis without an
application restart. The setting SHALL NOT route identifiers or fenced code
blocks; those keep their profile routes.

#### Scenario: Auto follows the engine capability

- GIVEN the TeraTTS profile and `english_reading: "auto"`
- WHEN the pipeline processes "Kubernetes нужен кластер"
- THEN the text stays Latin and carries a prose-Latin span
- AND the same setting under the Silero profile transliterates it instead

#### Scenario: Native request falls back on an incapable engine

- GIVEN the Silero profile and `english_reading: "native"`
- WHEN the pipeline processes "Kubernetes нужен кластер"
- THEN the output is the same transliterated text `auto` produces, and no
  prose-Latin span is emitted

#### Scenario: Narrated transliterates on a capable engine

- GIVEN the TeraTTS profile and `english_reading: "narrated"`
- WHEN the pipeline processes "Kubernetes нужен кластер"
- THEN "Kubernetes" is transliterated into Russian and no prose-Latin span is
  emitted, while the word timestamps map back to the source text

#### Scenario: Unknown setting value canonicalizes to auto

- GIVEN a `config.json` with `english_reading: "bilingual"` or a patch carrying
  that value
- WHEN the configuration is loaded or the patch applied
- THEN the stored value is `"auto"`

#### Scenario: Setting change reaches the next synthesis

- GIVEN a running app with `english_reading: "auto"` on a Latin-capable engine
- WHEN the setting is changed to `"narrated"` and a new synthesis starts
- THEN that synthesis transliterates English prose without an app restart
