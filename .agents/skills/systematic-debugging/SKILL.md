---
name: systematic-debugging
description: >-
  Systematic debugging of a bug, defect, crash, regression, or test failure:
  reproduce → localize → hypothesize → find the ROOT CAUSE (not just the
  symptom) → fix and lock it with a test/invariant → verify behaviour is
  unchanged except the fix. Core rule: when a candidate fix would only patch a
  symptom, stop and ask why the bug was possible at all; if the cause is
  structural (duplicated logic, no single source of truth, no compiler/type
  invariant, missing golden fixture, magic values), surface fixing the CAUSE
  as an explicit option with a recommendation instead of silently shipping
  the minimal patch. Use whenever investigating or fixing any bug or
  unexpected behaviour.
whenToUse: Use when investigating or fixing any bug, regression, crash, flaky test, or unexpected behaviour.
---

# Systematic debugging

A bug is not done when the symptom stops. It is done when you understand **why
it was possible** and have made that class of bug **harder or impossible to
reintroduce**. This skill is the loop that gets you there, and — most
importantly — the discipline to raise altitude from "fix this instance" to
"close this class" without being told.

## When to use

Any time you are investigating or fixing a bug, regression, crash, flaky test,
or "this behaves wrong" report. Even a one-line fix gets the altitude check
(step 4) — that is exactly where the value is, and exactly where it is easiest
to skip.

## The loop

### 1. Reproduce — don't fix what you haven't seen fail
- Pin down a **minimal, deterministic** repro: smallest input / steps that
  trigger it. Write down expected vs. actual.
- For pipeline bugs the repro is a text snippet — it becomes the golden
  fixture later.
- No repro ⇒ no fix. A fix you can't see fail is a guess; a fix you can't see
  pass is unverifiable.

### 2. Localize — narrow to a line and an invariant, not "somewhere here"
- Bisect: `git bisect` over commits, binary-search over the code path or the
  data, disable half and see which half carries the bug.
- Add observation at boundaries (logs/asserts on the values crossing a seam:
  pipeline stages, the IPC boundary, the ttsd protocol), not random print
  spraying.
- Stop when you can point at the exact line **and** state the invariant it
  violates ("every percent number must keep its sign through normalization,
  but this branch drops it").

### 3. Hypothesize — one variable at a time
- State a hypothesis and **predict what you'd observe if it's true**, then
  check.
- Change one thing per test. Don't fix on the first plausible guess without
  confirming it's the actual cause — correlation is not cause.

### 4. Root-cause altitude — the step that's easy to skip ⭐
This is the heart of the skill. Once you've found the broken line, **stop
before patching** and ask:

> **Why was this bug possible at all?**

If the honest answer is structural, the minimal patch leaves the *class* of
bug alive. Smell-triggers that mean the cause is structural:

- The same **rule / mapping / constant is duplicated** in N places (pipeline
  stages, Rust↔TS mirrors — they *will* drift; that's how this bug happened).
- There is **no single source of truth**; a mapping can be "forgotten" in one
  copy.
- Correctness rests on **discipline, not an invariant** — a comment that says
  "remember to also update X", an easily-omitted case.
- **Magic values / implicit contracts** that nothing enforces (char vs byte
  vs UTF-16 positions, env-dependent native data paths).
- A behavior with **no golden fixture / test** pinning it, so a refactor can
  silently change it.

When the cause is structural, **do not silently ship the minimal patch, and
do not silently rewrite the world either.** Surface the choice with a
recommendation:

- **(a) Minimal fix** — fast, lowest diff/risk, but the class of bug survives.
- **(b) Fix + remove the cause** — single source of truth / an enforced
  invariant, so this *can't* recur.

State which you recommend and why. Scope limits what you *do*; it never
excuses not *raising* the option. ("It's just a side-bug / out of scope" is a
reason to keep the change small, not a reason to stay silent about the root
cause.)

**Prefer fixes that make the error impossible over fixes that ask people to
remember.** Rank, best first: compiler/type enforcement (exhaustive `match`
with no `_`, non-optional fields, newtypes) > single source of truth derived
once (one dictionary/table for a normalization rule) > a test or golden
fixture that fails on the class > a comment. A comment is the weakest lock
and the first to rot.

### 5. Fix — and lock the class
- Add a **regression test** that reproduces the original symptom and now
  passes. For `src-tauri/src/pipeline/` that means a **golden fixture** in
  `src-tauri/tests/fixtures/pipeline/` with the repro text.
- Where step 4 found a structural cause, add the **structural lock** too
  (invariant / single source) so the next variant can't reintroduce it.
- A fix without a lock means the bug comes back under a new name.

### 6. Verify — prove nothing else moved
- Show the behaviour change is **exactly** the intended fix and nothing more
  (existing fixtures unchanged unless intended, IPC/protocol contract
  unchanged unless intended).
- Run the project's gates: `just test` (Rust incl. golden fixtures, TS
  typecheck + unit, ttsd pytest) and `just lint` (fmt, clippy, ruff). Green,
  with the new test among them.

## Anti-patterns (the ways this goes wrong)

- **Symptom patch without the "why".** Fixing the broken line without asking
  how it got broken — you'll fix its siblings one by one forever.
- **Minimal-diff bias silencing the root-cause question.** A small diff is
  good for risk; it does not buy you out of asking "why was this possible".
  Both thoughts can be true: ship small *and* name the cause.
- **Scope as an excuse.** Treating "minor / out-of-scope" as permission to
  skip the altitude check. Raise the option; let the human pick the size.
- **Fixing on the first guess** without reproducing or isolating.
- **"We'll remember not to forget"** — choosing a comment where an invariant
  was available.

## Worked example (this project)

Every Piper voice produced consistently wrong word stress. Localizing pointed
at espeak-ng phonemization; the hypothesis "data files not loaded" predicted
that `ru_dict`/`phondata` were missing at runtime, and observation confirmed
it: `espeak-rs` only checks `$CWD/espeak-ng-data` and the executable dir, and
the cmake-built data in `target/debug/build/.../out` is never consulted — so
the library initialised with a NULL data path and silently fell back to
skeleton defaults. The symptom patch would have been copying data files next
to the binary. The **root-cause** fix asked *why the path was forgettable*:
the crate gives no error on missing data, so the lock is environmental and
explicit — `PIPER_ESPEAKNG_DATA_DIRECTORY` is set in `nix/devshell.nix`'s
shellHook with a comment explaining the load-bearing invariant, and the
production wrapper sets it too. The bug is fixed **and** the failure mode
(silent NULL data path) is documented at the single place that defines the
environment.

## Project gates

For RuVox: gates are `just test` and `just lint` (inside `nix develop`);
structural locks favour Rust's exhaustive `match`, golden pipeline fixtures,
and single-source dictionaries; the spec cycle (OpenSpec change → verify →
archive) is the lock for behavior changes.
