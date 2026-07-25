# RuVox — code quality (craft rules)

The craft standard for writing and reviewing code here, across six categories.
**Single source — pulled on demand:** `openspec-apply-change` reads this while
implementing (write to it); `ruvox-reviewer` reads it while reviewing (it flags
departures). Project invariants (language, toolchain, architecture boundaries,
the TTS constraint, Rust/TS/Mantine/Python hard rules, testing gates) live in
[conventions.md](./conventions.md) and are not repeated here.

These are heuristics, not absolutes — correctness and clarity win over any
single rule. Each rule names its rationale so you can tell when an exception is
justified.

## Code-to-file layout

How to decompose code across files — optimized for two axes at once.

- **Split by seam (responsibility / cohesion), not by line count.** A line
  count is a prompt to think, never a verdict; mechanical ">1000 lines" splits
  raise coupling (`utils`/`helpers` grab-bags appear). A split is justified
  only when there's a concrete seam.
- **Axis 1 — comfort of writing correct code:** high cohesion (one reason to
  change per file), low coupling, narrow explicit interfaces, no mixed
  abstraction levels, no tangled shared mutable state.
- **Axis 2 — context-window economy** (an agent's budget is ~220k tokens before
  compaction): a task's working set is every file needed for a *correct* edit,
  and coupling multiplies it. So: keep core-logic files within one read pass
  (**target ≤ ~1500 lines**); make module boundaries narrow and explicit (load
  an interface, not an implementation); **physically separate read-only
  reference** (data, fixtures, generated code, tests) from edited logic;
  co-locate what changes together. The axes usually agree; where they conflict,
  correctness wins.
- **Inline tests over ~750 lines → move to a sibling test file** in the same
  module (Rust: `#[cfg(test)] #[path = "x_tests.rs"] mod tests;` reaching
  privates via `super::*`, or `x.rs` → `x/mod.rs` + `x/tests.rs`). Tests stay
  co-located; the source's real size becomes visible.
- **Some files are immune to the *size* trigger** (but not to content
  triggers): predominantly declarative data (golden pipeline fixtures, i18n,
  config), generated files (`*.lock`, `pnpm-lock.yaml`, `Cargo.lock` — never
  hand-edited), and a single cohesive algorithm / state machine where cutting
  would scatter tight invariants.
- **Cut well** (when a seam exists): along name clusters / prefixes; extract
  pure leaf functions first; one entry module re-exports, internals private;
  cut downward (helpers into a submodule), not sideways; group by
  feature/domain over technical layer.

## Tests & coverage

- **Cover changed behavior.** New/changed production logic needs a test —
  especially a new public fn, `match` arm, error variant, conditional, or
  boundary. Error paths and edges (empty / zero / boundary / Unicode incl.
  `е`≡`ё`) are the usual misses, not the happy path. Coverage % is a weak
  signal (Goodhart) — a line executed but unasserted is worse than an honest
  gap.
- **Pin behavior, not existence.** Assert the value / state change / side
  effect, not "doesn't panic" or a bare `is_ok()`. Drive through the **public
  surface** so harmless refactors don't break tests.
- **Pipeline changes → golden fixtures.** A change to
  `src-tauri/src/pipeline/` adds or updates a fixture in
  `src-tauri/tests/fixtures/pipeline/` that pins the new behavior; a pipeline
  bug fix first adds a fixture reproducing the bug.
- **Deterministic & isolated.** No wall-clock / `sleep` / `setTimeout` races /
  unseeded RNG; async UI via `waitFor`, not fixed delays; no real network /
  filesystem / TTS engine in unit tests; tests share no mutable state and pass
  in any order. Favor many fast unit tests over slow integration / e2e.
- **Clarity over DRY (DAMP).** Name = scenario + expected outcome;
  arrange-act-assert visible; no logic (`if`/loop/`match`) in tests; literals
  over computed values. A little duplication is fine; over-DRY setup that
  buries the values a test depends on is not.

## Duplication & reuse

- **DRY is one home for a piece of knowledge / intent, not identical text.**
  Consolidate when the *same fact / rule / decision* lives in 2+ places. Leave
  coincidental look-alikes that change for *different reasons* — merging them
  couples unrelated concerns. Heuristic: if consolidating would force a flag /
  param to tell the cases apart, keep them separate.
- **Resist hasty abstraction (AHA / Rule of Three).** Let a pattern emerge
  (~3rd occurrence) before abstracting — the wrong abstraction is costlier than
  a little duplication.
- **Reuse what exists.** Prefer an existing helper in `src/lib/`, a Mantine
  component/hook, the stdlib, or a current dependency over re-implementing it.
  Copy-pasted blocks are a bug-duplication trap.
- **Dictionaries and normalization tables have one home.** A pronunciation /
  transliteration rule must live in a single dictionary or table in the
  pipeline — never mirrored across stages (they *will* drift).

## Rust & TS idiom

Complements the automated gates (`clippy`, `tsc --strict`, `eslint`, `knip`,
`ruff`) — these are the idioms that compile and lint clean but are still
non-idiomatic or risky.

**Rust**
- Don't swallow `Result` (`let _ = fallible()`); propagate with `?` or handle
  it, adding context at the right layer (`thiserror` in the domain, `anyhow`
  only at edges).
- Avoid `unwrap` / `expect` / `panic!` evasions that still compile:
  `.ok().unwrap()`, panicking `[]` indexing. In tests prefer `expect("why")`
  over bare `unwrap()`.
- Ownership over cloning: avoid needless `clone()` and reflexive
  `Arc<Mutex<…>>`.
- **Exhaustive `match` in domain logic** — enumerate variants over a catch-all
  `_` so a new variant fails to compile (pipeline token/event types especially).
- Conversions & naming per the Rust API Guidelines (`as_` / `to_` / `into_` by
  cost).

**TypeScript**
- No `any` escapes the typechecker can miss: `as any`, `as unknown as T`
  double-casts, untyped `JSON.parse`, `@ts-expect-error` / `@ts-ignore` without
  a reason. Prefer `unknown` + narrowing.
- `as` casts and `!` bypass the checker — use a type guard or honest typing
  instead; `!` on a possibly-absent value is a latent crash.
- **Discriminated unions** over boolean / optional-field soup, so a new variant
  fails to compile (Tauri event payloads especially).
- `??` / `?.` over hand-rolled null checks; `readonly` / `as const` over
  `enum`.

**Python (ttsd)**
- Protocol messages are typed dataclasses / TypedDicts from
  `ttsd/protocol.py` — no ad-hoc dicts crossing the stdin/stdout boundary.
- Never print to stdout except protocol JSON; logging goes to stderr.

## Security & untrusted input

Weight proportionately — a single-user desktop app, not a hardened target. The
untrusted input here is **text the user pastes or loads** (clipboard, files),
which flows into two sensitive sinks:

- **Webview rendering:** markdown/HTML is rendered in the webview — it must
  pass through the existing sanitize pipeline (markdown-it + DOMPurify). No new
  `dangerouslySetInnerHTML` / raw-HTML sink without sanitization; no new HTML
  injection path that bypasses DOMPurify.
- **Subprocess spawning (ttsd, mpv):** arguments are passed as argv arrays —
  never shell-interpolated strings built from user text.
- **Secrets:** none in the repo; no tokens/keys in code, fixtures, or logs.

## Correctness & bugs

Reason about every input, state, and failure — not just the happy path. Tie a
concern to a concrete breaking input.

- **Logic & edges:** off-by-one (`<` vs `<=`, 0/1-based), inverted condition;
  empty / single / max-size inputs; first and last iteration. For the pipeline:
  boundary between tokens, adjacent code blocks, empty lines, `е`/`ё`,
  Cyrillic/Latin mixed words.
- **Text positions:** char vs byte vs UTF-16 code unit indexing — Rust strings
  are UTF-8 bytes, JS strings are UTF-16, and position mapping across the IPC
  boundary must state which unit it uses (spec `position-mapping`).
- **Absence & failure:** handle `None` / null / undefined; a default must not
  mask a real error; a multi-step mutation that can fail midway (synthesize →
  save audio → write history) needs a defined partial-failure behavior.
- **State & invariants:** recompute derived / cached state when its source
  changes; watch use-before-init; queue and player state machines must not
  reach impossible states (specs `queue-lifecycle`, `playback`).
- **Concurrency & async:** avoid read-decide-write races; **never hold a lock
  across `.await`**; no floating (un-awaited) promises in TS; React effects
  clean up their listeners/timers; Tauri event listeners are unlistened on
  unmount.
- **Process lifecycle:** ttsd can be slow to load, crash, or hang; mpv is an
  external process — every wait on them needs a defined timeout / failure path,
  and shutdown ordering matters (spec `ttsd-protocol`).
