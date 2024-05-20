{
  description = "My rust devenv nix flake";
  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  inputs.flake-utils.url = "github:numtide/flake-utils";
  inputs.flake-utils.inputs.nixpkgs.follows = "nixpkgs";
  inputs.wgslAnalyzer.url = "github:wgsl-analyzer/wgsl-analyzer";
  inputs.fenix = {
    url = "github:nix-community/fenix";
    inputs.nixpkgs.follows = "nixpkgs";
  };
  inputs.wgslAnalyzer.inputs.nixpkgs.follows = "nixpkgs";

  outputs = { self, nixpkgs, flake-utils, fenix, ... }@inputs:
    flake-utils.lib.eachDefaultSystem
      (system:
        let pkgs = nixpkgs.legacyPackages.${system}; lib = pkgs.lib; in
        {
          devShells.default = import ./shell.nix { inherit pkgs; inherit inputs; };
          packages.default =
            let 
              toolchain = fenix.packages.${system}.minimal.toolchain;
            in (pkgs.makeRustPlatform {cargo = toolchain; rustc = toolchain;}).buildRustPackage rec {
              pname = "life";
              version = "0.1.0";

              src = pkgs.fetchFromGitHub {
                owner = "nvim-ftw";
                repo = pname;
                rev = "5cd06ff";
                hash = "sha256-2ftsrV7+pAReyroFc32j49GdeQc9eMcK72rFbSNCy44=";
              };

              cargoHash = "sha256-zw1PY6Lv74LQu4w+w4AaxzCQ6KCTNIg02CWlReQmqH0=";

              meta = with lib; {
                description = "A cool implementation of conway's game of life!";
                homepage = "https://github.com/nvim-ftw/life";
                license = licenses.gpl3Only;
                maintainers = [];
              };
            };
        }
      );
}
