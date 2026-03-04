{
  description = "A minimal wallpaper daemon for Wayland, written in Rust.";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    systems.url = "github:nix-systems/default-linux";
  };

  outputs =
    {
      self,
      nixpkgs,
      systems,
    }:
    let
      inherit (nixpkgs) lib;
      eachSystem = lib.genAttrs (import systems);
      pkgsFor = system: nixpkgs.legacyPackages.${system};
    in
    {
      formatter = eachSystem (system: (pkgsFor system).alejandra);

      devShells = eachSystem (system: {
        default = (pkgsFor system).callPackage ./nix/shell.nix {
          inherit (self.packages.${system}) wallpaper-rs;
        };
      });

      packages = eachSystem (system: {
        wallpaper-rs = (pkgsFor system).callPackage ./nix/package.nix { };
        default = self.packages.${system}.wallpaper-rs;
      });

      homeManagerModules = {
        wallpaper-rs = import ./nix/module.nix { inherit self; };
        default = self.homeManagerModules.wallpaper-rs;
      };
    };
}
