# RuVox — conventions & invariants

Single source of truth for the project's hard rules and cross-cutting invariants.
**Pulled on demand, not pinned in always-on context:** the OpenSpec skills
(`openspec-propose`, `openspec-apply-change`) read this while generating /
implementing; `ruvox-reviewer` reads it while reviewing; `AGENTS.md` and
`openspec/config.yaml` point here. Don't restate these rules elsewhere — link
to this file.

Craft rules (layout, test quality, duplication, idiom, correctness) live in
[code-quality.md](./code-quality.md).

## Language

- Code (identifiers, comments) is in English. User-facing strings (UI,
  notifications, logs visible to the user) are in Russian.
- Dev-facing repo docs (`docs/`, `AGENTS.md`, GitHub issues/PRs, code comments)
  are in English. The user-facing `README.md` is Russian (primary); `README.en.md`
  is its English mirror and must be updated in the same PR as `README.md`.
- No emoji in code or commit messages.
- Commit format: `<type>(<module>): <short desc>`, type ∈ {feat, fix, chore,
  refactor, docs, test, build}. Subject and body in English. No AI-assistant
  attribution ("Co-Authored-By", "Generated with", etc.) — ever, in any GitHub
  text.
- Comments only when WHY is non-obvious (hidden invariant, workaround for a
  known bug). Do not comment WHAT.

## Toolchain

- All commands run via `nix develop -c "..."` (or `just <recipe>` inside the
  shell) — cargo/pnpm/uv/just/lefthook exist only inside the dev shell
  (`nix/devshell.nix`). Never run them "bare" from a stale shell after editing
  `nix/devshell.nix`: the `shellHook` env (XDG_DATA_DIRS / GIO_EXTRA_MODULES /
  WEBKIT_DISABLE_DMABUF_RENDERER) is required for WebKit2GTK in Tauri to read
  GSettings correctly (see tauri #7354).
- Package manager: pnpm (rationale in `openspec/config.yaml` context). Python
  tooling: `uv` only — no `pip`, no `python -m venv`.
- Pre-commit hooks run via lefthook (`lefthook.yml`): fmt and ruff on commit;
  clippy, typecheck, eslint and knip on push. Commit and push from inside
  `nix develop` so the hooks can find the toolchain.

## Architecture boundaries

- **Frontend (`src/`)** talks to the backend **only** through Tauri commands
  (`src-tauri/src/commands/`), via the `invoke` wrappers in `src/lib/tauri.ts`.
- **Text normalization pipeline** lives in `src-tauri/src/pipeline/` — pure
  Rust, no Tauri dependencies. Correctness is verified by golden fixtures in
  `src-tauri/tests/fixtures/pipeline/`.
- **ttsd** (Python sidecar) speaks strictly the stdin/stdout JSON protocol
  (`ttsd/ttsd/protocol.py`, spec `openspec/specs/ttsd-protocol/`): JSON requests
  on stdin, JSON responses on stdout, logs on **stderr only**.
- **TTS engines:** Silero v5 in-process via the `silero-native` crate is the
  default engine; Piper is the zero-dependency fallback; Silero via ttsd is
  the opt-in fallback (flake flag `withSilero`). Slim `.#ruvox` must not pull
  in ttsd — gated in CI.

## The TTS constraint (load-bearing)

- Silero TTS **cannot read English or special characters.** Every English
  fragment (URLs, code identifiers, headings) must be transliterated to
  Cyrillic before synthesis. Wherever English may appear, a transliteration
  fallback is mandatory; text processing must never leave English words behind
  without them remaining available to the English normalizer. (Behavior details:
  `openspec/specs/text-pipeline/`.)
- Mermaid blocks are never narrated — the pipeline replaces them with a marker
  sentence (spec `text-pipeline`).

## Rust

- Edition 2024 (MSRV 1.85).
- `tracing` for logging, `thiserror` for domain errors, `anyhow::Result` only
  at boundaries.
- No `unwrap` in production paths — use `?` + typed errors. `expect()` only for
  unreachable states, with a message explaining why.
- `cargo fmt` and `cargo clippy -- -D warnings` must be clean.

## TypeScript / React

- `strict: true` in tsconfig. Avoid `any` — use `unknown` + narrowing.
- Function components only. No `React.FC`. Hooks-first; no class components.
- Prettier for formatting.

## Mantine 8

- Styling via CSS Modules and the `classNames` prop. **Forbidden:** `sx`,
  `createStyles`, emotion, any Mantine 6/7 legacy.
- Forms: `@mantine/form`. Notifications: `@mantine/notifications`. Hooks:
  `@mantine/hooks`. Modals: `@mantine/modals` (no router).
- Use `--mantine-*` / `--ruvox-*` tokens; no hardcoded hex/px in CSS Modules
  where a token exists (spec `openspec/specs/ui/`). A new reusable token or UI
  pattern → update the `ui` spec via an OpenSpec change in the same PR.

## State

- No Redux, no React Query. Zustand or React context for global state; props +
  `useState` by default. Tauri `invoke` fits into `useEffect` + `useState`.

## Python (ttsd)

- Python 3.12, managed by `uv`.
- Logs to stderr; JSON requests on stdin, JSON responses on stdout.
- `ruff check` and `pytest` must be green.

## CI

- Every workflow job declares `timeout-minutes`, sized at ~2-3x the worst
  observed duration of that job. A hung job must fail fast with logs instead
  of idling until the 6h GitHub default (2026-08-19 apt-mirror throttling
  incident, #204).

## Testing gates

- `just lint` runs all static checks: `cargo fmt --check`, `clippy -D warnings`,
  `cargo deny check` (RustSec advisories, license whitelist — `src-tauri/deny.toml`),
  `eslint` (typescript-eslint recommended-type-checked + react-hooks), `knip`
  (dead code/unused deps), `tsc --noEmit`, `ruff`.
- `cargo test --manifest-path src-tauri/Cargo.toml` (incl. pipeline golden
  fixtures — a pipeline bug fix adds a fixture reproducing it).
- `pnpm typecheck` and `pnpm test:unit`.
- `cd ttsd && uv run ruff check && uv run python -m pytest`.
- `pnpm dlx @fission-ai/openspec@1.6.0 validate --specs --strict` when specs change.

> Implemented behavior is defined by the specs under `openspec/specs/` — the
> source of truth; on conflict with other docs, specs win. Tooling/architecture
> rationale lives in `openspec/config.yaml` context; decision history in
> `openspec/changes/archive/`.
