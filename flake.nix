{
  description = "A minimal wallpaper daemon for Wayland, written in Rust";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  };

  outputs =
    {
      self,
      nixpkgs,
    }:
    let
      inherit (nixpkgs.lib) genAttrs;
      systems = [
        "x86_64-linux"
        "aarch64-linux"
      ];
      forEachSystem =
        perSystem:
        genAttrs systems (
          system:
          let
            pkgs = nixpkgs.legacyPackages.${system};
          in
          perSystem { inherit pkgs system; }
        );
    in
    {
      overlays.default = final: prev: {
        wallpaper-rs = final.callPackage ./nix/package.nix { };
      };

      packages = forEachSystem (
        { pkgs, ... }: {
          default = pkgs.callPackage ./nix/package.nix { };
        }
      );

      devShells = forEachSystem (
        { pkgs, system }: {
          default = pkgs.callPackage ./nix/devshell.nix {
            wallpaper-rs = self.packages.${system}.default;
          };
        }
      );

      homeManagerModules.default = { lib, pkgs, ... }: {
        imports = [ ./nix/home-module.nix ];
        programs.wallpaper-rs.package =
          lib.mkDefault
            self.packages.${pkgs.stdenv.hostPlatform.system}.default;
      };
    };
}
