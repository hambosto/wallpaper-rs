# wallpaper-rs

A lightweight, daemonless wallpaper tool for Wayland compositors, written in Rust.

## Motivation

After moving to Niri on NixOS, I wanted a wallpaper tool that can be declared in my configuration and applied automatically after a rebuild. With Home Manager, changing the wallpaper in `configuration.nix` and rebuilding should be enough - no manual intervention needed.

The key requirements were:
- **Declarative** - configured via Nix/Home Manager, wallpaper persists across rebuilds
- **Lightweight** - like `swww`
- **Daemonless** - like `hyprpaper`

When the config changes (e.g., new wallpaper image path), the systemd service restarts automatically via `X-Restart-Triggers`, ensuring the new wallpaper is applied right after a NixOS rebuild.

This is a simple wallpaper setter. No fancy features, no daemons running in the background. Just set wallpaper and forget.

## Features

- **Minimal dependencies** - Uses only essential Wayland protocols
- **Daemonless** - Runs once on login, stays alive by processing Wayland events
- **Declarative configuration** - Configure via config file or Nix/Home Manager
- **Multi-monitor support** - Creates a separate layer surface for each connected output
- **Cover scaling** - Image is scaled to cover the entire screen (center-cropped)
- **Memory efficient** - Uses memfd for zero-copy buffer sharing
- **Fast startup** - Minimal initialization, renders directly to SHM buffers

## Requirements
- **Wayland compositor** with:
  - Layer Shell protocol (`zwlr_layer_shell_v1`)
  - Shared Memory protocol (`wl_shm`)
  - Output management (`wl_output`)
- **Wayland session** with `WAYLAND_DISPLAY` set

### Tested Compositors
- Niri

## Supported Image Formats
- PNG
- JPEG/JPG

## Installation

### Nix (Recommended)

Add to your flake inputs:
```nix
{
  inputs = {
    wallpaper-rs.url = "github:hambosto/wallpaper-rs";
  };
}
```

Then use in your Home Manager configuration:
```nix
{ inputs, ... }:

{
  imports = [ inputs.wallpaper-rs.homeManagerModules.default ];

  services.wallpaper-rs = {
    enable = true;
    image = /path/to/your/wallpaper.png;
  };
}
```

This automatically:
- Generates config at `$XDG_CONFIG_HOME/wallpaper-rs/config.toml`
- Starts wallpaper on Wayland login via systemd
- Restarts on failure
- Restarts after NixOS rebuild when the image path changes (via `X-Restart-Triggers`)

### Build from Source

Requirements:
- Rust 1.85+ (edition 2024)
- Cargo

```bash
cargo install --git https://github.com/hambosto/wallpaper-rs
```

Or clone and build:
```bash
git clone https://github.com/hambosto/wallpaper-rs
cd wallpaper-rs
cargo build --release
```

The binary will be at `target/release/wallpaper-rs`.

### Manual Installation

1. Build the binary (see above)
2. Copy to your PATH:
   ```bash
   cp target/release/wallpaper-rs ~/.local/bin/
   ```
3. Create the config directory:
   ```bash
   mkdir -p ~/.config/wallpaper-rs
   ```
4. Create config file (see Configuration section)
5. Create a systemd user service at `~/.config/systemd/user/wallpaper-rs.service`:

   ```
   [Unit]
   ConditionEnvironment=WAYLAND_DISPLAY
   After=graphical-session.target
   PartOf=graphical-session.target

   [Service]
   ExecStart=%h/.local/bin/wallpaper-rs
   Restart=on-failure
   RestartSec=10

   [Install]
   WantedBy=graphical-session.target
   ```

6. Enable and start the service:
   ```bash
   systemctl --user enable --now wallpaper-rs
   ```

   After changing the wallpaper image in config, restart the service:
   ```bash
   systemctl --user restart wallpaper-rs
   ```

## Configuration

The config file is read from `$XDG_CONFIG_HOME/wallpaper-rs/config.toml`. If `$XDG_CONFIG_HOME` is not set, it defaults to `$HOME/.config`.

### Config Options

```toml
# Path to the wallpaper image (required)
# Must be an absolute path
# Supported formats: PNG, JPEG
image = "/absolute/path/to/wallpaper.png"
```

### Example Config

```toml
image = "/home/username/Pictures/wallpaper.png"
```

## Usage

### Automatic (Home Manager)

The wallpaper is automatically set on login via the systemd service.

### Manual

To manually run:
```bash
wallpaper-rs
```

### Troubleshooting

**"WAYLAND_DISPLAY not set" error**
- Ensure you are running in a Wayland session
- Check that `echo $WAYLAND_DISPLAY` returns a value

**"Failed to read config" error**
- Verify the config file exists at `$XDG_CONFIG_HOME/wallpaper-rs/config.toml`
- Check file permissions

**"image must be an absolute path" error**
- The image path in config must be absolute (starting with `/`)
- Relative paths are not supported

**"Cannot access image" error**
- Verify the image file exists
- Check file permissions

**Wallpaper doesn't appear**
- Ensure your compositor supports layer shell protocol
- Check compositor logs for protocol errors
- Verify only one wallpaper daemon is running

**Image appears stretched or cropped**
- This is expected behavior - wallpaper-rs uses "cover" scaling
- The image is scaled to fill the entire screen, with overflow cropped from center

## How It Works

1. Reads config from `$XDG_CONFIG_HOME/wallpaper-rs/config.toml`
2. Connects to Wayland display via `WAYLAND_DISPLAY`
3. Binds required Wayland protocols: compositor, SHM, layer shell, and outputs
4. Creates layer surfaces (background) for each connected monitor
5. Renders the image directly to SHM buffers using memfd
6. Attaches buffers to surfaces and commits
7. Enters blocking event loop to keep surfaces alive

The wallpaper runs once per login and stays alive by processing Wayland events. This ensures the wallpaper persists even if the compositor re-creates surfaces.

## Environment Variables

| Variable | Required | Description |
|----------|----------|-------------|
| `WAYLAND_DISPLAY` | Yes | Wayland display socket name |
| `XDG_CONFIG_HOME` | No | Config directory (default: `$HOME/.config`) |

## Comparison with Other Tools

| Feature | wallpaper-rs | swww | hyprpaper |
|---------|--------------|------|-----------|
| Daemonless | ✓ | ✗ | ✓ |
| Multi-monitor | ✓ | ✓ | ✓ |
| Animations | ✗ | ✓ | ✓ |
| Zero-copy | ✓ | ✓ | ✓ |
| Memory usage | Very low | Low | Low |

## License

MIT
