# user-dictionary Specification

## Purpose
User-authored pronunciation overrides: a flat `from → to` dictionary that
users edit at runtime (UI, import, or hand-editing the file) and that wins
over every built-in pronunciation table at normalization time.
## Requirements
### Requirement: Dictionary entry semantics

An entry SHALL map a single source token (`from`) to its spoken form (`to`).
`from` SHALL be one word of letters and digits with at least one letter, at
most 64 characters, whose letters all belong to one script: ASCII Latin
(`A-Za-z`) or Russian Cyrillic (`а-я`, `А-Я`, `ё`, `Ё`). Tokens like `IPv6`,
`x86`, `замок`, or `А4` are valid; pure numbers, punctuation, mixed-script
tokens (`IPзамок`), and Cyrillic letters outside the Russian alphabet are not.
Matching SHALL be case-insensitive and exact: a token matches only the same word
form — `замок` matches `Замок` and `ЗАМОК` but not `замка`; `е` and `ё` are
different letters. The entry key SHALL be the lowercased `from`; one entry per
word — re-adding an existing word replaces the entry. `to` SHALL be a non-empty
free-form string whose spoken form is at most 256 characters; Latin letters or
digits in `to` are permitted but the editor SHALL warn that the replacement is
inserted verbatim and later phases will not re-normalize it.

`to` MAY carry at most one explicit stress mark, written as `+` immediately
before a Russian vowel (`а е ё и о у ы э ю я`, either case). The mark SHALL NOT
be part of the spoken form: the pipeline inserts `to` with the mark for an
engine whose profile declares stress support (`Auto` or `Manual`) and without it
otherwise. A `+` that does not precede a vowel, a second `+`, or a mark at the
end of `to` SHALL be rejected with an error naming the rule; an entry loaded
from a hand-edited file with a malformed mark SHALL be skipped like any other
invalid entry. The character limit SHALL count the spoken form, not the mark.
Import and export SHALL preserve the mark verbatim, so a round trip through the
file format is lossless.

The editor SHALL explain the mark where the spoken form is entered: a help icon
next to the field, whose tooltip states that `+` before a vowel sets the stress
and that only engines with stress support apply it, and an inline error for a
malformed mark. The help text SHALL NOT name specific engines. The editor SHALL
also state that a Russian word is matched by its exact form, so a different
inflection needs its own entry.

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

- GIVEN the entry `замок → зам+ок`
- WHEN the text contains "замка"
- THEN the word is left as it is — the entry matches one exact form

#### Scenario: Duplicate keys in the file

- GIVEN the dictionary file contains `Git = "гит"` and `GitHub = "гитхаб"`
- WHEN the dictionary is loaded
- THEN both entries exist (different keys), but two entries whose keys differ
  only by case ("Git" and "git") collapse into one — the last one in file
  order wins and a warning is logged

#### Scenario: Stress is accepted before a vowel

- GIVEN the entry `Москва → М+осква`
- WHEN it is saved
- THEN it is accepted, the spoken form is "Москва", and the mark is preserved
  in the file

#### Scenario: Malformed stress is rejected

- GIVEN an entry whose `to` is "Моск+ва", "М+оск+ва", "+Москва" or "Москва+"
- WHEN it is validated
- THEN it is rejected with an error explaining that a stress mark goes before
  a vowel and appears at most once

#### Scenario: Stress survives import and export

- GIVEN a dictionary containing `Москва → М+осква`
- WHEN it is exported and imported again
- THEN the entry is identical, mark included

#### Scenario: Editor explains the mark

- GIVEN the dictionary form is open
- WHEN the user asks for help on the spoken-form field
- THEN the tooltip states that `+` before a vowel sets the stress and that
  only engines with stress support apply it

### Requirement: Storage format and location

The dictionary SHALL persist as a single TOML file in the configuration
root (alongside `config.json`), with a `version = 1` field and one
`from = "to"` pair per entry under `[entries]`, keys preserving the case the
user typed. A missing file SHALL mean an empty dictionary. A corrupted file
SHALL be renamed to a `.bak` backup (the `config.json` recovery pattern) and
replaced by an empty dictionary, with a warning in the log. Writes SHALL be
atomic (temp file + rename). The file is the source of truth at startup;
changes made while the app runs take effect only through the dictionary
commands.

#### Scenario: First launch

- GIVEN no dictionary file exists
- WHEN the app starts
- THEN the dictionary is empty and no error is surfaced

#### Scenario: Hand-edited entry is picked up

- GIVEN the app is stopped and the user adds `nginx = "энджинкс"` to the file
- WHEN the app starts
- THEN the entry is active in normalization

#### Scenario: Corrupted file recovers to empty

- GIVEN the dictionary file contains invalid TOML
- WHEN the app starts
- THEN the file is moved to a backup, the dictionary starts empty, and a
  warning is logged

### Requirement: Import and export

Export SHALL write the current entries as a valid dictionary TOML file to
the chosen path. Import SHALL read a TOML file, validate every entry, and
apply one of two modes chosen by the user at import time: **merge** —
imported entries win on key collisions, invalid entries are skipped, and the
result is reported as counts (added / updated / skipped); **replace** — the
imported list fully replaces the current dictionary after validation.
Invalid entries SHALL never abort a merge import.

#### Scenario: Merge with collision

- GIVEN the current dictionary has `docker → докер` and the imported file has
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

- GIVEN the current dictionary has three entries and the imported file has
  one valid entry
- WHEN the user imports in replace mode
- THEN the dictionary contains exactly the imported entry

#### Scenario: Export round-trips

- GIVEN the dictionary contains two entries
- WHEN they are exported and the file is imported again
- THEN the dictionary is unchanged

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
also exists in a built-in table the dictionary applies to (`IT_TERMS`,
`AS_WORD`-style abbreviation maps, `CODE_WORDS`), so the editor can badge
entries that override built-in behavior.

#### Scenario: Override badge

- GIVEN the entry `github → хаб` and "github" exists in `IT_TERMS`
- WHEN the dictionary list is loaded in the UI
- THEN the entry is marked as overriding a built-in

