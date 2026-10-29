# Delta: user-dictionary

## MODIFIED Requirements

### Requirement: Dictionary entry semantics

An entry SHALL map a single source token (`from`) to its spoken form (`to`).
`from` SHALL match `^[A-Za-z0-9]+$` and contain at least one Latin letter —
letters-and-digits tokens like `IPv6` or `x86` are valid; pure numbers,
Cyrillic, hyphens, and other punctuation are not. Matching SHALL be
case-insensitive everywhere. The entry key SHALL be the lowercased `from`;
one entry per word — re-adding an existing word replaces the entry. `to`
SHALL be a non-empty free-form string of at most 256 characters; Latin
letters or digits in `to` are permitted but the editor SHALL warn that the
replacement is inserted verbatim and later phases will not re-normalize it.

`to` MAY carry at most one explicit stress mark, written as `+` immediately
before a vowel. The mark SHALL be parsed at load into the entry's stored stress
position and SHALL NOT remain part of the spoken form the pipeline inserts; a
`to` whose `+` does not precede a vowel, or that carries more than one `+`,
SHALL be rejected with an error naming the rule. Import and export SHALL
preserve the mark verbatim, so a round trip through the file format is
lossless. Rendering the mark for a model SHALL be the adapter's job: the
pipeline never emits `+`, and the mark is applied only by engines whose profile
declares stress support.

#### Scenario: Case-insensitive match

- GIVEN the entry `GitHub → гитхаб`
- WHEN the text contains "github", "GitHub", or "GITHUB"
- THEN every occurrence is replaced with "гитхаб"

#### Scenario: Alnum source token is valid

- GIVEN the entry `IPv6 → айпи ви шесть`
- WHEN the entry is saved
- THEN it is accepted and later applied wherever "IPv6" occurs

#### Scenario: Cyrillic source token is rejected

- GIVEN an entry attempt with `from` "Иванов"
- WHEN the entry is validated
- THEN it is rejected with an error explaining that source tokens must be
  Latin words (letters and digits, at least one letter)

#### Scenario: Duplicate keys in the file

- GIVEN the dictionary file contains `Git = "гит"` and `GitHub = "гитхаб"`
- WHEN the dictionary is loaded
- THEN both entries exist (different keys), but two entries whose keys differ
  only by case ("Git" and "git") collapse into one — the last one in file
  order wins and a warning is logged

#### Scenario: Explicit stress is parsed out of the spoken form

- GIVEN the entry `Москва → М+осква`
- WHEN it is saved and the text "москва" is normalized
- THEN the pipeline inserts "Москва" without the `+`, and the entry's stored
  stress position points at the "о"

#### Scenario: Malformed stress is rejected

- GIVEN an entry whose `to` is "Моск+ва" or "М+оск+ва"
- WHEN it is validated
- THEN it is rejected with an error explaining that a stress mark goes before
  a vowel and appears at most once

#### Scenario: Stress survives import and export

- GIVEN a dictionary containing `Москва → М+осква`
- WHEN it is exported and imported again
- THEN the entry is identical and the stress position is unchanged

#### Scenario: Stress is not applied by a profile without stress support

- GIVEN a profile declaring `stress: None`
- WHEN the pipeline normalizes an entry that carries a stress mark
- THEN the spoken form is inserted without any mark
