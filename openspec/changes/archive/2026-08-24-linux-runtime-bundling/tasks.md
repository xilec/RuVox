# Tasks: linux-runtime-bundling

## 1. Implementation

- [x] 1.1 `src-tauri/src/lib.rs`: `init_platform_env` — locate bundled `espeak-ng-data` and `libonnxruntime`, set `PIPER_ESPEAKNG_DATA_DIRECTORY` / `ORT_DYLIB_PATH` when found, never overriding pre-existing values
- [x] 1.2 Lookup helpers with own-product-dir priority over sibling wildcard and zero-byte placeholder rejection; unit tests for both (priority, placeholder, dev-build no-op)
- [x] 1.3 `tauri.linux.conf.json`: bundle both resources under the Linux resources map
- [x] 1.4 `scripts/fetch-linux-onnxruntime.sh`: pinned version + sha256 (+ `--check`)
- [x] 1.5 `.github/workflows/release.yml`: fetch step in linux-packages job
- [x] 1.6 `scripts/build-linux-packages.sh` + `scripts/docker/Dockerfile`: local Docker builder mirroring CI

## 2. Gates

- [x] 2.1 `cargo test --manifest-path src-tauri/Cargo.toml` green (1012 lib tests), fmt/clippy clean
- [x] 2.2 Manual pass in Ubuntu 24.04 VM on pure Wayland: install `.deb` → Silero synthesis + playback from bundled dylib (`/proc/<pid>/maps`); same for `.AppImage` (`--appimage-extract-and-run`, dylib resolved from extracted `usr/lib/RuVox/`)

## 3. Wrap-up

- [x] 3.1 Pre-PR reviewer pass over branch diff; findings fixed (product-dir priority tests, release.yml wayland-fix step, appimagetool pin, per-shell pkg-config shim) or deferred (env-priority unit test — documented behavior)
- [ ] 3.2 Validate specs, archive change (sync delta)
- [ ] 3.3 PR, merge
