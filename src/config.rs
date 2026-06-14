use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use fast_image_resize::FilterType;
use serde::Deserialize;
use smart_default::SmartDefault;

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

#[derive(SmartDefault, Deserialize, Debug)]
#[serde(default)]
pub struct ResizeConfig {
    #[default(_code = "ResizeStrategy::Crop")]
    pub strategy: ResizeStrategy,
    #[default(_code = "CropGravity::Center")]
    pub crop_gravity: CropGravity,
    #[default(_code = "[0x00, 0x00, 0x00, 0xFF]")]
    pub fill_color: [u8; 4],
    #[default(_code = "Filter::Lanczos3")]
    pub filter: Filter,
}

#[derive(SmartDefault, Deserialize, Debug)]
#[serde(default)]
pub struct TransitionConfig {
    #[default(_code = "TransitionType::Simple")]
    pub transition_type: TransitionType,
    #[default = 3.0]
    pub duration: f32,
    #[default = 30]
    pub fps: u16,
    #[default = 90]
    pub step: u8,
    #[default = 45.0]
    pub angle: f64,
    #[default(_code = "Position::default()")]
    pub pos: Position,
    #[default(_code = "(0.54, 0.0, 0.34, 0.99)")]
    pub bezier: (f32, f32, f32, f32),
    #[default(_code = "(20.0, 20.0)")]
    pub wave: (f32, f32),
    #[default = false]
    pub invert_y: bool,
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

#[derive(SmartDefault, Deserialize, Debug, Clone, Copy)]
pub struct Position {
    #[default(_code = "Coord::Percent(0.5)")]
    pub x: Coord,
    #[default(_code = "Coord::Percent(0.5)")]
    pub y: Coord,
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
    pub fn load_from_file(path: &Path) -> Result<Self> {
        let read_config = std::fs::read_to_string(path).context("cannot read from config file")?;
        let parse_config: Self = toml::from_str(&read_config).context("cannot parse config file")?;

        tracing::info!(
            image = %parse_config.image.path.display(),
            resize = ?parse_config.resize.strategy,
            crop_gravity = ?parse_config.resize.crop_gravity,
            transition = ?parse_config.transition.transition_type,
            "config loaded"
        );

        Ok(parse_config)
    }
}
