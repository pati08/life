{
  description = "My rust devenv nix flake";
  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  inputs.flake-utils.url = "github:numtide/flake-utils";
  inputs.flake-utils.inputs.nixpkgs.follows = "nixpkgs";
  inputs.wgslAnalyzer.url = "github:wgsl-analyzer/wgsl-analyzer";
  inputs.wgslAnalyzer.inputs.nixpkgs.follows = "nixpkgs";

  outputs = { self, nixpkgs, flake-utils, ... }@inputs:
    flake-utils.lib.eachDefaultSystem
      (system:
        let pkgs = nixpkgs.legacyPackages.${system}; in
        {
          devShells.default = import ./shell.nix { inherit pkgs; inherit inputs; };
        }
      );
}
