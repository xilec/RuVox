# RuVox — Development Guide

> **NOTE:** Always reply in Russian when communicating with the user (assistant
> instruction; the documentation itself is in English).

## Project overview

**RuVox 2.0** is a desktop application for narrating technical Russian-language texts.

**Stack:**
- **Shell:** Tauri 2 (Rust-based desktop shell with native webview)
- **Frontend:** React 18 + TypeScript 5 + Mantine 8
- **Backend:** Rust (text normalization pipeline, storage, TTS subprocess manager, player wrapper)
- **TTS engines:** Silero v5 in-process on ONNX Runtime (`silero-native` crate, default), Piper (native Rust via `piper-rs`, zero-dependency fallback), optional Python subprocess `ttsd` wrapping Silero TTS (fallback)

**Goal** unchanged: normalize technical text (API, URLs, code identifiers, numbers) before passing it to Silero TTS, which cannot read English or special characters.

### Problem → Solution

```
"Вызови getUserData() через API"
        ↓
"Вызови гет юзер дата через эй пи ай"
```

## Documentation

| File / Section | Description |
|----------------|-------------|
| [openspec/](openspec/) | Behavior specs (`specs/`, source of truth) and change proposals — see "Spec-driven workflow" |
| [ai/rules/](ai/rules/) | Hard rules ([conventions](ai/rules/conventions.md)) and craft standard ([code-quality](ai/rules/code-quality.md)) — pulled on demand by skills and the reviewer |
| [docs/install.md](docs/install.md) | Building from source without Nix |
| [docs/development.md](docs/development.md) | Dev environment, commands, debugging |
| [docs/contributing.md](docs/contributing.md) | Contribution rules (dictionaries, style) |
| [docs/index.md](docs/index.md) | Documentation index |

## Quick start

> **All commands must be run via `nix develop -c "..."`** — `cargo`, `pnpm`, `uv` and other tooling are only available inside the dev shell (defined in `nix/devshell.nix`, exposed as `devShells.default` in `flake.nix`).
>
> **Do not run commands from an "already open" `nix develop` session** after editing `nix/devshell.nix`. The `shellHook` (including `XDG_DATA_DIRS` / `GIO_EXTRA_MODULES` / `WEBKIT_DISABLE_DMABUF_RENDERER` — required for WebKit2GTK in Tauri to read GSettings correctly and avoid `devicePixelRatio`=negative, see [tauri #7354](https://github.com/tauri-apps/tauri/issues/7354)) only runs on shell entry. Each `nix develop -c "..."` forks a fresh subshell and always picks up the up-to-date env. Running `pnpm tauri dev` "bare" in the current session breaks fonts and window metrics.

```bash
nix develop -c pnpm install                                                # frontend deps
nix develop -c pnpm tauri dev                                              # run the app
nix develop -c just test                                                   # all tests (Rust + TS + Python)
nix develop -c just lint                                                   # all static checks
```

`justfile` is the single entry point for routine commands (`just --list`); inside
the shell call `just <recipe>` directly. Git hooks (lefthook) run fmt and ruff
pre-commit, clippy and typecheck pre-push — commit and push from inside
`nix develop` so they find the
toolchain.

### rust-analyzer MCP server

The repo ships a project-level MCP config (`.kimi-code/mcp.json`) exposing
rust-analyzer via the `rust-analyzer-mcp` server (tools: definition, references,
hover, diagnostics, code actions, …). Prerequisites on the machine: `cargo
install rust-analyzer-mcp` and `rust-analyzer` in `PATH`. The server starts with
`src-tauri` as its workspace (passed as a CLI arg, resolved relative to the repo
root). Path dependencies of `src-tauri` (e.g. the `silero-native` crate) are
covered by this root automatically; if a genuinely separate cargo workspace
appears in the repo, add a second named server entry to `.kimi-code/mcp.json`
with that workspace's path as its CLI arg (the server accepts exactly one
workspace). Cold indexing of the dependency tree takes ~1 minute; if the first
tool call returns an empty result, indexing is still in progress — wait and
retry the call.

## Project layout

```
/
├── src/              # React + TypeScript frontend (Vite + Mantine 8)
├── src-tauri/        # Rust backend
│   ├── src/
│   │   ├── pipeline/ # Text normalization (port of legacy Python pipeline)
│   │   ├── storage/  # JSON history + audio files
│   │   ├── tts/      # ttsd subprocess manager
│   │   ├── player/   # tauri-plugin-mpv wrapper
│   │   ├── commands/ # Tauri commands (#[tauri::command])
│   │   └── tray/     # System tray
│   └── tests/        # Rust integration tests (golden pipeline fixtures)
├── ttsd/             # Python subprocess (Silero TTS sidecar)
│   ├── pyproject.toml
│   └── ttsd/
│       ├── silero.py      # SileroEngine: load, synthesize
│       ├── timestamps.py  # Word-level timestamp estimation
│       ├── protocol.py    # request/response types
│       └── main.py        # main stdin→stdout JSON loop
├── silero-native/    # Native Silero v5 engine (ONNX Runtime, no Python) — third TTS engine
│   ├── src/          #   engine crate (frontend port + synthesis pipeline)
│   ├── export/       #   maintainer exporter: upstream .pt → ONNX bundle (uv)
│   ├── tests/        #   unit tier + bundle-gated parity suite
│   └── docs/         #   architecture.md (pipeline, debugging), benchmarks.md
├── ai/rules/         # conventions.md + code-quality.md — hard rules & craft standard
├── docs/             # Project documentation (process docs; behavior specs live in openspec/)
├── openspec/         # OpenSpec: specs/ (behavior source of truth), changes/ (proposals), config.yaml
├── scripts/          # Utility scripts
├── nix/
│   └── devshell.nix  # Nix dev environment (Rust + Node + Python), wired into flake.nix
├── justfile          # Task runner (single entry point for routine commands)
├── lefthook.yml      # Git hooks (fmt/ruff pre-commit, clippy/typecheck pre-push)
└── flake.nix         # Production build: `.#ruvox` (slim, Piper + native Silero) and `.#ruvox-with-silero` (full, adds the ttsd Python sidecar)
```

## Spec-driven workflow (OpenSpec)

This repo uses [OpenSpec](https://github.com/Fission-AI/OpenSpec). `openspec/specs/` is the **single source of truth** for current behavior; `openspec/changes/` holds in-flight proposals; `openspec/changes/archive/` is the audit history (it replaces the old ADR log — tooling/architecture rationale lives in `openspec/config.yaml` `context`).

- Any non-trivial behavior change goes through OpenSpec: proposal → delta spec → implement → archive. Trivial fixes (typos, one-liners) may go directly.
- **Primary path is the CLI:** `nix develop -c pnpm dlx @fission-ai/openspec <cmd>` (`list`, `show`, `validate`, `new`, `archive`, …). Slash commands (`/opsx:*` in Claude Code, `/skill:openspec-*` in Kimi Code) are convenience wrappers around the same CLI.
- Before changing behavior, read the relevant spec in `openspec/specs/`. Specs are updated by archiving a change with delta specs — do not edit `openspec/specs/` directly.
- Project context and per-artifact rules for artifact generation live in `openspec/config.yaml`.
- **Archive before PR:** the PR carries the implementation + the archived change + the synced specs together (see Branch & merge workflow below).
- The `openspec-*` skills are generated by the CLI but carry repo-specific notes (the `nix develop -c pnpm dlx` invocation path, links to `ai/rules/`) — re-apply those notes after regenerating skills via an openspec CLI update.

## Branch & merge workflow

General branch/workspace rules live in the global `~/.agents/AGENTS.md` (work in the current workspace by default; one branch per task off fresh `origin/main`; never commit directly to `main`; worktree — `tmp/wt/<task>/` — only when isolation is needed, e.g. parallel agents). The project layer on top:

1. **Full OpenSpec cycle on the branch.** Propose → implement → **archive** the change (archiving syncs the delta specs into `openspec/specs/`). **Archiving is autonomous:** once implementation is done and the manual pass (if any) has confirmed the behavior, sync the delta specs, move the change to `archive/`, and commit the archive without asking for step-by-step confirmation — the draft-approval rules do NOT apply to the archive commit message (they still apply to every other GitHub-bound text).
2. **Implementation → autonomous commit → reviewer → then the user.** When the implementation is ready, the agent works autonomously through the review pass and only reports back once the branch is review-clean:
   1. **Commit autonomously.** The agent commits the implementation (including any OpenSpec change artifacts) without drafting the commit message for approval — messages still follow the `<type>(<module>): <desc>` conventions and carry no AI attribution. `git push` remains separately confirmation-gated.
   2. **Run `ruvox-reviewer`** (read-only, non-blocking) over the branch's diff vs. the merge base on `origin/main` — skip it for docs-only diffs or diffs under ~50 changed lines (the gate stays where the risk is); **and** — *only if `tasks.md` carries a manual-test task* — start the app and hand the user a checklist for the manual pass.
   3. **Fix loop.** Fold accepted findings into the same branch as further autonomous commits (same rules as 2.1), rerun the test/lint gates; note deferrals as issues.
   4. **Report to the user.** Only after steps 2–3 present a summary of what was built and what the review found. Drafting the PR description and opening the PR remain draft-approved (step 4 below).
3. **Merge method: merge commit** (not squash, not rebase).
4. **Who merges.** The agent opens the PR (title/body via draft approval). Once the user has verified and accepted the task's result (manual pass completed, "ok" on the outcome), the rest is autonomous: the agent pushes, opens the PR, and merges it once CI is green without asking further questions. Before that acceptance, `git push` remains separately confirmation-gated.
5. **Autonomy default.** Outside the gates explicitly reserved for the user above (PR/issue/comment drafts, pre-acceptance push), do not ask the user intermediate questions — proceed through the established pipeline and report at its milestones. Ask a question only when a critical blocker appears (failing gate that cannot be resolved from context, destructive operation, ambiguous requirement with real consequences) or when the user has temporarily changed the workflow themselves.
6. **Lightweight paths:**
   - **CI/tooling changes** go through a PR but may skip the OpenSpec change. The pre-PR gate still applies.
   - **Trivial docs / typo fixes** may skip both the OpenSpec change and the PR: push the task branch straight to remote `main` via `git push origin HEAD:main` (fast-forward only). If rejected because `origin/main` moved, rebase the task branch onto fresh `origin/main` and push again.

The only sanctioned ways anything reaches `main` are a PR (points 2–4) or the fast-forward branch push for trivial fixes (point 6). After the merge, clean up: worktree tasks → remove the worktree + local branch and delete the remote branch; workspace tasks → delete the merged branch (local + remote).

## Development rules

Hard rules and the craft standard live in `ai/rules/` and are **pulled on demand** (the OpenSpec skills read them while proposing/implementing; `ruvox-reviewer` enforces them while reviewing) instead of being duplicated here:

- [ai/rules/conventions.md](ai/rules/conventions.md) — language, toolchain, architecture boundaries, the TTS constraint, Rust/TS/Mantine/Python hard rules, testing gates.
- [ai/rules/code-quality.md](ai/rules/code-quality.md) — craft standard: file layout, tests, duplication, idiom, security, correctness.

The load-bearing summary: code and comments in English, user-facing UI strings in Russian; no emoji; commits `<type>(<module>): <desc>` in English with no AI attribution; task-branch implementation commits are made autonomously (review-first workflow, point 2 above), while every other GitHub-bound text (PR, issue, comment) and the PR title itself is drafted and approved first; after the user accepts a task's result, push/PR/merge proceed autonomously (points 4–5); all tooling via `nix develop -c`; a significant user-visible change adds a 1–2-line `[Unreleased]` note to `CHANGELOG.md` in the task branch ([ai/rules/conventions.md](ai/rules/conventions.md#changelog)).

When a CI step, script flag, or workaround exists because of a specific incident, leave a comment explaining why it is load-bearing (see the slim/full gate in `.github/workflows/ci.yml`, the `shellHook` comments in `nix/devshell.nix`).

## Repository language policy

RuVox targets Russian-speaking end users but is developed in English. The split is
by **audience**, not by file type:

- **User-facing entry (Russian, primary):** the repository short description and
  `README.md` (Russian). `README.en.md` is the English mirror.
- **Developer-facing (English, canonical):** code, comments, issues, PRs, commit
  messages, `CHANGELOG.md`, and release notes. Hand-written highlights
  accumulate under `[Unreleased]` in `CHANGELOG.md` as changes land
  ([ai/rules/conventions.md](ai/rules/conventions.md#changelog)); only the
  release-time per-PR skeleton is generated, so PR/commit titles **must stay in
  English** to keep it coherent.

Translation between the two is cheap (LLMs), so there is no obligation to
localize deep docs by hand. If the audience grows, user docs move to a dedicated
site; the repo stays English-canonical for developers.

The authoritative hard rule lives in
[ai/rules/conventions.md](ai/rules/conventions.md#language). When you edit
`README.md`, regenerate `README.en.md` from it — do not hand-maintain both.

## Tests

```bash
nix develop -c just test                                                   # everything
nix develop -c cargo test --manifest-path src-tauri/Cargo.toml             # Rust (incl. golden pipeline fixtures)
nix develop -c cargo test --manifest-path silero-native/Cargo.toml         # native Silero engine (bundle-gated tests skip without SILERO_NATIVE_BUNDLE)
nix develop -c pnpm typecheck                                              # TS typecheck
nix develop -c pnpm test:unit                                              # TS unit tests
nix develop -c bash -c "cd ttsd && uv run python -m pytest"                # Python subprocess tests
```
