# Delta: ui

## ADDED Requirements

### Requirement: First-run Silero Native bundle prompt

On startup, when the persisted `UIConfig.engine` is `"silero_native"` and
the availability probe reports the Silero Native bundle as unavailable, the
app SHALL show a modal prompt offering a one-time download of the model
bundle (~230 MB) — the default engine cannot run without it, and the session
is otherwise silently served by the Piper fallback.

The prompt SHALL offer two actions:

- **Download** — starts `download_silero_native_bundle` and shows inline
  progress driven by the `bundle_download_*` events. On success the app
  SHALL activate the native engine for the running session (via
  `update_config` with `engine: "silero_native"`) and confirm with a green
  notification. On failure the prompt SHALL show the error and allow a
  retry.
- **Stay on Piper** — closes the prompt for the current run without
  persisting anything.

The prompt SHALL NOT appear when the user has explicitly chosen another
engine, nor once the bundle is on disk. It reappears on the next launch
while the condition holds (no "don't ask again" persistence in this
revision).

#### Scenario: fresh install is offered the bundle

- GIVEN a fresh install (default config `engine = "silero_native"`) and no
  downloaded bundle
- WHEN the app starts and the config loads
- THEN the bundle prompt opens, naming the engine and the download size

#### Scenario: download activates the native engine

- GIVEN the bundle prompt is open
- WHEN the user accepts and the download finishes successfully
- THEN the active engine switches to Silero Native without a restart, and a
  green confirmation is shown

#### Scenario: explicit Piper choice never prompts

- GIVEN the persisted config has `engine = "piper"`
- WHEN the app starts
- THEN the bundle prompt does not open, regardless of bundle availability

#### Scenario: downloaded bundle never prompts

- GIVEN the persisted config has `engine = "silero_native"` and the bundle
  probe reports available
- WHEN the app starts
- THEN the bundle prompt does not open

#### Scenario: decline is per-run

- GIVEN the bundle prompt is open
- WHEN the user chooses to stay on Piper
- THEN the prompt closes, the session continues on the Piper fallback, the
  persisted config is unchanged, and the prompt appears again on the next
  launch
