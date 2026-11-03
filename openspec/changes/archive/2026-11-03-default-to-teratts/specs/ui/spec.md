## RENAMED Requirements

- FROM: `### Requirement: First-run Silero Native bundle prompt`
- TO: `### Requirement: First-run model bundle prompt`

## MODIFIED Requirements

### Requirement: First-run model bundle prompt

On startup, when the persisted `UIConfig.engine` is an engine that needs a
downloadable model bundle — `"teratts"` (the default) or `"silero_native"` —
and the availability probe reports that engine's bundle as unavailable, the
app SHALL show a modal prompt offering the one-time download. `"piper"`,
`"silero"` (which needs the Python stack, not a bundle) and any unknown engine
value SHALL never prompt.

The prompt SHALL be parameterized by the persisted engine: the engine name,
the bundle size, the download command (`download_teratts_bundle` or
`download_silero_native_bundle`) and the activation call (`update_config` with
the prompted engine) all follow from it; the progress, error and decline
mechanics are shared between the variants.

The TeraTTSv2 prompt SHALL be titled «Движок по умолчанию — TeraTTSv2» and
SHALL state why it is the default (it reads Russian prose, English terms and
code naturally) and what it costs (a larger one-time ~410 MB download, a
slower first synthesis and higher memory use — ~3.9 GiB peak). Its primary
action SHALL be «Скачать (~410 МБ)», its secondary action «Остаться на Piper»,
and it SHALL mention Silero Native (~230 MB) as the faster, lighter
alternative available in Settings without offering a second download button in
the prompt. The TeraTTSv2 attribution SHALL remain visible in README/NOTICE;
the prompt copy does not replace it.

The Silero Native prompt SHALL keep offering the ~230 MB Silero bundle and
activating Silero Native on success.

For both variants:

- **Download** — starts the engine's `download_*_bundle` command and shows
  inline progress driven by the `bundle_download_*` events for that engine.
  On success the app SHALL activate the prompted engine for the running
  session (via `update_config` with that engine) and confirm with a green
  notification. On failure the prompt SHALL show the error and allow a retry.
- **Stay on Piper** — closes the prompt for the current run without
  persisting anything.

The prompt SHALL NOT appear when the persisted engine has no bundle to
download, nor once the prompted engine's bundle is on disk. It reappears on
the next launch while the condition holds (no "don't ask again" persistence in
this revision).

#### Scenario: fresh install is offered the bundle

- GIVEN a fresh install (default config `engine = "teratts"`) and no
  downloaded TeraTTS bundle
- WHEN the app starts and the config loads
- THEN the bundle prompt opens with the title «Движок по умолчанию —
  TeraTTSv2», names the ~410 MB bundle, its reason and its cost, and offers
  «Скачать (~410 МБ)» and «Остаться на Piper»

#### Scenario: existing Silero Native config is offered its own bundle

- GIVEN the persisted config has `engine = "silero_native"` and the Silero
  Native bundle is missing
- WHEN the app starts
- THEN the bundle prompt opens naming Silero Native and its ~230 MB bundle,
  and the download calls `download_silero_native_bundle`

#### Scenario: explicit Piper choice never prompts

- GIVEN the persisted config has `engine = "piper"`
- WHEN the app starts
- THEN the bundle prompt does not open, regardless of bundle availability

#### Scenario: downloaded bundle never prompts

- GIVEN the persisted config has `engine = "teratts"` or `"silero_native"` and
  that engine's bundle probe reports available
- WHEN the app starts
- THEN the bundle prompt does not open

#### Scenario: download activates the native engine

- GIVEN the bundle prompt is open for `teratts` (or `silero_native`)
- WHEN the user accepts and the download finishes successfully
- THEN `update_config` is called with that engine, the active engine switches
  to it without a restart, and a green confirmation is shown

#### Scenario: failed download keeps the retry path

- GIVEN the bundle prompt is open
- WHEN the download finishes with `ok: false`
- THEN the prompt shows the error inline and the download action can be
  retried

#### Scenario: decline is per-run

- GIVEN the bundle prompt is open
- WHEN the user chooses to stay on Piper
- THEN the prompt closes, the session continues on the Piper fallback, the
  persisted config is unchanged, and the prompt appears again on the next
  launch

#### Scenario: another engine's download event is ignored

- GIVEN the bundle prompt is open for `teratts`
- WHEN a `bundle_download_finished` event arrives with
  `engine: "silero_native"`
- THEN the prompt stays open and `update_config` is not called
