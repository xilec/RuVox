# Proposal: restore-dropped-e-and-y-in-inflections

## Why

Bounded inflection composition fires only when the *stripped* stem is itself an
`IT_TERMS` entry, so inflected forms whose base dropped a silent "e" or the "y"
of "-ies" miss the dictionary and degrade to raw transliteration:

- `uses` → "юзс" (expected "юзес", base `use` = "юз")
- `using` → "усинг" (expected "юзинг"), `used` → "усед" (expected "юзед")
- `caches` → "кэшс" (expected "кэшес"), `cached` → "качед" (expected "кэшед")
- `repositories` → "репоситорис" (expected "репозиторис", base
  `repository` = "репозиторий")

At the same time the "-ies" branch composes from any stem that happens to match,
so `species` is read as "спекис" from `spec` — a false match that must go back
to raw fallback.

## What Changes

- Before giving up on a suffix, retry with a restored silent "e" appended to the
  stem (`us` → `use`, `cach` → `cache`) and compose from that base.
- Read `-es` as the syllabic "ес" only when the restored stem ends in a sibilant
  (`use`/`cache` → "юзес"/"кэшес"); for other bases the plain "-s" composition
  still wins, so `features` stays "фичас" and `supports`/`building` are unchanged.
- Compose `-ies` only when `stem + "y"` is an `IT_TERMS` entry, dropping a
  trailing "й" from the base reading (`repository` → "репозитори" + "с");
  `species` therefore stops composing from `spec`.
- Keep the composition bounded: no general stemmer, no new dictionary entries,
  and unknown-words tracking still covers only raw-fallback words.
- Extend the unit tests with the issue's table and add a `text-pipeline`
  scenario for the restoration rules.

## Non-goals

- No new `IT_TERMS` entries or dictionary cleanup — that is tracked separately.
- No change to the suffix list, to non-inflected words, or to the all-uppercase
  abbreviation path for pure abbreviations (`API`, `CPU`, `JSON` keep their
  readings).
- No general morphological stemming beyond the two restorations above.
- No change to the readings of already-composing forms (`supports`,
  `supported`, `supporting`, `supporter`, `features`, `builds`, `building`).

## Capabilities

### Modified Capabilities
- text-pipeline: the inflection-composition rule gains silent-e restoration and
  a `-ies`-from-`-y` restriction, with scenarios locking the restored readings
  and the false-match removal.

## Impact

- `src-tauri/src/pipeline/normalizers/english.rs` — `compose_inflection` and
  unit tests
- `openspec/specs/text-pipeline/spec.md` — delta
- `CHANGELOG.md` `[Unreleased]`
- No change to `src-tauri/src/pipeline/mod.rs`: the pipeline keeps calling the
  shared helper.
