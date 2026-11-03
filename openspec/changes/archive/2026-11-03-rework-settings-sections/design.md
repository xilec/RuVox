## Context

`src/dialogs/Settings.tsx` had grown into a single ~950-line scroll holding
every control, with the save/reset buttons at the very bottom and no way to
navigate. #355 asks for a two-pane dialog with sections, search and dirty
markers. The load-bearing constraints: one form state, the same `UIConfigPatch`
for the same edits, no new settings, and the existing controls (including the
reading width/scale from #354/#357 and the nested models/cleanup/dictionary
dialogs) must keep their behavior.

## Decisions

### Section catalogue, search index and dirty mapping in a pure module

`src/lib/settingsSections.ts` carries the section/row catalogue, the
both-catalog search terms and the field-to-section mapping. It imports no React
and no Tauri commands, so the search and dirty rules unit-test in isolation
(same split as `engineSelection.ts`). Each rendered row carries
`data-settings-field`; the reachability test walks the nav and asserts every id
appears exactly once.

Rejected:

- **CSS `display: none` filtering.** Hidden controls stay focusable and in the
  accessibility tree, and "sections without matches hide" would become a
  styling assertion instead of a DOM one.
- **One component per section fed from the parent's whole form API by
  duplication.** Copying the form into per-section components would either
  duplicate state or need a context with the same surface; a single
  `SettingsSections` component switching on the section id keeps one form state
  and one prop list.

### Dirty state is a diff against the loaded config

The dialog stores `baseline = loaded form values` when the config arrives and
derives the changed fields with a strict per-field comparison (all fields are
primitives). This keeps «Сброс» honest: after reset the markers compare against
the saved config, not against the factory defaults.

Rejected: Mantine's `form.isDirty()` snapshot. `resetDirty(loaded)` is the
documented way to rebase the snapshot, but `form.reset()` restores the mount
snapshot, so after «Сброс» the dialog would report zero changes even when the
defaults differ from the saved config. A dedicated baseline state also makes
the coercion rule explicit: the baseline stores the coerced engine, so the
fallback can never count as a change.

### Search matches both catalogs

For every row the index contains `translate('ru', key)`, `translate('en', key)`
for each label variant plus the section title and the row's synonyms. Queries
are trimmed, lowercased and ё-folded; whitespace-separated tokens must all match
some term. The highlighter only marks a literal substring of the visible label —
a synonym or the other catalog's wording still filters the row but has nothing
to mark.

Rejected: indexing only the active locale (a Russian term typed while the UI
runs in English would return nothing, which is the exact complaint in #355);
regex/pinyin-style fuzzy matching (unnecessary for a dozen rows).

### Esc and Enter are explicitly gated

`closeOnEscape` is `false` while a query is present (or the nested cleanup modal
is open), and the search form's keydown handler clears the query, so Esc peels
the search first and only a second Esc closes the dialog. Enter calls
`preventDefault()` — a bare Enter in a text input inside the form would submit
and save — then records the first matching row in `jumpTarget`; an effect
scrolls that row after the section swap has committed.

### Layout and the narrow breakpoint

The modal uses `size={900}` (Mantine caps the content at the viewport) and a
CSS Module that fixes the content height, makes the panes flex and lets only the
content pane scroll; the footer sits outside the scroll container, so it cannot
scroll away. The narrow breakpoint uses Mantine's `useMediaQuery('(max-width:
700px)')` and conditionally renders a `Select` instead of the nav, so the
collapsed control is real keyboard-reachable DOM rather than a hidden duplicate.

Rejected: a CSS-only media query that hides the nav and shows the select — it
keeps two controls for the same purpose in the DOM and cannot be asserted as a
behavior in unit tests without computed-style emulation.

## Risks / trade-offs

- The section bodies moved to `SettingsSections.tsx`; the patch-building path is
  untouched, and the existing `Settings.test.tsx` cases still pin the submitted
  `UIConfigPatch`.
- The search index is static metadata; adding a control without adding its row
  to `SETTINGS_ROWS` leaves it out of search. The reachability test fails when a
  rendered `data-settings-field` is not in the catalogue, which is the guard.
- Dirty markers are computed on every render (16 primitive comparisons) — well
  below the cost of memoization.
