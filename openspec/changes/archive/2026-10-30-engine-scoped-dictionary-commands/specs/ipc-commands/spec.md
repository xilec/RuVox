## MODIFIED Requirements
### Requirement: User Dictionary Commands

The system SHALL expose four collection-aware commands for the user
dictionary. All follow the shared command error format and the
camelCase→snake_case argument casing convention.

- `get_dictionaries()` SHALL return a list of collections. Each collection
  carries `scope` (`global` or an engine id), `name`, and `entries`; each
  entry carries `from` (as typed), `to`, `forms`, and an
  `overrides_builtin` flag. Entries are sorted by the lowercased `from`
  inside their collection. The global collection is always present.
- `save_dictionary(scope, entries)` SHALL validate every entry and atomically
  replace the selected collection (all-or-nothing: any invalid entry rejects
  the save and leaves the file unchanged); a successful save refreshes the
  active normalization pipeline immediately. An empty engine collection is
  removed from the file; the global collection is kept.
- `import_dictionary(path, mode, scope)` SHALL read and validate a TOML file
  and apply it to the selected collection in `merge` mode (imported entries
  win on key collisions, invalid entries are skipped) or `replace` mode
  (validated entries fully replace the collection); it SHALL return counts of
  added, updated, and skipped entries. An unreadable or unparsable file SHALL
  reject with a typed error and change nothing.
- `export_dictionary(path, scope)` SHALL write the selected collection as
  valid dictionary TOML in the historical version 1 shape and reject with a
  typed error if the write fails.

#### Scenario: get returns collections with flags

- GIVEN the global dictionary contains `nginx → энджинкс` and
  `docker → докер` (docker exists in `IT_TERMS`), and the Piper collection
  contains one entry
- WHEN `get_dictionaries` is called
- THEN the global list is ordered "docker", "nginx", only the docker entry
  carries `overrides_builtin: true`, and the Piper collection is returned
  separately

#### Scenario: get returns sorted entries with flags

- GIVEN the dictionary contains `nginx → энджинкс` and `docker → докер`
  (docker exists in `IT_TERMS`)
- WHEN `get_dictionaries` is called
- THEN the global list is ordered "docker", "nginx" and only the docker entry
  carries `overrides_builtin: true`

#### Scenario: save refreshes normalization without restart

- GIVEN a save adds `kubectl → куб контрол`
- WHEN `preview_normalize` is called afterwards with "команда kubectl_apply"
- THEN the normalized text reads "куб контрол"

#### Scenario: import merge reports counts

- GIVEN the imported file has two new entries, one colliding entry, and one
  invalid entry
- WHEN `import_dictionary` is called with mode "merge"
- THEN the command succeeds and returns added: 2, updated: 1, skipped: 1

#### Scenario: import of a missing file is a typed error

- GIVEN the path points to a nonexistent file
- WHEN `import_dictionary` is called
- THEN the command rejects with a typed error and the dictionary is unchanged

#### Scenario: save rejects an invalid entry atomically

- GIVEN the selected collection is valid and a save is requested with one
  valid and one invalid entry (a multi-word `from`)
- WHEN `save_dictionary` is called
- THEN the command rejects with a validation error and the dictionary file
  is unchanged

#### Scenario: save accepts a Russian word

- GIVEN a save is requested with `замок → зам+ок` in the global scope
- WHEN `save_dictionary` is called
- THEN the command succeeds and `get_dictionaries` returns the entry with
  the mark preserved

#### Scenario: save targets one collection

- GIVEN the global collection has an entry and the Piper collection is empty
- WHEN `save_dictionary("piper", [docker → докка])` is called
- THEN the Piper collection contains the entry, the global collection is
  unchanged, and the merged map for Piper uses the Piper entry

#### Scenario: import and export use the selected scope

- GIVEN the Piper collection has two entries
- WHEN `export_dictionary(path, "piper")` then
  `import_dictionary(path, "replace", "piper")` are called
- THEN the Piper collection is unchanged and the global collection is not
  touched
