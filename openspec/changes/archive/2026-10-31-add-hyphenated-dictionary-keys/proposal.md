## Why

The user dictionary accepts exactly one alphanumeric token as `from`, so the
tokens users most want to fix are rejected: `UTF-8`, `e-mail`, `snake-case`.
Extending the charset alone would not make them match either — the
code-identifier phase splits a kebab token into parts before the dictionary
pre-pass runs, and the phase order is deliberate: entries apply to identifier
parts, never to whole identifiers. The current workaround (`utf = "у тэф"`)
can only compose a reading out of parts; it cannot express `e-mail` → «имейл»
(not «е мэйл»).

## What Changes

- Accept the ASCII hyphen inside a `from` token: the token stays one token,
  one script, at least one letter, at most 64 characters; every hyphen-separated
  segment is non-empty and made of letters and digits only. Leading, trailing
  and doubled hyphens are rejected, as is every other punctuation character.
  The rule is script-agnostic, so a Cyrillic hyphenated word (`по-русски`) is a
  valid key too, and the same shape applies to inflected forms.
- Add a whole-token dictionary pass ahead of the code-identifier split: a
  hyphenated entry replaces the entire matching token, case-insensitively,
  before kebab normalization. A whole-token entry therefore wins over its own
  parts and over the built-in tables; a part entry keeps working when no
  whole-token entry exists.
- Look up the whole token inside the kebab normalizer as well, so inline code
  (`\`UTF-8\``) follows the same rule.
- Keep the recorded span a dictionary override under a profile that routes
  identifiers `Native`; the surrounding identifiers keep their native span.
- Update the editor validation mirror, the ru/en error strings, the
  `user-dictionary` spec and the `text-pipeline` phase-order spec.

## Capabilities

### New Capabilities

- None.

### Modified Capabilities

- `user-dictionary`: the `from` shape gains the hyphen, and matching gains the
  whole-token rule and its precedence over part entries.
- `text-pipeline`: the fixed phase order gains the hyphenated whole-token pass
  between the abbreviation phase and the code-identifier phase.

## Impact

- Affected code: `src-tauri/src/dictionary/{mod,tests}.rs`,
  `src-tauri/src/pipeline/mod.rs`,
  `src-tauri/src/pipeline/normalizers/code.rs`, `src/lib/userDictionary.ts`
  (and its test), `src/i18n/{ru,en}.ts`.
- Fixtures: a `native_en/` pipeline fixture pinning the whole-token reading and
  the native-profile span behavior.
- Docs: `CHANGELOG.md` under `[Unreleased]`.

## Non-goals

- Multi-word entries (spaces) — that is #298 and it touches the same
  validation and pre-pass surface on purpose.
- Punctuation other than the ASCII hyphen (`C:`, `a.b`, underscores) and
  non-ASCII dashes: the structured phases own those fragments, and the
  phase-order rule keeps them out of reach.
- Partial matching inside a longer identifier: an entry `utf-8` does not
  rewrite `utf-8-extra`, because a partial replacement would leave a
  mixed-script fragment that no later phase owns.
- Stemming, normalization of the hyphen (`U+2010`, `U+2011`) or of letter case
  beyond the existing case-insensitive lookup.
