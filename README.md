# wallpaper-rs

A minimal wallpaper daemon for Wayland, written in Rust.

## What it does

Sets a wallpaper image on all connected outputs using the layer shell protocol. That's it. No animations, no effects, no caching daemon.

## Why I built this

On NixOS, my entire setup is declarative. Window manager, terminal, theme, keybindings — everything lives in my configuration.nix. Rebuild the system, and I get the same environment back.

Wallpaper was different. I had to:
1. Start a wallpaper tool manually or via compositor autostart
2. Remember to restart it when I wanted to change the wallpaper

After moving to Niri (which has its own way of handling wallpaper), I couldn't find a tool that fit nicely into my NixOS workflow. Most tools either:
- Run as a daemon with caching (more complexity than I need)
- Are tied to a specific compositor
- Don't integrate well with Home Manager's declarative model

So I wrote something minimal that:
- Just sets an image on login
- Can be configured via Home Manager
- Restarts automatically when the image path changes

That's it. No caching, no animations, no configuration beyond an image path. It works for my NixOS setup, and that's why it exists.

If you need animations or effects, swww or hyprpaper are better choices.

## Requirements

- Wayland compositor with layer shell support (Niri, Hyprland, Sway, etc.)
- `WAYLAND_DISPLAY` environment variable set
- Rust 1.85+

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
    image = /path/to/your/wallpaper.png;
  };
}
```

Home Manager handles:
- Creating the config file at `$XDG_CONFIG_HOME/wallpaper-rs/config.toml`
- Setting up a systemd user service
- Adding `X-Restart-Triggers` so the service restarts when you change the image path

### From source

```bash
cargo build --release
# binary at target/release/wallpaper-rs
```

### Manual setup

1. Copy binary to `~/.local/bin/wallpaper-rs`
2. Create config at `~/.config/wallpaper-rs/config.toml`:
   ```toml
   image = "/absolute/path/to/wallpaper.png"
   ```
3. Create `~/.config/systemd/user/wallpaper-rs.service`:
   ```
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

To change your wallpaper:
1. Edit the image path in the config
2. Restart the service: `systemctl --user restart wallpaper-rs`

## Configuration

Config path: `$XDG_CONFIG_HOME/wallpaper-rs/config.toml` (defaults to `~/.config/wallpaper-rs/config.toml`)

```toml
image = "/path/to/wallpaper.png"
```

- Image path must be absolute
- Supported formats: PNG, JPEG

## How it works

1. Reads the config file
2. Connects to the Wayland display
3. Enumerates all connected outputs
4. Creates a layer surface (background) for each output
5. Renders the image to SHM buffers using cover scaling (fills screen, crops excess from center)
6. Commits the surfaces
7. Enters an event loop

The event loop processes Wayland events, which keeps the surfaces alive. If your compositor recreates the surfaces (which happens in some scenarios), the event loop handles it and the wallpaper stays visible.

## Troubleshooting

**WAYLAND_DISPLAY not set**
- You're not in a Wayland session

**Failed to read config**
- Config file doesn't exist or isn't readable

**image must be an absolute path**
- Use an absolute path like `/home/user/pictures/wallpaper.png`, not a relative one

**Cannot access image**
- File doesn't exist or permissions are wrong

**Wallpaper doesn't appear**
- Your compositor might not support layer shell. Check compositor logs.
- Another wallpaper tool might be running and covering it

**Image looks wrong**
- Cover scaling fills the entire screen by cropping from the center. The image is never stretched or distorted. This is by design.

## Environment variables

| Variable | Required | Default |
|----------|----------|---------|
| `WAYLAND_DISPLAY` | Yes | - |
| `XDG_CONFIG_HOME` | No | `$HOME/.config` |

## License

[MIT](LICENSE) — see the LICENSE file for full text.
