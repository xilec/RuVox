# user-dictionary Specification

## Purpose
User-authored pronunciation overrides: a flat `from → to` dictionary that
users edit at runtime (UI, import, or hand-editing the file) and that wins
over every built-in pronunciation table at normalization time.
## Requirements
### Requirement: Dictionary entry semantics

An entry SHALL map a single source token (`from`) to its spoken form (`to`),
and MAY list additional inflected source forms that share that pronunciation.
`from` SHALL be one word of letters and digits with at least one letter, at
most 64 characters, whose letters all belong to one script: ASCII Latin
(`A-Za-z`) or Russian Cyrillic (`а-я`, `А-Я`, `ё`, `Ё`). Tokens like `IPv6`,
`x86`, `замок`, or `А4` are valid; pure numbers, punctuation, mixed-script
tokens (`IPзамок`), and Cyrillic letters outside the Russian alphabet are not.

Each listed form SHALL be one Russian Cyrillic word of letters and digits with
at least one letter, at most 64 characters, distinct (case-insensitively) from
the primary `from` and from every sibling form. A form SHALL NOT be owned by
another entry, either as its primary `from` or as one of its forms. An entry
with a non-empty form list SHALL have a Russian Cyrillic `from`; Latin entries
SHALL NOT carry forms. The editor SHALL reject these cases before saving, and
import/load SHALL treat them as invalid entries.

Every listed form SHALL use the entry's `to`; forms whose pronunciation
(including stress) differs need separate entries. Matching SHALL be
case-insensitive and exact for the primary `from` and for every listed form: `замок` matches `Замок` and `ЗАМОК`; with forms
`["замка", "замке"]` it also matches those two words, but an unlisted
`замки` is left alone. `е` and `ё` are different letters. The primary key
SHALL be the lowercased `from`; one primary entry per word — re-adding an
existing primary replaces the entry. `to` SHALL be a non-empty free-form
string whose spoken form is at most 256 characters; Latin letters or digits in
`to` are permitted but the editor SHALL warn that the replacement is inserted
verbatim and later phases will not re-normalize it.

`to` MAY carry at most one explicit stress mark, written as `+` immediately
before a Russian vowel (`а е ё и о у ы э ю я`, either case). The mark SHALL NOT
be part of the spoken form: the pipeline inserts `to` with the mark for an
engine whose profile declares stress support (`Auto` or `Manual`) and without it
otherwise. A `+` that does not precede a vowel, a second `+`, or a mark at the
end of `to` SHALL be rejected with an error naming the rule; an entry loaded
from a hand-edited file with a malformed mark SHALL be skipped like any other
invalid entry. The character limit SHALL count the spoken form, not the mark.
Import and export SHALL preserve the mark and the form list verbatim, so a
round trip through the file format is lossless.

The editor SHALL explain the mark where the spoken form is entered: a help icon
next to the field, whose tooltip states that `+` before a vowel sets the stress
and that only engines with stress support apply it, and an inline error for a
malformed mark. The help text SHALL NOT name specific engines. The editor SHALL
also explain that a Russian word is matched by its exact form and that only
listed forms share the pronunciation, so a different inflection needs its own
form in the list.

#### Scenario: Case-insensitive match

- GIVEN the entry `GitHub → гитхаб`
- WHEN the text contains "github", "GitHub", or "GITHUB"
- THEN every occurrence is replaced with "гитхаб"

#### Scenario: Alnum source token is valid

- GIVEN the entry `IPv6 → айпи ви шесть`
- WHEN the entry is saved
- THEN it is accepted and later applied wherever "IPv6" occurs

#### Scenario: Russian word is a valid source token

- GIVEN the entry `замок → зам+ок`
- WHEN the entry is saved
- THEN it is accepted and the stress is applied wherever the exact word
  "замок" (any case) occurs

#### Scenario: Multi-word source token is rejected

- GIVEN an entry attempt with `from` "Иванов Петр"
- WHEN the entry is validated
- THEN it is rejected with an error explaining that a source token is one word
  of Latin or Russian letters and digits

#### Scenario: Mixed-script source token is rejected

- GIVEN `from` "IPзамок"
- WHEN the entry is validated
- THEN it is rejected with the same error, because a token belongs to one script

#### Scenario: Inflected form is a different key

- GIVEN the entry `замок → зам+ок` with no listed forms
- WHEN the text contains "замка"
- THEN the word is left as it is — without a listed form the entry matches one
  exact form

#### Scenario: Listed forms share the pronunciation

- GIVEN the entry `замок → зам+ок` with forms "замка" and "замке"
- WHEN the text contains "замок", "замка", or "замке"
- THEN every occurrence is replaced with "зам+ок" under a stress-aware profile

#### Scenario: Forms share one pronunciation

- GIVEN the entry `Петров → Петр+ов` with forms "Петрова", "Петрову", "Петровым"
- WHEN the text contains any of those forms
- THEN every one is replaced with "Петр+ов"; a form with a different stress
  would need its own entry

#### Scenario: Unlisted inflection stays untouched

- GIVEN the entry `замок → зам+ок` with forms "замка" and "замке"
- WHEN the text contains "замки"
- THEN the word is left as it is

#### Scenario: Forms are Russian-only

- GIVEN an entry with `from` "docker" and a form "dockers"
- WHEN the entry is validated
- THEN it is rejected because forms are available for Russian entries only

#### Scenario: Duplicate or colliding forms are rejected

- GIVEN an entry whose forms repeat its `from`, repeat a sibling, or match
  another entry's word or form
- WHEN the entry is validated
- THEN it is rejected with an error naming the conflicting form

#### Scenario: Duplicate keys in the file

- GIVEN the dictionary file contains `Git = "гит"` and `GitHub = "гитхаб"`
- WHEN the dictionary is loaded
- THEN both entries exist (different keys), but two entries whose primary keys
  differ only by case ("Git" and "git") collapse into one — the last one in
  file order wins and a warning is logged

#### Scenario: Stress is accepted before a vowel

- GIVEN the entry `Москва → Москв+а`
- WHEN it is saved
- THEN it is accepted, the spoken form is "Москва", and the mark is preserved
  in the file

#### Scenario: Malformed stress is rejected

- GIVEN an entry whose `to` is "Моск+ва", "М+оск+ва", "+Москва" or "Москва+"
- WHEN it is validated
- THEN it is rejected with an error explaining that a stress mark goes before
  a vowel and appears at most once

#### Scenario: Stress survives import and export

- GIVEN a dictionary containing `Москва → Москв+а`
- WHEN it is exported and imported again
- THEN the entry is identical, mark included

#### Scenario: Forms survive import and export

- GIVEN a dictionary containing `замок → зам+ок` with forms "замка" and "замке"
- WHEN it is exported and imported again
- THEN the primary word and both forms are identical

#### Scenario: Editor explains the mark

- GIVEN the dictionary form is open
- WHEN the user asks for help on the spoken-form field
- THEN the tooltip states that `+` before a vowel sets the stress and that
  only engines with stress support apply it

#### Scenario: Editor explains forms

- GIVEN the dictionary form is open
- WHEN the user asks for help on the word-forms field
- THEN the tooltip states that listed forms share the pronunciation and that
  an unlisted inflection is left alone

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

### Requirement: Runtime refresh and audio cache

A dictionary saved through the commands SHALL apply to every subsequent
normalization (preview and synthesis) without an app restart. Already
synthesized audio SHALL NOT be invalidated or re-synthesized automatically —
regeneration stays a manual action by the user.

#### Scenario: Preview reflects a new entry immediately

- GIVEN the preview dialog shows a normalized text containing "kubernetes"
- WHEN the user adds `kubernetes → кубер` and the save completes
- THEN the next preview normalization reads "кубер"

#### Scenario: Existing audio is untouched

- GIVEN a history entry was synthesized before a dictionary change
- WHEN the change is saved
- THEN the stored audio and its status remain exactly as they were

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
table. The user dictionary SHALL still apply to a native fragment that the
pipeline renders as text — a Latin prose run or a code identifier — but SHALL
NOT apply to a fenced code block routed `Native`: the block reaches the engine
verbatim. The editor's `overridesBuiltin` marker SHALL reflect the profile that
actually runs the tables for the selected scope, so a table that is gated off
is not marked as overridden.

#### Scenario: Native route skips built-ins

- GIVEN a profile that routes Latin prose `Native`
- WHEN "Kubernetes" is normalized
- THEN the built-in English table does not rewrite it
- AND a matching user-dictionary entry still does

#### Scenario: Native code block bypasses the dictionary

- GIVEN a profile that routes fenced code blocks `Native`, the read mode and
  a global entry `let → лет`
- WHEN the block `let x = 42;` is normalized
- THEN the block stays `let x = 42;` and the dictionary does not rewrite it

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

### Requirement: Unknown dictionary scopes are preserved

A version 2 collection whose `scope` is not recognized by this build SHALL be
kept as an opaque TOML table and written back unchanged by the next save. It
SHALL NOT participate in normalization or in the editor list in the older
build, but it SHALL NOT be dropped from the file. A recognized collection with
the same scope as a preserved one SHALL win in lookup; the file MAY contain
both until the newer build rewrites it.

#### Scenario: Unknown scope survives a save

- GIVEN a version 2 file has a collection with `scope = "future_engine"`
- WHEN this build loads the file, edits the global collection, and saves
- THEN the file still contains the `future_engine` collection with its
  original entries

