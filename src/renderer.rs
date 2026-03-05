use std::path::Path;

use anyhow::{Context, Result};
use image::{ImageReader, RgbaImage};

pub struct ImageRenderer {
    src: RgbaImage,
}

impl ImageRenderer {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let src = ImageReader::open(path.as_ref()).context("Open image")?.decode().context("Decode image")?.into_rgba8();
        Ok(Self { src })
    }

    pub fn render(&self, width: u32, height: u32, dst: &mut [u8]) {
        let scale = cover_scale(self.src.width(), self.src.height(), width, height);
        let x_off = centring_offset(self.src.width(), width, scale);
        let y_off = centring_offset(self.src.height(), height, scale);

        for y in 0..height {
            let src_y = src_coord(y, y_off, scale, self.src.height());
            for x in 0..width {
                let src_x = src_coord(x, x_off, scale, self.src.width());
                let rgba = self.src.get_pixel(src_x, src_y).0;
                write_bgra(dst, x, y, width, rgba);
            }
        }
    }
}

fn cover_scale(src_w: u32, src_h: u32, dst_w: u32, dst_h: u32) -> f64 {
    f64::max(dst_w as f64 / src_w as f64, dst_h as f64 / src_h as f64)
}

fn centring_offset(src_dim: u32, dst_dim: u32, scale: f64) -> f64 {
    (src_dim as f64).mul_add(scale, -(dst_dim as f64)) / 2.0
}

fn src_coord(dst: u32, offset: f64, scale: f64, src_dim: u32) -> u32 {
    (((offset + dst as f64) / scale) as u32).min(src_dim - 1)
}

fn write_bgra(dst: &mut [u8], x: u32, y: u32, width: u32, [r, g, b, _]: [u8; 4]) {
    let base = ((y * width + x) * 4) as usize;
    dst[base] = b;
    dst[base + 1] = g;
    dst[base + 2] = r;
    dst[base + 3] = 0xFF;
}
