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
  libayatana-appindicator3-dev libmpv-dev libopus-dev libssl-dev \
  libasound2-dev libpulse-dev libpipewire-0.3-dev \
  libfontconfig1-dev libfreetype6-dev libgl-dev libdrm-dev \
  libwayland-dev libxkbcommon-dev wayland-protocols \
  espeak-ng espeak-ng-data \
  curl file
```

### Fedora 40+ / RHEL 10+

```bash
sudo dnf install -y \
  gcc gcc-c++ pkgconf-pkg-config cmake clang clang-devel \
  webkit2gtk4.1-devel libsoup3-devel gtk3-devel librsvg2-devel \
  libayatana-appindicator-gtk3-devel mpv-libs-devel opus-devel openssl-devel \
  alsa-lib-devel pulseaudio-libs-devel pipewire-devel \
  fontconfig-devel freetype-devel mesa-libGL-devel libdrm-devel \
  wayland-devel libxkbcommon-devel wayland-protocols-devel \
  espeak-ng espeak-ng-data \
  curl file
```

### Arch Linux

```bash
sudo pacman -S --needed \
  base-devel cmake clang \
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
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"
cargo install tauri-cli --version '^2.0' --locked
```

## 4. Node 20 + pnpm

Ubuntu's stock `nodejs` lags behind; use NodeSource:

```bash
curl -fsSL https://deb.nodesource.com/setup_20.x | sudo -E bash -
sudo apt install -y nodejs
corepack enable pnpm
```

## 5. (Optional) Python 3.12 + uv — for Silero

Skip this whole section if you only want Piper (the default engine);
the Settings dialog will simply grey out the Silero option, and Piper
handles every narration in-process.

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

`piper-rs` looks for `espeak-ng-data/` under
`PIPER_ESPEAKNG_DATA_DIRECTORY`. On Ubuntu, the apt package ships the
data at `/usr/share/espeak-ng-data/`, so point the variable at the
parent:

```bash
echo 'export PIPER_ESPEAKNG_DATA_DIRECTORY=/usr/share' >> ~/.bashrc
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

## Troubleshooting

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

This guide is derived from `nix/devshell.nix` (the Nix dev shell, which
is the source of truth for build dependencies). It has not been
end-to-end tested on a fresh Ubuntu install — if a step fails on your
machine, please open an issue or PR with the correction.
