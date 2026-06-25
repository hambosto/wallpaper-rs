{
  config,
  lib,
  pkgs,
  ...
}:
let
  cfg = config.services.wallpaper-rs;
  tomlFormat = pkgs.formats.toml { };
in
{
  options.services.wallpaper-rs = {
    enable = lib.mkEnableOption "Whether to enable wallpaper-rs";

    package = lib.mkOption {
      type = lib.types.nullOr lib.types.package;
      description = "The wallpaper-rs package to use.";
    };

    settings = lib.mkOption {
      inherit (tomlFormat) type;
      default = { };
      example = lib.literalExpression ''
        {
          image.path = "~/wallpapers/wallpaper.png";
        }
      '';
      description = ''
        Configuration written to
        {file}`$XDG_CONFIG_HOME/wallpaper-rs/config.toml`.
        See <https://github.com/hambosto/wallpaper-rs>
        for the full list of options.
      '';
    };
  };

  config = lib.mkIf cfg.enable {

    xdg.configFile = {
      "wallpaper-rs/config.toml" = lib.mkIf (cfg.settings != { }) {
        source = tomlFormat.generate "wallpaper-rs-config.toml" cfg.settings;
      };
    };

    systemd.user.services.wallpaper-rs = lib.mkIf (cfg.package != null) {
      Install.WantedBy = [ config.wayland.systemd.target ];

      Service = {
        ExecStart = lib.getExe cfg.package;
        Restart = "on-failure";
      };

      Unit = {
        After = [ config.wayland.systemd.target ];
        ConditionEnvironment = "WAYLAND_DISPLAY";
        Description = "A minimal wallpaper daemon for Wayland, written in Rust.";
        Documentation = "https://github.com/hambosto/wallpaper-rs";
        PartOf = [ config.wayland.systemd.target ];
        X-Restart-Triggers = lib.mkIf (cfg.settings != { }) [
          "${config.xdg.configFile."wallpaper-rs/config.toml".source}"
        ];
      };
    };
  };
}
