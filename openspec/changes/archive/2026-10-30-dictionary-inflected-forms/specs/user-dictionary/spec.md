# Delta: user-dictionary

## MODIFIED Requirements

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
root (alongside `config.json`), with a `version = 1` field and one entry per
word under `[entries]`, keys preserving the case the user typed. An entry with
no listed forms SHALL be written as a `from = "to"` pair; an entry with forms
SHALL be written as a table value with `to` and `forms` (an inline table
`from = { to = "...", forms = [...] }` when hand-written; the exporter may
write the equivalent nested `[entries.<word>]` table). Both shapes SHALL load,
so existing files remain valid. A missing file SHALL
mean an empty dictionary. A corrupted file SHALL be renamed to a `.bak` backup
(the `config.json` recovery pattern) and replaced by an empty dictionary, with
a warning in the log. Writes SHALL be atomic (temp file + rename). The file is
the source of truth at startup; changes made while the app runs take effect
only through the dictionary commands.

#### Scenario: First launch

- GIVEN no dictionary file exists
- WHEN the app starts
- THEN the dictionary is empty and no error is surfaced

#### Scenario: Hand-edited entry is picked up

- GIVEN the app is stopped and the user adds `nginx = "энджинкс"` to the file
- WHEN the app starts
- THEN the entry is active in normalization

#### Scenario: Hand-edited form entry is picked up

- GIVEN the app is stopped and the user adds
  `"замок" = { to = "зам+ок", forms = ["замка", "замке"] }`
- WHEN the app starts
- THEN the primary word and both forms are active in normalization

#### Scenario: Corrupted file recovers to empty

- GIVEN the dictionary file contains invalid TOML
- WHEN the app starts
- THEN the file is moved to a backup, the dictionary starts empty, and a
  warning is logged
