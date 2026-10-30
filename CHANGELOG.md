# Changelog

Notable changes in RuVox, in chronological order.

Format: [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), versions follow [SemVer](https://semver.org/).

## [Unreleased]

### Changed
- **English spelling rules** — the transliteration fallback now understands
  silent `e`, vowel digraphs, soft `c`/`g`, voiced `s`, and the common
  `-tion`/`-sion`/`-ture`/`-cial`/`-tial` endings, so words read closer to
  English without a dictionary entry (`page` → «пейдж», `file` → «файл»,
  `package` → «пэкэдж», `feature` → «фиче»); redundant `IT_TERMS` entries were
  removed and the irregular/guard ones kept; listening-verified readings such
  as `method` → «мэтод» and `function` → «фанкшин» are curated (#306).

### Added
- **Engine-scoped dictionaries** — the dictionary file now holds named
  collections: a global one for every engine and optional per-engine ones.
  Engine entries win on the same source word, the editor has a scope selector,
  and import/export work on the selected collection; existing files migrate
  losslessly into the global collection (#310).
- **TeraTTSv2 engine (experimental)** — a fourth TTS engine runs the 99M
  TeraTTSv2 flow-matching model fully in-process on ONNX Runtime at 44.1 kHz,
  with four Russian voices (`ru_m5` default, `ru_f1`, `ru_f2`, `ru_m1`) and
  dictionary-based accentuation; its ~410 MB model bundle is downloaded on
  demand from Settings. Text-to-speech powered by TeraTTSv2 by TeraSpace
  (Open RAIL-M; see `teratts/NOTICE`).
- **Mixed Russian/English narration on TeraTTSv2** — Latin prose, brands and
  code identifiers stay for the engine, which reads them in English with the
  same selected voice, while numbers, units, URLs, emails and code blocks stay
  with the Russian pipeline; the engine synthesizes language segments into one
  44.1 kHz file with continuous word highlighting (#308, #311).
- **Russian words in the dictionary** — the source word may be a Russian
  Cyrillic token (`замок`), so a wrong stress can be fixed with `зам+ок`; the
  entry matches the exact word form, case-insensitively (#315).
- **Inflected forms in the dictionary** — a Russian entry can list additional
  exact word forms (`замка`, `замке`, …) that share its pronunciation, so one
  entry covers the forms the user confirms without any runtime stemming; the
  editor has a «Формы слова» field, forms persist in the TOML file, and an
  unlisted inflection is still left alone (#316).
- **Dictionary stress marks** — a pronunciation entry may now carry one `+`
  immediately before a Russian vowel (`Москв+а`), and the mark reaches an
  engine that supports stress: today the in-process Silero, where it
  overrides the automatic accentuation. The `ttsd` sidecar and Piper strip
  the mark; the dictionary form shows a help icon and validates the syntax
  (#312).

### Changed
- **Letter-digit tokens are spoken** — `p95`, `v2`, `x86`, `sha256` and
  similar tokens no longer disappear from the audio on Russian-only engines:
  a curated whole-token reading wins (`mp3`), otherwise the letter and digit
  runs are read by their existing normalizers (`p95` → `пи девяносто пять`);
  a native engine keeps the token for its own reading (#313).
- **`IPv6` and `HTTP/2` read correctly** — an alphanumeric abbreviation is now
  read as one Russian token (`ай пи ви шесть`, `эйч ти ти пи слэш два`) on
  every engine instead of dropping the digit or half-transliterating the
  token; a slash inside such a token is no longer mistaken for a file path.
- **Curated reading audit** — acronyms now use the letter name `эл` that the spelling fallback already produces (`HTML` → `эйч ти эм эл`, `XML` → `икс эм эл`, `SQL` → `эс кью эл`, `GRAPHQL` → `граф кью эл`), `JPG` reads `джейпег` like `JPEG`, `PNG` reads `пи эн джи` instead of the unpronounceable `пнг`, `succ` reads `сак`, and code identifiers read `hello`, `image`, and `node` exactly like prose (`хеллоу`, `имидж`, `нод`), with `node` now reading `нод`; a test now fails when two dictionaries disagree on a shared word (#282).
- **`IT_TERMS` keeps pronunciation only** — semantic translations and redundant entries are gone from the curated reading table: `function`, `string`, `array`, `object`, `repository`, `boolean`, and `refactor` now read as English words (`функшн`, `стринг`, `аррэй`, `обджект`, `репоситори`, `булин`, `рефактор`) instead of Russian translations, `method` reads `месод`, and 38 entries that only repeated the transliteration (`commit`, `index`, `docker`, `query`, …) no longer shadow it; the entries that keep an uppercase token from being spelled out (`TEST`, `DATA`, `APP`, `STRING`, …) stay, so every other reading is unchanged (#292).
- **Long all-caps words are read as words, not spelled out** — `DEVELOPER`, `LINUX`, `UBUNTU`, `JAVASCRIPT`, and `SYSTEM` now read as `девелопер`, `линукс`, `убунту`, `джаваскрипт`, `систем` instead of letter by letter, while short or vowel-poor abbreviations (`API`, `HTML`, `SQL`, `JSON`) keep their letter-by-letter readings (#290).
- **`stack`, `live`, `dry`, `run`, and `edge` read correctly in phrases** — `stack trace`, `live reload`, `dry run`, and `edge case` now read as `стэк трейс`, `лайв релоуд`, `драй ран`, `эдж кейс` from their curated single words, so the reading no longer depends on the separator between the words (#287).
- **Closing the window quits by default** — a new Settings switch «Сворачивать в трей при закрытии окна» (off by default) brings back the old behavior of keeping the app running in the tray; the tray icon now appears only while the switch is on, so its menu and clipboard actions are unavailable when closing quits.
- **English normalization handles common inflected forms** — known IT terms now compose a curated base reading with a phonetic suffix (e.g. `support` + `ing`), while raw transliteration remains the fallback for unknown forms.
- **All-caps inflected IT terms are composed, not spelled out** — an uppercase form such as `SUPPORTING` or `FEATURES` now reads by its curated stem (`саппортинг`, `фичас`) instead of letter by letter (#288).
- **Inflected forms that drop an "e" or a "y" read from their base** — `uses`/`using`/`cached`, `caches`, and `driver` now compose the curated `use`/`cache`/`drive` readings (`юзес`, `юзинг`, `кэшед`, `кэшес`, `драйвер`) instead of degrading to raw transliteration, and `species` no longer composes from `spec` (#291).

## [0.6.0] — 2026-09-05

### Added
- **User pronunciation dictionary** — a Settings «Словарь» editor (and a
  «В словарь» quick-add in the preview dialog) lets you teach RuVox any
  Latin word or token like `IPv6`: it is spoken as you write it in prose,
  code identifiers, URLs, and code blocks, overriding the built-in tables.
  The dictionary persists as a hand-editable TOML file in the config folder
  and imports/exports (merge or replace) via dialogs or drag-and-drop (#10).

### Changed
- **Piper narrates long texts without freezing the machine** — synthesis now
  runs in sentence-bounded chunks instead of one inference, so pasting long
  documents no longer risks a system-wide freeze with the Piper engine;
  paragraph breaks in the text now get a distinct silence pause (#155).
- **Piper uses less memory on long texts** — ORT's memory-pattern cache is
  disabled for the Piper session; it retained a pattern per chunk shape and
  added ~200 MB to the memory plateau during narration (#155).
- **Cancelling synthesis is explicit** — a confirmation toast appears and the
  entry gets its own `Отменено` status instead of silently returning to
  `Ожидание` while the spinner toast kept hanging (#155).
- **Code block narration is now a working setting** — the "Озвучка блоков
  кода" selector in Settings («Кратко» / «Читать полностью») drives the
  pipeline live: brief mode (new default) replaces fenced code blocks with a
  short marker sentence, full mode reads the code out; the change applies
  without restarting the app (#89).

### Removed
- **The 100 000-character input limit for the Piper engine** — chunked
  synthesis makes it redundant; long texts are now narrated with Piper just
  like with Silero (#155).
- **The inline `<!-- ruvox-code: … -->` directives** — code block narration
  is controlled solely by the Settings option (#89).
- **The dead `read_operators` config field** — it never affected synthesis;
  old configs carrying it keep parsing (#89).

### Fixed
- **Normalization no longer crashes on `===` / `!==` in the text** — two
  overlapping operator replacements queued for the same spot panicked the
  pipeline ("ошибка нормализации: Внутренняя ошибка"); shorter operators now
  skip occurrences already covered by a longer one.

## [0.5.1] — 2026-08-30

### Added
- **Linux AppImage bundles the `mpv` player** — the AppImage now works on a
  clean system without installing anything; the `.deb` keeps installing the
  system `mpv` automatically via `Depends` (#265).

## [0.5.0] — 2026-08-30 — Localization, import and export

### Added
- **Russian and English interface** — the whole UI is localized; the language
  is picked in Settings (Russian by default), and backend and engine errors
  are shown as readable localized messages instead of raw error strings (#240).
- **Import from files and web pages** — «Добавить» becomes a split button with
  «Файл…», «Файл с кодировкой…» and «По ссылке…», and dragging a .txt/.md/.html
  file or a link onto the window adds a new entry; text encoding is
  auto-detected (BOM, UTF-8, then CP1251/KOI8-R/CP866 and other Cyrillic
  encodings) with a manual override for misdetected files (#224).
- **Auto-detected source format in the preview dialog** — the format selector now defaults to «Авто», which classifies pasted text as HTML, Markdown or plain instead of always assuming Markdown; file and URL imports still preselect the format they know.
- **"Save audio as…" audio export** — the queue context menu exports an entry's audio to any folder via the native save dialog (#225).
- **WAV audio export** — «Сохранить аудио как…» gains a format chooser in the save dialog (WAV default, Ogg Opus alternative), converting an Opus recording to editable 16-bit PCM WAV on export; the cache keeps the Opus original (#252).
- **Voiceover parameters in the history** — the queue context menu gains «Параметры записи…», a read-only view of the source, engine, voice, sample rate, model and settings that produced each recording (#243).
- **Linux AppImage auto-updates** — AppImage installs now check GitHub releases in-app and self-update with signature verification; .deb and source builds get no update UI (#226).
- **Linux packages need `mpv`** — the .deb installs the player binary automatically as a dependency; AppImage users install it once via the system package manager (`sudo apt install mpv`). Bundling `mpv` into the AppImage is planned for 0.5.1 (#265).

### Changed
- **Playback speed up to 3.0× and restored on startup** — the speed limit
  rises from 2.0× to 3.0×, and the saved speech rate is applied to the player
  itself on startup, not just to the slider (#227).
- **Regeneration asks before overwriting** — «Перегенерировать аудио» now opens the normalization preview first: the old audio is deleted only after you confirm, cancelling keeps it, and «Читать сейчас» plays the fresh audio.

### Fixed
- **Silero (native) recordings are stored as Opus again** — the Opus transcode rejected the engine's 16-bit WAV, so its entries stayed as lossy-playback `.wav` fallbacks; existing `.wav` recordings are converted to `.opus` on the next launch (#254).

## [0.4.0] — 2026-08-25 — Linux packages

### Added
- **Linux packages (`.deb`, `.AppImage`)** — release artifacts for Ubuntu 24.04-class systems, built by a GitHub Actions job. The `.deb` installs via `dpkg -i`; the AppImage runs directly on systems with FUSE and via `--appimage-extract-and-run` anywhere else.
- **TTS works out of the box on Linux** — packages bundle `espeak-ng-data` and a pinned `libonnxruntime.so` (sha256-verified at build time), so Silero (native) and Piper synthesize on a clean system without system-wide ONNX Runtime; previously Silero hung silently at startup.

### Fixed
- **Linux user data moved out of XDG cache** — history and audio live in XDG data/config dirs now, so cache cleaners cannot wipe them; corrupted `config.json` writes recover from `.bak` (#222).

## [0.3.1] — 2026-08-20

### Added
- **Update checks are logged** — both the background check and "Проверить обновления" in Settings now write their outcome (up to date / update available / failure reason) to the diagnostic log file (#207).

## [0.3.0] — 2026-08-19 — Windows support

### Added
- **"Silero (native)" engine** — a third TTS engine: Silero v5 runs in-process on ONNX Runtime (the `silero-native/` crate), no Python sidecar. The model bundle (~230 MB) downloads on demand from GitHub Releases with sha256 verification; full parity with the Python version (auto stress marks, homographs, ё). The Python Silero build remains as a fallback.
- **Windows 10 22H2+ / 11 (x86_64) support** — NSIS installer with an embedded WebView2 bootstrapper; bundled mpv, ONNX Runtime and the VC++ CRT are provisioned at build time (pinned resources, sha256-verified).
- **Auto-updates** — `tauri-plugin-updater`: checks from Settings and in the background, signed updates (private key in GitHub Secrets), manifest via `latest.json` from GitHub Releases.
- **Release pipeline** — pushing a `v*` tag runs a GitHub Actions job: builds on `windows-latest`, signs the updater artifacts, publishes a draft release with the installer and `latest.json`.

### Changed
- **Default engine is now "Silero (native)"** (aidar voice, 24000 Hz) instead of Piper. Existing configs are untouched: new defaults apply only to keys missing from `config.json`; if the model bundle is not downloaded, the app still starts on Piper and offers to download the bundle in Settings.

## [0.2.0] — 2026-04-20 — RuVox 2 rewrite

Full migration from PyQt6 + Python single-process to Tauri 2 + Rust + Python subprocess. Goal: keep the 0.1.x feature set, decouple UI ↔ TTS, and move the normalization pipeline to typed Rust with golden tests.

### Added
- **Tauri 2 shell** — native webview, no Python on the main thread.
- **React 18 + Mantine 8 UI** — functional components, CSS Modules, typed IPC.
- **Rust pipeline** — port of all normalizers (Number, English, Abbreviations, Code, URL, Symbol, CodeBlock) from `legacy/src/ruvox/tts_pipeline/` to Rust, verified by golden tests against legacy.
- **ttsd subprocess** — Silero TTS isolated in `ttsd/`, talking to the Rust backend over a stdin/stdout JSON protocol.
- **Storage service** — typed JSON history with separate timestamp files.
- **Preview dialog** (FF 1.1) — preview of the normalized text before synthesis for long inputs.
- **Edit mode** (FF 1.2) — edit text right in the viewer, persisting `edited_text` on the entry and using it on re-synthesis.
- **Word highlighting** — synchronized highlighting of the spoken word in markdown mode via `data-orig-start/end` attributes and binary search over WordTimestamp.
- **Mermaid rendering** — diagrams rendered with mermaid.js in the UI, click-to-zoom via Mantine Modal; replaced with a marker for TTS.
- **HTML format support** — dedicated render mode with sanitization (DOMPurify).
- **Settings dialog** — centralized settings (speaker, rate, hotkeys, cache).
- **Notifications** — `@mantine/notifications` for user-facing messages (TTS errors, edit saving, etc.).
- **Tray menu** — Read Now / Read Later / Open Settings.
- **Nix flake** — `nix build .#ruvox` builds the production binary with bundled `ttsd` and `mpv`.

### Changed
- **UI framework:** PyQt6 → React 18 + Mantine 8.
- **Main process:** Python + PyQt6 QApplication → Rust + Tauri 2 + webview.
- **Pipeline:** Python (moving target, slow tests) → Rust (typed, golden tests, ~10x faster).
- **TTS:** embedded Python in the UI process → `ttsd` subprocess with a JSON protocol (the UI survives a model warmup crash).
- **Player:** `python-mpv` → `tauri-plugin-mpv` (mpv process managed over IPC, scaletempo2 preserved).
- **Hotkeys:** `dasbus` + PyGObject → `@mantine/hooks::useHotkeys` (in-app) + Tauri global shortcuts (system-wide).
- **Storage:** SQLite layer via SQLAlchemy → JSON files with typed schemas in Rust.

### Removed
- `legacy/` — the old PyQt6 implementation temporarily kept as a reference; to be removed once feature parity is confirmed.

### Fixed
- TTS subprocess crash on startup (SIGSEGV in BERT TorchScript on homographs): `torch` is now imported at module level before spawning the worker thread.
- Word-highlighting listener leak on rapid entry switching: added an `editMode` guard in the subscribe effect; listeners are re-subscribed when leaving edit mode (not a blocker; follow-up — migrate to the canonical `Promise<UnlistenFn>[]` pattern as in `Player.tsx`).

### Developer notes
- All commands run inside `nix-shell` or `nix develop`.
- Rust edition 2021, `tracing` for logs, `thiserror` for domain errors, `anyhow` only at boundaries.
- TypeScript `strict: true`, no `React.FC`, CSS Modules (no `sx`/emotion/createStyles).
- Commit format: `<type>(<module>): <desc>`, `type ∈ {feat, fix, chore, refactor, docs, test, build}`. No emoji in code or commits.

## [0.1.x] — Legacy PyQt6

The PyQt6 implementation, original stack: Python 3.11 + PyQt6 + PyQt6-WebEngine + Silero TTS + python-mpv + dasbus. Kept in `legacy/` as a reference; removed once RuVox 2 reaches feature parity.
