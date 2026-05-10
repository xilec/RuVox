# Contributing

Guide to contributing to RuVox.

## Ways to help

1. **Extending dictionaries** — adding IT terms, abbreviations, operators.
2. **Fixing pronunciation** — correcting existing transliterations.
3. **Bug reports** — concrete failures with a minimal reproduction.
4. **New features** — discuss via an issue first, then send a PR.
5. **Documentation** — improving descriptions, keeping them up to date after changes.

## Extending dictionaries

The normalization pipeline lives in `src-tauri/src/pipeline/normalizers/`. Each normalizer is a separate Rust file with its own substitution table. A golden test is **required** when extending a dictionary.

### Adding an IT term

**1. File:** `src-tauri/src/pipeline/normalizers/english.rs`

Find the `IT_TERMS` table and add an entry (keep alphabetical order within each section):

```rust
pub static IT_TERMS: phf::Map<&'static str, &'static str> = phf::phf_map! {
    // ... existing terms ...
    "kubernetes" => "кубернетис",
    "terraform"  => "терраформ",
};
```

**2. Golden fixture:** `src-tauri/tests/fixtures/pipeline/`

```bash
# Create input and reference output
echo "Используем Kubernetes и Terraform" > src-tauri/tests/fixtures/pipeline/it_kubernetes.input.txt
echo "Используем кубернетис и терраформ" > src-tauri/tests/fixtures/pipeline/it_kubernetes.expected.txt
```

> The easiest way to produce the `char_map.json` for a new case is to write it by hand or copy the structure from a similar fixture in this directory.

**3. Run:**

```bash
nix develop -c cargo test --manifest-path src-tauri/Cargo.toml --test golden -- it_kubernetes
```

**4. Commit:**

```bash
git checkout -b feat/it-term-kubernetes
git add src-tauri/src/pipeline/normalizers/english.rs src-tauri/tests/fixtures/pipeline/it_kubernetes.*
git commit -m "feat(pipeline): add 'kubernetes' to IT_TERMS dictionary"
```

### Adding an abbreviation

File: `src-tauri/src/pipeline/normalizers/abbreviations.rs`.

- **As a word** — the `AS_WORD` table (`"json" → "джейсон"`).
- **Letter by letter** — no need to add: anything not in `AS_WORD` is read letter by letter via `LETTER_MAP` by default.

### Adding an operator / symbol

File: `src-tauri/src/pipeline/normalizers/symbols.rs` or `src-tauri/src/pipeline/constants.rs` (for GREEK_LETTERS, MATH_SYMBOLS, ARROW_SYMBOLS).

Multi-character operators (`===`, `=>`, `>=`) are handled in `pipeline/mod.rs::TRACKED_OPERATOR_KEYS` — the "longest before shortest" order is mandatory.

## Bug reports

### What to include

1. **Input text** — what was being processed (a minimal example).
2. **Expected result** — how it should sound.
3. **Actual result** — what came out (`normalized_text` from `~/.cache/ruvox/history.json`).
4. **Version** — `git log -1 --oneline`.
5. **Environment** — OS, Nix (`nix develop`) or manual install.

### Example

```markdown
**Input:**
`Версия >= 2.0`

**Expected:**
"Версия больше или равно два точка ноль"

**Actual:**
"Версия >= два точка ноль"

**Version:** abc1234
**Environment:** NixOS 24.11, nix develop
```

## Code style

See full rules in [development.md](development.md#code-rules). In short:

- Rust: edition 2021, `tracing` + `thiserror`, no `unwrap` in production paths, `cargo fmt` + `cargo clippy`.
- TypeScript: `strict: true`, no `React.FC`, no `any`, functional components + hooks.
- Mantine 8: CSS Modules + `classNames` prop. No `sx`, `createStyles`, `emotion`, or Mantine 6/7 legacy.
- Python (ttsd): 3.12, uv-managed, clean `ruff check`, JSON over stdin/stdout, logs to stderr.

### Commits

```
feat(pipeline): add 'kubernetes' term to IT_TERMS
fix(player): mask mpv seek latency to prevent slider snap-back
docs(ipc-contract): add add_text_entry command
test(pipeline): golden fixture for size_units
refactor(commands): extract spawn_synthesis helper
chore(deps): bump tauri to 2.10
```

- No emoji.
- **Forbidden:** "Co-Authored-By: Claude …".
- `git push` to `main` — only with explicit approval (see CLAUDE.md).

## Pull request

### Checklist

- [ ] `nix develop -c cargo test --manifest-path src-tauri/Cargo.toml` is green.
- [ ] `nix develop -c pnpm typecheck` is green.
- [ ] `nix develop -c pnpm lint` is green.
- [ ] If the pipeline was touched — a golden fixture has been added/updated.
- [ ] If ttsd was touched — `cd ttsd && uv run python -m pytest` is green.
- [ ] Documentation (`docs/`) is updated if behavior changed.
- [ ] Commit messages follow the `<type>(<module>): <desc>` format.

### PR description

```markdown
## Summary

Add the term "kubernetes" to the English-normalization `IT_TERMS` dictionary.

## Why

It shows up often in technical docs and was previously transliterated as
«кьюбернетес» via fallback, which is unacceptable.

## Test plan

- Added a golden case `it_kubernetes`.
- All existing golden tests pass.
- Smoke in `pnpm tauri dev`: paste a string with Kubernetes into the clipboard,
  synthesis is correct.
```

## Questions

If something is unclear — open an issue with the `question` tag.
