# RuVox

## Language

### Pronunciation dictionaries

**User Dictionary**:
The set of user-authored pronunciation overrides, persisted per user and merged
over the built-in dictionaries at normalization time. User entries win over
built-ins on collision.
_Avoid_: custom dictionary, personal dictionary

**Dictionary Entry**:
A single user-dictionary mapping: a source token (`from` — letters and
digits, at least one letter, e.g. `IPv6`; an ASCII hyphen may join its parts,
e.g. `UTF-8`) and its spoken form (`to`). The unit
of CRUD, import, and export. One entry per word, keyed by the lowercased
`from`; matching is case-insensitive. An entry applies wherever the word
occurs — prose, ALL-CAPS abbreviations, parts of split code identifiers (a
hyphenated `from` matches the whole token instead), URL
components, and code blocks read aloud — and wins over every built-in table
at each of those sites.
_Avoid_: custom term (implementation name inside `EnglishNormalizer`), entry
type, layer, case-sensitive entry

### Text preparation

**Text Profile**:
Declarative per-engine description of what the engine can read and of its
limits — supported languages, per-kind fragment routes, native digits and units,
stress support (which decides whether an explicit dictionary mark survives), allowed charset, chunking limit and
its unit, language-tag syntax, pause policy — keyed by `EngineKind`, holding
data only and no behavior.
_Avoid_: capability profile, engine config, pipeline settings

**Shared block**:
A normalization rule that runs for every engine because it only touches
document structure or identifies fragments — markdown structure, quotes,
whitespace, extraction of code, URLs, emails, user-dictionary application — and
never the pronunciation of a fragment the engine could read itself.
_Avoid_: generic block, common stage

**Gated block**:
A normalization rule whose execution depends on the Text Profile: Latin
transliteration (including abbreviations), number and unit expansion — and
therefore the Russian narration of such a fragment, which runs when its
Fragment route says `Narrate`. Symbol substitution and postprocessing are
shared; chunking reads the profile's limit and pauses but runs for every
engine.
_Avoid_: engine-specific block, conditional stage

**Fragment route**:
The Text Profile's per-fragment-kind decision — `Narrate` or `Native` —
carried for prose Latin runs, code identifiers and fenced code blocks today;
abbreviations travel with the prose route, and URLs/emails keep structural
narration until their own route lands. The code-block route applies only in
the read narration mode.
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
filtering against the engine charset, tag rendering, and the synthesis calls
themselves.
_Avoid_: engine block, engine-specific normalization

**Span annotation**:
A `(kind, language, range)` decision carried by the pipeline output, the range
expressed in final normalized text, saying what a fragment is (code identifier,
prose Latin run, dictionary override) instead of how to mark it up.
_Avoid_: markup, tag, token

