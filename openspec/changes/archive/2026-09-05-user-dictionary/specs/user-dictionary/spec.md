## Purpose

User-authored pronunciation overrides: a flat `from → to` dictionary that
users edit at runtime (UI, import, or hand-editing the file) and that wins
over every built-in pronunciation table at normalization time.

## ADDED Requirements

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

- GIVEN the imported file contains an entry with Cyrillic `from`
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
- WHEN the user exports it and imports the file back in merge mode
- THEN the dictionary is unchanged and the report says two entries updated

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
