use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use image::{ImageReader, RgbaImage};

pub struct ImageRenderer {
    path: PathBuf,
}

impl ImageRenderer {
    pub fn open(path: impl AsRef<Path>) -> Self {
        Self { path: path.as_ref().to_path_buf() }
    }

    pub fn render(&self, width: u32, height: u32, dst: &mut [u8]) -> Result<()> {
        let src = load_image(&self.path)?;
        let viewport = Viewport::new(width, height);
        let sampler = CoverSampler::new(src, viewport);
        sampler.rasterize(dst);
        Ok(())
    }
}

#[derive(Clone, Copy)]
struct Viewport {
    width: u32,
    height: u32,
}

impl Viewport {
    fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }
}

struct CoverSampler {
    src: RgbaImage,
    src_xs: Vec<u32>,
    src_ys: Vec<u32>,
}

impl CoverSampler {
    fn new(src: RgbaImage, viewport: Viewport) -> Self {
        let scale = cover_scale(src.width(), src.height(), viewport.width, viewport.height);
        let x_off = centring_offset(src.width(), viewport.width, scale);
        let y_off = centring_offset(src.height(), viewport.height, scale);
        let src_xs = (0..viewport.width).map(|x| src_coord(x, x_off, scale, src.width())).collect();
        let src_ys = (0..viewport.height).map(|y| src_coord(y, y_off, scale, src.height())).collect();

        Self { src, src_xs, src_ys }
    }

    fn rasterize(&self, dst: &mut [u8]) {
        let width = self.src_xs.len() as u32;

        for (y, &src_y) in self.src_ys.iter().enumerate() {
            for (x, &src_x) in self.src_xs.iter().enumerate() {
                let rgba = self.src.get_pixel(src_x, src_y).0;
                write_bgra(dst, x as u32, y as u32, width, rgba);
            }
        }
    }
}

fn load_image(path: &Path) -> Result<RgbaImage> {
    ImageReader::open(path).context("Open image")?.decode().context("Decode image").map(|img| img.into_rgba8())
}

fn cover_scale(src_w: u32, src_h: u32, dst_w: u32, dst_h: u32) -> f64 {
    f64::max(dst_w as f64 / src_w as f64, dst_h as f64 / src_h as f64)
}

fn centring_offset(src_dim: u32, dst_dim: u32, scale: f64) -> f64 {
    ((src_dim as f64 * scale) - dst_dim as f64) / 2.0
}

fn src_coord(dst: u32, offset: f64, scale: f64, src_dim: u32) -> u32 {
    let raw = ((offset + dst as f64) / scale) as u32;
    raw.min(src_dim - 1)
}

fn write_bgra(dst: &mut [u8], x: u32, y: u32, width: u32, [r, g, b, _]: [u8; 4]) {
    let base = ((y * width + x) * 4) as usize;
    dst[base] = b;
    dst[base + 1] = g;
    dst[base + 2] = r;
    dst[base + 3] = 0xFF;
}
