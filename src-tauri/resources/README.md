# src-tauri/resources — placeholder tree

`tauri-build` verifies that every `bundle.resources` path exists at
**compile** time (even for `cargo check` / `cargo test`), while the real
Windows binaries are downloaded only at release-build time. These
placeholders keep local Linux builds and Linux CI green.

At release time they are replaced with real content:

- `mpv/` — shinchiro mpv Windows build, extracted by
  `scripts/fetch-windows-resources.sh`
- `onnxruntime.dll` — from the pinned microsoft/onnxruntime release
  (same script)
- `libonnxruntime.so` — Linux twin of the above, from the pinned
  microsoft/onnxruntime release (`scripts/fetch-linux-onnxruntime.sh`,
  release.yml linux-packages job); bundled into the .deb/.AppImage for the
  silero-native engine
- `espeak-ng-data/` — copied out of the espeak-rs-sys build tree by the
  release workflow (`.github/workflows/release.yml`)

Never bundle a build where these are still placeholders — the release
workflow always runs the fetch/extract steps before `pnpm tauri build`.
