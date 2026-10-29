# Delta: text-pipeline

## MODIFIED Requirements

### Requirement: User dictionary application in normalization

User dictionary entries SHALL win over every built-in table at every lookup
site: the prose pre-pass (before English-word resolution), code-identifier
parts (before `CODE_WORDS`), URL word reading (before `IT_TERMS`), and code
blocks read aloud (through the identifier path). The pre-pass SHALL match
tokens of Latin letters and digits containing at least one Latin letter —
including alnum tokens no other phase captures, such as "IPv6" or "mp3" — and,
when the dictionary holds at least one Cyrillic key, tokens of Russian Cyrillic
letters and digits containing at least one Cyrillic letter. Each candidate
pattern SHALL run only when the dictionary has a key of its script, and the two
patterns SHALL be disjoint, so one text token is matched by at most one of
them. Replacement SHALL apply only to exact case-insensitive key hits;
non-hits SHALL flow into the English-word phase unchanged. Lookups in
identifiers and URLs SHALL use the split parts/words those phases already
produce.

The replacement SHALL be the entry's `to` as typed. When the profile declares
`Stress::None`, the replacement SHALL be the spoken form instead — the same
text with the stress mark removed; otherwise the mark SHALL stay, so the engine
applies it. A malformed mark never reaches this phase: validation rejects it on
save and skips it on load. The mark SHALL be inserted as part of the
replacement region, so no later phase reads it as an operator or a symbol, and
`CharMapping` SHALL map it to the original word like the rest of the
replacement.

#### Scenario: Entry overrides an IT term in prose

- GIVEN the entry `docker → докер` ("docker" normally transliterates)
- WHEN the input "запусти docker" is processed
- THEN the output reads "докер"

#### Scenario: Entry applies to a Russian word

- GIVEN the entry `замок → зам+ок` and a profile declaring `Stress::Auto`
- WHEN the input "старый замок стоит" is processed
- THEN the output reads "старый зам+ок стоит" and every other Russian word is
  untouched

#### Scenario: Latin-only dictionary skips the Cyrillic scan

- GIVEN a dictionary whose keys are all Latin
- WHEN a Russian document is processed
- THEN the Cyrillic candidate pattern is not run and the output matches the
  output of the same document under the previous implementation

#### Scenario: Alnum token is normalized via the dictionary

- GIVEN the entry `IPv6 → айпи ви шесть` and the input "сеть IPv6 работает"
- WHEN the pipeline processes it
- THEN the output reads "айпи ви шесть"

#### Scenario: Entry applies to a part of a code identifier

- GIVEN the entry `kubectl → куб контрол`
- WHEN the input "команда kubectl_apply" is processed
- THEN the identifier is split and the "kubectl" part reads "куб контрол"

#### Scenario: Entry applies inside a URL

- GIVEN the entry `github → хаб`
- WHEN the input "см. https://github.com/ruvox" is processed
- THEN the host component reads "хаб" instead of the built-in "гитхаб"

#### Scenario: Entry applies to code read aloud

- GIVEN the entry `user → юзер` and code block narration mode "full"
- WHEN a fenced code block containing `user_id = 1` is processed
- THEN the "user" part reads "юзер"

#### Scenario: Stress mark reaches an engine that supports it

- GIVEN the entry `Москва → М+осква` and a profile declaring `Stress::Auto`
- WHEN the input "в Москва сегодня" is processed
- THEN the output contains "М+осква" and no phase reads the `+` as an operator

#### Scenario: Stress mark is dropped for an engine without support

- GIVEN the same entry and a profile declaring `Stress::None`
- WHEN the input is processed
- THEN the output contains "Москва" without the mark
