# Design: settings-app-version

## 1. Settings version display (frontend)

`Settings.tsx` already loads `commands.getConfig()` and `commands.getCacheDir()`
in the `useEffect` that runs on open (the `opened` dependency). Extend that same
effect to also call `getVersion()` from `@tauri-apps/api/app` and store the
result in a new `appVersion` state. Render it as a right-aligned
`Версия: <version>` line in the dialog footer (bottom-right), above the
"Сбросить"/"Сохранить" button row, so it is always visible on every platform.

```tsx
import { getVersion } from '@tauri-apps/api/app';

// in the open effect, mirroring getCacheDir:
getVersion().then(setAppVersion).catch(() => setAppVersion(''));
```

```tsx
<Text size="xs" c="dimmed" ta="right" mt="md">Версия: {appVersion || '—'}</Text>
```

The `—` fallback matches the existing `getCacheDir` failure handling (state set
to `''`, rendered as `—`), so a version read error never blocks the dialog.

### Why not a new Rust command

Tauri v2 already exposes `getVersion()` from `@tauri-apps/api/app`, which
resolves the `version` field of `tauri.conf.json` baked in at build time. Adding
a `get_app_version` command would be a second source of truth for the same
value with no benefit.

### Why not gate it behind the updater section

The updater section is `UPDATER_ENABLED`-gated and only meaningful on Windows
(`navigator.userAgent` includes `Windows`). Version visibility must work on
Linux/nix too, because that is where most bug reports originate in this repo.

## 2. Tray tooltip version (backend)

`src-tauri/src/tray/mod.rs::init` builds the tray with `.tooltip("RuVox")`.
Replace the literal with `format!("RuVox v{}", app.package_info().version)` —
`package_info().version` is the same `tauri.conf.json` version, available in
`tray::Builder` init. One line, no new command, the tooltip now reads
`RuVox v0.3.1`.

## Alternatives considered

- **Dedicated About dialog:** rejected — `#94` allows "About dialog or tray
  tooltip", and reusing Settings keeps the version next to the updater check
  without introducing a new route/modal.
- **Read version from a Rust command / env:** rejected — `getVersion()` and
  `package_info().version` already cover frontend and backend with zero extra
  surface.
- **Show version only on Windows (next to updater):** rejected — bug reports
  against Linux/nix builds would still have no version; display must be
  platform-independent.
