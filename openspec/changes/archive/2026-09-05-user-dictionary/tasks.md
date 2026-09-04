> Task sizing note: each task is one focused change with its own verification
> — one integration point, one commit. Do not batch tasks; run the stated
> verification before checking the box. Specs:
> `specs/user-dictionary/`, `specs/text-pipeline/`, `specs/ipc-commands/`,
> `specs/ui/`, `specs/preview-dialog/` (delta paths under this change).

## 1. Dictionary storage module (backend, no pipeline changes yet)

- [x] 1.1 Add the `toml` crate (with serde support) to `src-tauri/Cargo.toml`
      and verify `nix develop -c cargo check --manifest-path src-tauri/Cargo.toml`
      passes
- [x] 1.2 Create `src-tauri/src/dictionary/mod.rs`: `DictionaryEntry { from,
      to }` (from stored as typed), `UserDictionary` (map keyed by lowercased
      `from`, sorted iteration), `validate_entry` (from: `^[A-Za-z0-9]+$`
      with ≥1 letter, max 64; to: non-empty, max 256), insert-with-replace
      semantics; unit tests for valid/invalid charset, Cyrillic/digit-only
      rejection, case-collision replace — verify with
      `nix develop -c cargo test --manifest-path src-tauri/Cargo.toml dictionary`
- [x] 1.3 Create `src-tauri/src/dictionary/store.rs`: `load` (missing file →
      empty; parse error → rename to `.bak` + empty + `tracing::warn`;
      keys differing only by case deduped last-wins + warn) and `save`
      (atomic temp+rename, reuse the existing atomic-write helper from
      `storage`), TOML schema `version = 1` + `[entries]` map; unit tests for
      round-trip, dedupe, corruption recovery — same verification command
- [x] 1.4 Add `dictionary_path()` to `src-tauri/src/paths.rs` (config root +
      `user_dictionary.toml`) with a unit test mirroring the existing path
      tests — same verification command

## 2. Pipeline integration (one lookup site per task)

- [x] 2.1 Give `TTSPipeline` a `user_dictionary: UserDictionary` field
      (default empty) and `set_user_dictionary(&mut self, dict)`; verify ALL
      existing tests stay green (proves the empty-dictionary no-op):
      `nix develop -c cargo test --manifest-path src-tauri/Cargo.toml`
- [x] 2.2 Add the pre-pass phase in `src-tauri/src/pipeline/mod.rs`: regex
      `\b[A-Za-z0-9]*[A-Za-z][A-Za-z0-9]*\b`, a `process_user_dictionary`
      step called between code-identifier splitting and
      `process_english_tracked`, skipped when the map is empty, replacing
      only exact lowercased-key hits; extend the golden-fixture harness
      (`src-tauri/tests/`) to accept an optional user dictionary per
      fixture; add fixtures: IT-term override ("docker"), ALL-CAPS override
      ("SQL"), alnum token ("IPv6"), lone-letter override ("x"), and an
      empty-dictionary fixture identical to an existing one's output —
      verify with the cargo test command above
- [x] 2.3 Apply the dictionary to code-identifier parts: consult the map
      before `CODE_WORDS` in `CodeIdentifierNormalizer` (pass
      `&UserDictionary` through the existing call chain from the pipeline);
      fixture: entry "kubectl" inside `kubectl_apply` — same verification
- [x] 2.4 Apply the dictionary in URLs: `URLPathNormalizer::transliterate_word`
      consults the map before `IT_TERMS` (add the ref to `new`); fixture:
      entry "github" changes the reading of `https://github.com/ruvox` —
      same verification
- [x] 2.5 Prove code-block coverage: add a fixture with `code_block_mode`
      full where an entry ("user") applies inside a fenced block; if it
      fails, wire the map into the code-block identifier path exactly as in
      2.3 — same verification
- [x] 2.6 Remove the superseded `EnglishNormalizer::custom_terms` /
      `add_custom_terms` hook and its test (coverage replaced by the
      pre-pass tests in 2.2); verify clippy is clean:
      `nix develop -c cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings`

## 3. Commands and app wiring

- [x] 3.1 Load the dictionary at startup in `src-tauri/src/lib.rs` (next to
      pipeline construction) and inject it; verify the app compiles and all
      tests pass
- [x] 3.2 Create `src-tauri/src/commands/dictionary.rs` with
      `get_user_dictionary` (sorted list + `overrides_builtin` flag computed
      against `IT_TERMS`, the abbreviation `as_word` map, and `CODE_WORDS`),
      `save_user_dictionary` (validate all-or-nothing → atomic save →
      pipeline refresh via `set_user_dictionary` under the existing mutex),
      `import_user_dictionary(path, mode)` (merge: imported wins / invalid
      skipped, or replace; returns `{added, updated, skipped}`; typed error
      on unreadable file, dictionary unchanged), `export_user_dictionary`;
      register all four in `lib.rs::invoke_handler`; tests with temp dirs
      for save-rejection, merge counts, replace — verify with the cargo
      test command
- [x] 3.3 Add typed wrappers + types (`DictionaryEntry`, `ImportReport`) to
      `src/lib/tauri.ts`; verify `nix develop -c pnpm typecheck`

## 4. Frontend editor

- [x] 4.1 Create `src/dialogs/DictionaryModal.tsx` skeleton: loads entries
      on open, Mantine table (from as typed, to, "переопределяет встроенное"
      badge), case-insensitive substring search with alphabetical order,
      CSS Modules + `--mantine-*`/`--ruvox-*` tokens, Russian strings;
      extract the search filter as a pure helper in `src/lib/` with a unit
      test — verify `nix develop -c pnpm test:unit && nix develop -c pnpm typecheck`
- [x] 4.2 Add CRUD: add/edit form (`@mantine/form`, validation messages per
      the user-dictionary spec, duplicate `from` casefolded → open the
      existing entry for edit, soft warning on Latin/digits in `to`), delete
      via `openConfirmModal`; every action saves immediately and drives the
      footer status line ("Все изменения сохранены" / "Сохранение…" /
      "Не сохранено — повторить" with retry); no success toasts; unit tests
      for the form-validation helper — same verification
- [x] 4.3 Add import/export: footer buttons with the standard file dialogs
      (follow the existing `pick_import_file` pattern), a drop zone using
      Tauri drag-drop events, a mode-choice modal ("Объединить" /
      "Заменить список") before import applies, and a result notification
      with added/updated/skipped counts — same verification
- [x] 4.4 Add the "Словарь" section to `src/dialogs/Settings.tsx` (entry
      count + "Открыть…" button opening `DictionaryModal`, nested-modal
      pattern of `CleanupCacheModal`) — verify `nix develop -c pnpm typecheck`
- [x] 4.5 Add quick-add to `src/dialogs/PreviewDialog.tsx`: track text
      selection in both panes, "В словарь" footer action enabled only for a
      single valid token (same regex as backend; Cyrillic/multi-word/punctuated
      stays disabled with a hint), opens the editor prefilled (from =
      selection, to empty); token check as a tested pure helper — same
      verification

## 5. Validation and wrap-up

- [x] 5.1 Run the full test gate: `nix develop -c just test` (Rust incl.
      golden fixtures, TS unit, Python) — all green
- [x] 5.2 Run the full lint gate: `nix develop -c just lint` (fmt, clippy
      -D warnings, cargo deny, eslint, knip, tsc, ruff) — all green
- [x] 5.3 Validate the change: `nix develop -c pnpm dlx @fission-ai/openspec@1.6.0
      validate user-dictionary --strict` passes
- [x] 5.4 Add the `[Unreleased]` entry to `CHANGELOG.md` (user dictionary
      with editor and import/export; English, 1–2 lines per the changelog
      conventions)
- [x] 5.5 Manual pass (run `nix develop -c pnpm tauri dev`): add an entry
      via the editor and see the preview change without restart; add from
      the preview selection; import a file by drop in both modes and read
      the counts notification; hand-edit the TOML file and restart; corrupt
      the file and restart (recovers to empty with a `.bak`); confirm old
      synthesized audio is untouched
