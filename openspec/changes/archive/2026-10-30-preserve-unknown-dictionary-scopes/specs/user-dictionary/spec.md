## ADDED Requirements
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
