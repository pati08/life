{
  description = "My rust devenv nix flake";
  inputs = {
    cargo2nix.url = "github:cargo2nix/cargo2nix/release-0.11.0";
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs = { self, nixpkgs, flake-utils, cargo2nix, rust-overlay, ... }@inputs:
    flake-utils.lib.eachDefaultSystem
      (system:
        let
          pkgs = import nixpkgs {
            inherit system;
            overlays = [ cargo2nix.overlays.default (import rust-overlay) ];
          };
          rustPkgs = pkgs.rustBuilder.makePackageSet {
            rustChannel = "nightly";
            rustProfile = "minimal";
            packageFun = import ./Cargo.nix;
            packageOverrides = pkgs: pkgs.rustBuilder.overrides.all ++ [
              (pkgs.rustBuilder.rustLib.makeOverride {
                name = "wayland-sys";
                overrideAttrs = drv: {
                  propagatedBuildInputs = drv.propagatedBuildInputs or [ ] ++ (with pkgs; [
                    wayland.dev
                  ]);
                };
              })
            ];
          };
        in {
          # devShells.default = import ./shell.nix { inherit pkgs; inherit inputs; };
          devShells.default = pkgs.mkShell rec {
            buildInputs = with pkgs; [
              (rust-bin.fromRustupToolchainFile ./rust-toolchain.toml)
              clang
              # Replace llvmPackages with llvmPackages_X, where X is the latest LLVM version (at the time of writing, 16)
              # llvmPackages_18.bintools
              # rustup
              egl-wayland

              udev alsa-lib vulkan-loader
              xorg.libX11 xorg.libXcursor xorg.libXi xorg.libXrandr # To use the x11 feature
              libxkbcommon wayland # To use the wayland feature

              libGL
              vulkan-headers
              vulkan-loader
              vulkan-tools
              vulkan-tools-lunarg
              vulkan-extension-layer
              vulkan-validation-layers # don't need them *strictly* but immensely helpful
              cmake

              wasm-pack
              binaryen
              wasm-bindgen-cli
              # nodejs

              # command automation
              just

              # for the crate graph
              graphviz

              static-web-server
            ];
            nativeBuildInputs = with pkgs; [ pkg-config ] ;
            LIBCLANG_PATH = pkgs.lib.makeLibraryPath [pkgs.llvmPackages_latest.libclang.lib];
            LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath buildInputs;
          };
          packages.default = (rustPkgs.workspace.life {}).overrideAttrs (drv: rec {
            buildInputs = drv.buildInputs or [ ] ++ (with pkgs; [
              udev alsa-lib vulkan-loader
              xorg.libX11 xorg.libXcursor xorg.libXi xorg.libXrandr # To use the x11 feature
              libxkbcommon wayland # To use the wayland feature
            ]);
            nativeBuildInputs = drv.nativeBuildInputs or [ ] ++ (with pkgs; [
              makeWrapper
              pkg-config
              libxkbcommon
              wayland
              autoPatchelfHook
            ]);
            runtimeDependencies = buildInputs;
          });
        }
      );
}
