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
      example = lib.literalExpression ''
        {
          image.path = "~/wallpapers/wallpaper.png";

          transition = {
            transition_type = "fade";
            duration = 3.0;
            fps = 30;
            step = 90;
            angle = 45.0;
            bezier = [ 0.54 0.0 0.34 0.99 ];
            wave = [ 20.0 20.0 ];
            invert_y = false;
          };

          resize = {
            strategy = "crop";
            crop_gravity = "center";
            fill_color = [ 0 0 0 255 ];
            filter = "lanczos3";
          };
        }
      '';
      description = ''
        Configuration for wallpaper-rs, written as a TOML attribute set.

        **image** (required):
        - `path`: Path to the wallpaper image file.

        **transition** (optional, defaults shown):
        - `transition_type`: Visual effect — `"none"`, `"simple"`, `"fade"`, `"grow"`, `"outer"`, `"wipe"`, `"wave"`.
        - `duration`: Transition duration in seconds (default: `3.0`).
        - `fps`: Target frames per second (default: `30`).
        - `step`: Max pixel change per frame for convergence (default: `90`).
        - `angle`: Angle in degrees for wipe/wave effects (default: `45.0`).
        - `bezier`: Cubic bezier control points for easing (default: `[ 0.54 0.0 0.34 0.99 ]`).
        - `wave`: Wave frequency and amplitude (default: `[ 20.0 20.0 ]`).
        - `invert_y`: Invert Y axis for position calculations (default: `false`).

        **resize** (optional, defaults shown):
        - `strategy`: How to fit the image — `"no"`, `"crop"`, `"fit"`, `"stretch"`.
        - `crop_gravity`: Crop anchor — `"top-left"`, `"top"`, `"top-right"`, `"left"`, `"center"`, `"right"`, `"bottom-left"`, `"bottom"`, `"bottom-right"`.
        - `fill_color`: RGBA fill color for letterboxing (default: `[ 0 0 0 255 ]`).
        - `filter`: Resampling filter — `"nearest"`, `"bilinear"`, `"catmull-rom"`, `"mitchell"`, `"lanczos3"`.

        See <https://github.com/hambosto/wallpaper-rs> for more details.
      '';
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
