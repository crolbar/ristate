{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-parts = {
      url = "github:hercules-ci/flake-parts";
      inputs.nixpkgs-lib.follows = "nixpkgs";
    };
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = {
    self,
    nixpkgs,
    flake-parts,
    rust-overlay,
    ...
  } @ inputs: let
    overlays = [(import rust-overlay)];
  in
    flake-parts.lib.mkFlake {inherit inputs;} {
      systems = ["x86_64-linux"];
      imports = [
        inputs.flake-parts.flakeModules.easyOverlay
      ];

      perSystem = {
        system,
        config,
        ...
      }: let
        pkgs = import nixpkgs {inherit system overlays;};
      in {
        _module.args.pkgs = pkgs;

        devShells.default = let
          rust = pkgs.rust-bin.stable.latest.default.override {
            extensions = ["rust-src" "rust-analyzer"];
          };
        in
          with pkgs;
            mkShell {
              nativeBuildInputs = [rust];
            };

        packages = rec {
          ristate = pkgs.callPackage ./package.nix {};
          default = ristate;
        };

        overlayAttrs = {
          inherit (config.packages) ristate;
        };
      };
    };
}
