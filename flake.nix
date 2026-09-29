{
  description = "nixpresence — NixOS VRChat chatbox + Discord presence";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs { inherit system; };
        nixpresence = pkgs.callPackage ./packaging/nix/package.nix {
          nixpresenceSrc = self;
        };
      in {
        packages = {
          default = nixpresence;
          inherit nixpresence;
        };
        apps.default = {
          type = "app";
          program = "${nixpresence}/bin/nixpresence";
        };
        devShells.default = pkgs.mkShell {
          packages = with pkgs; [
            rustc cargo rustfmt clippy pkg-config dbus
          ];
          LD_LIBRARY_PATH = "/run/opengl-driver/lib";
        };
      }
    ) // {
      nixosModules.default = import ./nix/modules/nixos.nix;
      nixosModules.nixpresence = import ./nix/modules/nixos.nix;
      homeManagerModules.default = import ./nix/modules/home-manager.nix;
      homeManagerModules.nixpresence = import ./nix/modules/home-manager.nix;
      overlays.default = final: prev: {
        nixpresence = final.callPackage ./packaging/nix/package.nix {
          nixpresenceSrc = self;
        };
      };
    };
}
