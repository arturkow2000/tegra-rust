{
  description = "Nix-based devcontainer for development of Tegra Rust driver";

  inputs = {
    flake-parts.url = "github:hercules-ci/flake-parts";
    nixpkgs.url = "github:NixOS/nixpkgs";
    nix-devcontainer = {
      url = "github:arturkow2000/nix-devcontainer";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    {
      self,
      flake-parts,
      nixpkgs,
      nix-devcontainer,
      rust-overlay,
      ...
    }@inputs:
    flake-parts.lib.mkFlake { inherit inputs; } {
      systems = [ "x86_64-linux" ];
      perSystem =
        { system, inputs', ... }:
        let
          # Glibc requires GCC which we need to compile. Otherwise we are already using Clang
          # for everything else and disabling this will get rid of glibc completely.
          # Can shorten container build time to a minute, as we need only to compile a few core
          # libs (musl, libunwind, libc++) and clang wrappers.
          includeGlibc = true;

          inherit (inputs.nixpkgs) lib;
          mkPkgs =
            {
              libc,
              thumb,
              isStatic,
            }:
            import nixpkgs.outPath {
              localSystem = inputs'.nixpkgs.legacyPackages.stdenv.hostPlatform.config;
              crossSystem =
                let
                  abi =
                    {
                      glibc = "gnueabihf";
                      musl = "musleabihf";
                    }
                    .${libc};
                in
                {
                  config = "armv7l-unknown-linux-${abi}";
                  useLLVM = true;
                  inherit isStatic;
                  inherit libc;
                  gcc = {
                    # Target Tegra 2 which has no Neon and VFP limited to 16 instead of standard 32 registers.
                    arch = "armv7-a";
                    fpu = "vfpv3-d16";
                    float-abi = "hard";
                  }
                  // lib.optionalAttrs (libc != "glibc") {
                    # Building for CA-9 broken on glibc. Just do generic build, at runtime we will link to system's glibc anyway.
                    cpu = "cortex-a9";
                  }
                  // lib.optionalAttrs thumb {
                    thumb = true;
                  };
                };
              overlays = [
                rust-overlay.overlays.default
                nix-devcontainer.overlays.nix-devcontainer
              ];
            };
          mkRustToolchain =
            pkgs:
            (pkgs.buildPackages.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml).overrideAttrs (
              self: super: {
                # Remove dependency on GCC. Allows us to save hours by not having to cross-compile compiler.
                # We use LLVM instead. Currently requires musl as glibc can't be built with clang.
                # Still, even on glibc builds can save us some time as we need only stage2 gcc, no need for stage3.
                depsHostHostPropagated = [ ];
                propagatedBuildInputs = [ ];
              }
            );
          pkgsMusl = mkPkgs {
            libc = "musl";
            thumb = true;
            isStatic = true;
          };
          pkgsGlibc = mkPkgs {
            libc = "glibc";
            thumb = true;
            isStatic = false;
          };
          rustMusl = mkRustToolchain pkgsMusl;
          rustGlibc = mkRustToolchain pkgsGlibc;
          mkEnv =
            stdenv:
            let
              # For building for Tegra we use custom target spec JSONs where we explicitly
              # set what CPU features can be used.
              # This can be done through RUSTFLAGS but then rustc will keep emitting warnings:
              # warning: unstable feature specified for `-Ctarget-feature`
              # currently there is no other way to silence them but to use -Zjson-target-spec with custom JSON.
              # https://github.com/rust-lang/rust/issues/162235
              # Still we need to set some env variables for compiler to work.
              baseToTegra =
                let
                  t20Flags = "--target=${stdenv.hostPlatform.config} -mcpu=cortex-a9 -mfpu=vfpv3-d16 -mfloat-abi=hard";
                in
                {
                  armv7l-unknown-linux-gnueabihf = {
                    target = "TEGRA20_UNKNOWN_LINUX_GNUEABIHF";
                    cflags = t20Flags;
                    cxxflags = t20Flags;
                  };
                  armv7l-unknown-linux-musleabihf = {
                    target = "TEGRA20_UNKNOWN_LINUX_MUSLEABIHF";
                    cflags = t20Flags;
                    cxxflags = t20Flags;
                  };
                };
              f =
                {
                  target,
                  cflags ? null,
                  cxxflags ? null,
                }:
                {
                  # For cargo itself.
                  "CARGO_TARGET_${target}_LINKER" = "${stdenv.hostPlatform.config}-clang";
                  # For building C/C++ code (currently tracy).
                  "C_${lib.strings.toLower target}" = "${lib.getExe stdenv.cc}";
                  "CXX_${lib.strings.toLower target}" = "${lib.getExe stdenv.cc}++";
                  # For Clang's libc++
                  "CXXSTDLIB_${lib.strings.toLower target}" = "c++";
                }
                // lib.optionalAttrs (cflags != null) {
                  # FIXME: nix flags defined crossSystem don't propagate properly. When tracy-sys builds tracy C++ lib
                  # it is built with Neon enabled, resulting in crash on T20.
                  "CFLAGS_${lib.strings.toLower target}" = cflags;
                }
                // lib.optionalAttrs (cxxflags != null) {
                  # FIXME: nix flags defined crossSystem don't propagate properly. When tracy-sys builds tracy C++ lib
                  # it is built with Neon enabled, resulting in crash on T20.
                  "CXXFLAGS_${lib.strings.toLower target}" = cxxflags;
                };
            in
            # Set envvars for generic target (e.g. armv7l-unknown-linux-gnueabihf).
            f { target = stdenv.hostPlatform.rust.cargoEnvVarTarget; }
            # Set envvars for Tegra 20 (in the future may be used for other platforms) - this CPU doesn't have Neon, code compiled for generic ARM target may not work!
            // lib.optionalAttrs (baseToTegra ? "${stdenv.hostPlatform.config}") (
              f baseToTegra.${stdenv.hostPlatform.config}
            );
          mkL2ClangWrapper =
            { cc }:
            pkgsMusl.runCommand "${cc.pname or cc.name}-l2-wrapper" { depsBuildTarget = [ cc ]; } ''
              mkdir -p $out/bin
              {
                echo "#!${pkgsMusl.pkgsBuildBuild.runtimeShell}"
                echo "set -e"
                while IFS='=' read -r k v; do
                  echo "export ''${k}=\"$v\""
                done < <(env | grep -E '(_FOR_TARGET|^NIX_(C|LD)FLAGS|^NIX_CC_WRAPPER.*|^NIX_BINTOOLS_WRAPPER.*)=')
                echo 'exec "${cc}/bin/$(${pkgsMusl.pkgsBuildBuild.coreutils}/bin/basename "$0")" "$@"'
              } > "$out/wrapper"
              chmod +x "$out/wrapper"

              wrap() {
                local app="$1"
                ln -s ../wrapper "$out/bin/$app"
              }

              while IFS= read -rd "" app; do
                app="''${app#./}"
                wrap "$app"
              done < <(cd ${cc}/bin; find -mindepth 1 -maxdepth 1 ! -type d -print0)
            '';
          mkCrossClangToolchain =
            { cc }:
            let
              bintools =
                (cc.bintools.override {
                  # overriding sharedLibraryLoader does not work as expected.
                  # build script checks for path presence and won't
                  # set nix-support/dynamic-linker if linker is absent during build.
                }).overrideAttrs
                  (
                    self: super: {
                      # Strip out nix store path. This, together with NIX_DONT_SET_RPATH
                      # allow built binaries to run on target without Nix store.
                      postFixup = (super.postFixup or "") + ''
                        dyld="$(basename "$(cat $out/nix-support/dynamic-linker)")"
                        echo "/usr/lib/$dyld" > $out/nix-support/dynamic-linker
                      '';
                    }
                  );
            in
            mkL2ClangWrapper {
              cc = cc.override {
                inherit bintools;
              };
            };

          env = mkEnv pkgsMusl.stdenv // lib.optionalAttrs includeGlibc (mkEnv pkgsGlibc.stdenv);
          # For rust build scripts.
          depsBuildBuild = [
            pkgsMusl.pkgsBuildBuild.stdenv.cc
          ];
          depsBuildTarget = [ ];
          nativeBuildInputs = [
            pkgsMusl.pkgsBuildBuild.openssh
            rustMusl
            (mkCrossClangToolchain { inherit (pkgsMusl.stdenv) cc; })
          ]
          ++ lib.optional includeGlibc rustGlibc
          # When using clang this will be just another wrapper around same compiler.
          # In case of GCC those will be 2 distinct compilers.
          ++ lib.optional includeGlibc (mkCrossClangToolchain {
            inherit (pkgsGlibc.stdenv) cc;
          });
        in
        {
          devShells.default = pkgsMusl.mkShellNoCC {
            inherit
              depsBuildBuild
              depsBuildTarget
              nativeBuildInputs
              env
              ;
          };
          packages.devcontainer = pkgsMusl.buildPackages.mkDevcontainer {
            name = "tegra-rust";
            inherit
              depsBuildBuild
              depsBuildTarget
              nativeBuildInputs
              ;
            env = env // {
              # TODO: why this happens?
              # Fix this in nix-devcontainer.
              NIX_ENFORCE_PURITY = 0;
              # Built binaries will run in non-NixOS environment, we are using
              # Nix only to cross-compile for target environment (currently based on yocto).
              NIX_DONT_SET_RPATH_FOR_TARGET = 1;
            };
            defaultShell = "fish";
            enabledShells = [
              "nushell"
              "fish"
              "zsh"
            ];
            shellTheme = "starship";
          };
        };
    };
}
