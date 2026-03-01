{ self }:
{
  config,
  lib,
  pkgs,
  ...
}:
let
  cfg = config.services.wallpaper-rs;
in
{
  options.services.wallpaper-rs = {
    enable = lib.mkEnableOption "wallpaper-rs wallpaper setter";

    package = lib.mkOption {
      type = lib.types.package;
      default = self.packages.${pkgs.stdenv.system}.default;
      defaultText = lib.literalExpression "pkgs.wallpaper-rs";
      description = "The wallpaper-rs package to use.";
    };

    image = lib.mkOption {
      type = lib.types.path;
      description = "Path to the wallpaper image.";
      example = lib.literalExpression "./image.jpg";
    };
  };

  config = lib.mkIf cfg.enable {
    xdg.configFile."wallpaper-rs/config.toml".text = lib.generators.toTOML { } {
      image = toString cfg.image;
    };

    systemd.user.services.wallpaper-rs = {
      Install = {
        WantedBy = [ config.wayland.systemd.target ];
      };
      Unit = {
        ConditionEnvironment = "WAYLAND_DISPLAY";
        Description = "wallpaper-rs";
        After = [ config.wayland.systemd.target ];
        PartOf = [ config.wayland.systemd.target ];
        X-Restart-Triggers = [
          "${config.xdg.configFile."wallpaper-rs/config.toml".source}"
        ];
      };
      Service = {
        ExecStart = "${lib.getExe cfg.package}";
        Restart = "always";
        RestartSec = "10";
      };
    };
  };
}
