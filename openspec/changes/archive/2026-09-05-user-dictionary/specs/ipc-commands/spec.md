## ADDED Requirements

### Requirement: User Dictionary Commands

The system SHALL expose four commands for the user dictionary. All follow
the shared command error format and the camelCase→snake_case argument
casing convention.

- `get_user_dictionary()` SHALL return the entry list sorted by the
  lowercased `from`, each entry carrying `from` (as typed), `to`, and an
  `overrides_builtin` flag.
- `save_user_dictionary(entries)` SHALL validate every entry and atomically
  replace the whole dictionary (all-or-nothing: any invalid entry rejects
  the save and leaves the file unchanged); a successful save refreshes the
  active normalization pipeline immediately.
- `import_user_dictionary(path, mode)` SHALL read and validate a TOML file
  and apply it in `merge` mode (imported entries win on key collisions,
  invalid entries are skipped) or `replace` mode (validated entries fully
  replace the current dictionary); it SHALL return counts of added, updated,
  and skipped entries. An unreadable or unparsable file SHALL reject with a
  typed error and change nothing.
- `export_user_dictionary(path)` SHALL write the current entries as valid
  dictionary TOML to the path and reject with a typed error if the write
  fails.

#### Scenario: get returns sorted entries with flags

- GIVEN the dictionary contains `nginx → энджинкс` and `docker → докер`
  (docker exists in `IT_TERMS`)
- WHEN `get_user_dictionary` is called
- THEN the list is ordered "docker", "nginx" and only the docker entry
  carries `overrides_builtin: true`

#### Scenario: save rejects an invalid entry atomically

- GIVEN the current dictionary is valid and a save is requested with one
  valid and one invalid entry (Cyrillic `from`)
- WHEN `save_user_dictionary` is called
- THEN the command rejects with a validation error and the dictionary file
  is unchanged

#### Scenario: save refreshes normalization without restart

- GIVEN a save adds `kubectl → куб контрол`
- WHEN `preview_normalize` is called afterwards with "команда kubectl_apply"
- THEN the normalized text reads "куб контрол"

#### Scenario: import merge reports counts

- GIVEN the imported file has two new entries, one colliding entry, and one
  invalid entry
- WHEN `import_user_dictionary` is called with mode "merge"
- THEN the command succeeds and returns added: 2, updated: 1, skipped: 1

#### Scenario: import of a missing file is a typed error

- GIVEN the path points to a nonexistent file
- WHEN `import_user_dictionary` is called
- THEN the command rejects with a typed error and the dictionary is unchanged
