{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-parts.url = "github:hercules-ci/flake-parts";
  };

  outputs = inputs@{ self, flake-parts, nixpkgs, rust-overlay, ... }:
    flake-parts.lib.mkFlake { inherit inputs; } {
      systems = [ "x86_64-linux" ];
      flake = {
        overlays.default = final: prev: {
          pta-template-engine = final.callPackage ./default.nix {
            inherit (prev) rustPlatform;
          };
        };
      };

      perSystem = { config, system, pkgs, ... }: {
        _module.args.pkgs = import nixpkgs {
          inherit system;
          overlays = [ (import inputs.rust-overlay) self.overlays.default ];
        };

        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [ rust-bin.stable.latest.default ];
        };

        packages.pta-template-engine = pkgs.pta-template-engine;
        packages.default = config.packages.pta-template-engine;

        apps.pta-template-engine.program = "${config.packages.pta-template-engine}/bin/ptatemp";
        apps.default.program = config.apps.pta-template-engine.program;

        formatter = pkgs.nixpkgs-fmt;
      };
    };
}
