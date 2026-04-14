A minimal wallpaper daemon for Wayland, written in Rust.

## What it does

Sets a wallpaper image on all connected outputs using the layer shell protocol. Nothing else.

No animations. No effects. No caching.

## Why this exists

On NixOS, my setup is declarative. Window manager, terminal, theme, keybindings — everything lives in configuration. Rebuild the system and the same environment comes back.

Wallpaper didn’t follow that model.

Changing it required:

1. Starting a tool manually or through compositor autostart  
2. Restarting it when the image changed  

Updating the image path in config and rebuilding wasn’t enough. The wallpaper stayed the same until the process was restarted.

After moving to Niri, I couldn’t find a tool that fit cleanly into this workflow.

So this exists to do exactly three things:

- Set a wallpaper on login  
- Be configured declaratively through Home Manager  
- Restart automatically when the image path changes  

Nothing more.

Just a small tool that fits my needs. Happy to share in case it’s useful to others.

## Requirements

- Wayland compositor with layer shell support (Niri, Hyprland, Sway, etc.)  
- `WAYLAND_DISPLAY` set  
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
````

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
image = "/absolute/path/to/wallpaper.png"
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

1. Update the image path
2. Restart:

```bash
systemctl --user restart wallpaper-rs
```

## Configuration

Path:

```text
$XDG_CONFIG_HOME/wallpaper-rs/config.toml
```

Example:

```toml
image = "/path/to/wallpaper.png"
```

* Must be an absolute path
* Supported formats: PNG, JPEG

## How it works

1. Reads the config
2. Connects to Wayland
3. Enumerates outputs
4. Creates a background layer surface per output
5. Renders image to SHM buffers using cover scaling
6. Commits surfaces
7. Enters an event loop

The event loop keeps surfaces alive and handles compositor events.

## Troubleshooting

**WAYLAND_DISPLAY not set**

* Not running inside a Wayland session

**Failed to read config**

* File missing or unreadable

**image must be an absolute path**

* Use `/home/user/...`, not relative paths

**Cannot access image**

* File missing or permissions issue

**Wallpaper doesn’t appear**

* Compositor may not support layer shell
* Another wallpaper process may be running

**Image looks wrong**

* Uses cover scaling: fills screen, crops center, no stretching

## Environment

| Variable        | Required | Default         |
| --------------- | -------- | --------------- |
| WAYLAND_DISPLAY | Yes      | -               |
| XDG_CONFIG_HOME | No       | `$HOME/.config` |

## License

MIT — see LICENSE file.
