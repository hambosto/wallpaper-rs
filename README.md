A minimal wallpaper daemon for Wayland, written in Rust.

## What it does

Sets a wallpaper image on all connected outputs using the layer shell protocol, with optional animated transitions between wallpapers.

## Features

- Sets wallpaper on login
- Animated transitions (fade, grow, outer, wipe, wave)
- Multiple resize strategies (crop, fit, stretch)
- Declarative configuration via Home Manager
- Automatic restarts on config changes
- Multi-monitor support

## Why this exists

On NixOS, my setup is declarative. Window manager, terminal, theme, keybindings — everything lives in configuration. Rebuild the system and the same environment comes back.

Wallpaper didn't follow that model.

After moving to Niri, I couldn't find a tool that fit cleanly into this workflow.

So this exists to do exactly three things:

- Set a wallpaper on login
- Be configured declaratively through Home Manager
- Restart automatically when the image path changes

Just a small tool that fits my needs. Happy to share in case it's useful to others.

## Requirements

- Wayland compositor with layer shell support (Niri, Hyprland, Sway, etc.)
- `WAYLAND_DISPLAY` set
- Rust 2024 edition (1.85+)

## Installation

### Nix (recommended)

Add to your flake inputs:

```nix
{
  inputs = {
    wallpaper-rs.url = "github:hambosto/wallpaper-rs";
  };
}
```

Use with Home Manager:

```nix
{
  imports = [ inputs.wallpaper-rs.homeManagerModules.default ];

  services.wallpaper-rs = {
    enable = true;
    settings = {
      image.path = "~/wallpapers/wallpaper.png";

      transition = {
        transition_type = "fade";
        duration = 3.0;
        fps = 30;
      };

      resize = {
        strategy = "crop";
        crop_gravity = "center";
        filter = "lanczos3";
      };
    };
  };
}
```

Home Manager handles:

* Config file at `$XDG_CONFIG_HOME/wallpaper-rs/config.toml`
* systemd user service
* `X-Restart-Triggers` for automatic restarts on config changes

### From source

```bash
cargo build --release
```

Binary:

```bash
target/release/wallpaper-rs
```

### Manual setup

1. Copy binary:

```bash
cp target/release/wallpaper-rs ~/.local/bin/
```

2. Create config:

```toml
# ~/.config/wallpaper-rs/config.toml

[image]
path = "/absolute/path/to/wallpaper.png"

[transition]
transition_type = "fade"
duration = 3.0
fps = 30

[resize]
strategy = "crop"
crop_gravity = "center"
filter = "lanczos3"
```

3. Create systemd service:

```ini
# ~/.config/systemd/user/wallpaper-rs.service
[Unit]
ConditionEnvironment=WAYLAND_DISPLAY
After=graphical-session.target

[Service]
ExecStart=%h/.local/bin/wallpaper-rs
Restart=on-failure
RestartSec=10

[Install]
WantedBy=graphical-session.target
```

4. Enable and start:

```bash
systemctl --user enable --now wallpaper-rs
```

To change wallpaper:

1. Update the image path in config
2. Restart:

```bash
systemctl --user restart wallpaper-rs
```

## Configuration

Path:

```text
$XDG_CONFIG_HOME/wallpaper-rs/config.toml
```

All sections except `[image]` are optional. Omitted fields use their defaults.

### `[image]` (required)

```toml
[image]
path = "/absolute/path/to/wallpaper.png"
```

| Option | Type | Required | Description |
|--------|------|----------|-------------|
| `path` | string (path) | Yes | Absolute path to the wallpaper image |

Supported formats: PNG, JPEG, WebP

---

### `[transition]` (optional)

Controls the visual effect when switching wallpapers. The entire section defaults to sensible values.

```toml
[transition]
transition_type = "fade"
duration = 3.0
fps = 30
step = 90
angle = 45.0
bezier = [0.54, 0.0, 0.34, 0.99]
wave = [20.0, 20.0]
invert_y = false
```

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `transition_type` | string | `simple` | Transition effect (see below) |
| `duration` | float | `3.0` | Duration of the transition in seconds |
| `fps` | integer | `30` | Target frames per second for the animation |
| `step` | integer | `90` | Maximum pixel change per frame during convergence |
| `angle` | float | `45.0` | Angle in degrees for `wipe` and `wave` effects |
| `bezier` | array of 4 floats | `[0.54, 0.0, 0.34, 0.99]` | Cubic bezier control points `(x1, y1, x2, y2)` for easing |
| `wave` | array of 2 floats | `[20.0, 20.0]` | Wave frequency (x) and amplitude (y) for `wave` effect |
| `invert_y` | bool | `false` | Invert the Y axis for the `pos` coordinate system |

**`transition_type` values:**

| Value | Effect |
|-------|--------|
| `none` | Instant cut, no animation |
| `simple` | Per-pixel convergence toward target |
| `fade` | Alpha blend from old to new |
| `grow` | Circle expands outward from `pos` |
| `outer` | Circle contracts inward toward `pos` |
| `wipe` | Linear sweep across the screen at `angle` |
| `wave` | Sine-wave modulated sweep at `angle` |

---

### `[transition.pos]` (optional)

Sets the origin point for `grow` and `outer` transitions. Can use pixels or percentages.

```toml
[transition.pos]
x = 0.5
y = 0.5
```

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `x` | float | `0.5` | Horizontal position (0.0–1.0 = percentage, or pixel value) |
| `y` | float | `0.5` | Vertical position (0.0–1.0 = percentage, or pixel value) |

- Values between 0.0 and 1.0 are treated as percentages (0.5 = center)
- Values outside 0.0–1.0 are treated as pixel coordinates
- Default `(0.5, 0.5)` = center of the screen

---

### `[resize]` (optional)

Controls how the image is resized to fit each output.

```toml
[resize]
strategy = "crop"
crop_gravity = "center"
fill_color = [0, 0, 0, 255]
filter = "lanczos3"
```

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `strategy` | string | `crop` | Resize strategy (see below) |
| `crop_gravity` | string | `center` | Anchor point when cropping (see below) |
| `fill_color` | array of 4 integers | `[0, 0, 0, 255]` | RGBA fill color for padding/letterboxing (0–255 each) |
| `filter` | string | `lanczos3` | Resampling filter algorithm (see below) |

**`strategy` values:**

| Value | Behavior |
|-------|----------|
| `no` | No resizing. Places the image centered on a canvas filled with `fill_color`. |
| `crop` | Resizes to fill the output, cropping overflow. Uses `crop_gravity` for alignment. |
| `fit` | Resizes to fit inside the output, letterboxing with `fill_color`. |
| `stretch` | Stretches to exact output dimensions (may distort aspect ratio). |

**`crop_gravity` values:**

| Value | Anchor |
|-------|--------|
| `top-left` | Top-left corner |
| `top` | Top-center |
| `top-right` | Top-right corner |
| `left` | Left-center |
| `center` | Center |
| `right` | Right-center |
| `bottom-left` | Bottom-left corner |
| `bottom` | Bottom-center |
| `bottom-right` | Bottom-right corner |

**`filter` values:**

| Value | Quality | Speed |
|-------|---------|-------|
| `nearest` | Lowest (box filter) | Fastest |
| `bilinear` | Low | Fast |
| `catmull-rom` | Medium | Moderate |
| `mitchell` | Medium-high | Moderate |
| `lanczos3` | Highest | Slowest |

---

### Full example

```toml
[image]
path = "/home/user/wallpapers/mountains.jpg"

[transition]
transition_type = "wave"
duration = 5.0
fps = 60
step = 120
angle = 30.0
bezier = [0.25, 0.1, 0.25, 1.0]
wave = [15.0, 25.0]
invert_y = false

[transition.pos]
x = 0.5
y = 0.5

[resize]
strategy = "crop"
crop_gravity = "center"
fill_color = [0, 0, 0, 255]
filter = "lanczos3"
```

### Minimal example

Only `[image]` is required. Everything else has defaults.

```toml
[image]
path = "/home/user/wallpapers/wallpaper.png"
```

## How it works

1. Reads the config
2. Connects to Wayland
3. Enumerates outputs
4. Creates a background layer surface per output
5. Renders image to SHM buffers with configured resize strategy
6. Commits surfaces (with transition from black on first load)
7. Enters event loop driven by calloop timers

## Troubleshooting

**WAYLAND_DISPLAY not set**

* Not running inside a Wayland session

**Failed to read config**

* File missing or unreadable

**path does not exist**

* Image path in config points to a non-existent file

**path is not a file**

* Image path points to a directory instead of a file

**Failed to detect image format**

* File is not a supported image format (PNG, JPEG, WebP)

**Wallpaper doesn't appear**

* Compositor may not support layer shell
* Another wallpaper process may be running

## Environment

| Variable | Required | Default |
| --- | --- | --- |
| WAYLAND_DISPLAY | Yes | - |
| XDG_CONFIG_HOME | No | `$HOME/.config` |

## License

[MIT](LICENSE)
