## ADDED Requirements
### Requirement: Scoped dictionary collections

The dictionary SHALL consist of named collections, each bound to the global
scope or to exactly one engine. An entry SHALL belong to exactly one
collection. The global collection SHALL apply to every engine; an engine
collection SHALL apply only when that engine is active. On the same primary
`from`, an engine-bound entry SHALL win over a global entry. The user
dictionary SHALL win over every built-in table under every profile.

#### Scenario: Global entry applies to every engine

- GIVEN the global collection contains `nginx → энджинкс`
- WHEN any engine normalizes "nginx"
- THEN the replacement is "энджинкс"

#### Scenario: Engine entry applies only to its engine

- GIVEN the TeraTTSv2 collection contains `nginx → engine-x`
- WHEN TeraTTSv2 normalizes "nginx"
- THEN the replacement is "engine-x"
- WHEN Piper normalizes "nginx"
- THEN the global entry, if any, applies instead

#### Scenario: Engine entry wins on the same source word

- GIVEN the global collection has `docker → докер` and the Piper collection
  has `docker → докка`
- WHEN Piper normalizes "docker"
- THEN the replacement is "докка"

#### Scenario: User dictionary still applies under a native route

- GIVEN a global entry `Kubernetes → кубернетес`
- WHEN a profile with native Latin prose normalizes "Kubernetes"
- THEN the replacement is "кубернетес"

### Requirement: Profile-gated built-in tables

Built-in pronunciation tables SHALL run only for fragments the active profile
narrates. A fragment routed `Native` SHALL NOT be rewritten by a built-in
table; the user dictionary SHALL still apply to it. The editor's
`overridesBuiltin` marker SHALL reflect the profile that actually runs the
tables for the selected scope, so a table that is gated off is not marked as
overridden.

#### Scenario: Native route skips built-ins

- GIVEN a profile that routes Latin prose `Native`
- WHEN "Kubernetes" is normalized
- THEN the built-in English table does not rewrite it
- AND a matching user-dictionary entry still does

#### Scenario: Override marker follows the profile

- GIVEN a native-Latin profile where built-in English tables do not run
- WHEN a global entry shadows a built-in word
- THEN the editor does not mark it as overriding a built-in table

### Requirement: Scope selection in the editor

The dictionary editor SHALL expose a scope selector with the global scope and
each installed engine. The entry list, add/edit form, import and export SHALL
operate on the selected collection. The editor SHALL explain that the global
collection applies everywhere and an engine collection only to that engine,
and that engine entries win on the same source word.

#### Scenario: Switching scope shows that collection

- GIVEN the global collection has one entry and the Piper collection is empty
- WHEN the user selects the Piper scope
- THEN the list is empty and saving adds the entry to the Piper collection

## MODIFIED Requirements
### Requirement: Storage format and location

The dictionary SHALL persist as a single TOML file in the configuration
root (alongside `config.json`), with a `version = 2` field and an array of
named collections under `[[dictionaries]]`, each carrying a `name`, a `scope`
(`global` or an engine id), and an `entries` table keyed by the source word
with the case the user typed. An entry with no listed forms SHALL be written
as a `from = "to"` pair; an entry with forms SHALL be written as a table value
with `to` and `forms` (an inline table `from = { to = "...", forms = [...] }`
when hand-written; the exporter may write the equivalent nested
`[dictionaries.entries.<word>]` table). An empty engine collection SHALL NOT
be written; the global collection SHALL always exist. A version 1 file (or a
file without `version` that has `[entries]`) SHALL load as one global
collection named `Global`, losslessly, and saving SHALL write version 2. A
missing file SHALL mean one empty global collection. A corrupted file SHALL be
renamed to a `.bak` backup (the `config.json` recovery pattern) and replaced
by an empty global collection, with a warning in the log. A file with a newer
format version SHALL load as empty without a backup. Writes SHALL be atomic
(temp file + rename). The file is the source of truth at startup; changes made
while the app runs take effect only through the dictionary commands.

#### Scenario: First launch

- GIVEN no dictionary file exists
- WHEN the app starts
- THEN the global collection is empty and no error is surfaced

#### Scenario: Version 1 file migrates

- GIVEN a version 1 file with `nginx = "энджинкс"` under `[entries]`
- WHEN the app starts
- THEN the entry is in the global collection and behaves as before
- WHEN the dictionary is saved
- THEN the file is version 2 with one global collection

#### Scenario: Version 2 round-trips

- GIVEN a global collection and a Piper collection
- WHEN the file is saved and loaded again
- THEN both collections and their entries are identical

#### Scenario: Empty engine collection is omitted

- GIVEN the Piper collection has no entries
- WHEN the dictionary is saved
- THEN the file contains no Piper collection

#### Scenario: Hand-edited entry is picked up

- GIVEN the app is stopped and the user adds `nginx = "энджинкс"` under the
  global collection
- WHEN the app starts
- THEN the entry is active in normalization

#### Scenario: Hand-edited form entry is picked up

- GIVEN the app is stopped and the user adds
  `"замок" = { to = "зам+ок", forms = ["замка", "замке"] }`
  under the global collection
- WHEN the app starts
- THEN the primary word and both forms are active in normalization

#### Scenario: Corrupted file recovers to empty

- GIVEN the dictionary file contains invalid TOML
- WHEN the app starts
- THEN the file is moved to a backup, the global collection starts empty, and
  a warning is logged

### Requirement: Import and export

Export SHALL write the selected collection as a valid dictionary TOML file to
the chosen path, in the historical single-dictionary shape so files stay
portable. Import SHALL read a TOML file, validate every entry, and apply one of
two modes chosen by the user at import time to the selected collection:
**merge** — imported entries win on key collisions, invalid entries are
skipped, and the result is reported as counts (added / updated / skipped);
**replace** — the imported list fully replaces the selected collection after
validation. Invalid entries SHALL never abort a merge import.

#### Scenario: Merge with collision

- GIVEN the selected collection has `docker → докер` and the imported file has
  `docker → докка`
- WHEN the user imports in merge mode
- THEN the resulting entry is `docker → докка` and the report says one entry
  updated

#### Scenario: Merge skips invalid entries

- GIVEN the imported file contains an entry with a multi-word `from`
- WHEN the user imports in merge mode
- THEN valid entries are applied, the invalid one is skipped, and the report
  counts it as skipped

#### Scenario: Replace mode

- GIVEN the selected collection has three entries and the imported file has
  one valid entry
- WHEN the user imports in replace mode
- THEN the collection contains exactly the imported entry

#### Scenario: Export round-trips

- GIVEN the selected collection contains two entries
- WHEN they are exported and the file is imported again into that collection
- THEN the collection is unchanged

### Requirement: Built-in override marker

The list returned to the UI SHALL mark each entry whose lowercased `from`
also exists in a built-in table that actually runs for the selected scope's
profile (`IT_TERMS`, `AS_WORD`-style abbreviation maps, `CODE_WORDS`), so the
editor can badge entries that override built-in behavior. A table gated off
by a `Native` route SHALL NOT produce the marker; a global entry is checked
against the active engine's profile, an engine entry against that engine's
profile.

#### Scenario: Override badge

- GIVEN the entry `github → хаб` and "github" exists in `IT_TERMS`
- WHEN the dictionary list is loaded in the UI
- THEN the entry is marked as overriding a built-in

#### Scenario: Native route has no built-in marker

- GIVEN a native-Latin profile where `IT_TERMS` does not run
- WHEN a global entry shadows a built-in word
- THEN the entry is not marked as overriding a built-in
