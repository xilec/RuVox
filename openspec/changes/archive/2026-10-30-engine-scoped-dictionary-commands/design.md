## Context

`get_user_dictionary`, `save_user_dictionary`, `import_user_dictionary`, and
`export_user_dictionary` were replaced by `get_dictionaries`,
`save_dictionary(scope, entries)`, `import_dictionary(path, mode, scope)`, and
`export_dictionary(path, scope)`. The old names are not registered anymore, so
the old spec would mislead any client.

## Decisions

- Keep the historical error format and snake_case argument casing; add one
  `scope` string argument (`global` or an engine id).
- `get_dictionaries` returns a list of collections instead of one entry list.
- Import and export operate on the selected collection; the exported file
  keeps the version 1 shape for portability.

## Risks

- A stale client calling the old names gets an unknown-command error; the
  editor is the only in-repo caller and is updated in the same branch.
