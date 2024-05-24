{
  description = "My rust devenv nix flake";
  inputs = {
    cargo2nix.url = "github:cargo2nix/cargo2nix/release-0.11.0";
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils, cargo2nix, ... }@inputs:
    flake-utils.lib.eachDefaultSystem
      (system:
        let
          pkgs = import nixpkgs {
            inherit system;
            overlays = [cargo2nix.overlays.default];
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
          devShells.default = import ./shell.nix { inherit pkgs; inherit inputs; };
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
