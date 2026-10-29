# Delta: user-dictionary

## MODIFIED Requirements

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

#### Scenario: Cyrillic source token is rejected

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
