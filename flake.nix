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

        # Shared system deps for Tauri 2 + libmpv + audio.
        systemLibs = with pkgs; [
          webkitgtk_4_1
          libsoup_3
          gtk3
          glib
          glib-networking
          libappindicator-gtk3
          librsvg
          openssl
          dbus
          mpv-unwrapped
          wayland
          wayland-protocols
          libxkbcommon
          xorg.libX11
          xorg.libXcursor
          xorg.libXrandr
          xorg.libXi
          xorg.libxcb
          libpulseaudio
          pipewire
          alsa-lib
          libGL
          fontconfig
          freetype
          libdrm
          stdenv.cc.cc.lib
          zlib
          zstd
        ];

        pkgConfigPaths = lib.makeSearchPathOutput "dev" "lib/pkgconfig" [
          pkgs.webkitgtk_4_1
          pkgs.libsoup_3
          pkgs.gtk3
          pkgs.glib
          pkgs.openssl
          pkgs.mpv-unwrapped
          pkgs.libappindicator-gtk3
          pkgs.librsvg
          pkgs.wayland
          pkgs.libxkbcommon
          pkgs.alsa-lib
          pkgs.libpulseaudio
        ];

        runtimeLibPath = lib.makeLibraryPath systemLibs;

        # ttsd — Python subprocess wrapping Silero TTS.
        # Packaged as a stand-alone Python application; the resulting binary
        # is placed next to the Tauri bundle as a sidecar.
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

        # Frontend assets: pnpm install + pnpm build → dist/.
        # pnpm.fetchDeps requires pnpm-lock.yaml; fakeHash kicks off the
        # first build with a hash mismatch that Nix reports as the real one.
        frontend = pkgs.stdenv.mkDerivation (finalAttrs: {
          pname = "ruvox-frontend";
          version = "0.2.0";
          src = ./.;

          nativeBuildInputs = [
            pkgs.nodejs_20
            pkgs.pnpm.configHook
          ];

          pnpmDeps = pkgs.pnpm.fetchDeps {
            inherit (finalAttrs) pname version src;
            hash = lib.fakeHash;
          };

          buildPhase = ''
            runHook preBuild
            pnpm run build
            runHook postBuild
          '';

          installPhase = ''
            runHook preInstall
            cp -r dist $out
            runHook postInstall
          '';
        });

        # Rust + Tauri binary. Frontend assets are staged from `frontend`.
        ruvox = pkgs.rustPlatform.buildRustPackage {
          pname = "ruvox";
          version = "0.2.0";
          src = ./.;

          cargoLock = {
            lockFile = ./src-tauri/Cargo.lock;
          };

          sourceRoot = "source/src-tauri";

          nativeBuildInputs = with pkgs; [
            pkg-config
            cargo-tauri
            wrapGAppsHook3
          ];

          buildInputs = systemLibs;

          # Stage pre-built frontend so `tauri build` finds ../dist.
          preBuild = ''
            cp -r ${frontend} ../dist
          '';

          # Use `cargo tauri build` to produce the bundle, not plain `cargo
          # build` — tauri-build copies icons, injects window config.
          buildPhase = ''
            runHook preBuild
            cargo tauri build --no-bundle
            runHook postBuild
          '';

          installPhase = ''
            runHook preInstall
            install -Dm755 target/release/ruvox-tauri $out/bin/ruvox
            # Sidecar: ttsd is linked in via wrapper below.
            runHook postInstall
          '';

          postFixup = ''
            wrapProgram $out/bin/ruvox \
              --prefix PATH : ${lib.makeBinPath [ ttsd pkgs.mpv ]} \
              --prefix LD_LIBRARY_PATH : ${runtimeLibPath} \
              --set GIO_EXTRA_MODULES ${pkgs.glib-networking}/lib/gio/modules
          '';

          PKG_CONFIG_PATH = pkgConfigPaths;
          OPENSSL_DIR = "${pkgs.openssl.dev}";
          OPENSSL_LIB_DIR = "${pkgs.openssl.out}/lib";

          meta = {
            description = "RuVox 2 — desktop app for reading technical texts aloud";
            mainProgram = "ruvox";
            platforms = lib.platforms.linux;
          };
        };
      in
      {
        packages = {
          default = ruvox;
          inherit ruvox ttsd;
          frontend = frontend;
        };

        devShells.default = import ./shell.nix { inherit pkgs; };

        apps.default = {
          type = "app";
          program = "${ruvox}/bin/ruvox";
        };
      });
}
