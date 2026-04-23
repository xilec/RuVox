{
  description = "RuVox 2 — desktop app for reading technical texts aloud";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs { inherit system; };
        lib = pkgs.lib;

        # Runtime libs not in buildInputs (and therefore not picked up by
        # wrapGAppsHook3): mpv audio backends.  webkitgtk_4_1 / glib etc.
        # come in via buildInputs and the wrapper handles them.
        extraRuntimeLibs = with pkgs; [
          libpulseaudio
          pipewire
          alsa-lib
        ];

        extraRuntimeLibPath = lib.makeLibraryPath extraRuntimeLibs;

        # ttsd — Python subprocess wrapping Silero TTS.
        ttsd = pkgs.python312Packages.buildPythonApplication {
          pname = "ruvox-ttsd";
          version = "0.1.0";
          pyproject = true;
          src = ./ttsd;

          nativeBuildInputs = [ pkgs.python312Packages.hatchling ];

          propagatedBuildInputs = with pkgs.python312Packages; [
            torch
            numpy
            scipy
            pydantic
            omegaconf
            torchaudio
          ];

          # Tests depend on network (Silero model download) — run separately.
          doCheck = false;

          meta = {
            description = "RuVox TTS daemon (Silero TTS subprocess)";
            mainProgram = "ttsd";
          };
        };

        # ruvox — Tauri binary built via cargo-tauri.hook.
        #
        # Front-end is NOT a separate derivation: cargo-tauri.hook calls
        # `cargo tauri build`, which in turn runs the `beforeBuildCommand`
        # from tauri.conf.json (`pnpm build`).  pnpmConfigHook prepares an
        # offline node_modules from pnpmDeps so that `pnpm build` works in
        # the hermetic Nix sandbox.
        #
        # What the hook brings:
        #   - buildPhase: `cargo tauri build --bundles <tauriBundleType>`
        #     with CARGO_TARGET_DIR injected into config.toml (workaround
        #     for tauri#10190, cargo tauri ignores env var).
        #   - installPhase (Linux): unpacks target/bundle/deb/*/data/usr/*
        #     into $out, so the binary lands in $out/bin.
        #   - fixupScript (Linux): extends gappsWrapperArgs with
        #     WEBKIT_GST_ALLOWED_URI_PROTOCOLS=asset,
        #     GST_PLUGIN_SYSTEM_PATH_1_0 for tauri asset-protocol,
        #     and __NV_DISABLE_EXPLICIT_SYNC=1 for NVIDIA+Wayland.
        #
        # What wrapGAppsHook3 brings:
        #   - XDG_DATA_DIRS + GIO_EXTRA_MODULES + GI_TYPELIB_PATH on the
        #     wrapper, so WebKit can read GSettings schemas on NixOS and
        #     devicePixelRatio reports a sane value (see issue #2).
        #
        # What we add on top:
        #   - ttsd + mpv binaries in PATH (sidecar + player).
        #   - mpv audio-backend libs in LD_LIBRARY_PATH (pulse/pipewire/alsa
        #     are not in buildInputs because Tauri itself doesn't need them).
        #   - WEBKIT_DISABLE_DMABUF_RENDERER=1 for KDE Plasma 6 Wayland
        #     (see issue #3).
        ruvox = pkgs.rustPlatform.buildRustPackage (finalAttrs: {
          pname = "ruvox";
          version = "0.2.0";
          src = ./.;

          cargoLock = {
            lockFile = ./src-tauri/Cargo.lock;
          };

          # cargo-tauri.hook uses both to know where the Tauri app lives.
          cargoRoot = "src-tauri";
          buildAndTestSubdir = "src-tauri";

          pnpmDeps = pkgs.fetchPnpmDeps {
            inherit (finalAttrs) pname version src;
            fetcherVersion = 3;
            hash = "sha256-/FNFfLZqu/ndlHtg8ee2Qa1tNiarwT7hI8t0m/LsLbo=";
          };

          nativeBuildInputs = with pkgs; [
            cargo-tauri.hook
            nodejs_20
            pnpm
            pnpmConfigHook
            pkg-config
            wrapGAppsHook3
          ];

          # Transitive deps (gtk3, glib, libsoup_3, wayland, x11, libGL,
          # fontconfig, dbus, ...) come in through webkitgtk_4_1 +
          # wrapGAppsHook3 and are injected into the wrapper automatically.
          buildInputs = with pkgs; [
            webkitgtk_4_1
            glib-networking
            libayatana-appindicator
            librsvg
            openssl
            mpv-unwrapped
          ];

          # Single target is enough for Nix — we only want a usable binary,
          # not an OS-native package.  "deb" is the cheapest Linux bundle.
          tauriBundleType = "deb";

          preFixup = ''
            gappsWrapperArgs+=(
              --prefix PATH : ${lib.makeBinPath [ ttsd pkgs.mpv ]}
              --prefix LD_LIBRARY_PATH : ${extraRuntimeLibPath}
              --set-default WEBKIT_DISABLE_DMABUF_RENDERER 1
            )
          '';

          meta = {
            description = "RuVox 2 — desktop app for reading technical texts aloud";
            mainProgram = "ruvox-tauri";
            platforms = lib.platforms.linux;
          };
        });
      in
      {
        packages = {
          default = ruvox;
          inherit ruvox ttsd;
        };

        devShells.default = import ./shell.nix { inherit pkgs; };

        apps.default = {
          type = "app";
          program = "${ruvox}/bin/ruvox-tauri";
        };
      });
}
