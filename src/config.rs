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

#[serde_inline_default::serde_inline_default]
#[derive(Deserialize, Debug)]
pub struct ResizeConfig {
    #[serde_inline_default(ResizeStrategy::Crop)]
    pub strategy: ResizeStrategy,
    #[serde_inline_default(CropGravity::Center)]
    pub crop_gravity: CropGravity,
    #[serde_inline_default([0x00, 0x00, 0x00, 0xFF])]
    pub fill_color: [u8; 4],
    #[serde_inline_default(Filter::Lanczos3)]
    pub filter: Filter,
}

impl Default for ResizeConfig {
    fn default() -> Self {
        Self {
            strategy: ResizeStrategy::Crop,
            crop_gravity: CropGravity::Center,
            fill_color: [0x00, 0x00, 0x00, 0xFF],
            filter: Filter::Lanczos3,
        }
    }
}

#[serde_inline_default::serde_inline_default]
#[derive(Deserialize, Debug)]
pub struct TransitionConfig {
    #[serde_inline_default(TransitionType::Simple)]
    pub transition_type: TransitionType,
    #[serde_inline_default(3.0_f32)]
    pub duration: f32,
    #[serde_inline_default(30_u16)]
    pub fps: u16,
    #[serde_inline_default(90_u8)]
    pub step: u8,
    #[serde_inline_default(45.0_f64)]
    pub angle: f64,
    #[serde(default)]
    pub pos: Position,
    #[serde_inline_default((0.54_f32, 0.0_f32, 0.34_f32, 0.99_f32))]
    pub bezier: (f32, f32, f32, f32),
    #[serde_inline_default((20.0_f32, 20.0_f32))]
    pub wave: (f32, f32),
    #[serde_inline_default(false)]
    pub invert_y: bool,
}

impl Default for TransitionConfig {
    fn default() -> Self {
        Self {
            transition_type: TransitionType::Simple,
            duration: 3.0,
            fps: 30,
            step: 90,
            angle: 45.0,
            pos: Position::default(),
            bezier: (0.54, 0.0, 0.34, 0.99),
            wave: (20.0, 20.0),
            invert_y: false,
        }
    }
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
    pub fn as_centering(self) -> (f64, f64) {
        match self {
            Self::TopLeft => (0.0, 0.0),
            Self::Top => (0.5, 0.0),
            Self::TopRight => (1.0, 0.0),
            Self::Left => (0.0, 0.5),
            Self::Center => (0.5, 0.5),
            Self::Right => (1.0, 0.5),
            Self::BottomLeft => (0.0, 1.0),
            Self::Bottom => (0.5, 1.0),
            Self::BottomRight => (1.0, 1.0),
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

impl From<Filter> for FilterType {
    fn from(f: Filter) -> Self {
        match f {
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

#[derive(Deserialize, Debug, Clone, Copy)]
#[serde(untagged)]
pub enum Coord {
    Pixel(f32),
    Percent(f32),
}

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
    pub fn to_pixel(self, dim: (u32, u32), invert_y: bool) -> (f32, f32) {
        let x = match self.x {
            Coord::Pixel(v) => v,
            Coord::Percent(v) => v * dim.0 as f32,
        };
        let y = match self.y {
            Coord::Pixel(v) if invert_y => v,
            Coord::Pixel(v) => dim.1 as f32 - v,
            Coord::Percent(v) if invert_y => v * dim.1 as f32,
            Coord::Percent(v) => (1.0 - v) * dim.1 as f32,
        };
        (x, y)
    }
}

impl Config {
    pub fn load() -> Result<Self> {
        let path = Self::path()?;
        let raw = std::fs::read_to_string(&path).context("cannot read config file")?;
        let config: Self = toml::from_str(&raw).context("cannot parse config file")?;
        config.validate()?;
        tracing::info!(
            image        = %config.image.path.display(),
            resize       = ?config.resize.strategy,
            crop_gravity = ?config.resize.crop_gravity,
            transition   = ?config.transition.transition_type,
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
        assert_eq!(tc.transition_type, TransitionType::Simple);
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
        assert_eq!(CropGravity::TopLeft.as_centering(), (0.0, 0.0));
        assert_eq!(CropGravity::Center.as_centering(), (0.5, 0.5));
        assert_eq!(CropGravity::BottomRight.as_centering(), (1.0, 1.0));
    }

    #[test]
    fn position_to_pixel_center() {
        assert_eq!(
            Position::default().to_pixel((1920, 1080), false),
            (960.0, 540.0)
        );
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
