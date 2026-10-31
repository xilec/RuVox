# Change: make English reading a user choice, keep native support an engine capability

## Why

How Latin prose is read is currently a property of the engine profile:
`TextProfile::Route.latin_prose` is `Native` for TeraTTSv2 and `Narrate` for
Piper and both Silero profiles. The choice is invisible to the user and
changes when the engine changes, even though the preference — hear English
words in English, or hear them transliterated into Russian — is about the
listener, not about the model. A user who prefers English pronunciation on
Piper and Russian transliteration on Silero cannot express it, and adding
another Latin-capable engine silently flips the behaviour for everyone who
selects it.

## What Changes

- **The profile carries a capability, not a policy.** `Route.latin_prose` is
  replaced by `TextProfile.reads_latin_natively: bool`; `Route` keeps the
  identifier and code-block kinds. `validate()` requires `"en"` among the
  profile's languages when the flag is set, and `identifier: Native` now
  requires the capability instead of requiring native prose.
- **One user setting decides the prose route.** `UIConfig.english_reading`
  (`"auto"` default | `"native"` | `"narrated"`, unknown values canonicalize
  to `"auto"`) is resolved against the capability at the start of every
  synthesis. `auto` reproduces today's per-engine behaviour; `native` on an
  engine without the capability falls back to narration without touching the
  stored value; the setting applies to the next synthesis without a restart
  and is independent of the selected engine.
- **The pipeline owns the resolution.** A new `EnglishReading` value, applied
  to the shared pipeline the same way `code_block_mode` is, gates the Latin
  prose block and the prose spans. Native identifiers stay native when prose
  is narrated: the English phase no longer re-reads a fragment the identifier
  route left to the engine.
- **Settings gains one control.** A segmented «Английский текст» («Авто» /
  «Нативно» / «По-русски») with a hint; when the selected engine has no
  native Latin reading and the user picked «Нативно», the hint explains that
  the next synthesis will transliterate. The capability reaches the UI through
  `get_available_engines` so the dialog hard-codes no engine names.
- **Tests, fixtures, specs.** Route-resolution matrix, pipeline spans and
  `CharMapping` for both routes, config serde/migration, Settings control
  state, one golden fixture for `narrated` on TeraTTS and one for the `native`
  fallback on Silero; delta specs for `text-pipeline`,
  `multi-language-synthesis`, `storage`, `ui` and `ipc-commands`; a CHANGELOG
  entry.

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `text-pipeline`: the engine profile carries the Latin capability instead of
  a prose route; the fragment route covers identifiers and code blocks only;
  a new requirement defines the English reading setting and its resolution.
- `multi-language-synthesis`: native prose routing follows the user setting
  resolved against the engine capability; profiles only declare what they can
  read.
- `storage`: `UIConfig.english_reading` with its default, canonicalization and
  migration rules.
- `ui`: the Settings dialog exposes the English reading selector and the
  fallback hint.
- `ipc-commands`: `UIConfig` carries `english_reading`, `EngineAvailability`
  carries the Latin capability stamped from the profile, and `update_config`
  applies the setting without an app restart.

## Non-goals

- Per-user choice for identifiers and fenced code blocks: they keep their
  profile-driven routes (#307, #325).
- Abbreviations, URLs, emails, digits and units: they keep today's behaviour.
- Per-engine overrides of the setting, per-language voices, better
  transliteration or native English quality, new engines.
- Recording `english_reading` in the generation snapshot; the snapshot schema
  is unchanged by this change.
