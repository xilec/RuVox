## Why

The dictionary commands now operate on scoped collections, but the live
`ipc-commands` spec still documents the old single-dictionary commands. The
wire contract and the behavior spec must match.

## What Changes

- Rename the four dictionary commands to their collection-aware names.
- Document the `scope` argument and the per-collection behavior.
- Document the `get_dictionaries` DTO shape and the scope-aware override flag.

## Capabilities

### New Capabilities

- None.

### Modified Capabilities

- `ipc-commands`: the user-dictionary command contract now takes a scope and
  returns collections.

## Impact

- Spec only; implementation is in `feat/dictionary-scopes`.
