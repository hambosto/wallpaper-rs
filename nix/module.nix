{ self }:
{
  config,
  lib,
  pkgs,
  ...
}:

let
  cfg = config.services.wallpaper-rs;
  tomlFormat = pkgs.formats.toml { };
  configFile = tomlFormat.generate "config.toml" cfg.settings;
in
{
  options.services.wallpaper-rs = {
    enable = lib.mkEnableOption "A minimal wallpaper daemon for Wayland, written in Rust.";

    package = lib.mkPackageOption self.packages.${pkgs.stdenv.system} "wallpaper-rs" { };

    settings = lib.mkOption {
      inherit (tomlFormat) type;
      default = { };
      description = "Configuration for wallpaper-rs (TOML)";
    };
  };

  config = lib.mkIf cfg.enable {
    xdg.configFile."wallpaper-rs/config.toml".source = configFile;

    systemd.user.services.wallpaper-rs = {
      Unit = {
        Description = "A minimal wallpaper daemon for Wayland, written in Rust.";
        After = [ config.wayland.systemd.target ];
        PartOf = [ config.wayland.systemd.target ];
        X-Restart-Triggers = [ configFile ];
      };

      Service = {
        ExecStart = lib.getExe cfg.package;
        Restart = "on-failure";
        RestartSec = 10;
        ConditionEnvironment = "WAYLAND_DISPLAY";
      };

      Install = {
        WantedBy = [ config.wayland.systemd.target ];
      };
    };
  };
}
