## Context

The dictionary pre-pass (phase 15.5) runs after the code-identifier phase and
matches one alphanumeric token at a time with `\b`-anchored patterns
(`re_user_dict_token`, `re_user_dict_token_cyrillic`). Kebab tokens are already
split by then: `UTF-8` reaches the pre-pass as `UTF` + `8`, which is exactly why
a part entry (`utf = "у тэф"`) reads it as «у тэф восемь» today and why a
hyphenated key cannot match at all. The abbreviation phase (14.5) already
consults the dictionary on the whole token before its built-in reading — the
precedent this pass mirrors.

Two routes have to be served by one mechanism:

- narrating identifiers (Silero, TeraTTSv2, Piper read mode): phase 15 calls
  `normalize_kebab_case`, which resolves per part;
- native identifiers (`NATIVE_EN_PROFILE`, `Native` code routing): phase 15
  records an identifier span and leaves the text to the engine, so no kebab
  normalization happens and a lookup hidden inside the normalizer would never
  fire.

## Goals / Non-Goals

**Goals:**

- A hyphenated `from` is accepted by validation, the editor and the file
  format, and round-trips.
- A hyphenated entry rewrites its whole token before the identifier split, on
  both routes, case-insensitively, ahead of its own parts and of any built-in
  reading.
- A dictionary without hyphenated keys pays no extra scan and produces
  byte-identical output.

**Non-Goals:**

- Partial matching inside a longer identifier.
- Punctuation other than `-`; structured fragments (sizes, dates, versions,
  URLs, paths, operators) keep their precedence.
- Multi-word entries (#298).

## Decisions

**A dedicated pass between the abbreviation and identifier phases.**
Alternatives:

- *Widen the existing pre-pass regex to include `-`* — rejected: the pre-pass
  runs after the split, so the hyphenated token no longer exists there.
- *Whole-token lookup inside `normalize_kebab_case` only* — rejected: the
  native-identifier route never calls it, and the override must fire there too.
- *Move the whole pre-pass ahead of the identifier phase* — rejected: it would
  drop per-part matching for every ordinary identifier and contradict the
  established rule that entries apply to identifier parts.
- *Replace dash-bounded runs inside a longer identifier* — rejected: the rest
  of the token would stay raw Latin (`у тэф восемь-sig`) and no later phase
  owns it.

The pass scans one candidate regex covering both scripts
(`\b[0-9A-Za-zА-Яа-яЁё]+(?:-[0-9A-Za-zА-Яа-яЁё]+)+\b`), so a mixed-script or
digit-only candidate simply never matches a validated key. It records the usual
`DictionaryOverride` span and replaces the token through `TrackedText`, exactly
like the existing pre-pass.

**The kebab normalizer also looks up the whole token.** Inline code (`\`UTF-8\``)
is normalized in phase 5, before any pre-pass, and it uses
`normalize_kebab_case` directly. Repeating the lookup there keeps one rule for
every site that resolves a kebab token; the lookup is a `BTreeMap` hit and is
skipped entirely when the dictionary is empty.

**Skip when no key carries a hyphen.** `UserDictionary::has_hyphenated_keys()`
gates the pass. Without it a dictionary of ordinary words would pay a second
full scan of the document for nothing, and the "empty dictionary leaves output
unchanged" guarantee would rest on the regex failing rather than on not running.

**Shape rule lives in `key_script`.** The hyphen is accepted as an internal
separator: segments are non-empty and alphanumeric, at least one letter overall,
one script, length counted as before. Forms reuse `key_script`, so they follow
the same shape. `has_keys_in` keeps working because a hyphenated key still
reports its script.

**Span consistency under native identifiers.** The pass runs before phase 15,
so the replaced token is Cyrillic by the time the native scan runs and never
becomes an identifier span. No new priority rule is needed; the surrounding
identifiers are annotated as before.

## Risks / Trade-offs

- *A hyphenated entry shadows part entries for exactly that token* → intended
  precedence, pinned by a unit test and a spec scenario.
- *`e-mail` inside a longer kebab token is not rewritten* → documented as a
  non-goal; the user can add the longer token as its own entry.
- *A hyphenated candidate that is a date or a version* → the structured phases
  run earlier and have already consumed such fragments; the pass only sees
  leftovers.
- *Perf* → one extra regex scan per document, gated by
  `has_hyphenated_keys()`; dictionaries with only plain keys are unaffected.

## Migration Plan

No file-format change: an existing file loads unchanged, and a hyphenated key
is a normal `from = "to"` pair. Entries that were rejected before simply start
to be accepted; nothing to migrate or roll back.

## Open Questions

None.
