# linux-runtime Delta

## ADDED Requirements

### Requirement: Linux startup environment for bundled TTS resources

On Linux, before any worker threads or TTS engines are initialized, the
startup sequence SHALL locate bundled runtime resources and, when found, set
the corresponding process environment variables:

- `espeak-ng-data/` (Piper phonemization) → `PIPER_ESPEAKNG_DATA_DIRECTORY`
- `libonnxruntime.so` (Silero Native ONNX Runtime dylib) → `ORT_DYLIB_PATH`

The lookup SHALL examine the executable's own directory, its parent directory,
and the sibling product directories under the parent's `lib/` directory. The
application's own product directory (`RuVox`) SHALL be tried before any
wildcard match over siblings, so another product's files can never shadow the
bundled resources. A candidate that exists but is a zero-byte placeholder
SHALL be rejected.

Pre-existing values of these variables in the process environment SHALL NOT
be overridden (a nix wrapper or the user keeps priority). If no bundled
resources are found — e.g. a dev build run via `cargo tauri dev` — startup
MUST proceed unchanged with the variables left unset.

#### Scenario: Deb install synthesizes from bundled resources

- GIVEN the app was installed from a Linux `.deb` package on a clean system
  without a system-wide `libonnxruntime`
- WHEN the application starts and Silero Native synthesis is requested
- THEN `PIPER_ESPEAKNG_DATA_DIRECTORY` points at `/usr/lib/RuVox/espeak-ng-data`,
  `ORT_DYLIB_PATH` points at `/usr/lib/RuVox/libonnxruntime.so`, and synthesis
  completes without falling back to another engine

#### Scenario: Own product dir wins over sibling products

- GIVEN `lib/RuVox/espeak-ng-data` and `lib/<Other>/espeak-ng-data` both exist
  next to the executable
- WHEN the startup lookup resolves the bundled data directory
- THEN the `RuVox` copy is selected regardless of alphabetical order

#### Scenario: Nix build keeps wrapper-provided values

- GIVEN the app runs as a nix build with `PIPER_ESPEAKNG_DATA_DIRECTORY` and
  `ORT_DYLIB_PATH` already set by the wrapper
- WHEN the application starts
- THEN both values are preserved exactly as provided

#### Scenario: Dev build without bundles starts normally

- GIVEN the app runs from a dev checkout without any bundled resource
  directories
- WHEN the application starts
- THEN neither variable is set and engine selection behaves as before this
  capability existed

### Requirement: Linux packages bundle pinned runtime resources

Linux release packages (`.deb`, `.AppImage`) SHALL include as resources:
`espeak-ng-data/` and `libonnxruntime.so` fetched at a pinned upstream version
whose API matches the `ort-sys` crate ABI, verified by checksum. The release
workflow and the local Docker builder SHALL fetch it the same way.

#### Scenario: Package contents

- GIVEN a Linux package built by the release workflow or the local builder
- WHEN its payload is inspected
- THEN `usr/lib/RuVox/espeak-ng-data/` and `usr/lib/RuVox/libonnxruntime.so`
  are present and non-empty
