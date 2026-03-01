{
  description = "A very small, very simple, yet very wallpaper tool written in rust.";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    systems.url = "github:nix-systems/default-linux";
  };

  outputs =
    {
      nixpkgs,
      systems,
      self,
      ...
    }:
    let
      inherit (nixpkgs) lib;
      eachSystem = f: lib.genAttrs (import systems) (system: f nixpkgs.legacyPackages.${system});
    in
    {
      formatter = eachSystem (pkgs: pkgs.alejandra);

      devShells = eachSystem (pkgs: {
        default = pkgs.callPackage ./nix/shell.nix;
      });

      packages = eachSystem (pkgs: {
        default = self.packages.${pkgs.stdenv.system}.wallpaper-rs;
        wallpaper-rs = pkgs.callPackage ./nix/package.nix { };
      });

      homeManagerModules = {
        default = self.homeManagerModules.wallpaper-rs;
        wallpaper-rs = import ./nix/module.nix { inherit self; };
      };
    };
}
