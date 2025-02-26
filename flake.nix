{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-parts.url = "github:hercules-ci/flake-parts";
    crane.url = "github:ipetkov/crane";
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.rust-analyzer-src.follows = "";
    };
    treefmt-nix = {
      url = "github:numtide/treefmt-nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    inputs@{ flake-parts, crane, ... }:
    flake-parts.lib.mkFlake { inherit inputs; } {
      imports = [
        inputs.treefmt-nix.flakeModule
      ];
      systems = [
        "x86_64-linux"
        "aarch64-linux"
        "aarch64-darwin"
        "x86_64-darwin"
      ];
      perSystem =
        {
          config,
          self',
          inputs',
          pkgs,
          system,
          ...
        }:
        let
          craneLib = (crane.mkLib pkgs).overrideToolchain inputs'.fenix.packages.stable.toolchain;
        in
        {
          devShells.default =
            (craneLib.devShell.override {
              mkShell = pkgs.mkShell.override {
                stdenv = pkgs.stdenvAdapters.useMoldLinker pkgs.clangStdenv;
              };
            })
              {
                buildInputs = with pkgs; [
                  cmake
                ];
                LIBCLANG_PATH = "${pkgs.llvmPackages.libclang.lib}/lib";
              };

          treefmt = {
            projectRootFile = "flake.nix";
            programs = {
              nixfmt.enable = true;
              rustfmt.enable = true;
            };
          };
        };
      flake = {
      };
    };
}
