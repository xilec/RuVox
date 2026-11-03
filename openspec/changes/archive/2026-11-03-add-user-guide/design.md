# Design — add-user-guide

## Context

`README.md` sells the product, `openspec/specs/` records behavior for
developers, and `CHANGELOG.md` records history. None of them tells a user what
to click after installing RuVox. The guide is the missing user-facing layer:
one short section per topic, with schematic ASCII mockups that survive UI
drift and stay reviewable in a diff.

The same change removes `UIConfig::player_hotkeys`: it still has defaults for
ten shortcuts, but nothing reads it. Documenting from the schema would promise
keys that do nothing, so the field goes away and the hotkey section lists only
the keys implemented by the components.

## Goals / Non-Goals

**Goals:**

- A Russian guide at `docs/user-guide.md`, one click from `README.md`, covering
  the full user path and every shipped feature that changes what the user sees.
- ASCII mockups in one convention: fenced `text`, box drawing, Russian labels,
  window chrome elided, only the control the section is about, ~80 columns.
- A hotkey table that matches the code exactly.
- Removal of the dead `player_hotkeys` config field without breaking old
  `settings.json`/`config.json` files.
- A hard rule that keeps the guide from rotting, with a pointer from
  `AGENTS.md`.

**Non-Goals:**

- English translation, screenshots, in-app onboarding.
- Configurable hotkeys.
- Rewriting developer docs or the OpenSpec specs outside the `ipc-commands`
  delta.

## Discovery pass

Walked the README feature list and the requirements of
`openspec/specs/{ui,queue-lifecycle,playback,text-display,text-import,model-bundles,viewer-copy-actions,tray,auto-update}`,
plus the capabilities the issue listed as candidates (`storage`, `logging`,
`preview-dialog`, `user-dictionary`, `html-ingestion`, `word-highlight`).
"Source" names the spec or README section that records the behavior.

| # | Feature | Source | Decision | Reason |
|---|---------|--------|----------|--------|
| 1 | First-run TeraTTSv2 prompt, ~410 MB bundle, "stay on Piper" | `ui` first-run prompt, `model-bundles`, README | Include | Default path for every fresh install; declining is non-persistent. |
| 2 | Silero Native prompt (~230 MB) for an existing config | `ui` first-run prompt, README | Include | The second bundle engine; users switching later must recognize the prompt. |
| 3 | Engine inventory, versions, download/update/delete, Piper voices | `model-bundles`, `ui` Settings dialog | Include | The only place to manage models; active-model restrictions need explaining. |
| 4 | Engine/voice/sample-rate settings and availability coercion | `ui` Settings dialog | Include | Core quality choice; a disabled engine with a reason is otherwise confusing. |
| 5 | Expected first-synthesis latency and memory cost | `teratts-engine`/`silero-native-engine` benchmarks, README | Include | Issue asks for it; the guide states cold-load times and background synthesis. |
| 6 | Clipboard add via the primary «Добавить» button | `ui`, `queue-lifecycle` | Include | Primary add path. |
| 7 | Paste (`Ctrl+V`) into the window | `ui` application shell | Include | Undocumented but shipped; a user expects it. |
| 8 | Drag & drop files and links with the drop overlay | `text-import` drag & drop | Include | Primary import path; overlay text is user-visible. |
| 9 | «Файл…», «Файл с кодировкой…», «По ссылке…» menu | `text-import` split-button menu | Include | The three import variants need a one-line difference each. |
| 10 | Encoding detection and manual override | `text-import` encoding requirements | Include | The main cause of garbled text; the override is non-obvious. |
| 11 | URL import and JS-rendered page error | `text-import` URL fetching | Include | User-facing; the error is self-explanatory, so only the happy path is documented. |
| 12 | HTML clipboard extraction | `html-ingestion` | Include | Explains why browser paste keeps markup but not scripts. |
| 13 | Preview original vs normalized panes | `preview-dialog` | Include | The product's trust mechanism before synthesis. |
| 14 | Source format selector and auto-detection | `preview-dialog` | Include | Wrong detection is a common surprise; manual override lives here. |
| 15 | Editing text before synthesis | `preview-dialog` | Include | The only way to fix a text before it is queued. |
| 16 | «В словарь» from the preview | `user-dictionary` quick-add | Include | Cross-links preview and dictionary. |
| 17 | Read-now, sync scroll, "don't show again" | `preview-dialog` | Include | Each changes the add flow. |
| 18 | Regeneration preview | `preview-dialog` regeneration | Include | Regeneration is destructive-looking; the confirmation explains the old audio is kept. |
| 19 | Queue statuses, progress, inline cancel | `queue-lifecycle`, `ui` | Include | The user must read state at a glance. |
| 20 | Queue search over title, note and text | `queue-lifecycle` queue search | Include | The only way to find an entry in a long queue. |
| 21 | Per-entry actions: play, export, regenerate, params, delete | `queue-lifecycle` per-entry actions | Include | The context menu is the queue's main surface. |
| 22 | Export WAV / Ogg Opus and the save dialog | `queue-lifecycle`, README | Include | Format choice happens in the native dialog, not in the app. |
| 23 | Recording parameters dialog | `ui` recording parameters | Include | Answers "what made this audio?" and is reachable by double-click. |
| 24 | Persisted failure reason, «Повторить», «Показать полностью» | `queue-lifecycle`, `ui` | Include | Failure recovery is a user path, not an internal detail. |
| 25 | Editable title/note and metadata row (#353) | `text-display` viewer entry header | Include | Required by the issue; title/note also feed search. |
| 26 | Playback controls, speed up to 3×, volume, seek | `playback`, `ui` player controls | Include | Core playback. |
| 27 | Word highlight and seek re-highlight | `word-highlight`, `text-display` | Include | The fallback for unclear words is the point of the viewer. |
| 28 | Display modes (Markdown/HTML vs source) | `text-display` | Include | The switch is in the header and affects what is highlighted. |
| 29 | Mermaid rendering/click-to-zoom, table scrolling | `text-display` | Include | User-visible in technical articles. |
| 30 | Viewer copy actions: link, selection, image | `viewer-copy-actions` | Include | Links are inert, so copying is the only action; issue lists it as a candidate. |
| 31 | Select-all inside the viewer | `text-display` select-all | Include | `Ctrl+A` is in the hotkey scope. |
| 32 | Reading width and scale (#354/#357) | `ui` reading controls | Include | Required by the issue; persisted app-wide. |
| 33 | Settings sections and search (#355) | `ui` Settings dialog | Include | The dialog is now navigable; search semantics are not obvious. |
| 34 | English reading mode and fallback hint | `ui` Settings dialog, multi-language synthesis | Include | Changes pronunciation and has a per-engine fallback. |
| 35 | Code block narration modes | `ui` Settings dialog | Include | Changes what is heard in technical texts. |
| 36 | Notifications for ready/error | `ui` Settings dialog | Include | Users may want them off. |
| 37 | Storage: cache limit, data folder, cleanup | `storage`, `ui` Settings dialog | Include | Issue lists "storage location + cleanup" as a candidate; destructive cleanup needs warning. |
| 38 | Logs folder and open-folder action | `logging`, `ui` Settings dialog | Include | Issue lists log/diagnostics location as a candidate; needed for troubleshooting. |
| 39 | Theme and UI language | `ui` Settings dialog | Include | Visible preferences. |
| 40 | Auto-update prompts and manual check | `auto-update`, `ui` | Include | Issue lists autoupdate prompts as a candidate; platform limits need stating. |
| 41 | Dictionary scopes: global vs per-engine | `user-dictionary` | Include | The scope selector changes which words apply. |
| 42 | TOML file, import/export, drop, merge/replace | `user-dictionary` | Include | Hand-editing and sharing dictionaries are supported. |
| 43 | Stress `+`, forms, override badge | `user-dictionary` | Include | The main correctness tools for pronunciation. |
| 44 | Dictionary pronunciation preview (#349) | `user-dictionary` | Include | Required by the issue. |
| 45 | Tray: enable, icon, menu, show/add/quit | `tray` | Include | Background mode is a shipped feature with a surprising playback caveat. |
| 46 | Hotkeys `Space`, `Left`/`Right`, `Ctrl/Cmd+F`, `Ctrl/Cmd+A`/`Ctrl/Cmd+C` on a link, `Esc` | `playback`, `ui`, `text-display`, `viewer-copy-actions` | Include | Issue scope; verified in `Player.tsx`, `QueueList`, `TextViewer` and dialogs. |
| 47 | Troubleshooting: bundle, no sound, garbled text, pronunciation | README, `model-bundles`, `playback`, `text-import`, `user-dictionary` | Include | Issue scope. |
| 48 | Entry text edit mode | `docs/index.md` feature list | Skip | No implementation exists in the current UI (only title/note are editable); the stale claim is removed in this PR and the guide says queued text cannot be edited. |
| 49 | Global (system-wide) hotkeys | `playback` window-scoped hotkeys | Skip | Explicitly not implemented; documenting them would be false. |
| 50 | Configurable player hotkeys | `player_hotkeys` schema | Skip | Dead field; removed by this change. |
| 51 | In-app onboarding, tooltips, help overlay | issue scope | Skip | Out of scope. |
| 52 | Screenshots | issue scope | Skip | Stale screenshots mislead; ASCII mockups replace them. |
| 53 | English translation of the guide | issue scope | Skip | Out of scope; README.en only links to the Russian guide. |
| 54 | Developer docs, crate architecture, spec rationale | `docs/`, `openspec/` | Skip | Different audience; the guide stays user-facing. |
| 55 | Log rotation, levels, `RUST_LOG` | `logging` | Skip | Developer-facing; the guide only points at the log folder. |
| 56 | Channel files, manifest hashes, release procedure | `model-bundles`, `docs/model-bundles.md` | Skip | Maintainer procedure, not a user action. |
| 57 | History file format, legacy layouts, eviction internals | `storage` | Skip | Internal; only the data folder and cleanup UI are user-facing. |
| 58 | Position mapping, timestamp estimation internals | `position-mapping`, `word-highlight` | Skip | Internal; the observable highlight is covered. |
| 59 | Windows installer / Linux runtime packaging details | `windows-installer`, `linux-runtime` | Skip | Installation docs cover them; the guide only needs the mpv consequence. |
| 60 | ttsd protocol, Piper/TeraTTS engine internals | `ttsd-protocol`, `piper-engine`, `teratts-engine` | Skip | Developer-facing. |
| 61 | Dictionary TOML version/migration details | `user-dictionary` storage format | Skip | Internal; the guide names the file and says to restart after hand-editing. |
| 62 | Mermaid click directives and sanitization | `text-display` | Skip | Security/internal detail; the zoom behavior is covered. |

## Decisions

**D1. Russian guide in `docs/user-guide.md`, linked from README.** The
repository language policy splits by audience: `README.md` is the Russian
user-facing entry, developer docs are English. The guide sits with README on
the user side. `README.en.md` gets a link to the same Russian guide (the
translation is an explicit non-goal) and `docs/index.md` gets a row so the
guide is discoverable from the documentation index.

**D2. ASCII mockups, one convention.** Fenced `text`, box drawing, Russian
labels, window chrome elided, only the control the section is about, target
≤80 columns. The mockups are diffable and cheap to update; screenshots are
rejected because a stale screenshot misleads more than no screenshot.

**D3. Document only implemented hotkeys; delete the dead config.** The
player hardcodes `Space`, `Left`, `Right` in `Player.tsx`; `Ctrl/Cmd+F` lives
in `AppShell`; `Ctrl/Cmd+A` and link-copy are in `TextViewer`/viewer copy
actions; `Esc` closes dialogs and clears search. `player_hotkeys` has no
consumer, so the field, its default map, the patch branch, the TS type and the
test mocks are removed. Rejected: keeping the field and documenting it —
that would promise non-working keys; keeping it undocumented — dead code with
a misleading schema.

**D4. Old config files keep working.** `UIConfig` does not use
`#[serde(deny_unknown_fields)]`, so an existing `config.json` with
`player_hotkeys` deserializes normally; the next save serializes only the
struct fields, so the key disappears. A new Rust test loads a config with the
legacy key, asserts the rest is intact, and asserts the serialized value has
no `player_hotkeys` key. Rejected: a migration that rewrites the file at
startup — unnecessary I/O for a cosmetic cleanup, and the load rule already
covers it.

**D5. One OpenSpec change with an `ipc-commands` MODIFIED delta, archived in
the same PR.** `openspec/specs/` is never edited in place. The delta reproduces
the whole "Shared IPC Types" requirement, removes the `player_hotkeys` line,
adds the missing `language` field so the delta describes the actual `UIConfig`
shape, and adds a legacy-key scenario. The change is archived after
`validate --strict`, then `validate --specs --strict` confirms the synced
specs. Rejected: a separate change for the guide and another for the config
removal — they are one reviewable docs/cleanup unit and the hotkey section is
the reason for the removal.

**D6. A hard rule, not a suggestion.** `ai/rules/conventions.md` gains a rule
that a PR changing UI structure or observable feature behavior updates
`docs/user-guide.md` in the same PR (ASCII mockup and hotkey list included),
and that a stale guide is a review finding. `AGENTS.md`'s documentation table
links the rule and the guide. Rejected: relying on the issue text alone —
future PRs would not see it.

## Risks / Trade-offs

- The guide duplicates labels that live in `src/i18n/ru.ts`; a UI label
  rename can stale the mockups. The new convention rule makes the guide a
  required review artifact for UI changes.
- The guide is Russian only, so English readers get a link to Russian text.
  Accepted: the guide is user-facing and the project's user entry is Russian.
- Removing `player_hotkeys` changes the persisted config shape; a downgrade to
  an older build would default the map again (harmless, since nothing reads
  it).
- The mockups are schematic and intentionally omit chrome; a user looking for
  pixel-perfect placement will not find it. Accepted per the issue.
- The `ipc-commands` delta is large relative to the change; a whitespace
  difference after archive would surface as an unrelated diff. Mitigation:
  the delta is generated from the synced requirement and diffed after archive.

## Migration Plan

1. Remove the field and its patch branch; add the legacy-key test.
2. Update the TS type and the UI test mocks.
3. Write the guide, README links and the convention rule.
4. Add the `ipc-commands` delta, validate, archive.
5. Run the gates and open the PR.

Rollback: revert the branch; old config files are unaffected either way.

## Open Questions

(none)
