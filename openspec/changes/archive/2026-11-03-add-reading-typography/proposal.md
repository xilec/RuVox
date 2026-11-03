## Why

At 1440 px the reading pane is ~110–120 characters wide: the eye loses the
line, and the text has no size control at all — `--ruvox-reading-font-size` is
fixed at 17 px. That works against the product goal of resting eyes strained by
a day of reading.

#354 proposed per-mode font-size presets, but a font-size control only reaches
text: an HTML entry keeps its headings at `em` ratios, its tables and images at
their natural size, so raising the base distorts the document instead of
scaling it the way browser zoom does. #357 therefore replaces the size presets
with one browser-like scale factor applied to the whole reading pane, in every
display mode. This change implements the combined result: the reading column
width from #354 and the reading scale from #357.

## What Changes

- `UIConfig` gains `reading_width: String` (`"narrow"` default / `"wide"`,
  canonicalized like `code_block_mode`: unknown values resolve to `"narrow"`)
  and `reading_scale: u32` (percent, default 100, clamped to 80–200 on load and
  through `UIConfigPatch`); both join `UIConfigPatch`. `reading_font_size` is
  **not** added — #357 cancelled the size presets.
- The viewer toolbar, right of the display-mode switch, hosts
  `Узко | Широко` (`SegmentedControl`) and `[ − ] 100% [ + ]` (steps
  80/100/125/150/175/200, clicking the value resets to 100%, buttons disabled at
  the bounds) in every display mode; both persist immediately through
  `update_config` with optimistic UI and rollback + toast on failure.
- `.content` gets `max-width: min(70ch, 100%); margin-inline: auto` in the
  narrow mode and the current full-pane layout in wide; the scale is a CSS
  `zoom` on the same container (not `transform: scale()`), so text, headings,
  code, tables, borders and images scale together while the pane keeps its
  width. Wide `pre` blocks and tables still scroll horizontally inside the
  column (tables get a scroll wrapper, since `overflow-x` on the table itself
  does not scroll in WebKitGTK).
- Settings gains a discoverable «Чтение» block with the same two controls
  (saved with the rest of the form); #355 will reorganize the sections later.
- RU/EN catalogs, Rust and TS tests (defaults, canonicalization, clamping,
  round-trip, control state, applied CSS, rollback), OpenSpec deltas for `ui`,
  `storage` and `ipc-commands`, CHANGELOG `[Unreleased]`.

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `ui`: a new "Text viewer reading controls" requirement covers the toolbar
  segments and stepper, the immediate persistence with rollback, the CSS
  `zoom`/narrow-column behavior in every display mode and horizontal scrolling
  for wide blocks; "Design tokens" gains `--ruvox-reading-measure` and
  `--ruvox-reading-scale`.
- `storage`: "Config File Schema" gains `reading_width` and `reading_scale`
  with their defaults, canonicalization and clamping rules, and scenarios for
  both.
- `ipc-commands`: "Shared IPC Types" gains the two `UIConfig` fields and their
  defaults; "Configuration Commands" canonicalizes both values when they arrive
  through a `UIConfigPatch`.

## Impact

- Affected code: `src-tauri/src/storage/schema.rs`,
  `src-tauri/src/storage/service.rs`, `src-tauri/src/commands/mod.rs`,
  `src-tauri/src/commands/tests.rs`, `src/lib/tauri.ts`, `src/lib/reading.ts`
  (new), `src/components/ReadingControls.tsx` (new),
  `src/components/TextViewer.tsx`, `src/components/TextViewer.module.css`,
  `src/components/AppShell.tsx`, `src/dialogs/Settings.tsx`,
  `src/lib/engineSelection.ts`, `src/i18n/ru.ts`, `src/i18n/en.ts`,
  `src/globals.css`, tests, `openspec/`, `CHANGELOG.md`.
- `config.json` gains two optional fields; configs written by earlier builds
  parse unchanged and get the defaults. No history migration.
- User-visible: a reading pane controlled by width and scale in every viewer
  mode, plus the same controls in Settings; both choices survive restarts.

## Non-goals

- Font family or bundled fonts; per-mode, per-entry or per-format typography.
- A `reading_font_size` field or the A−/A+ font-size presets from #354.
- Line-height control, arbitrary percent input, Ctrl+wheel zoom, UI-wide zoom,
  or image-only zoom.
- Reorganizing the Settings dialog (#355) beyond adding the «Чтение» block.
- Changing playback, synthesis, queue-card or display-mode behavior.
