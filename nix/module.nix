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
  configFile = tomlFormat.generate "config.toml" {
    image = toString cfg.image;
  };
in
{
  options.services.wallpaper-rs = {
    enable = lib.mkEnableOption "wallpaper-rs wayland wallpaper setter";

    package = lib.mkPackageOption self.packages.${pkgs.stdenv.system} "wallpaper-rs" { };

    image = lib.mkOption {
      type = lib.types.path;
      description = "Path to the wallpaper image file.";
      example = lib.literalExpression "/home/user/wallpaper.png";
    };
  };

  config = lib.mkIf cfg.enable {
    xdg.configFile."wallpaper-rs/config.toml".source = configFile;

    systemd.user.services.wallpaper-rs = {
      Install.WantedBy = [ config.wayland.systemd.target ];

      Service = {
        ExecStart = lib.getExe cfg.package;
        Restart = "on-failure";
        RestartSec = 10;
      };

      Unit = {
        ConditionEnvironment = "WAYLAND_DISPLAY";
        After = [ config.wayland.systemd.target ];
        PartOf = [ config.wayland.systemd.target ];
        X-Restart-Triggers = [ "${configFile}" ];
      };
    };
  };
}
