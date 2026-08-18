# VM verification checklist — windows-installer-and-release

Manual pass before publishing the first draft release. Run on **Windows 10
22H2** (lower bound, most common in the RU segment) and, if resources allow,
**Windows 11 Enterprise Evaluation**.

## VM setup (quickemu on NixOS)

```bash
nix shell nixpkgs#quickemu
quickget windows 10    # or: quickget windows 11
quickemu --vm windows-10-22H2.conf   # the .conf quickget generated
```

- quickget downloads the ISO **into the current directory** (not the nix
  store), generates an unattended-setup `autounattend.xml` and a QEMU/KVM
  config. The ISO survives `nix-collect-garbage` — it is just a file in the
  cwd. Free space needed: ~6 GB ISO + ~30 GB disk image per VM.
- Do this inside `tmp/` (e.g. `tmp/win-vm/`) so nothing lands outside the
  session tree.
- WebView2 note: the installer embeds the bootstrapper
  (`webviewInstallMode: embedBootstrapper`), so it works on a machine
  without WebView2 **if online**. Evaluation VMs have no WebView2
  preinstalled — that is exactly the case we want to exercise.

## Checklist

1. **Install**
   - [ ] `RuVox_<ver>_x64-setup.exe` runs; SmartScreen warning appears
     (expected, unsigned — «Подробнее → Выполнить в любом случае»).
   - [ ] WebView2 bootstrapper kicks in on a machine without the runtime.
   - [ ] Install finishes, app launches.
2. **Smoke**
   - [ ] First launch: main window renders, tray icon present.
   - [ ] Piper: pick voice ruslan, «Скачать сейчас», synthesize a short
     Russian+English mixed text, playback works (bundled mpv, no system mpv
     installed).
   - [ ] Silero Native: «Скачать модели Silero» from Settings, synthesize.
   - [ ] Close window → app stays in tray; tray «Выход» quits cleanly.
   - [ ] Check `mpv.exe`, `espeak-ng-data/`, `onnxruntime.dll` exist next to
     `RuVox.exe` in the install dir.
3. **Update check**
   - [ ] Settings → «Проверить обновления» → «Обновлений нет» (no newer
     release published).
   - [ ] (Optional, after a second release exists) startup check offers the
     update; «Обновить и перезапустить» installs and relaunches.
4. **Uninstall**
   - [ ] «Установка и удаление программ» → uninstall works, no errors.
5. **Release pipeline**
   - [ ] Tag push produced a **draft** release with: `*-setup.exe`,
     `*-setup.nsis.zip`, `*.sig`, `latest.json`.
   - [ ] Publish the draft only after this checklist passes.
