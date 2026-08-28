# Design: add-normalization-explainer

## Context

`PreviewDialog.tsx` is a `react-rnd` floating window inside a Mantine
`Portal`. The header is also the drag handle; the panes live in
`classes.panes`; the footer holds the controls. i18n is a flat key→string map
in `src/i18n/{ru,en}.ts` with `{0}`-style placeholders. The opener plugin
(`@tauri-apps/plugin-opener`, capability `opener:default`) is already a
dependency — used in Settings for `revealItemInDir` — and its default scope
permits `https` URLs, so opening the README needs no capability change.

## Goals / Non-goals

- Goals: the first-time user understands what the right pane is before
  pressing «Синтезировать»; a durable place (README) documents normalization
  in user terms.
- Non-goals: no pipeline/backend changes; no in-app help viewer; no wiring of
  the dormant `code_block_mode` / `read_operators` config fields (they are
  not consumed by the pipeline — the README documents only behavior that
  exists: the source-format selector and the `<!-- ruvox-code: full|brief
  -->` directives).

## Decisions

### D1: explainer placement — a fixed line under the header, not a dismissible banner

A one–two sentence `Text` in a muted style, rendered between the header and
`classes.panes` on every open (per the delta spec). No dismiss state: the
line is short, and a dismiss flag would add config surface to solve a
non-problem. The dialog's `MIN_H` (380) already leaves room; the line wraps
inside a normal flow element, so no geometry changes are needed.

### D2: help affordance — a Popover, not a Tooltip

The fuller explanation must contain a clickable link. A Mantine `Tooltip`
hides on pointer movement toward its content (no pointer events by default)
and is hostile to links; a click-toggled `Popover` anchored to the header
icon keeps the content stable and is straightforward to drive in unit tests
(open → assert text → click link). The icon is a `ActionIcon variant="subtle"`
with an aria-label, placed before the close button. Popover content carries
the full copy plus the «Подробнее в README» link. The spec's "toggle the
tooltip" wording is satisfied by this toggle behavior.

### D3: the README link target and opener

Target: `https://github.com/xilec/RuVox#нормализация` — the user-facing
README's normalization section on the default branch. GitHub lowercases
Cyrillic heading anchors (github-slugger), so the fragment must be lowercase
to match the `## Нормализация` heading. Opened via `openUrl` from
`@tauri-apps/plugin-opener`; fire-and-forget is not acceptable per the craft
rules, so a rejected promise surfaces as a red error notification (same
pattern as other frontend command errors). If the heading is ever renamed,
the anchor update is part of that edit (both README mirrors).

### D4: copy ownership and language

All new strings live under `preview.explain.*` in both dictionaries; Russian
is canonical (user-facing), English is the mirror, same as existing keys. The
copy avoids the words «парсинг/пайплайн» and speaks in outcomes («то, что
будет прочитано вслух»).

### D5: README section structure

`README.md` gains a «Нормализация» section after «Возможности»: what gets
rewritten (identifiers camelCase/snake_case, abbreviations, numbers and
dates, URLs/emails, operators, code blocks), then «Как управлять»: the
source-format selector (Авто / Обычный текст / Markdown / HTML) in the
preview dialog, the `<!-- ruvox-code: brief|full -->` per-document
directives with the «далее следует пример кода…» brief form, and the
Mermaid marker. `README.en.md` is regenerated from it (translation, not a
hand edit). The existing one-line bullet in «Возможности» stays and links
to the new section.

## Risks / Trade-offs

- **Anchor fragility** (D3): a renamed README heading breaks the deep link.
  Mitigated by the mirror-regeneration rule tying heading edits to both
  files; the link failing open (browser shows the README top) is benign.
- **Vertical space in the dialog** (D1): one wrapped line at `MIN_H`. If it
  ever becomes cramped at 380 px, the fallback is moving the line into the
  Popover only — a copy move, not a spec change (the spec fixes presence,
  not exact pixels; the scenario asserts reachability of controls).
- **Link opens a browser outside the app**: acceptable single-user desktop
  behavior; an in-app viewer is explicitly out of scope.

## Migration Plan

None — purely additive UI copy and docs.

## Open Questions

None.
