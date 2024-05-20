{
  description = "My rust devenv nix flake";
  inputs = {
    # fenix = {
    #   url = "github:nix-community/fenix";
    #   inputs.nixpkgs.follows = "nixpkgs";
    # };
    cargo2nix.url = "github:cargo2nix/cargo2nix/release-0.11.0";
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    # nixpkgs.follows = "cargo2nix/nixpkgs";
  };

  outputs = { self, nixpkgs, flake-utils, cargo2nix, ... }@inputs:
    flake-utils.lib.eachDefaultSystem
      (system:
        let
          pkgs = import nixpkgs {
            inherit system;
            overlays = [cargo2nix.overlays.default];
          };
          lib = pkgs.lib;
          rustPkgs = pkgs.rustBuilder.makePackageSet {
            # rustVersion = "2024-04-16";
            rustChannel = "nightly";
            rustProfile = "minimal";
            packageFun = import ./Cargo.nix;
          };
        in {
          devShells.default = import ./shell.nix { inherit pkgs; inherit inputs; };
          packages.default = (rustPkgs.workspace.life {});
        }
      );
}
