## Context

`teratts-engine` already specifies the full voice catalogue and takes it from
the bundle manifest (`voices[]`); `model-bundles` already surfaces the same ids
in the inventory. Only Settings lags behind: `src/dialogs/Settings.tsx` keeps a
four-entry `TERATTS_VOICES` constant, so the six English styles in the shipped
manifest cannot be selected. The spec `ui` still names the four ids, and
`storage` describes `teratts_voice` as a closed four-value set.

Constraints that shape the solution:

- The manifest is the root of trust for the bundle (format-version gate,
  per-file verification). A catalogue duplicated in TypeScript would drift with
  the next bundle release and has nothing in the repository to synchronise
  against, so it cannot be tested honestly.
- `get_available_engines` is a cheap, generic `{available, reason}` probe
  shared by four engines and specified that way.
- `get_model_status` walks bundle directories, reads install records and can
  touch the update channel — too heavy for opening Settings.
- The TeraTTS voice selector is only rendered when the engine is selectable,
  which requires a complete bundle, so an empty catalogue never has to drive a
  usable selector.

## Goals / Non-Goals

**Goals:**

- The selector lists every voice of the installed bundle, including English
  styles, without maintaining a second catalogue.
- Saving the dialog keeps producing a `teratts_voice` the engine accepts.
- A manifest/UI drift is caught by tests on both sides of the IPC boundary.

**Non-Goals:**

- Choosing a voice per language segment; the selected voice stays cross-lingual
  (`multi-language-synthesis`).
- Validating or coercing an unknown `teratts_voice` at save time.
- Changing the default (`ru_m5`) or the config schema.
- Grouped labels or human-friendly names for the voices.

## Decisions

### Dedicated `get_teratts_voices` command

Read `manifest.json` with the existing `teratts::bundle::Manifest::load` parser
and return `voices[].id` in manifest order. Settings calls it together with
`get_config` / `get_available_engines` when the dialog opens and again when the
model inventory reports an install.

Alternatives considered:

- **Static TS catalogue extended to ten ids.** Rejected: the authoritative list
  lives in `teratts/export/export.py` and the installed manifest, neither of
  which the frontend can import; a `test` would have to parse Python or ship a
  copy of the manifest, and it still goes stale on the next bundle.
- **Add `voices` to the availability payload.** Rejected: `EngineAvailability`
  is a shared four-engine contract; a TeraTTS-only field forces a separate type
  (or a field every other engine leaves empty) and changes a widely used IPC
  shape for one selector. A second small command keeps that contract stable.
- **Reuse `get_model_status().bundled_voices`.** Rejected: the inventory walks
  the bundle and may consult the update channel; Settings must open instantly
  and offline.

### Errors become an empty list

`get_teratts_voices` returns `[]` when the manifest is missing, unreadable or
has an unsupported format version. The availability probe already carries the
coded reason, and a failing command would turn a cosmetic catalogue read into a
Settings load error. The frontend therefore needs no error path beyond an empty
list.

### The selector keeps an unknown saved id visible

Settings builds its options from the catalogue and appends
`form.values.teratts_voice` when the manifest does not list it. This preserves
the stored value through a save instead of silently substituting a different
voice, and it keeps the Mantine `Select` from showing no selection if the
catalogue arrives empty. The engine still rejects the id with `voice_unknown`
at synthesis, which is the existing contract.

### Labels stay the ids

The manifest ids (`ru_m5`, `eng_f5`, …) are model-native names like Piper's
voice ids; they are shown verbatim in manifest order (Russian voices first, as
the exporter emits them). Friendly labels or language groups would add i18n
surface without changing the choice.

### Description wording

The selector description drops "Russian": the voices are cross-lingual and the
list now includes English styles. `ru_m5` remains the documented default.

## Risks / Trade-offs

- **A bundle older than this build lists fewer voices** → the selector shows
  exactly what the installed bundle can synthesize; after the bundle update,
  the inventory's install callback refreshes the list.
- **The selector depends on a second IPC round trip** → the call is a
  manifest-sized JSON parse in `spawn_blocking`; the dialog already waits on
  `get_config` + `get_available_engines`.
- **Empty catalogue while the engine reports available** (I/O race, manual
  deletion between probes) → the saved id stays selectable; the next synthesis
  fails with `voice_unknown` rather than the UI guessing.
- **No manifest/UI coupling in the type system** → both sides are covered by
  tests: Rust pins manifest-to-ids (including English), TypeScript pins that
  the selector renders every id it is given and preselects the saved one.
