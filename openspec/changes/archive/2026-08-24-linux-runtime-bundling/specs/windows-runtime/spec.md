# windows-runtime Delta

## MODIFIED Requirements

### Requirement: Windows startup environment

On Windows, before any worker threads or TTS engines are initialized, the
application SHALL point `PIPER_ESPEAKNG_DATA_DIRECTORY` at the bundled
`espeak-ng-data/` resource directory so that Piper phonemization loads full
Russian dictionaries. On Linux the equivalent behavior is specified by the
`linux-runtime` capability: the startup sequence MAY set
`PIPER_ESPEAKNG_DATA_DIRECTORY` (and `ORT_DYLIB_PATH`) when bundled data ships
with the package; nix builds keep relying on the wrapper-provided values.

#### Scenario: Windows startup with bundled data

- GIVEN the app runs on Windows and the `espeak-ng-data/` resource exists
- WHEN the application starts
- THEN `PIPER_ESPEAKNG_DATA_DIRECTORY` points at the bundled directory
  before any Piper synthesis is attempted

#### Scenario: Nix build keeps wrapper-provided data directory

- GIVEN the app runs on Linux as a nix build with the wrapper setting
  `PIPER_ESPEAKNG_DATA_DIRECTORY`
- WHEN the application starts
- THEN the wrapper-provided value is preserved and startup does not override it

#### Scenario: Non-Windows startup

- GIVEN the app runs on Linux from a dev checkout without bundled
  `espeak-ng-data` resources
- WHEN the application starts
- THEN the startup code does not set `PIPER_ESPEAKNG_DATA_DIRECTORY`
