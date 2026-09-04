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

