# RuVox

## Language

### Pronunciation dictionaries

**User Dictionary**:
The set of user-authored pronunciation overrides, persisted per user and merged
over the built-in dictionaries at normalization time. User entries win over
built-ins on collision.
_Avoid_: custom dictionary, personal dictionary

**Dictionary Entry**:
A single user-dictionary mapping: a Latin source word (`from` — letters and
digits, at least one letter, e.g. `IPv6`) and its spoken form (`to`). The unit
of CRUD, import, and export. One entry per word, keyed by the lowercased
`from`; matching is case-insensitive. An entry applies wherever the word
occurs — prose, ALL-CAPS abbreviations, parts of split code identifiers, URL
components, and code blocks read aloud — and wins over every built-in table
at each of those sites.
_Avoid_: custom term (implementation name inside `EnglishNormalizer`), entry
type, layer, case-sensitive entry

### Text preparation

**Text Profile**:
Declarative per-engine description of what the engine can read and of its
limits — script support, native digits and units, stress support, allowed
charset, chunking limit and its unit, language-tag syntax, inter-segment pause
— keyed by `EngineKind`, holding data only and no behavior.
_Avoid_: capability profile, engine config, pipeline settings

**Shared block**:
A normalization rule that runs for every engine because it only touches
document structure or identifies fragments — markdown, quotes, whitespace,
extraction of code, URLs, emails, abbreviations, user-dictionary application —
and never the pronunciation of a fragment the engine could read itself.
_Avoid_: generic block, common stage

**Gated block**:
A normalization rule whose execution depends on the Text Profile: Latin
transliteration, number and unit expansion, symbol substitution, stress
marking, chunking — and the Russian narration of a fragment whose Fragment
route says `Native`.
_Avoid_: engine-specific block, conditional stage

**Fragment route**:
The Text Profile's per-kind decision — `Narrate` or `Native` — for a prose
Latin run, an identifier, an abbreviation, a URL/email or a fenced code block.
`Narrate` is the pipeline rewriting the fragment into Russian (the crutch a
Russian-only engine needs); `Native` is the fragment surviving as-is for the
engine to read in its own language.
_Avoid_: latin mode, transliteration flag, engine mode

**Markup**:
The tag syntax an engine wants around a `Native` span: `None`, `RuEnTags`
(`<ru>…</ru>`, `<en>…</en>`) or `LangTag`. Rendered by the adapter only — the
pipeline never emits tags and `CharMapping` never sees them.
_Avoid_: tag format, ssml mode

**Adapter step**:
Work an engine implementation must do that no pipeline rule can own — symbol
filtering against the engine charset, tag rendering, stress rendering, chunked
synthesis.
_Avoid_: engine block, engine-specific normalization

**Span annotation**:
A `(kind, language, range)` decision carried by the pipeline output, the range
expressed in final normalized text, saying what a fragment is (code, URL/email,
Latin run, number, abbreviation, dictionary override) instead of how to mark it
up.
_Avoid_: markup, tag, token

