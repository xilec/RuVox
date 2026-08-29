# Building RuVox from source on Linux without Nix

Documented distributions: **Ubuntu 24.04 LTS (Noble)** /
**Debian 13 (Trixie)**, **Fedora 40+** / **RHEL 10+**, **Arch Linux**.
The hard requirement is `webkit2gtk-4.1` (Tauri 2 dropped 4.0); on
older Ubuntu releases (22.04 and earlier) the package is missing and
this guide will not work — use the Nix flake instead.

If you have Nix or NixOS, prefer the flake build documented in
`README.md` — it is fully reproducible.

## 1. System packages

Pick the block matching your distribution.

### Ubuntu 24.04+ / Debian 13 (Trixie)+

```bash
sudo apt update
sudo apt install -y \
  build-essential pkg-config cmake clang libclang-dev \
  libwebkit2gtk-4.1-dev libsoup-3.0-dev libgtk-3-dev librsvg2-dev \
  libayatana-appindicator3-dev libmpv-dev mpv libopus-dev libssl-dev \
  libasound2-dev libpulse-dev libpipewire-0.3-dev \
  libfontconfig1-dev libfreetype6-dev libgl-dev libdrm-dev \
  libwayland-dev libxkbcommon-dev wayland-protocols \
  espeak-ng espeak-ng-data \
  curl file git
```

> `libmpv-dev` provides the headers to compile against; `mpv` is the
> player binary the app drives at run time — both are required.

### Fedora 40+ / RHEL 10+

```bash
sudo dnf install -y \
  gcc gcc-c++ pkgconf-pkg-config cmake clang clang-devel git \
  webkit2gtk4.1-devel libsoup3-devel gtk3-devel librsvg2-devel \
  libayatana-appindicator-gtk3-devel mpv-libs-devel mpv opus-devel openssl-devel \
  alsa-lib-devel pulseaudio-libs-devel pipewire-devel \
  fontconfig-devel freetype-devel mesa-libGL-devel libdrm-devel \
  wayland-devel libxkbcommon-devel wayland-protocols-devel \
  espeak-ng espeak-ng-data \
  curl file
```

> The `mpv` player binary lives in RPM Fusion on Fedora/RHEL — enable
> the repo if `dnf` cannot find it.

### Arch Linux

```bash
sudo pacman -S --needed \
  base-devel cmake clang git \
  webkit2gtk-4.1 libsoup3 gtk3 librsvg libayatana-appindicator \
  mpv opus openssl \
  alsa-lib libpulse pipewire \
  fontconfig freetype2 libglvnd libdrm \
  wayland libxkbcommon wayland-protocols \
  espeak-ng \
  curl file
```

> **openSUSE / other distros:** not yet documented. Package names
> should map closely to the lists above. PRs welcome.

## 2. ONNX Runtime (Piper engine)

`piper-rs` dlopens `libonnxruntime.so` at run time. There is no apt
package — download a release tarball from
[microsoft/onnxruntime](https://github.com/microsoft/onnxruntime/releases)
and unpack it somewhere stable:

```bash
ORT_VERSION=1.20.1
mkdir -p ~/.local/onnxruntime
curl -L -o /tmp/onnxruntime.tgz \
  "https://github.com/microsoft/onnxruntime/releases/download/v${ORT_VERSION}/onnxruntime-linux-x64-${ORT_VERSION}.tgz"
tar -xzf /tmp/onnxruntime.tgz -C ~/.local/onnxruntime --strip-components=1

# Used by piper-rs at build- and run-time.
echo 'export ORT_DYLIB_PATH="$HOME/.local/onnxruntime/lib/libonnxruntime.so"' >> ~/.bashrc
source ~/.bashrc
```

## 3. Rust toolchain

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source "$HOME/.cargo/env"
```

The Tauri CLI comes from the repo's dev dependencies (`pnpm tauri …`), so
there is nothing else to install globally.

## 4. Node 20 + pnpm

Ubuntu's stock `nodejs` lags behind; use NodeSource:

```bash
curl -fsSL https://deb.nodesource.com/setup_20.x | sudo -E bash -
sudo apt install -y nodejs
sudo corepack enable pnpm
```

`sudo` is required: `corepack enable` writes symlinks into `/usr/bin`.
The first `pnpm` invocation asks to download the pnpm release — answer
«Y» in a terminal; scripted/non-interactive setups should export
`COREPACK_ENABLE_DOWNLOAD_PROMPT=0` (or install pnpm via
`sudo npm install -g pnpm` instead of corepack).

## 5. (Optional) Python 3.12 + uv — for the Python Silero engine (`ttsd`)

Skip this whole section if you only want the native Silero engine (the
default) or Piper — both synthesize in-process without Python;
the Settings dialog will simply grey out the Python Silero option, and
the other engines handle every narration locally.

```bash
sudo apt install -y python3.12 python3.12-venv
curl -LsSf https://astral.sh/uv/install.sh | sh
```

For a "run from a clone" Silero setup, after a successful build you
can launch the sidecar manually from the repo:

```bash
cd ttsd && uv run python -m ttsd
```

…and the app's runtime probe will pick it up. Bundling `ttsd` into a
single distributable binary on Ubuntu is out of scope for this guide
(the Nix flake's `.#ruvox-with-silero` output does it for you).

## 6. Configure espeak-ng data directory

`piper-rs` looks for `espeak-ng-data/` **under**
`PIPER_ESPEAKNG_DATA_DIRECTORY` — the variable points at the parent
directory, not at `espeak-ng-data` itself. Where that parent lives
depends on the distribution:

- **Ubuntu 24.04 / Debian 13:** the apt package ships the data under the
  multiarch lib dir, so point the variable at
  `/usr/lib/x86_64-linux-gnu`.
- **Fedora / Arch:** check `dpkg -L espeak-ng-data` equivalent
  (`rpm -ql espeak-ng-data`, `pacman -Ql espeak-ng`) and use the parent
  of the `espeak-ng-data` directory.

```bash
# Ubuntu 24.04 / Debian 13:
echo 'export PIPER_ESPEAKNG_DATA_DIRECTORY=/usr/lib/x86_64-linux-gnu' >> ~/.bashrc
source ~/.bashrc
```

Without this step Piper still synthesizes audio, but Russian word
stress is consistently wrong because the library falls back to
skeleton phoneme defaults.

## 7. Build

```bash
git clone https://github.com/xilec/RuVox.git
cd RuVox
pnpm install
pnpm tauri build
```

Outputs:

- `src-tauri/target/release/ruvox-tauri` — the binary.
- `src-tauri/target/release/bundle/deb/*.deb` — installable package
  (`sudo dpkg -i`).
- `src-tauri/target/release/bundle/appimage/*.AppImage` — portable
  AppImage.

### Release artifacts & self-updates

Prebuilt artifacts (Windows NSIS installer, Linux .deb and .AppImage) are
attached to [GitHub releases](https://github.com/xilec/RuVox/releases). The
Windows installer and the Linux **AppImage** self-update in-app: Settings →
«Проверить обновления» (or the startup check) downloads a
signature-verified artifact and relaunches. The .deb and source builds are
not self-updating — reinstall the new version yourself.

## Troubleshooting

- **`mpv init failed … Is mpv installed and in your PATH?` on
  startup** — the headers (`libmpv-dev`) are installed but the player
  binary is missing. Install the `mpv` package (step 1).
- **`failed to run custom build command for espeak-rs-sys`** — you are
  missing `cmake` or `libclang-dev`. Re-run step 1.
- **`libonnxruntime.so: cannot open shared object file`** —
  `ORT_DYLIB_PATH` is unset or points at a missing file. Re-run step
  2 and re-source your shell.
- **Piper voice has wrong word stress on every voice** —
  `PIPER_ESPEAKNG_DATA_DIRECTORY` is unset or wrong. Re-run step 6.
- **`devicePixelRatio` is negative / window metrics garbled** — known
  WebKitGTK issue, see
  [tauri#7354](https://github.com/tauri-apps/tauri/issues/7354). On
  vanilla Ubuntu desktops this should not happen; on minimal/headless
  setups, install `gsettings-desktop-schemas`.

## Verifying

The **Ubuntu 24.04** path was verified end-to-end (2026-08-30) on a
fresh desktop install in a VM: the step-1 package list, ONNX Runtime
download, rustup, Node 20 + pnpm, the espeak-ng data directory, the
clone + `pnpm tauri build` (producing the binary, the .deb and the
AppImage), and launching the built binary (first-run Silero download
prompt renders; the app starts once `mpv` is installed).

The Debian / Fedora / RHEL / Arch blocks are derived from
`nix/devshell.nix` (the source of truth for build dependencies) and
package-name equivalences — they have not been run on those
distributions. The Piper stress fix (step 6) was verified at the
path-layout level, not by listening to synthesized speech. If a step
fails on your machine, please open an issue or PR with the correction.
