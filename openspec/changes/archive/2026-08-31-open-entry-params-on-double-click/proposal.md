# Proposal: open-entry-params-on-double-click

## Why

«Параметры записи…» is reachable only through the context menu. A double-click
on a queue item is the natural shortcut for the read-only parameters view, and
the dialog already handles every entry state safely.

## What Changes

- Double-clicking a queue item opens the recording parameters dialog under the
  same enablement gate as the "Параметры записи…" menu item: the entry has a
  generation snapshot or a generation timestamp. Single-click selection and
  the context menu keep working unchanged.
- Drop the stale "operator reading" mention from the "Recording parameters
  dialog" requirement row list — the `read_operators` field was removed from
  the snapshot in wire-code-block-mode (#271), but this requirement text was
  not synced then. No runtime change: the dialog already has no such row.

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `ui`: the "Queue list behavior" requirement gains the double-click binding
  with the params-dialog gate; the "Recording parameters dialog" requirement
  row list loses the removed `read_operators` mention and names the
  double-click as a second open path.

## Impact

- `src/components/QueueList.tsx`: `QueueItem` gains an `onOpenParams` prop and
  a `dblclick` handler; `QueueList` passes an opener gated on the snapshot /
  timestamp.
- `src/components/QueueList.test.tsx`: positive and gated-negative tests.
- No backend, storage, or i18n changes (the dialog exists).

## Non-goals

- No keyboard duplicate of the double-click (the context menu stays the
  accessible path, unchanged).
- No double-click bindings for other actions (play, edit, delete).
- No changes to the dialog content itself.
