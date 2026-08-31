# Design: open-entry-params-on-double-click

## Context

`QueueItem` (`src/components/QueueList.tsx`) renders each entry as a
`role="button"` div with `onClick` (select), `onContextMenu` (menu) and
Enter/Space keyboard activation. The parameters dialog state
(`paramsEntryId`) lives in `QueueList`, opened today only from the context
menu item, which is disabled when the entry has neither a generation snapshot
nor a generation timestamp.

## Goals / Non-Goals

**Goals:** a double-click shortcut to the existing dialog, gated identically
to the menu item; zero behavior change for single click, right click, and
keyboard activation.

**Non-Goals:** keyboard duplicate of the double-click (menu remains the
accessible path); other double-click bindings; dialog content changes.

## Decisions

- **Thread an `onOpenParams(entry)` callback prop** from `QueueList` into
  `QueueItem` and handle `onDoubleClick` on the item div. Matches the
  existing `onSelect` / `onContextMenu` prop pattern; the gate stays in one
  place (`QueueList`), next to the menu-item gate.
- **Gate in the handler, not on the DOM:** a `dblclick` always fires; the
  opener checks `entry.generation !== null || entry.audio_generated_at !== null`
  and no-ops otherwise — exactly the menu item's disabled condition.
- **Let the double-click also select.** The browser fires `click` before
  `dblclick`; selection is idempotent, so both single clicks selecting and
  the dialog opening is the natural composition (mirrors desktop file
  managers).
- While editing the "Recording parameters dialog" requirement for the
  double-click mention, drop the stale "operator reading" row-list mention
  left behind when wire-code-block-mode (#271) removed `read_operators` —
  spec text only, the dialog already has no such row.

## Risks / Trade-offs

- [Text selection from rapid double-click on a div] → cosmetic and bounded;
  `user-select` styling is out of scope unless it annoys.

## Open Questions

None.
