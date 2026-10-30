## Context

`UserDictionary` is a flat `BTreeMap<lowercased from, DictionaryEntry>` and the
pipeline pre-pass replaces exact words. That keeps the hot path simple and
predictable, but it cannot express "these inflections share one pronunciation".
The issue offers three approaches; the choice below is explicit forms because
it preserves exactness and keeps all guessing out of the pipeline.

## Goals / Non-Goals

**Goals:**
- Let one entry cover a user-confirmed list of Russian word forms.
- Keep matching exact and case-insensitive; never rewrite an unlisted word.
- Keep the hot path a map lookup: no stemming, no morphology, no per-token
  suffix stripping.
- Preserve the existing dictionary file and editor behavior for entries without
  forms.

**Non-Goals:**
- Automatic morphological analysis or form generation at save time. The user
  lists the forms; the editor does not guess them.
- Inflected forms for Latin entries.
- Matching a shared stem with arbitrary endings (`пила` vs `пил` is exactly the
  false-positive class this design avoids).
- Per-form pronunciation overrides: every listed form uses the entry's `to`.

## Decisions

1. **Explicit forms, matched exactly.** The user enters the forms; each is a
   first-class exact key. This is the only option that cannot turn `все`/`всё`
   or `пила`/`пил` into a wrong rewrite, and it needs no runtime morphology.
2. **A form is an alias of one primary entry.** `UserDictionary` keeps the
   primary map and a lowercased form→primary index. `resolve` checks the primary
   map first, then the alias index. The primary always wins, so even a
   hand-edited file with an ambiguous form cannot shadow a real entry.
3. **Validate forms at the boundary.** Save/import reject a form that repeats
   the primary, a sibling, another entry's primary, or another entry's form.
   Load skips an entry with an invalid form like any other invalid entry;
   aliases from a hand-edited collision are first-wins and cannot shadow a
   primary. The editor surfaces the same rules before saving.
4. **Keep TOML backward compatible.** A string value keeps the historical
   meaning; a table value carries `to` and `forms` (inline when hand-written,
   nested when the exporter serializes it). The format version stays 1,
   so older files load unchanged and an older build sees a table value as a
   non-string entry and skips it rather than corrupting the file.
5. **Forms are Russian-only.** The feature exists for Russian inflection;
   allowing Latin forms would duplicate the existing exact-key behavior and
   invite mixed-script keys. `from` must be Cyrillic when `forms` is non-empty.
6. **Store forms as typed.** Display case is preserved, matching is
   case-insensitive through the alias index, and export writes the same list
   back.

## Risks / Trade-offs

- The user must know the forms; there is no automatic generation. This is a
  deliberate trade for predictability and is explained in the editor help.
- A form list is stored per entry, so editing a form requires editing the base
  entry. The editor exposes the list as one comma-separated field.
- A hand-edited file can still contain an ambiguous alias; it is ignored in
  favor of the primary/first entry rather than causing a rewrite of a different
  word. The save/import paths reject such files.

## Migration

No migration: string entries keep loading and exporting as strings; entries
with forms serialize as inline tables. The version stays 1.
