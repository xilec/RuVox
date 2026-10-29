# Proposal: dictionary stress marks

## Why

A user can respell a word in the dictionary but cannot say *where the stress
goes*. For names, abbreviations and loanwords that is the difference between
"з**а**мок" and "зам**о**к", and the engine's automatic accentuation guesses
wrong often enough that #307 lists stress as a per-engine capability. The
infrastructure for that capability now exists (`TextProfile::stress`), and the
next engine (#308 TeraTTSv2) expects explicit `+` marks to survive the
pipeline — but nothing produces them: a dictionary entry `М+осква` would
insert the `+` verbatim into the synthesis text for every engine, including
the ones that cannot read it.

## What Changes

- **Stress mark in the dictionary** — `to` may carry one `+` immediately
  before a vowel. It is validated when the entry is saved, preserved verbatim
  in the TOML file so hand-editing and export/import round-trip, and stripped
  from the spoken form the pipeline inserts for engines without stress support.
- **Rendering by the pipeline, gated by the profile** — when the active
  profile declares stress support (`Stress::Auto` or `Stress::Manual`), the
  replacement keeps the mark; otherwise it is inserted without it. The mark is
  part of the dictionary replacement, so no later phase re-reads it.
- **`Stress::Auto` for the native Silero engine** — its accentor already
  honours user `+` markers and otherwise predicts stress itself; declaring the
  capability is what makes the mark reach it. The Python sidecar and Piper stay
  `None` until the probe (#311) says otherwise.
- **Editor affordance** — the dictionary form gets a help icon next to the
  spoken form explaining the mark and that it applies only to engines with
  stress support, and an inline error for a malformed mark. The count limit
  keeps applying to the spoken form, not to the mark.

## Capabilities

- Modified: `user-dictionary` (entry semantics, storage round-trip, editor
  hint), `text-pipeline` (dictionary application renders the mark).

## Non-goals

- Automatic stress placement for words without an entry: engines that do it
  already do it internally, and pre-placing marks is a different problem
  (language model or dictionary of stresses).
- Per-engine stress syntax: the engines that matter here take the same `+`
  before the vowel; a different syntax lands with the engine that needs it.
- Stress support for the Python sidecar and Piper: no evidence yet (#311).

## Impact

- `src-tauri/src/dictionary/` (parse, validate, storage round-trip),
  `src-tauri/src/pipeline/` (gated insertion), `src-tauri/src/tts/profiles.rs`
  (capability values), the dictionary editor and its i18n strings, and the
  golden corpus (one profile-pinned fixture for the mark).
