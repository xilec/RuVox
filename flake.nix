{
  description = "RuVox — desktop app for reading technical texts aloud";

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
        #   - mpv binary in PATH (player).
        #   - ttsd binary in PATH only when withSilero = true (the Silero
        #     subprocess is opt-in; with withSilero = false the runtime
        #     probe in tts::availability::probe_silero greys the option
        #     out in Settings and Piper handles all narration in-process).
        #   - mpv audio-backend libs in LD_LIBRARY_PATH (pulse/pipewire/alsa
        #     are not in buildInputs because Tauri itself doesn't need them).
        #   - WEBKIT_DISABLE_DMABUF_RENDERER=1 for KDE Plasma 6 Wayland
        #     (see issue #3).
        mkRuvox = { withSilero ? false }: pkgs.rustPlatform.buildRustPackage (finalAttrs: {
          pname = if withSilero then "ruvox-with-silero" else "ruvox";
          version = "0.2.0";
          src = ./.;

          cargoLock = {
            lockFile = ./src-tauri/Cargo.lock;
          };

          # cargo-tauri.hook uses both to know where the Tauri app lives.
          cargoRoot = "src-tauri";
          buildAndTestSubdir = "src-tauri";

          # Pin pname here independently of withSilero — the pnpm
          # lockfile and the on-disk content of fetched deps are
          # identical for slim and full, so we want a single shared
          # fixed-output derivation. Inheriting `pname` from finalAttrs
          # would make the full build's pname leak into the deps
          # derivation name, change its hash, and break the build.
          pnpmDeps = pkgs.fetchPnpmDeps {
            pname = "ruvox";
            inherit (finalAttrs) version src;
            fetcherVersion = 3;
            hash = "sha256-5lpLq7SoTnKyW6jPS8HfROaC8S7uPefG4NmMNFue4EY=";
          };

          nativeBuildInputs = with pkgs; [
            cargo-tauri.hook
            nodejs_20
            pnpm
            pnpmConfigHook
            pkg-config
            wrapGAppsHook3
            # cmake: espeak-rs-sys 0.1.9 vendors libespeak-ng and builds it
            # via cmake from its own build.rs.
            cmake
            # bindgenHook: sets LIBCLANG_PATH and BINDGEN_EXTRA_CLANG_ARGS
            # (with the C system header search paths from stdenv.cc) so
            # bindgen invocations in espeak-rs-sys's build script can find
            # `<stdio.h>` etc. inside the hermetic sandbox.
            rustPlatform.bindgenHook
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
            # libopus: storage::audio encodes WAV → Ogg-Opus via the
            # `opus = "0.3"` FFI crate, which links against C libopus 1.x.
            # buildInputs is enough — wrapGAppsHook3 picks it up for the
            # runtime LD_LIBRARY_PATH automatically.
            libopus
            # espeak-ng — present for its data files only (see preFixup).
            # piper-rs uses espeak-rs-sys's vendored libespeak-ng for
            # phonemization, but its data dir lives in the cargo OUT_DIR
            # which espeak-rs doesn't search at runtime. Pointing
            # PIPER_ESPEAKNG_DATA_DIRECTORY at the nixpkgs share/ dir loads
            # the full ru_dict / phondata / intonations files instead of
            # the skeleton defaults that produce wrong Russian stress.
            espeak-ng
            # onnxruntime — `ort` is configured with `load-dynamic` and
            # dlopens libonnxruntime.so at runtime via ORT_DYLIB_PATH (set
            # in preFixup). At build time `ort-sys` probes pkg-config for
            # libonnxruntime; nixpkgs onnxruntime ships a .pc, so the probe
            # succeeds and the build script becomes a no-op.
            onnxruntime
            # sonic — espeak-ng's CMakeLists has
            # `find_library(SONIC_LIB sonic)` and falls back to git-cloning
            # https://github.com/waywardgeek/sonic if not found, which is
            # blocked by the nix sandbox. Providing the system library
            # short-circuits the FetchContent path.
            sonic
          ];

          # Single target is enough for Nix — we only want a usable binary,
          # not an OS-native package.  "deb" is the cheapest Linux bundle.
          tauriBundleType = "deb";

          # espeak-ng on Linux pins `path_home[N_PATH_HOME=160]` and then
          # writes `<path_home>/<phoneme-file>` into an 180-byte buffer
          # (compiledata.c::LoadSpect). The nix sandbox phsource path is
          # `/build/<src>/target/.../espeak-rs-sys-<hash>/out/build/espeak-ng-data/../phsource`,
          # already over 180 bytes, so snprintf truncates filenames and the
          # phoneme compiler errors out with "Bad vowel file" / "Failed to
          # open: ...vwl_en_us_nyc/a_ra". Bumping the buffer to 1024 fixes
          # the `cargo tauri build` cmake step deterministically.
          preBuild = ''
            substituteInPlace "$NIX_BUILD_TOP/cargo-vendor-dir/espeak-rs-sys-0.1.9/espeak-ng/src/libespeak-ng/speech.h" \
              --replace-fail 'N_PATH_HOME_DEF  160' 'N_PATH_HOME_DEF  1024'
          '';

          preFixup = ''
            gappsWrapperArgs+=(
              --prefix PATH : ${lib.makeBinPath ([ pkgs.mpv ] ++ lib.optional withSilero ttsd)}
              --prefix LD_LIBRARY_PATH : ${extraRuntimeLibPath}
              --set-default WEBKIT_DISABLE_DMABUF_RENDERER 1
              --set-default PIPER_ESPEAKNG_DATA_DIRECTORY ${pkgs.espeak-ng}/share
              --set-default ORT_DYLIB_PATH ${pkgs.onnxruntime}/lib/libonnxruntime.so
            )
          '';

          meta = {
            description =
              if withSilero
              then "RuVox — desktop app for reading technical texts aloud (with Silero/Python ttsd)"
              else "RuVox — desktop app for reading technical texts aloud";
            mainProgram = "ruvox-tauri";
            platforms = lib.platforms.linux;
          };
        });

        ruvox = mkRuvox { };
        ruvox-with-silero = mkRuvox { withSilero = true; };
      in
      {
        packages = {
          default = ruvox;
          inherit ruvox ruvox-with-silero ttsd;
        };

        devShells.default = import ./nix/devshell.nix { inherit pkgs; };

        apps.default = {
          type = "app";
          program = "${ruvox}/bin/ruvox-tauri";
        };
      });
}
