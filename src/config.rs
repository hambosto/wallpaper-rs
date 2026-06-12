use std::path::PathBuf;

use anyhow::{Context, Result};
use fast_image_resize::FilterType;
use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct Config {
    pub image: ImageConfig,
    #[serde(default)]
    pub transition: TransitionConfig,
    #[serde(default)]
    pub resize: ResizeConfig,
}

#[derive(Deserialize, Debug)]
pub struct ImageConfig {
    pub path: PathBuf,
}

#[derive(Deserialize, Debug)]
pub struct ResizeConfig {
    #[serde(default = "default_resize_strategy")]
    pub strategy: ResizeStrategy,
    #[serde(default = "default_crop_gravity")]
    pub crop_gravity: CropGravity,
    #[serde(default = "default_fill_color")]
    pub fill_color: [u8; 4],
    #[serde(default = "default_filter")]
    pub filter: Filter,
}

#[derive(Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ResizeStrategy {
    No,
    Crop,
    Fit,
    Stretch,
}

#[derive(Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CropGravity {
    TopLeft,
    Top,
    TopRight,
    Left,
    Center,
    Right,
    BottomLeft,
    Bottom,
    BottomRight,
}

impl CropGravity {
    pub fn as_centering_tuple(self) -> (f64, f64) {
        match self {
            CropGravity::TopLeft => (0.0, 0.0),
            CropGravity::Top => (0.5, 0.0),
            CropGravity::TopRight => (1.0, 0.0),
            CropGravity::Left => (0.0, 0.5),
            CropGravity::Center => (0.5, 0.5),
            CropGravity::Right => (1.0, 0.5),
            CropGravity::BottomLeft => (0.0, 1.0),
            CropGravity::Bottom => (0.5, 1.0),
            CropGravity::BottomRight => (1.0, 1.0),
        }
    }
}

#[derive(Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Filter {
    Nearest,
    Bilinear,
    CatmullRom,
    Mitchell,
    Lanczos3,
}

impl Filter {
    pub fn to_fast_image_resize(self) -> FilterType {
        match self {
            Filter::Nearest => FilterType::Box,
            Filter::Bilinear => FilterType::Bilinear,
            Filter::CatmullRom => FilterType::CatmullRom,
            Filter::Mitchell => FilterType::Mitchell,
            Filter::Lanczos3 => FilterType::Lanczos3,
        }
    }
}

#[derive(Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TransitionType {
    None,
    Simple,
    Fade,
    Grow,
    Outer,
    Wipe,
    Wave,
}

/// Pixel coordinate, either absolute or relative.
#[derive(Deserialize, Debug, Clone, Copy)]
#[serde(untagged)]
pub enum Coord {
    /// Absolute pixel value.
    Pixel(f32),
    /// Relative value in [0.0, 1.0] range.
    Percent(f32),
}

/// A 2D position used for transition origin (Grow/Outer center point).
#[derive(Deserialize, Debug, Clone, Copy)]
pub struct Position {
    pub x: Coord,
    pub y: Coord,
}

impl Default for Position {
    fn default() -> Self {
        Self {
            x: Coord::Percent(0.5),
            y: Coord::Percent(0.5),
        }
    }
}

impl Position {
    /// Convert to pixel coordinates. `invert_y` controls Y-axis direction.
    pub fn to_pixel(&self, dim: (u32, u32), invert_y: bool) -> (f32, f32) {
        let x = match self.x {
            Coord::Pixel(x) => x,
            Coord::Percent(x) => x * dim.0 as f32,
        };
        let y = match self.y {
            Coord::Pixel(y) => {
                if invert_y {
                    y
                } else {
                    dim.1 as f32 - y
                }
            }
            Coord::Percent(y) => {
                if invert_y {
                    y * dim.1 as f32
                } else {
                    (1.0 - y) * dim.1 as f32
                }
            }
        };
        (x, y)
    }
}

#[derive(Deserialize, Debug)]
pub struct TransitionConfig {
    #[serde(default = "default_transition_type")]
    pub r#type: TransitionType,
    #[serde(default = "default_duration")]
    pub duration: f32,
    #[serde(default = "default_fps")]
    pub fps: u16,
    #[serde(default = "default_step")]
    pub step: u8,
    #[serde(default = "default_angle")]
    pub angle: f64,
    #[serde(default)]
    pub pos: Position,
    #[serde(default = "default_bezier")]
    pub bezier: (f32, f32, f32, f32),
    #[serde(default = "default_wave")]
    pub wave: (f32, f32),
    #[serde(default)]
    pub invert_y: bool,
}

fn default_resize_strategy() -> ResizeStrategy {
    ResizeStrategy::Crop
}
fn default_crop_gravity() -> CropGravity {
    CropGravity::Center
}
fn default_fill_color() -> [u8; 4] {
    [0x00, 0x00, 0x00, 0xFF]
}
fn default_filter() -> Filter {
    Filter::Lanczos3
}

fn default_transition_type() -> TransitionType {
    TransitionType::Simple
}
fn default_duration() -> f32 {
    3.0
}
fn default_fps() -> u16 {
    30
}
fn default_step() -> u8 {
    90
}
fn default_angle() -> f64 {
    45.0
}
fn default_bezier() -> (f32, f32, f32, f32) {
    (0.54, 0.0, 0.34, 0.99)
}
fn default_wave() -> (f32, f32) {
    (20.0, 20.0)
}

impl Default for ResizeConfig {
    fn default() -> Self {
        Self {
            strategy: default_resize_strategy(),
            crop_gravity: default_crop_gravity(),
            fill_color: default_fill_color(),
            filter: default_filter(),
        }
    }
}

impl Default for TransitionConfig {
    fn default() -> Self {
        Self {
            r#type: default_transition_type(),
            duration: default_duration(),
            fps: default_fps(),
            step: default_step(),
            angle: default_angle(),
            pos: Position::default(),
            bezier: default_bezier(),
            wave: default_wave(),
            invert_y: false,
        }
    }
}

impl Config {
    pub fn load() -> Result<Self> {
        let path = Self::path()?;
        let raw = std::fs::read_to_string(&path).context("cannot read config file")?;
        let config: Self = toml::from_str(&raw).context("cannot parse config file")?;
        config.validate()?;
        tracing::info!(
            image = %config.image.path.display(),
            resize = ?config.resize.strategy,
            crop_gravity = ?config.resize.crop_gravity,
            transition = ?config.transition.r#type,
            "config loaded"
        );

        Ok(config)
    }

    fn path() -> Result<PathBuf> {
        let base = std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
            .context("neither $XDG_CONFIG_HOME nor $HOME is set")?;

        Ok(base.join("wallpaper-rs").join("config.toml"))
    }

    fn validate(&self) -> Result<()> {
        if !self.image.path.is_absolute() {
            anyhow::bail!(
                "image path must be absolute, got: {}",
                self.image.path.display()
            );
        }

        let meta = std::fs::metadata(&self.image.path)
            .with_context(|| format!("cannot access image: {}", self.image.path.display()))?;
        if !meta.is_file() {
            anyhow::bail!("{} is not a regular file", self.image.path.display());
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_rejects_relative_path() {
        let config = Config {
            image: ImageConfig {
                path: PathBuf::from("relative/wallpaper.png"),
            },
            transition: TransitionConfig::default(),
            resize: ResizeConfig::default(),
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn validate_rejects_nonexistent_file() {
        let config = Config {
            image: ImageConfig {
                path: PathBuf::from("/nonexistent/wallpaper.png"),
            },
            transition: TransitionConfig::default(),
            resize: ResizeConfig::default(),
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn validate_accepts_existing_file() {
        let config = Config {
            image: ImageConfig {
                path: PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml"),
            },
            transition: TransitionConfig::default(),
            resize: ResizeConfig::default(),
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn default_transition_config() {
        let tc = TransitionConfig::default();
        assert_eq!(tc.r#type, TransitionType::Simple);
        assert_eq!(tc.duration, 3.0);
        assert_eq!(tc.fps, 30);
        assert_eq!(tc.step, 90);
        assert!(!tc.invert_y);
    }

    #[test]
    fn default_resize_config() {
        let rc = ResizeConfig::default();
        assert_eq!(rc.strategy, ResizeStrategy::Crop);
        assert_eq!(rc.crop_gravity, CropGravity::Center);
        assert_eq!(rc.fill_color, [0x00, 0x00, 0x00, 0xFF]);
    }

    #[test]
    fn crop_gravity_centering() {
        assert_eq!(CropGravity::TopLeft.as_centering_tuple(), (0.0, 0.0));
        assert_eq!(CropGravity::Center.as_centering_tuple(), (0.5, 0.5));
        assert_eq!(CropGravity::BottomRight.as_centering_tuple(), (1.0, 1.0));
    }

    #[test]
    fn position_to_pixel_center() {
        let pos = Position::default();
        assert_eq!(pos.to_pixel((1920, 1080), false), (960.0, 540.0));
    }

    #[test]
    fn position_to_pixel_top_left() {
        let pos = Position {
            x: Coord::Percent(0.0),
            y: Coord::Percent(1.0),
        };
        assert_eq!(pos.to_pixel((1920, 1080), false), (0.0, 0.0));
    }

    #[test]
    fn position_to_pixel_invert_y() {
        let pos = Position {
            x: Coord::Percent(0.5),
            y: Coord::Percent(0.0),
        };
        assert_eq!(pos.to_pixel((1920, 1080), true), (960.0, 0.0));
    }
}
